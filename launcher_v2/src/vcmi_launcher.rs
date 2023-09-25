/*
 * vcmi_launcher.rs, part of VCMI engine
 * Program top/main data structure & GUI drawing entry point
 *
 * Authors: listed in file AUTHORS in main folder
 *
 * License: GNU General Public License v2.0 or later
 * Full text of license available in license.txt file, in main folder
 *
 */
use eframe::egui;
use egui::{Align, Align2, FontData, FontDefinitions, FontFamily, ImageButton, Layout, Ui};
use egui_extras::{Size, Strip, StripBuilder};
use egui_toast::Toasts;
use rust_i18n::ToStringI18N;
use std::future::Future;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use crate::about_project::FetchUpdate;
use crate::first_launch::FirstLaunchState;
use crate::gui_primitives::icons;
use crate::platform::{NativeParams, VDirs};
use crate::settings::Settings;

rust_i18n::i18n!("./translate", fallback = "en");
#[derive(ToStringI18N, Default, PartialEq, Clone, Copy)]
#[module(menu)]
pub enum TabName {
    #[default]
    Mods = 0,
    Downloads,
    Settings,
    Lobby,
    About,
    MapEditor,
    StartGame,
}
pub struct VCMILauncher {
    pub dirs: VDirs,
    pub settings: Settings,
    pub first_launch: FirstLaunchState,
    pub tab: TabName,
    pub icons: icons::Icons,
    pub native: NativeParams,
    pub update_fetch: FetchUpdate,
}

impl eframe::App for VCMILauncher {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let screen_size = ctx.screen_rect().size();
        if self.settings.launcher.setup_completed {
            let tab_count = if cfg!(any(target_os = "android", target_os = "ios")) {
                6
            } else {
                7
            };
            let tab_panel_height = f32::min(screen_size.x, screen_size.y) / tab_count as f32;
            let icon_size =
                0.7 * (tab_panel_height - egui::TextStyle::Body.resolve(&ctx.style()).size);

            let show_tabs = |mut strip: Strip<'_, '_>| {
                let mut show_tab_button = |ui: &mut Ui, tab: TabName, enabled: bool| {
                    ui.set_enabled(enabled);
                    if ui
                        .add(
                            ImageButton::new(&self.icons.menu[tab as usize], [icon_size; 2])
                                .selected(self.tab == tab),
                        )
                        .clicked()
                    {
                        self.tab = tab;
                    }
                    ui.label(tab.to_string_i18n());
                };
                strip.cell(|ui| show_tab_button(ui, TabName::Mods, true));
                strip.cell(|ui| show_tab_button(ui, TabName::Downloads, true));
                strip.cell(|ui| show_tab_button(ui, TabName::Settings, true));
                strip.cell(|ui| show_tab_button(ui, TabName::Lobby, true));
                strip.cell(|ui| show_tab_button(ui, TabName::About, true));
                if !cfg!(any(target_os = "android", target_os = "ios")) {
                    strip.cell(|ui| show_tab_button(ui, TabName::MapEditor, true));
                }
                strip.cell(|ui| show_tab_button(ui, TabName::StartGame, true));
            };

            if screen_size.y > screen_size.x {
                egui::TopBottomPanel::bottom("tabs_panel")
                    .exact_height(tab_panel_height + 6.)
                    .show(ctx, |ui: &mut egui::Ui| {
                        ui.add_space(6.0);
                        StripBuilder::new(ui)
                            .sizes(Size::remainder(), tab_count)
                            .cell_layout(Layout::top_down(Align::Center))
                            .horizontal(show_tabs);
                    });
            } else {
                egui::SidePanel::left("tabs_panel")
                    .exact_width(tab_panel_height)
                    .show(ctx, |ui: &mut egui::Ui| {
                        StripBuilder::new(ui)
                            .sizes(Size::remainder(), tab_count)
                            .cell_layout(Layout::top_down(Align::Center))
                            .vertical(show_tabs);
                    });
            }

            egui::CentralPanel::default().show(ctx, |ui| match self.tab {
                TabName::Mods => self.show_mods(ui),
                TabName::Downloads => self.show_downloads(ui),
                TabName::Settings => self.show_settings(ui),
                TabName::Lobby => self.show_lobby(ui),
                TabName::About => self.show_about(ui),
                TabName::MapEditor => self.start_map_editor(frame),
                TabName::StartGame => self.start_game(frame),
            });
        } else {
            self.show_first_launch(ctx);
        }
        // Show and update all toasts
        Toasts::new()
            .anchor(Align2::RIGHT_BOTTOM, (-10.0, -10.0)) // 10 units from the bottom right corner
            .direction(egui::Direction::BottomUp)
            // .custom_contents(kind, add_contents)
            .show(ctx);
    }
}

impl VCMILauncher {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, native: NativeParams, dirs: VDirs) -> Self {
        let mut _out_of_window_size = Default::default(); //may be used to detect notch?
        if let Some(monitor_size) = cc.integration_info.window_info.monitor_size {
            _out_of_window_size = monitor_size - cc.integration_info.window_info.size;
        }

        // Install additionall fonts (supporting non-latin characters):
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "WenQuanYi-Micro-Hei".to_owned(),
            FontData::from_static(include_bytes!("../assets/WenQuanYi-Micro-Hei-Regular.ttf")),
        ); // .ttf and .otf supported
           // Put font as last fallback:
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .push("WenQuanYi-Micro-Hei".to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .push("WenQuanYi-Micro-Hei".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let mut ret = Self {
            dirs,
            settings: Default::default(),
            first_launch: Default::default(),
            tab: Default::default(),
            icons: icons::Icons::load(&cc.egui_ctx),
            native: native.clone(),
            update_fetch: Default::default(),
        };
        ret.load_settings();
        if ret.settings.launcher.update_on_startup {
            ret.spawn_update_check_vcmi();
        }
        ret
    }
}

/////////////////////////////////////////////////////////////////
///////////////////Async task handle management//////////////////
/////////////////////////////////////////////////////////////////

lazy_static::lazy_static! {
    static ref RUNTIME: Runtime =  Runtime::new().unwrap();
}

#[derive(Default)]
pub enum AsyncHandle<T: Send, P> {
    #[default]
    Uninit,
    Running(JoinHandle<Result<T, ()>>, Arc<P>),
    Finished(Result<T, ()>),
}
use AsyncHandle::*;

impl<T: Send + 'static, P> AsyncHandle<T, P> {
    pub fn run<F>(&mut self, progress: Arc<P>, future: F)
    where
        F: Future<Output = Result<T, ()>> + Send + 'static,
    {
        if !matches!(self, Running(_, _)) {
            *self = Running(RUNTIME.spawn(future), progress);
        }
    }
    fn fetch_handle(&mut self) {
        if let Running(handle, _) = self {
            if handle.is_finished() {
                *self = Finished(RUNTIME.block_on(handle).unwrap_or(Err(())))
            }
        }
    }
    pub fn is_finished(&mut self) -> bool {
        self.fetch_handle();
        matches!(self, Finished(_))
    }
    pub fn is_success(&mut self) -> bool {
        self.fetch_handle();
        matches!(self, Finished(Ok(_)))
    }
    pub fn is_failure(&mut self) -> bool {
        self.fetch_handle();
        matches!(self, Finished(Err(())))
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
