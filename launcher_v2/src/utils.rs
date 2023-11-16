/*
 * utils.rs, part of VCMI engine
 * Helper functions used across several modules, including:
 * - json save & load
 * - checking homm data dir
 * - async task managament
 * - file download
 * - static IndexMap
 *
 * Authors: listed in file AUTHORS in main folder
 *
 * License: GNU General Public License v2.0 or later
 * Full text of license available in license.txt file, in main folder
 *
 */

use anyhow::{bail, Context};
use egui_toast::Toast;
use parking_lot::RwLock;
use reqwest::{Client, IntoUrl};
use rust_i18n::t;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Display;
use std::future::Future;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use crate::platform::{VDirs, VDIRS};
use crate::settings::{AtomicLanguage, ExtraRepository};

pub static LANGUAGE: AtomicLanguage = AtomicLanguage::new();
pub static MOBILE_VIEW: AtomicBool = AtomicBool::new(false);
pub static EXTRA_REPO: RwLock<ExtraRepository> =
    RwLock::new(ExtraRepository::new(false, String::new()));

pub mod hash_helper {
    pub type IndexMap<Q, V> = indexmap::IndexMap<Q, V, ahash::RandomState>;
    const fn hasher() -> ahash::RandomState {
        ahash::RandomState::with_seeds(
            0x31ea130313257450,
            0xc5be849988ac2d14,
            0x37a78a23bab3ca5f,
            0xd94bfa9705d99e8d,
        )
    }
    pub const fn hashmap<Q, V>() -> IndexMap<Q, V> {
        IndexMap::with_hasher(hasher())
    }
}
pub use hash_helper::*;
/////////////////////////////////////////////////////////////////
///////////////////Async task handle management//////////////////
/////////////////////////////////////////////////////////////////

lazy_static::lazy_static! {
    pub static ref RUNTIME: Runtime =  Runtime::new().unwrap();
}

#[derive(Debug, Default)]
pub enum AsyncHandle<T: Send, P> {
    #[default]
    Uninit, // or Aborted
    Running(JoinHandle<anyhow::Result<T>>, Arc<P>),
    Finished(anyhow::Result<T>),
}
use AsyncHandle::*;

impl<T: Send + 'static, P> AsyncHandle<T, P> {
    pub fn run<F>(&mut self, progress: Arc<P>, future: F)
    where
        F: Future<Output = anyhow::Result<T>> + Send + 'static,
    {
        if !matches!(self, Running(_, _)) {
            *self = Running(RUNTIME.spawn(future), progress);
        }
    }
    pub fn fetch_handle(&mut self) {
        if let Running(handle, _) = self {
            if handle.is_finished() {
                *self = Finished(
                    RUNTIME
                        .block_on(handle)
                        .unwrap_or(Err(anyhow::anyhow!("Failed to join task!"))),
                )
            }
        }
    }
    pub fn is_running(&mut self) -> bool {
        self.fetch_handle();
        matches!(self, Running(_, _))
    }
    pub fn is_success(&mut self) -> bool {
        self.fetch_handle();
        matches!(self, Finished(Ok(_)))
    }
    pub fn if_running(&mut self, op: &mut dyn FnMut(Arc<P>)) -> bool {
        self.fetch_handle();
        if let Running(_, progress) = self {
            op(progress.clone());
            return true;
        }
        false
    }
    pub fn if_success(&mut self, op: &mut dyn FnMut(&mut T)) -> bool {
        self.fetch_handle();
        if let Finished(Ok(result)) = self {
            op(result);
            return true;
        }
        false
    }
    pub fn if_state<R>(
        &mut self,
        running_op: &mut dyn FnMut(Arc<P>) -> R,
        success_op: &mut dyn FnMut(&mut T) -> R,
        failed_op: &mut dyn FnMut() -> R,
        uninit: &mut dyn FnMut() -> R,
    ) -> R {
        self.fetch_handle();
        match self {
            Finished(Ok(result)) => success_op(result),
            Finished(Err(_)) => failed_op(),
            Uninit => uninit(),
            Running(_, progress) => running_op(progress.clone()),
        }
    }
}

/////////////////////////////////////////////////////////////////
/////////////////////////Json file helpers///////////////////////
/////////////////////////////////////////////////////////////////

