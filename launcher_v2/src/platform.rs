/*
 * platform.rs, part of VCMI engine
 * Platform specific code (start game/mapEditor, data directories)
 *
 * Authors: listed in file AUTHORS in main folder
 *
 * License: GNU General Public License v2.0 or later
 * Full text of license available in license.txt file, in main folder
 *
 */
use std::path::{Path, PathBuf};
use std::process::Command;

use egui_toast::Toast;
use rust_i18n::t;
#[cfg(target_os = "ios")]
use std::ffi::c_char;
#[cfg(target_os = "ios")]
use std::ffi::c_int;
use std::sync::OnceLock;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

use crate::vcmi_launcher::{TabName, VCMILauncher};

#[cfg(target_os = "android")]
#[derive(Clone)]
pub struct NativeParams(pub AndroidApp);

#[cfg(not(target_os = "android"))]
#[derive(Clone)]
pub struct NativeParams();

pub static VDIRS: OnceLock<VDirs> = OnceLock::new();

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct VDirs {
    pub internal: PathBuf,
    pub user_cache: PathBuf,
    pub user_config: PathBuf,
    pub user_data: PathBuf,
    pub log: PathBuf,

    pub internal_mods: PathBuf,
    pub settings: PathBuf,
    pub settings_mod: PathBuf,
    pub downloads: PathBuf,
    pub mods: PathBuf,
}

