/*
 * main.rs, part of VCMI engine
 * Program entry point
 *
 * Authors: listed in file AUTHORS in main folder
 *
 * License: GNU General Public License v2.0 or later
 * Full text of license available in license.txt file, in main folder
 *
 */
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
mod about_project;
mod first_launch;
mod gui_primitives;
mod lobby;
mod mod_manager;
mod platform;
mod settings;
mod utils;
mod vcmi_launcher;

use std::path::Path;
use std::{fs, io};

use eframe::{IconData, NativeOptions, Renderer};
use egui::Vec2;
use log::error;
use platform::{NativeParams, VDirs};
use utils::{get_dirs, RUNTIME};
use vcmi_launcher::*;

#[cfg(target_os = "android")]
pub use platform::Java_eu_vcmi_vcmi_MainActivity_GetHoMMDirProgress;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

fn logging_setup(log_path: &Path) {
    let mut base_log = fern::Dispatch::new().format(|out, message, record| {
        out.finish(format_args!(
            "[{} {} {}] {}",
            humantime::format_rfc3339(std::time::SystemTime::now()),
            record.level(),
            record.target(),
            message
        ))
    });

    #[cfg(target_os = "android")]
    {
        // android logging to LogCat
        base_log = base_log.chain(fern::Output::call(android_logger::log));
    }
    #[cfg(not(target_os = "android"))]
    {
        // stdout logging
        base_log = base_log.chain(
            fern::Dispatch::new()
                // by default only accept warn messages
                .level(log::LevelFilter::Warn)
                // accept info messages from the current crate too
                .level_for("vcmilauncherv2", log::LevelFilter::Info) //TODO ULP switch to Trace in debug
                .chain(io::stdout()),
        );
        // stderr logging
        base_log = base_log.chain(
            fern::Dispatch::new()
                .level(log::LevelFilter::Error)
                .chain(io::stderr()),
        );
    }
    //file logging
    match fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(log_path)
    {
        Ok(log_file) => {
            base_log = base_log.chain(
                fern::Dispatch::new()
                    // by default only accept warn messages
                    .level(log::LevelFilter::Warn)
                    // accept info messages from the current crate too
                    .level_for("vcmilauncherv2", log::LevelFilter::Info) //TODO ULP switch to Trace in debug
                    .chain(log_file),
            );
            // and finally, set as the global logger!
            let _ = base_log.apply();
        }
        Err(err) => {
            // and finally, set as the global logger!
            let _ = base_log.apply();
            let _ = error!("Error creating log file: {}", err);
        }
    }
}

fn _main(mut options: NativeOptions, native: NativeParams) {
    VDirs::init(native.clone());
    logging_setup(&get_dirs().log);
    options.renderer = Renderer::Wgpu;
    // options.renderer = Renderer::Glow;
    options.initial_window_size = Some(Vec2::new(800., 500.));

    let icon_raw = include_bytes!("../icons/VCMI_launcher.ico");
    let icon = image::load_from_memory_with_format(icon_raw.as_slice(), image::ImageFormat::Ico)
        .unwrap()
        .to_rgba8();
    let (icon_width, icon_height) = icon.dimensions();
    options.icon_data = None;
    Some(IconData {
        rgba: icon.into_raw(),
        width: icon_width,
        height: icon_height,
    });

    let _rt_guard = RUNTIME.enter();
    let _ = eframe::run_native(
        "VCMI Launcher",
        options,
        Box::new(|cc| Box::new(VCMILauncher::new(cc))),
    )
    .unwrap_or_else(|err| {
        log::error!("Failure while running EFrame application: {err:?}");
    });
}

#[cfg(target_os = "android")]
#[no_mangle]
#[inline(never)]
#[allow(dead_code)]
fn android_main(app: AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    // android_logger::init_once(
    //     android_logger::Config::default().with_max_level(log::LevelFilter::Debug),
    // );
    let appc = app.clone();

    let options = NativeOptions {
        event_loop_builder: Some(Box::new(move |builder| {
            builder.with_android_app(appc);
        })),
        ..Default::default()
    };

    // let _ =
    _main(options, NativeParams(app));
    // .unwrap_or_else(|err| {
    //     log::error!("Failure while running EFrame application: {err:?}");
    // });
}

#[cfg(all(not(target_os = "android"), not(target_os = "ios")))]
#[allow(dead_code)]
fn main() {
    _main(NativeOptions::default(), NativeParams());
}
#[cfg(target_os = "ios")]
#[no_mangle]
#[inline(never)]
#[allow(dead_code)]
//qt_main_wrapper should be changed for sth diferent e.g. ios_launcher_main, but it requires also change in client/ios/main.m
extern "C" fn qt_main_wrapper(_argc: std::ffi::c_int, _argv: *const *const std::ffi::c_char) {
    _main(NativeOptions::default(), NativeParams());
}
