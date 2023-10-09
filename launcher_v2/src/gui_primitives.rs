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
use egui::{Color32, InnerResponse, RichText, Ui, Widget};
use indexmap::IndexSet;
use parking_lot::RwLock;
use rust_i18n::set_locale;
use std::fmt::Display;
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
/////////////////DisplayGUI trait & implementations////////////////
///////////////////////////////////////////////////////////////////
pub trait DisplayGUI {
    fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool;
}

impl DisplayGUI for usize {
    fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        ui.label(label);
        egui::DragValue::new(self).ui(ui).changed()
    }
}
impl DisplayGUI for bool {
    fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        ui.label(label);

        egui::Checkbox::without_text(self).ui(ui).changed()
    }
}

impl<const MIN: isize, const MAX: isize> DisplayGUI for RangedVal<MIN, MAX> {
    fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        ui.label(label);
        egui::DragValue::new(&mut self.0)
            .clamp_range(MIN..=MAX)
            .ui(ui)
            .changed()
    }
}

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

impl DisplayGUI for GameLanguage {
    fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        let mut langs = GAME_LANGUAGES.write();
        let mut current = if let Some(idx) = langs.get_index_of(&self.0) {
            idx
        } else {
            langs.insert(self.0.clone(), self.0.clone());
            langs.get_index_of(&self.0).unwrap()
        };
        egui::Label::new(label).ui(ui);
        if egui::ComboBox::from_id_source(ui.next_auto_id())
            .show_index(ui, &mut current, langs.len(), |i| &langs[i])
            .changed()
        {
            *self = GameLanguage(langs.get_index(current).unwrap().0.clone());
            return true;
        }
        return false;
    }
}
impl DisplayGUI for Language {
    fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        let mut idx = self.int();
        if idx >= APP_LANGUAGES.len() {
            idx = 0;
        }
        egui::Label::new(label).ui(ui);
        if egui::ComboBox::from_id_source(ui.next_auto_id())
            .show_index(ui, &mut idx, APP_LANGUAGES.len(), |i| &APP_LANGUAGES[i])
            .changed()
        {
            *self = Language::from_repr(idx).unwrap();
            set_locale(&LANGUAGES_SHORT[idx]);
            LANGUAGE.set(self.clone());
            return true;
        }
        return false;
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