impl VDirs {
    pub fn init(_native: NativeParams) {
        let _development_mode: bool = Path::new("vcmiserver").exists()
            && Path::new("vcmiclient").exists()
            && Path::new("Mods").exists()
            && Path::new("config").exists()
            && Path::new("AI").exists();
        #[cfg(target_os = "windows")]
        {
            let user_data = directories::UserDirs::new()
                .unwrap()
                .home_dir()
                .join("Documents")
                .join("My Games")
                .join("vcmi");

            let internal = dunce::canonicalize(Path::new(".")).unwrap().to_path_buf();

            let user_config = user_data.join("config");

            _ = VDIRS.set(VDirs {
                settings: user_config.join("settings.json"),
                settings_mod: user_config.join("modSettings.json"),
                internal_mods: internal.join("Mods"),
                user_cache: user_data.clone(),
                log: user_data.join("VCMI_Launcher_log.txt"),
                downloads: user_data.join("downloads"),
                mods: user_data.join("Mods"),
                internal,
                user_config,
                user_data,
            });
        }
        #[cfg(target_os = "linux")]
        {
            //TODO CHECK
            let user_data = directories::UserDirs::new()
                .unwrap()
                .data_dir()
                .join("vcmi");
            let user_cache = directories::UserDirs::new()
                .unwrap()
                .cache_dir()
                .join("vcmi");
            let user_config = directories::UserDirs::new()
                .unwrap()
                .config_dir()
                .join("vcmi");
            let internal = if _development_mode {
                Path::new(".").to_path_buf().canonicalize().unwrap()
            } else {
                Path::new("/usr/share").to_path_buf()
            };
            _ = VDIRS.set(VDirs {
                settings: user_config.join("settings.json"),
                settings_mod: user_config.join("modSettings.json"),
                internal_mods: internal.join("Mods"),
                log: home
                    .join("Library")
                    .join("Logs")
                    .join("vcmi")
                    .join("VCMI_Launcher_log.txt"),
                downloads: user_cache.join("downloads"),
                mods: user_data.join("Mods"),
                internal,
                user_cache,
                user_config,
                user_data,
            });
        }
        #[cfg(target_os = "macos")]
        {
            //TODO CHECK
            let home = directories::UserDirs::new().unwrap().home_dir(); //TODO handle Err
            let user_data = home
                .join("Library")
                .join("Application Support")
                .join("vcmi");
            let user_cache = user_data.clone(); //TODO handle Err
            let internal = if _development_mode {
                Path::new(".").to_path_buf().canonicalize().unwrap()
            } else {
                Path::new("../Resources/Data")
                    .to_path_buf()
                    .canonicalize()
                    .unwrap()
            };
            let user_config = user_data.join("config");
            _ = VDIRS.set(VDirs {
                settings: user_config.join("settings.json"),
                settings_mod: user_config.join("modSettings.json"),
                internal_mods: internal.join("Mods"),
                log: home
                    .join("Library")
                    .join("Logs")
                    .join("vcmi")
                    .join("VCMI_Launcher_log.txt"),
                downloads: user_cache.join("downloads"),
                mods: user_data.join("Mods"),
                internal,
                user_cache,
                user_config,
                user_data,
            });
        }
        #[cfg(target_os = "android")]
        {
            let internal = _native
                .0
                .clone()
                .internal_data_path()
                .unwrap()
                .join("vcmi-data");
            let user_data = _native
                .0
                .clone()
                .external_data_path()
                .unwrap()
                .join("vcmi-data");
            let user_cache = user_data.join("cache");
            let user_config = user_data.join("config");
            _ = VDIRS.set(VDirs {
                settings: user_config.join("settings.json"),
                settings_mod: user_config.join("modSettings.json"),
                internal_mods: internal.join("Mods"),
                log: user_config.join("VCMI_Launcher_log.txt"),
                downloads: user_data.join("downloads"),
                mods: user_data.join("Mods"),
                internal,
                user_cache,
                user_config,
                user_data,
            });
        }
        #[cfg(target_os = "ios")]
        {
            //TODO CHECK
            let user_data = directories::UserDirs::new()
                .unwrap()
                .document_dir()
                .unwrap();
            let user_cache = directories::BaseDirs::new().unwrap().cache_dir().unwrap();
            let internal = Path::new(".").to_path_buf().canonicalize().unwrap(); // ???
            let user_config = user_data.join("config");
            _ = VDIRS.set(VDirs {
                settings: user_config.join("settings.json"),
                settings_mod: user_config.join("modSettings.json"),
                internal_mods: internal.join("Mods"),
                log: user_data.join("VCMI_Launcher_log.txt"),
                downloads: user_cache.join("downloads"),
                mods: user_data.join("Mods"),
                internal,
                user_cache,
                user_config,
                user_data,
            });
        }
        use std::fs::create_dir_all as cda;
        let mut result = cda(&VDIRS.get().unwrap().downloads);
        result = result.and(cda(&VDIRS.get().unwrap().internal_mods));
        result = result.and(cda(&VDIRS.get().unwrap().user_cache));
        result = result.and(cda(&VDIRS.get().unwrap().mods));
        result = result.and(cda(&VDIRS.get().unwrap().internal));
        result = result.and(cda(&VDIRS.get().unwrap().user_config));
        result = result.and(cda(&VDIRS.get().unwrap().user_data));
        if let Err(err) = result {
            panic!("{}", err)
        }
    }
}

#[cfg(target_os = "ios")]
#[link(name = "UIKit", kind = "framework")]
#[link(name = "iosmain", kind = "static")]
extern "C" {
    fn launchGame(argc: c_int, argv: *const *const c_char);
}

impl VCMILauncher {
    pub fn start_game(&mut self, _frame: &mut eframe::Frame) {
        log::info!("starting game");
        self.tab = TabName::Mods;
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            match Command::new("./VCMI_client").spawn() {
                Err(err) => {
                    log::error!("Failed to start game; err: {}", err);
                    Toast::error(t!("general.Failed to start game!"))
                }
                Ok(_) => _frame.close(),
            }
        }

        #[cfg(target_os = "android")]
        {
            call_java("onLaunchGameBtnPressed", "()V", &[]);
            log::info!("game client started");
        }

