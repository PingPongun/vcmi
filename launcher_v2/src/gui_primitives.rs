/*
 * gui_primitives.rs, part of VCMI engine
 * GUI related traits & their implementations; Icons & toasts management;
 *
 * Authors: listed in file AUTHORS in main folder
 *
 * License: GNU General Public License v2.0 or later
 * Full text of license available in license.txt file, in main folder
 *
 */
use egui::{Color32, Id, InnerResponse, Response, RichText, Ui};
use egui_struct::*;
use indexmap::IndexSet;
use parking_lot::RwLock;
use rust_i18n::{set_locale, t};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;
use strum::IntoEnumIterator;

use crate::mod_manager::{ModMng, ModPath};
use crate::settings::*;
use crate::utils::hash_helper::IndexMap;
use crate::utils::*;

#[macro_export]
macro_rules! icon {
    ($ui:ident,$path: literal) => {
        $ui.add(
            egui::Image::new(egui::include_image!($path))
                .fit_to_exact_size([$ui.text_style_height(&egui::TextStyle::Body); 2].into())
                .sense(egui::Sense::click()),
        )
    };
}
///////////////////////////////////////////////////////////////////
/////////////////EguiStruct trait & implementations////////////////
///////////////////////////////////////////////////////////////////

lazy_static::lazy_static! {
    pub static ref GAME_LANGUAGES: RwLock<IndexMap<String,String>> =  RwLock::new(Language::iter()
    .map(|lang| (lang.to_string(),lang.translated().to_owned()))
    .filter(|(name, _translated_name)|!name.is_empty())
    .chain(std::iter::once(("Auto".to_string(),"Auto".to_string())))
    .collect());
}
lazy_static::lazy_static! {
    static ref APP_LANGUAGES: Vec<String> =  Language::iter()
        .map(|lang| lang.translated().to_owned())
        .filter(|x|!x.is_empty())
        .collect();
}
lazy_static::lazy_static! {
    static ref  LANGUAGES_SHORT: Vec<String> = Language::iter()
    .map(|lang| lang.short().to_owned())
    .filter(|x|!x.is_empty())
    .collect();
}

impl_eeqclone! {GameLanguage}
impl EguiStruct for GameLanguage {
    type ConfigType<'a> = ();
    fn show_primitive(
        &mut self,
        ui: &mut Ui,
        _config: Self::ConfigType<'_>,
        id: impl Hash,
    ) -> Response {
        let mut langs = GAME_LANGUAGES.write();
        let mut current = if let Some(idx) = langs.get_index_of(&self.0) {
            idx
        } else {
            langs.insert(self.0.clone(), self.0.clone());
            langs.get_index_of(&self.0).unwrap()
        };
        let ret =
            egui::ComboBox::from_id_source(id)
                .show_index(ui, &mut current, langs.len(), |i| &langs[i]);
        if ret.changed() {
            *self = GameLanguage(langs.get_index(current).unwrap().0.clone());
        }
        ret
    }
}

impl_eeqclone! {Language}
impl EguiStruct for Language {
    type ConfigType<'a> = ();
    fn show_primitive(
        &mut self,
        ui: &mut Ui,
        _config: Self::ConfigType<'_>,
        id: impl Hash,
    ) -> Response {
        let mut idx = self.int();
        if idx >= APP_LANGUAGES.len() {
            idx = 0;
        }
        let ret =
            egui::ComboBox::from_id_source(id)
                .show_index(ui, &mut idx, APP_LANGUAGES.len(), |i| &APP_LANGUAGES[i]);
        if ret.changed() {
            *self = Language::from_repr(idx).unwrap();
            set_locale(&LANGUAGES_SHORT[idx]);
            LANGUAGE.set(self.clone());
        }
        ret
    }
}

//Select display mode for game
#[derive(Clone, Copy, PartialEq, EguiStruct)]
#[eguis(prefix = "settings.SettingsVideo")]
enum FullscreenMode {
    #[eguis(hint = "Game will run inside a window that covers part of your screen")]
    Windowed,

