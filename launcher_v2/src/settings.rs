/*
 * settings.rs, part of VCMI engine
 * Settings management
 * - handle "settings.json" & related data structures
 * - settings view/GUI generation (automaticly, with macros from structures)
 *
 * Authors: listed in file AUTHORS in main folder
 *
 * License: GNU General Public License v2.0 or later
 * Full text of license available in license.txt file, in main folder
 *
 */
use educe::Educe;
use egui::{ScrollArea, Ui, Widget};
use egui_toast::Toast;
use indexmap::IndexMap;
use rust_i18n::*;
use serde::Deserialize;
use serde::Serialize;
use serde_enum_str::Deserialize_enum_str;
use serde_enum_str::Serialize_enum_str;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::atomic::AtomicUsize;
use strum::*;
use vcmi_launcher_macros::*;

use crate::gui_primitives::DisplayGUI;
use crate::utils::*;
use crate::vcmi_launcher::*;

impl VCMILauncher {
    pub fn load_settings(&mut self) {
        log::info!("Loading config/settings file ...");
        let path = get_dirs().settings.clone();
        self.settings = load_file_settings(&path);

        set_locale(self.settings.general.language.short());
        LANGUAGE.set(self.settings.general.language.clone());

        // check if homm data is present in vcmi dir
        if let Err(err) = check_data_dir_valid(&get_dirs().user_data)
            .or(check_data_dir_valid(&get_dirs().internal))
        {
            if self.settings.launcher.setup_completed {
                self.settings.launcher.setup_completed = false;
                Toast::error(t!("toasts.error.vcmi_data_verification_failed"));
                log::error!("Failed to verify vcmi data!; Error: {}", err)
            }
        }
    }

    pub fn save_settings(&mut self) {
        let path = get_dirs().settings.clone();
        save_file(&path, &self.settings);
    }

    pub fn show_settings(&mut self, ui: &mut Ui) {
        if ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| self.settings.show_ui(ui, "settings:"))
            .inner
        {
            self.save_settings();
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize, DisplayGUI)]
#[serde(default, rename_all = "camelCase")]
#[module(settings)]
#[uncollapsed]
pub struct Settings {
    pub general: SettingsGeneral,
    pub video: SettingsVideo,
    pub server: SettingsServer,
    pub launcher: SettingsLauncher,
    #[serde(flatten)] //capture/preserve not recognized fields
    #[skip]
    extra: IndexMap<String, serde_json::Value>,
}

#[derive(serde::Deserialize, serde::Serialize, DisplayGUI, Educe)]
#[educe(Default)]
#[serde(default, rename_all = "camelCase")]
#[module(settings)]
pub struct SettingsGeneral {
    pub language: Language,
    pub game_data_language: GameLanguage,
    #[educe(Default = 5)]
    autosave_count_limit: usize,
    #[educe(Default(expression = "RangedVal(50)"))]
    music: RangedVal<0, 100>,
    #[educe(Default(expression = "RangedVal(50)"))]
    sound: RangedVal<0, 100>,
    #[serde(flatten)]
    #[skip]
    extra: IndexMap<String, serde_json::Value>,
}
#[derive(Default, serde::Deserialize, serde::Serialize, DisplayGUI)]
#[serde(default, rename_all = "camelCase")]
#[module(settings)]
pub struct SettingsLauncher {
    pub auto_check_repositories: Tbool,
    pub update_on_startup: Tbool,
    // defaultRepositoryEnabled: Tbool,
    // extraRepositoryEnabled: bool,
    // extraRepositoryURL: String,
    #[skip]
    pub lobby_username: String,
    #[skip]
    pub setup_completed: bool,
    #[serde(flatten)]
    #[skip]
    extra: IndexMap<String, serde_json::Value>,
}
#[derive(serde::Deserialize, serde::Serialize, DisplayGUI, Educe)]
#[educe(Default)]
#[serde(default, rename_all = "camelCase")]
#[module(settings)]
pub struct SettingsServer {
    #[educe(Default(expression = "AIAdventure::VCAI"))]
    allied_ai: AIAdventure,
    player_ai: AIAdventure,
    #[educe(Default(expression = "AIBattle::StupidAI"))]
    neutral_ai: AIBattle,
    friendly_ai: AIBattle,
    enemy_ai: AIBattle,
    #[serde(flatten)]
    #[skip]
    extra: IndexMap<String, serde_json::Value>,
}
#[derive(Default, serde::Deserialize, serde::Serialize, DisplayGUI)]
#[serde(default, rename_all = "camelCase")]
#[module(settings)]
pub struct SettingsVideo {
    fullscreen: bool,
    real_fullscreen: bool,
    // resolution
    show_intro: Tbool,
    // targetfps
    #[serde(flatten)]
    #[skip]
    extra: IndexMap<String, serde_json::Value>,
}

