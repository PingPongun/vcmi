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
use egui::{
    include_image, Align, Align2, FontData, FontDefinitions, FontFamily, Image, ImageButton,
    ImageSource, Layout, Ui, Vec2,
};
use egui_extras::{Size, Strip, StripBuilder};
use egui_toast::Toasts;
use std::time::Duration;
use rust_i18n::ToStringI18N;

use crate::about_project::VcmiUpdatesJson;
use crate::first_launch::FirstLaunchState;
use crate::settings::Settings;
use crate::utils::AsyncHandle;

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
#[derive(Default)]
pub struct VCMILauncher {
    pub settings: Settings,
    pub first_launch: FirstLaunchState,
    pub tab: TabName,
    pub update_fetch: AsyncHandle<VcmiUpdatesJson, ()>,
    pub mobile_view: bool,
}

impl eframe::App for VCMILauncher {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let screen_size = ctx.screen_rect().size();
        self.mobile_view = screen_size.y > screen_size.x;
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

                    const TAB_ICONS: [ImageSource; 7] = [
                        include_image!("../icons/menu-mods.png"),
                        include_image!("../icons/menu-downloads.png"),
                        include_image!("../icons/menu-settings.png"),
                        include_image!("../icons/menu-lobby.png"),
                        include_image!("../icons/about-project.png"),
                        include_image!("../icons/menu-editor.png"),
                        include_image!("../icons/menu-game.png"),
                    ];
                    if ui
                        .add(
                            ImageButton::new(
                                Image::new(TAB_ICONS[tab as usize].clone())
                                    .fit_to_exact_size(Vec2::new(icon_size, icon_size)),
                            )
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

            if self.mobile_view {
                //mobile view
                egui::TopBottomPanel::bottom("tabs_panel")
                    .exact_height(tab_panel_height + 6.)
                    .show(ctx, |ui| {
                        ui.add_space(6.0);
                        StripBuilder::new(ui)
                            .sizes(Size::remainder(), tab_count)
                            .cell_layout(Layout::top_down(Align::Center))
                            .horizontal(show_tabs);
                    });
            } else {
                //desktop view
                egui::SidePanel::left("tabs_panel")
                    .exact_width(tab_panel_height)
                    .show(ctx, |ui| {
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
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

impl VCMILauncher {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
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

        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut ret = Self::default();
        ret.load_settings();
        if *ret.settings.launcher.update_on_startup {
            ret.spawn_update_check_vcmi();
        }
        ret
    }
}