    #[eguis(
        hint = "Game will run in a window that covers entirely of your screen, using same resolution as your screen"
    )]
    BorderlessFullscreen,

    #[eguis(hint = "Game will cover entirety of your screen and will use selected resolution")]
    ExclusiveFullscreen,
}

impl_eeqclone! {DisplayOptions}
impl EguiStruct for DisplayOptions {
    type ConfigType<'a> = ();
    fn has_primitive(&self) -> bool {
        true
    }
    fn has_childs(&self) -> bool {
        true
    }
    fn show_primitive(
        self: &mut Self,
        ui: &mut Ui,
        _config: Self::ConfigType<'_>,
        id: impl Hash + Clone,
    ) -> Response {
        let mut fm = match (self.fullscreen, self.real_fullscreen) {
            (_, true) => FullscreenMode::ExclusiveFullscreen,
            (true, false) => FullscreenMode::BorderlessFullscreen,
            (false, false) => FullscreenMode::Windowed,
        };
        let ret = ui.horizontal(|ui| fm.show_primitive(ui, (), id)).inner;
        if ret.changed() {
            (self.fullscreen, self.real_fullscreen) = match fm {
                FullscreenMode::Windowed => (false, false),
                FullscreenMode::BorderlessFullscreen => (true, false),
                FullscreenMode::ExclusiveFullscreen => (true, true),
            };
        }
        ret
    }

    fn show_childs(
        self: &mut Self,
        ui: &mut Ui,
        indent_level: isize,
        response: Response,
        reset2: Option<&Self>,
        id: Id,
    ) -> Response {
        let mut ret = response;
        //(640,480),(800,600),(1024,768),(1280,720),(1360,768),(1366,768),(1280,1024),(1600,900),(1680,1050),(1920,1080)
        // TODO this will require breaking into eframe internals OR dropping eframe in favor of raw winit+wgpu?
        // if (self.fullscreen, self.real_fullscreen) != (true, false) {
        //     ret |= self.resolution.resolution.show_collapsing(
        //         ui,
        //         t!("settings.SettingsVideo.Resolution"),
        //         "",
        //         indent_level,
        //         (),
        //         reset2.map(|x| &x.resolution.resolution),
        //     );
        // }
        let a = [
            50, 60, 75, 90, 100, 110, 125, 150, 175, 200, 225, 250, 300, 350, 400,
        ];
        let mut c = egui_struct::Combobox(self.resolution.scaling);
        ret |= c.show_collapsing(
            ui,
            t!("settings.SettingsVideo.Interface scalling"),
            "",
            indent_level,
            Some(&mut a.into_iter()),
            reset2.map(|x| Combobox(x.resolution.scaling)).as_ref(),
            id,
        );
        self.resolution.scaling = c.0;
        ret
    }
}
#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct InterfaceScale(pub f32);
impl Default for InterfaceScale {
    fn default() -> Self {
        Self(1.0)
    }
}
impl_eeqclone! {InterfaceScale}
impl EguiStruct for InterfaceScale {
    const SIMPLE: bool = false;
    type ConfigType<'a> = ();

    fn show_primitive(
        self: &mut Self,
        ui: &mut Ui,
        _config: Self::ConfigType<'_>,
        _id: impl Hash + Clone,
    ) -> Response {
        ui.horizontal(|ui| {
            let zoom_in = ui.button(t!("settings.SettingsLauncher.Zoom in"));
            let zoom_out = ui.button(t!("settings.SettingsLauncher.Zoom out"));
            if zoom_in.clicked() {
                self.zoom_in()
            }
            if zoom_out.clicked() {
                self.zoom_out()
            }
            zoom_in | zoom_out
        })
        .inner
    }
}
impl InterfaceScale {
    pub fn zoom_in(&mut self) {
        self.0 *= 1.1;
        self.normalize();
    }
    pub fn zoom_out(&mut self) {
        self.0 /= 1.1;
        self.normalize();
    }
    fn normalize(&mut self) {
        self.0 = self.0.clamp(0.2, 4.0);
        self.0 = (self.0 * 10.).round() / 10.;
    }
}