///////////////////////////////////////////////////////////////
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Deserialize_enum_str,
    Serialize_enum_str,
    FromRepr,
    EnumIter,
    EnumMessage,
)]
#[serde(rename_all = "lowercase")]
#[repr(usize)]
pub enum Language {
    #[strum(message = "en", detailed_message = "English")]
    English = 0,
    #[strum(message = "pl", detailed_message = "polski")]
    Polish,
    #[strum(message = "de", detailed_message = "Deutsch")]
    German,
    #[strum(message = "zh", detailed_message = "简体中文")]
    Chinese,
    #[strum(message = "fr", detailed_message = "Français")]
    French,
    #[strum(message = "ru", detailed_message = "Русский")]
    Russian,
    #[strum(message = "uk", detailed_message = "Українська")]
    Ukrainian,
    #[strum(message = "es", detailed_message = "Español")]
    Spanish,
    #[strum(message = "cs", detailed_message = "čeština")]
    Czech,
    #[serde(other)]
    Other(String), //add other languages
}

#[derive(Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct GameLanguage(pub String);

impl Default for Language {
    fn default() -> Self {
        //get system locale
        let locale = sys_locale::get_locale().unwrap_or_else(|| String::from("en-US"));
        let locale = locale
            .split(|c: char| !c.is_alphabetic())
            .next()
            .unwrap_or_default();
        let mut ret = Language::English;
        Language::iter().for_each(|lang| {
            if lang.short() == locale {
                ret = lang;
            }
        });
        ret
    }
}
impl Language {
    pub const fn int(&self) -> usize {
        unsafe { *(self as *const Self as *const usize) }
    }
    pub fn short(&self) -> &str {
        if let Language::Other(lang) = self {
            lang
        } else {
            self.get_message().unwrap()
        }
    }
    pub fn translated(&self) -> &str {
        if let Language::Other(lang) = self {
            lang
        } else {
            self.get_detailed_message().unwrap()
        }
    }
}

pub struct AtomicLanguage(pub AtomicUsize);
impl AtomicLanguage {
    pub const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }
    pub fn get(&self) -> Language {
        Language::from_repr(self.0.load(std::sync::atomic::Ordering::Relaxed)).unwrap()
    }
    pub fn set(&self, val: Language) {
        self.0
            .store(val.int(), std::sync::atomic::Ordering::Relaxed)
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct RangedVal<const MIN: isize, const MAX: isize>(pub isize);

// #[derive(Default)]
// struct OptionalVal<T: DisplayGUI, S: OptionalValTrait>(Option<T>, PhantomData<S>);
// trait OptionalValTrait {
//     const ENABLE_NAME: &'static str;
//     const INNER_NAME: &'static str;
// }
// macro_rules! OptionalVal {
//     ($enable_name:ident, $inner_name:ident) => {};
// }
// impl<T: DisplayGUI, S: OptionalValTrait> DisplayGUI for OptionalVal<T, S> {
//     fn show_ui(&mut self, ui: &mut Ui, label: &str) {
//         ui.label(label);
//         if let Some(inner) = &mut self.0 {}else{}
//     }
// }

#[derive(
    Default, Clone, Copy, serde::Deserialize, serde::Serialize, FromRepr, EnumComboboxI18N,
)]
#[module(settings.SettingsServer)]
enum AIBattle {
    #[default]
    BattleAI,
    StupidAI,
}
#[derive(
    Default, Clone, Copy, serde::Deserialize, serde::Serialize, FromRepr, EnumComboboxI18N,
)]
#[module(settings.SettingsServer)]
enum AIAdventure {
    #[default]
    Nullkiller,
    VCAI,
}

///Same as bool but defaults to true
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Tbool(bool);

impl Default for Tbool {
    fn default() -> Self {
        Self(true)
    }
}
impl Deref for Tbool {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Tbool {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
