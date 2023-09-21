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
use egui::{InnerResponse, TextureHandle, Ui, Widget};
use rust_i18n::set_locale;
use strum::{EnumMessage, IntoEnumIterator};

use crate::settings::{Language, RangedVal};

pub mod icons {
    use egui::Context;

    use super::*;
    macro_rules! load_icon {
        ($name:literal, $ctx:ident) => {{
            let icon_raw = include_bytes!($name);
            let icon =
                image::load_from_memory_with_format(icon_raw.as_slice(), image::ImageFormat::Png)
                    .unwrap()
                    .to_rgba8();
            let (width, height) = icon.dimensions();
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize],
                &icon.as_raw(),
            );
            $ctx.load_texture($name, image, Default::default())
        }};
    }
    macro_rules! load_icons {
        ($ctx:ident, [$($name:literal),*]) => {[$(load_icon!($name,$ctx)),*]};
    }
    #[derive(Default, PartialEq, Clone, Copy)]
    #[allow(dead_code)]
    pub enum IconModName {
        #[default]
        Delete = 0,
        Download,
        Disabled,
        Enabled,
        Update,
        Invalid,
    }
    pub struct Icons {
        pub menu: [TextureHandle; 7],
        pub mod_state: [TextureHandle; 6],
        pub lobby: [TextureHandle; 1],
    }

    impl Icons {
        pub fn load(ctx: &Context) -> Self {
            Icons {
                menu: load_icons!(
                    ctx,
                    [
                        "../icons/menu-mods.png",
                        "../icons/menu-downloads.png",
                        "../icons/menu-settings.png",
                        "../icons/menu-lobby.png",
                        "../icons/about-project.png",
                        "../icons/menu-editor.png",
                        "../icons/menu-game.png"
                    ]
                ),
                mod_state: load_icons!(
                    ctx,
                    [
                        "../icons/mod-delete.png",
                        "../icons/mod-download.png",
                        "../icons/mod-disabled.png",
                        "../icons/mod-enabled.png",
                        "../icons/mod-update.png",
                        "../icons/mod-invalid.png"
                    ]
                ),
                lobby: load_icons!(ctx, ["../icons/room-private.png"]),
            }
        }
    }
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
impl<const LAUNCHER: bool> DisplayGUI for Language<LAUNCHER> {
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
            if LAUNCHER {
                set_locale(LANGUAGES_SHORT[idx])
            };
            *self = Language::from_repr(idx).unwrap();
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
