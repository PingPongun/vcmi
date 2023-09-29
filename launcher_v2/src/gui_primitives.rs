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
use egui::{InnerResponse, Ui, Widget};
use rust_i18n::set_locale;
use strum::{EnumMessage, IntoEnumIterator};

use crate::{
    settings::{Language, RangedVal},
    vcmi_launcher::LANGUAGE,
};

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
    static ref LANGUAGES_I18N: Vec<&'static str> =  Language::<true>::iter()
        .map(|lang| lang.get_detailed_message().unwrap())
        .collect();
}
lazy_static::lazy_static! {
    static ref  LANGUAGES_SHORT: Vec<&'static str> = Language::<true>::iter()
        .map(|lang| lang.get_message().unwrap())
        .collect();
}
impl DisplayGUI for Language<false> {
    fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        let mut idx = *self as usize;
        egui::Label::new(label).ui(ui);
        egui::ComboBox::from_id_source(ui.next_auto_id()).show_index(
            ui,
            &mut idx,
            LANGUAGES_I18N.len(),
            |i| LANGUAGES_I18N[i],
        );
        if idx != *self as usize {
            *self = Language::from_repr(idx).unwrap();
            return true;
        }
        return false;
    }
}
impl DisplayGUI for Language<true> {
    fn show_ui(&mut self, ui: &mut Ui, label: &str) -> bool {
        let mut idx = *self as usize;
        egui::Label::new(label).ui(ui);
        egui::ComboBox::from_id_source(ui.next_auto_id()).show_index(
            ui,
            &mut idx,
            LANGUAGES_I18N.len(),
            |i| LANGUAGES_I18N[i],
        );
        if idx != *self as usize {
            *self = Language::from_repr(idx).unwrap();
            set_locale(LANGUAGES_SHORT[idx]);
            *LANGUAGE.write() = self.clone();
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
