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
                .join("vcmi")
                .canonicalize()
                .unwrap(); //TODO handle Err
            let internal = Path::new(".").canonicalize().unwrap().to_path_buf();
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
                .join("vcmi")
                .canonicalize()
                .unwrap(); //TODO handle Err
            let user_cache = directories::UserDirs::new()
                .unwrap()
                .cache_dir()
                .join("vcmi")
                .canonicalize()
                .unwrap(); //TODO handle Err
            let user_config = directories::UserDirs::new()
                .unwrap()
                .config_dir()
                .join("vcmi")
                .canonicalize()
                .unwrap(); //TODO handle Err
            let internal = if _development_mode {
                Path::new(".").to_path_buf().canonicalize().unwrap()
            } else {
                Path::new("/usr/share")
                    .to_path_buf()
                    .canonicalize()
                    .unwrap()
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
                .join("vcmi")
                .canonicalize()
                .unwrap();
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
                .canonicalize()
                .unwrap();
            let user_data = _native
                .0
                .clone()
                .external_data_path()
                .unwrap()
                .canonicalize()
                .unwrap();
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
                .unwrap()
                .canonicalize()
                .unwrap(); //TODO handle Err
            let user_cache = directories::BaseDirs::new()
                .unwrap()
                .cache_dir()
                .unwrap()
                .canonicalize()
                .unwrap(); //TODO handle Err
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
            let ctx = ndk_context::android_context();
            // Create a VM for executing Java calls
            let vm = if let Ok(vm) = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) } {
                vm
            } else {
                log::error!("Expected to find JVM via ndk_context crate");
                panic!()
            };
            let context = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
            let mut env = if let Ok(env) = vm.attach_current_thread_permanently() {
                env
            } else {
                log::error!("Thread atach to VM has failed");
                panic!()
            };
            env.call_method(context, "onLaunchGameBtnPressed", "()V", &[]);
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