pub fn hjson_deser<T: DeserializeOwned>(reader: impl Read) -> anyhow::Result<T> {
    let json = nu_json::from_reader::<_, serde_json::Value>(reader);
    let json = json.context("File is not valid HJSON")?;
    Ok(serde_json::from_value(json).context("File has wrong structure")?)
}
macro_rules! gen_fn_load_file_settings {
    ($name:ident, $toast_deser:literal, $toast_open:literal) => {
        pub fn $name<T: DeserializeOwned + Default>(path: &Path) -> T {
            match std::fs::File::open(path) {
                Ok(file) => match hjson_deser(file) {
                    Err(err) => {
                        Toast::error(t!($toast_deser));
                        log::error!(
                            "Deserialization from file: {} failed!; Error: {:#}",
                            path.display(),
                            err
                        );
                        Default::default()
                    }
                    Ok(loaded) => {
                        log::info!("Deserialization from file: {} finished!", path.display(),);
                        loaded
                    }
                },
                Err(err) => match err.kind() {
                    std::io::ErrorKind::NotFound => Default::default(),
                    _ => {
                        Toast::error(t!($toast_open));
                        log::error!("Open file: {} failed!; Error: {}", path.display(), err);
                        Default::default()
                    }
                },
            }
        }
    };
}
gen_fn_load_file_settings! {load_file_settings,"toasts.error.Settings file corrupted!","toasts.error.Failed to open settings file!"}
gen_fn_load_file_settings! {load_file_mod,"toasts.error.Mod file corrupted!","toasts.error.Failed to open mod file!"}

pub fn save_file<T: ?Sized + Serialize>(path: &Path, data: &T) {
    match std::fs::File::create(&path) {
        Ok(file) => {
            if let Err(err) = serde_json::to_writer_pretty(file, data) {
                Toast::error(t!("toasts.error.Settings save failed!"));
                log::error!(
                    "Serialization to file: {} failed!; Error: {}",
                    path.display(),
                    err
                )
            }
        }
        Err(err) => {
            Toast::error(t!("toasts.error.Settings save failed!"));
            log::error!(
                "Open file: {} for writing failed!; Error: {}",
                path.display(),
                err
            )
        }
    }
}
pub fn get_dirs() -> &'static VDirs {
    VDIRS.get().unwrap()
}
pub fn check_data_dir_valid(dir: &Path) -> anyhow::Result<()> {
    if !dir.is_dir() || !dir.exists() {
        bail!("Invalid path")
    }
    let (mut data, mut mp3, mut maps) = Default::default();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.file_name().eq_ignore_ascii_case("data") {
                    data = Some(entry.path());
                }
                if entry.file_name().eq_ignore_ascii_case("maps") {
                    maps = Some(entry.path());
                }
                if entry.file_name().eq_ignore_ascii_case("mp3") {
                    mp3 = Some(entry.path());
                }
            }
        }
    } else {
        bail!("Unable to read dir")
    }
    if data == None || maps == None || mp3 == None {
        bail!(
            "Folder does not contain required subdirs: data: {:?}, maps: {:?}, mp3: {:?}",
            data,
            maps,
            mp3
        )
    }
    let lod = data.unwrap().join("H3bitmap.lod");
    if !lod.exists() {
        bail!("Folder does not contain H3bitmap.lod file")
    }
    //TODO ? more complex check
    Ok(())
}
/////////////////////////////////////////////////////////////////
//////////////////////////Download helpers///////////////////////
/////////////////////////////////////////////////////////////////

lazy_static::lazy_static! {
    pub static ref REQWEST:Client=  Client::new();
}

pub async fn get_file_from_url<U: IntoUrl + Display, T: DeserializeOwned>(
    url: U,
    toast: &str,
) -> anyhow::Result<T> {
    async {
        let urls = url.to_string();
        let download_err = format!("Unable to download file from: {}!", urls);

        let responce = REQWEST.get(url).send().await;
        let responce = responce.context(download_err.clone())?;

        let downloaded = responce.bytes().await.context(download_err)?;

        let deser = hjson_deser::<T>(&*downloaded);
        deser.context(format!("Unable to parse file from: {}!", urls))
    }
    .await
    .map_err(|err| {
        Toast::error(toast);
        log::error!("{:#}", err);
        err
    })
}