        #[cfg(target_os = "ios")]
        {
            //TODO CHECK
            // create a vector of zero terminated strings
            let args = ["vcmiclient"]
                .map(|arg| CString::new(arg).unwrap())
                .collect::<Vec<CString>>();
            // convert the strings to raw pointers
            let c_args = args
                .iter()
                .map(|arg| arg.as_ptr())
                .collect::<Vec<*const c_char>>();
            unsafe {
                // pass the pointer of the vector's internal buffer to a C function
                launchGame(c_args.len() as c_int, c_args.as_ptr());
            };
        }
    }
    pub fn start_map_editor(&mut self, _frame: &mut eframe::Frame) {
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            log::info!("starting map editor");
            self.tab = TabName::Mods;
            match Command::new("./VCMI_mapeditor").spawn() {
                Err(err) => {
                    log::error!("Failed to start map editor; err: {}", err);
                    Toast::error(t!("general.Failed to start map editor!"))
                }
                Ok(_) => _frame.close(),
            }
        }
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            // Map editor works only on desktop
            unreachable!()
        }
    }
}

#[cfg(target_os = "android")]
pub use android::*;
#[cfg(target_os = "android")]
mod android {
    use jni::objects::JObject;
    use jni::objects::JString;
    use jni::objects::JValue;
    use jni::objects::JValueOwned;
    use jni::strings::JNIString;
    use jni::JNIEnv;
    use jni::JavaVM;
    use std::ffi::CStr;
    use std::ffi::CString;
    use std::sync::atomic::Ordering::Relaxed;
    use std::sync::OnceLock;

    static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();
    pub fn call_java<'local, S, T>(
        name: S,
        sig: T,
        args: &[JValue<'_, '_>],
    ) -> jni::errors::Result<JValueOwned<'local>>
    where
        S: Into<JNIString>,
        T: Into<JNIString> + AsRef<str>,
    {
        let ctx = ndk_context::android_context();
        let ctx = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
        let vm = JAVA_VM.get_or_init(|| {
            let ctx = ndk_context::android_context();
            // Create a VM for executing Java calls
            if let Ok(vm) = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) } {
                vm
            } else {
                log::error!("Expected to find JVM via ndk_context crate");
                panic!()
            }
        });
        let mut env = if let Ok(env) = vm.get_env() {
            env
        } else {
            if let Ok(env) = vm.attach_current_thread_permanently() {
                env
            } else {
                log::error!("Thread atach to VM has failed");
                panic!()
            }
        };
        env.call_method(ctx, name, sig, args)
    }

    #[atomic_enum::atomic_enum]
    #[derive(PartialEq)]
    pub enum DataCopyState {
        NotSelected,
        Selecting,
        NotFound,
        Copying,
        CopyFail,
        Copied,
    }
    pub static GET_HOMM_DIR_PROGRESS: AtomicDataCopyState =
        AtomicDataCopyState::new(DataCopyState::NotSelected);

    #[no_mangle]
    pub unsafe extern "C" fn Java_eu_vcmi_vcmi_MainActivity_GetHoMMDirProgress(
        mut env: JNIEnv,
        _: JObject,
        progress: JString,
    ) {
        let progress = CString::from(CStr::from_ptr(env.get_string(&progress).unwrap().as_ptr()));
        GET_HOMM_DIR_PROGRESS.store(
            match progress.to_str().unwrap() {
                "NULL" => DataCopyState::NotSelected,
                "INVALID" => DataCopyState::NotFound,
                "COPY_START" => DataCopyState::Copying,
                "COPY_END" => DataCopyState::Copied,
                "COPY_FAIL" => DataCopyState::CopyFail,
                _ => unreachable!(),
            },
            Relaxed,
        );
    }

    pub fn open_file_dialog() {
        if GET_HOMM_DIR_PROGRESS.load(Relaxed) == DataCopyState::NotSelected {
            GET_HOMM_DIR_PROGRESS.store(DataCopyState::Selecting, Relaxed);
            call_java("onSelectHoMMDataBtnPressed", "()V", &[]);
            log::info!("onSelectHoMMDataBtnPressed called through jni");
        }
    }
}