pub trait EguiUiExt {
    fn group_wrapped<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R>;
}
impl EguiUiExt for Ui {
    fn group_wrapped<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        self.group(|ui| ui.horizontal_wrapped(add_contents).inner)
    }
}

pub trait DisplayGUI2 {
    fn show(&self, ui: &mut Ui, label: impl ToString);
}
impl DisplayGUI2 for String {
    fn show(&self, ui: &mut Ui, label: impl ToString) {
        if !self.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.strong(RichText::new(label.to_string() + ":"));
                ui.label(self);
            });
        }
    }
}
pub trait DisplayGUI2Iter {}
impl<T: Display> DisplayGUI2Iter for std::slice::Iter<'_, T> {}
impl<T: Display> DisplayGUI2Iter for indexmap::set::Iter<'_, T> {}
impl<T: Display, I: Iterator<Item = T>, U> DisplayGUI2Iter for std::iter::Map<I, U> {}

impl<T: DisplayGUI2Iter> DisplayGUI2 for T
where
    T: ExactSizeIterator + DoubleEndedIterator + Clone,
    T::Item: Display,
{
    fn show(&self, ui: &mut Ui, label: impl ToString) {
        let mut s = self.clone();
        match s.len() {
            0 => (),
            1 => s.next().unwrap().to_string().show(ui, label),
            x if x < 5 => {
                ui.vertical(|ui| {
                    ui.strong(RichText::new(label.to_string() + ":"));
                    ui.indent(ui.next_auto_id(), |ui| {
                        s.for_each(|x| {
                            ui.label(x.to_string());
                        })
                    })
                });
            }
            _ => {
                ui.collapsing(RichText::new(label.to_string() + ":").strong(), |ui| {
                    s.rev().for_each(|x| {
                        ui.label(x.to_string());
                    });
                });
            }
        }
    }
}

impl DisplayGUI2 for IndexMap<String, Vec<String>> {
    fn show(&self, ui: &mut Ui, label: impl ToString) {
        if !self.is_empty() {
            ui.collapsing(RichText::new(label.to_string() + ":").strong(), |ui| {
                self.iter().rev().for_each(|(version, changes)| {
                    changes.iter().show(ui, version);
                });
            });
        }
    }
}

pub trait DisplayGUI3 {
    fn show3(&self, ui: &mut Ui, label: impl ToString, color: Option<Color32>, mng: &mut ModMng);
}
impl DisplayGUI3 for IndexSet<ModPath> {
    fn show3(&self, ui: &mut Ui, label: impl ToString, color: Option<Color32>, mng: &mut ModMng) {
        let mut s = self.iter();
        let c = color.unwrap_or(ui.style().visuals.widgets.active.fg_stroke.color);
        let mut ret = None;
        match s.len() {
            0 => (),
            1 => {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(RichText::new(label.to_string() + ":").color(c));
                    let s = s.next().unwrap();
                    if ui.selectable_label(false, s.to_string()).clicked() {
                        ret = Some(s.clone());
                    }
                });
            }
            x if x < 5 => {
                ui.vertical(|ui| {
                    ui.strong(RichText::new(label.to_string() + ":").color(c));
                    ui.indent(ui.next_auto_id(), |ui| {
                        s.for_each(|x| {
                            if ui.selectable_label(false, x.to_string()).clicked() {
                                ret = Some(x.clone());
                            }
                        })
                    })
                });
            }
            _ => {
                ui.collapsing(
                    RichText::new(label.to_string() + ":").color(c).strong(),
                    |ui| {
                        s.rev().for_each(|x| {
                            if ui.selectable_label(false, x.to_string()).clicked() {
                                ret = Some(x.clone());
                            }
                        });
                    },
                );
            }
        }
        if let Some(mod_) = ret {
            mng.highlighted_scroll = true;
            mod_.unfold_ancestors();
            mng.highlighted_mod = Some(mod_);
        }
    }
}
