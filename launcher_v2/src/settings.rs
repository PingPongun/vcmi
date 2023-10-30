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
use egui::{RichText, Ui};
use egui_struct::*;
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
use ConfigNum::*;

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
                Toast::error(t!("toasts.error.HoMM data not found!"));
                log::error!("Failed to verify vcmi data!; Error: {}", err)
            }
        }
    }

    pub fn save_settings(&mut self) {
        let path = get_dirs().settings.clone();
        save_file(&path, &self.settings);
    }

    pub fn show_settings(&mut self, ui: &mut Ui) {
        if self
            .settings
            .show_top_mut(
                ui,
                RichText::new(t!("menu.TabName.Settings")).heading(),
                None,
            )
            .changed()
        {
            self.save_settings();
        }
    }
}

#[derive(Default, Deserialize, Serialize, EguiStruct)]
#[serde(default, rename_all = "camelCase")]
#[eguis(
    prefix = "settings",
    rename_all = "Sentence",
    resetable = "struct_default"
)]
pub struct Settings {
    pub general: SettingsGeneral,
    pub video: SettingsVideo,
    pub server: SettingsServer,
    pub launcher: SettingsLauncher,
    #[serde(flatten)] //capture/preserve not recognized fields
    #[eguis(skip)]
    extra: IndexMap<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize, EguiStruct, Educe)]
#[educe(Default)]
#[serde(default, rename_all = "camelCase")]
#[eguis(prefix = "settings", rename_all = "Sentence")]
pub struct SettingsGeneral {
    #[eguis(hint = "Select language you prefer to use in launcher")]
    pub language: Language,
    pub game_data_language: GameLanguage,
    #[educe(Default = 5)]
    autosave_count_limit: usize,
    #[educe(Default(50))]
    #[eguis(config = "Slider(0,100)")]
    music: isize,
    #[educe(Default(50))]
    #[eguis(config = "Slider(0,100)")]
    sound: isize,
    #[serde(flatten)]
    #[eguis(skip)]
    extra: IndexMap<String, serde_json::Value>,
}
#[derive(Default, Deserialize, Serialize, EguiStruct)]
#[serde(default, rename_all = "camelCase")]
#[eguis(prefix = "settings", rename_all = "Sentence")]
pub struct SettingsLauncher {
    pub auto_check_repositories: Tbool,
    pub update_on_startup: Tbool,
    // defaultRepositoryEnabled: Tbool,
    // extraRepositoryEnabled: bool,
    // extraRepositoryURL: String,
    #[eguis(skip)]
    pub lobby_username: String,
    #[eguis(skip)]
    pub setup_completed: bool,
    #[serde(flatten)]
    #[eguis(skip)]
    extra: IndexMap<String, serde_json::Value>,
}

// impl PartialEq for SettingsLauncher {
//     fn eq(&self, other: &Self) -> bool {
//         self.auto_check_repositories == other.auto_check_repositories
//             && self.update_on_startup == other.update_on_startup
//             && self.lobby_username == other.lobby_username
//     }
// }
#[derive(Deserialize, Serialize, EguiStruct, Educe)]
#[educe(Default)]
#[serde(default, rename_all = "camelCase")]
#[eguis(prefix = "settings")]
pub struct SettingsServer {
    #[educe(Default(expression = "AIAdventure::VCAI"))]
    allied_ai: AIAdventure,
    player_ai: AIAdventure,
    #[educe(Default(expression = "AIBattle::StupidAI"))]
    neutral_ai: AIBattle,
    friendly_ai: AIBattle,
    enemy_ai: AIBattle,
    #[serde(flatten)]
    #[eguis(skip)]
    extra: IndexMap<String, serde_json::Value>,
}
#[derive(Default, Deserialize, Serialize, EguiStruct)]
#[serde(default, rename_all = "camelCase")]
#[eguis(prefix = "settings", rename_all = "Sentence")]
pub struct SettingsVideo {
    fullscreen: bool,
    real_fullscreen: bool,
    // resolution
    show_intro: Tbool,
    // targetfps
    #[serde(flatten)]
    #[eguis(skip)]
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

#[derive(Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct GameLanguage(pub String);

impl Default for GameLanguage {
    fn default() -> Self {
        Self(Language::default().to_string())
    }
}

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

// #[derive(Default)]
// struct OptionalVal<T: EguiStruct, S: OptionalValTrait>(Option<T>, PhantomData<S>);
// trait OptionalValTrait {
//     const ENABLE_NAME: &'static str;
//     const INNER_NAME: &'static str;
// }
// macro_rules! OptionalVal {
//     ($enable_name:ident, $inner_name:ident) => {};
// }

#[derive(Default, Clone, Copy, PartialEq, Deserialize, Serialize, FromRepr, EguiStruct)]
#[eguis(prefix = "settings.SettingsServer")]
enum AIBattle {
    #[default]
    BattleAI,
    #[eguis(hint = "More stupid, but faster AI in battles")]
    StupidAI,
}

#[derive(Default, Clone, Copy, PartialEq, Deserialize, Serialize, FromRepr, EguiStruct)]
#[eguis(prefix = "settings.SettingsServer")]
enum AIAdventure {
    #[default]
    #[eguis(hint = "Advanced, but slow AI, AI sees whole map (not recomended as AI for Alies)")]
    Nullkiller,
    VCAI,
}

///Same as bool but defaults to true
#[derive(Clone, Copy, Serialize, Deserialize, EguiStruct)]
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
