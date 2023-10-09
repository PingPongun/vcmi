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
use indexmap::IndexSet;
use parking_lot::RwLock;
use egui::{InnerResponse, Ui, Widget};
use rust_i18n::set_locale;
use std::fmt::Display;
use strum::IntoEnumIterator;

use crate::settings::*;
use crate::vcmi_launcher::*;

#[macro_export]
macro_rules! icon {
    ($ui:ident,$path: literal) => {
        $ui.add(
            egui::Image::new(egui::include_image!($path))
                .max_height($ui.text_style_height(&egui::TextStyle::Body))
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

///////////////////////////////////////////////////////////////////
/////////////////////////Custom Toasts/////////////////////////////
///////////////////////////////////////////////////////////////////
// pub enum CustomToastKind{
//     VcmiUpdateAvailable=1,
// }
