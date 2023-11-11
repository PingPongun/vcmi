/*
 * mod_manager.rs, part of VCMI engine
 * Mod & Downloads views, related data structures, mod download & managament
 *
 * Authors: listed in file AUTHORS in main folder
 *
 * License: GNU General Public License v2.0 or later
 * Full text of license available in license.txt file, in main folder
 *
 */
use anyhow::Context;
use atomic_enum::atomic_enum;
use egui::{
    Button, Checkbox, Color32, Grid, Image, Key, Layout, ProgressBar, ScrollArea, Sense, Ui, Widget,
};
use egui_toast::Toast;
use futures::Future;
use indexmap::IndexSet;
use parking_lot::{RwLock, RwLockReadGuard};
use rust_i18n::{t, ToStringI18N};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;
use std::time::Duration;
use strum::{EnumIter, IntoEnumIterator};

use crate::gui_primitives::DisplayGUI2;
use crate::gui_primitives::{DisplayGUI3, GAME_LANGUAGES};
use crate::icon;
use crate::utils::AsyncHandle::*;
use crate::utils::*;
use crate::{settings::Language, vcmi_launcher::*};

static MODS: RwLock<ModSettingsJson> = RwLock::new(ModSettingsJson::new());

#[derive(Default)]
pub struct ModMng {
    pub ops: ModOpsQueue,
    pub zoomed_screenshot: Option<String>,
    selected_mod: Option<ModPath>,
    pub highlighted_mod: Option<ModPath>,
    pub highlighted_scroll: bool,
    problems: bool,
    sort: ModSort,
    sort_rev: bool,
}
#[derive(Default, PartialEq)]
enum ModSort {
    #[default]
    Name,
    Selected,
    Enabled,
    Update,
    Type,
}
impl VCMILauncher {
    pub fn show_mods(&mut self, ui: &mut Ui) {
        if let Some(mods) = MODS.try_read_for(Duration::from_millis(50)) {
            let mng = &mut self.mod_mng;
            if let Some(selected) = &mng.selected_mod {
                if let Some(selected) = mods.get_mod(selected) {
                    if MOBILE_VIEW.load(Relaxed) {
                        //mobile view
                        selected.show_desc(ui, mng);
                    } else {
                        //desktop view
                        egui::SidePanel::right("mod_desc_panel")
                            .default_width(250.)
                            .show_inside(ui, |ui| selected.show_desc(ui, mng));
                        egui::CentralPanel::default().show_inside(ui, |ui| {
                            mods.show_list(ui, mng);
                        });
                    }
                } else {
                    mng.selected_mod = None;
                }
            } else {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    mods.show_list(ui, mng);
                });
            }
        } else {
            //this should be almost never seen, as self.mods shouldn't be locked for such a long time
            ui.centered_and_justified(|ui| ui.spinner());
        }
        if let Some(url) = &self.mod_mng.zoomed_screenshot {
            if !MOBILE_VIEW.load(Relaxed) {
                let url = url.clone();
                egui::Window::new("Screenshot viewer")
                    .title_bar(false)
                    .fixed_rect(ui.clip_rect())
                    .show(ui.ctx(), |ui| {
                        if Image::from_uri(url)
                            .show_loading_spinner(true)
                            .sense(Sense::click())
                            .ui(ui)
                            .clicked()
                        {
                            self.mod_mng.zoomed_screenshot = None;
                        }
                    });
            }
        }
    }

    pub fn show_downloads(&mut self, ui: &mut Ui) {
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                Grid::new(ui.next_auto_id()).show(ui, |ui| {
                    self.mod_mng
                        .ops
                        .iter_mut()
                        .filter(|op| {
                            op.op_type == ModOpType::Install || op.op_type == ModOpType::Update
                        })
                        .for_each(|op| op.show(ui));
                })
            });

        self.mod_mng.ops.retain(|op| !matches!(op.handle, Uninit)); //remove all operations with Uninit state
    }

    pub fn ongoing_ops(&mut self) -> bool {
        self.mod_mng.ops.iter_mut().any(|op| op.handle.is_running())
    }
    pub fn mods_not_ready(&mut self) -> bool {
        self.ongoing_ops() || self.mod_mng.problems
    }
}

mod mod_json {
    use super::*;

    //mod.json
    //eg. https://raw.githubusercontent.com/vcmi-mods/vcmi-extras/vcmi-1.4/mod.json
    #[derive(Clone, Debug, Deserialize, Serialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    #[allow(non_snake_case)]
    pub struct ModFile {
        pub name: String,
        pub description: String,
        pub mod_type: ModType,
        pub author: String,
        pub download_size: f32,
        pub contact: String,
        pub license_name: String,
        pub licenseURL: String,
        pub version: String,
        pub changelog: IndexMap<String, Vec<String>>,
        pub compatibility: CompatibilityVCMIVer,
        pub depends: IndexSet<ModPath>,
        pub conflicts: IndexSet<ModPath>,
        pub keep_disabled: bool,
        pub language: Language,
        translations: Vec<String>, //only used by translation mods
        #[serde(flatten)]
        pub maps: ModFileMaps,
    }
    #[derive(Clone, Debug, Default, Deserialize, Serialize)]
    #[serde(default)]
    pub struct ModFileTranslated {
        pub name: String,
        pub description: String,
        pub author: String,
        pub changelog: IndexMap<String, Vec<String>>,
        pub translations: Vec<String>,
        #[serde(flatten)]
        _extra: IndexMap<String, serde_json::Value>,
    }

    pub struct ModFileTranslatedRef<'a> {
        pub name: &'a String,
        pub description: &'a String,
        pub author: &'a String,
        pub changelog: &'a IndexMap<String, Vec<String>>,
        pub size: f32, //this does not fit here, but similar to others it should be taken from mod_file_update if possible
    }

    #[derive(Clone, Debug, Default, Deserialize, Serialize)]
    #[serde(default)]
    pub struct CompatibilityVCMIVer {
        min: String,
        max: String,
    }

    #[derive(Clone, Debug, Default, Serialize)]
    #[allow(non_snake_case)]
    pub struct ModFileMaps {
        ///mod content translation
        pub mod_translations: IndexMap<Language, ModFileTranslated>,
        ///including: settings, filesystem, factions,heroClasses,heroes,skills,creatures,artifacts,spells,objects,bonuses,terrains,roads,rivers,battlefields,obstacles,templates,scripts
        extra: IndexMap<String, serde_json::Value>,
    }

    #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize, EnumIter, ToStringI18N)]
    #[module(mod)]
    #[rustfmt::skip]
    pub enum ModType {
        AI, Artifacts, Creatures, Expansion, Graphical, Heroes, Interface, Maps, Mechanics, Music, Objects,
        #[default] Other, Skills, Sounds, Spells, Templates, Test, Town, Translation, Utility,
    }

    impl ModFile {
        pub fn update_available(&self, curr_ver: &str) -> bool {
            let s = self.version.split('.');
            let curr_ver = curr_ver.split('.');
            self.compatibility.satisfied() && s.gt(curr_ver)
        }
    }

    ///custom implmentation is here necessary as serde was not able to work correctly with two maps in struct
    impl<'de> Deserialize<'de> for ModFileMaps {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let map = IndexMap::<String, serde_json::Value>::deserialize(deserializer)?;
            let mut ret = Self::default();
            map.into_iter().for_each(|(key, val)| {
                if let Ok(translations) = serde_json::from_value(val.clone()) {
                    if let Ok(key) = serde_plain::from_str(&key) {
                        ret.mod_translations.insert(key, translations);
                    } else {
                        ret.extra.insert(key, val);
                    }
                } else {
                    ret.extra.insert(key, val);
                }
            });
            Ok(ret)
        }
    }

    impl CompatibilityVCMIVer {
        pub fn satisfied(&self) -> bool {
            let vcmi = env!("CARGO_PKG_VERSION").split('.');
            let min = self.min.split('.');
            let max = self.max.split('.');
            vcmi.clone().ge(min) && (max.ge(vcmi) || self.max.is_empty())
        }
    }
}
pub use mod_json::*;
/////////////////////////////////////////////////////////////
///////////////////Internal mod type/////////////////////////
/////////////////////////////////////////////////////////////
/// modSettings.json & runtime mod management
mod local {
    use super::*;
    //modSettings.json
    // {
    //     "activeMods" : {
    //         "magic-fader" : {
    //             "active" : false
    //         },
    //         "reworked-commanders" : {
    //             "mods" : {
    //                 "reworkedwindow" : {
    //                     "active" : false
    //                 }
    //             }
    //         }
    //     }
    // }
    #[derive(Debug, Default, Deserialize, Serialize)]
    #[serde(default, rename_all = "camelCase")]
    pub struct ModSettingsJson {
        pub active_mods: Mods,
        pub extra: IndexMap<String, serde_json::Value>,
    }

    #[derive(Debug, Default, Deserialize)]
    #[serde(default)]
    pub struct Mods(pub IndexMap<String, Mod>);

    #[derive(Debug, Default, Deserialize, Serialize)]
    #[serde(default)]
    pub struct Mod {
        pub active: AtomicModTriState,
        #[serde(skip_serializing_if = "String::is_empty")]
        checksum: String,
        validated: bool,
        #[serde(skip_serializing_if = "Mods::is_empty")]
        pub mods: Mods,
        #[serde(skip)]
        pub volatile: ModVolatile,
    }
    #[atomic_enum]
    #[derive(Default, PartialEq)]
    pub enum ModTriState {
        #[default]
        Uninstalled,
        Enabled,
        Disabled,
    }
    use ModTriState::*;

    #[derive(Clone, Debug, Default, PartialEq, ToStringI18N)]
    #[module(mod)]
    pub enum ModSource {
        #[default]
        Unknown,
        MainRepository,
        ExtraRepository,
    }

    #[derive(Debug, Default)]
    pub struct ModVolatile {
        pub path: ModPath,
        pub disk_path: PathBuf,
        pub dependants: ModRelations,
        depends: ModRelations,
        conflicts: ModRelations,
        conflict_vcmi: bool, //wrong vcmi version
        selected: AtomicBool,
        unfolded: AtomicBool, //has meaning only when there are some submods
        pub ongoing_op: AtomicBool,
        pub src: ModSource,
        pub mod_file: ModFile,
        pub mod_file_update: Option<ModFile>,
        pub mod_download_url: String,
        pub screenshots: Vec<String>,
    }

    #[derive(Debug, Default)]
    pub struct ModRelations(RwLock<_ModRelations>);

    #[derive(Debug, Default)]
    struct _ModRelations {
        active: IndexSet<ModPath>,
        inactive: IndexSet<ModPath>,
    }

    #[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
    pub struct ModPath(pub Vec<String>);

    #[derive(PartialEq, PartialOrd, EnumIter)]
    enum ModStateEnabled {
        Disabled,
        Conflict,
        SubModConflict,
        Enabled,
        None,
    }
    #[derive(PartialEq, PartialOrd, EnumIter)]
    enum ModStateUpdate {
        Install,
        Update,
        Processing,
        None,
    }

    impl ModSettingsJson {
        pub const fn new() -> Self {
            Self {
                active_mods: Mods(hashmap()),
                extra: hashmap(),
            }
        }
        pub fn save() {
            let data: &Self = &*MODS.read_recursive();
            save_file(&get_dirs().settings_mod, data);
        }
    }
    impl Deref for ModSettingsJson {
        type Target = Mods;

        fn deref(&self) -> &Self::Target {
            &self.active_mods
        }
    }
    impl DerefMut for ModSettingsJson {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.active_mods
        }
    }

    impl Mods {
        fn is_empty(&self) -> bool {
            self.0.is_empty()
        }
        fn _show<'a>(&'a self, ui: &'a mut Ui, indent_level: usize, mng: &mut ModMng) -> bool {
            //this sorting implementation is not efficient (but it's lock-less)
            let mut ret = false;

            macro_rules! sort {
                ($iter: expr,$f:ident, $k:ident=>$key:expr) => {
                    $iter.$f((),|_,k| {
                        self.0.iter()
                            .filter(|(_, $k)| $key == k)
                            .for_each(|(_, mod_data)| { ret |= mod_data.show_list_elem(ui, indent_level, mng); })
                    })
                };
                ($iter: expr, $k:ident=>$key:expr) => {
                    if mng.sort_rev {
                        sort!($iter,rfold, $k=>$key)
                    }else{
                        sort!($iter,fold, $k=>$key)
                    }
                };
            }
            macro_rules! sort_name {
                ($f:ident) => {
                    self.0.iter().$f((), |_, (_, mod_data)| {
                        ret |= mod_data.show_list_elem(ui, indent_level, mng);
                    })
                };
                () => {
                    if mng.sort_rev {
                        sort_name!(rfold)
                    } else {
                        sort_name!(fold)
                    }
                };
            }

            use ModSort::*;
            match mng.sort {
                Name => sort_name!(),
                Selected => sort!([true, false].into_iter(),m=>m.volatile.selected.load(Relaxed)),
                Enabled => sort!(ModStateEnabled::iter(), m=>m.state_enabled()),
                Update => sort!(ModStateUpdate::iter(),m=>m.state_update()),
                Type => sort!(ModType::iter(),m=>m.volatile.mod_file.mod_type),
            }
            ret
        }

        fn show_buttons_top(&self, ui: &mut Ui, mng: &mut ModMng) {
            ui.horizontal_wrapped(|ui| {
                ui.group(|ui| {
                    ui.label(t!("mod.select.Selected") + ":");
                    if ui.small_button(t!("_common.Disable")).clicked() {
                        self.for_each(true, true, &mut |m| {
                            _ = m
                                .active
                                .compare_exchange(Enabled, Disabled, Relaxed, Relaxed);
                            m.volatile.unfolded.store(false, Relaxed);
                            m.conflicts_update();
                        })
                    }
                    if ui.small_button(t!("_common.Enable")).clicked() {
                        self.for_each(true, true, &mut |m| {
                            _ = m
                                .active
                                .compare_exchange(Disabled, Enabled, Relaxed, Relaxed);
                            m.conflicts_update();
                        })
                    }
                    if ui.small_button(t!("_common.Update")).clicked() {
                        self.for_each(false, true, &mut |m| {
                            mng.ops.update(m.volatile.path.clone());
                        })
                    }
                    if ui.small_button(t!("mod.Install")).clicked() {
                        self.for_each(false, true, &mut |m| {
                            mng.ops.install(m.volatile.path.clone());
                        })
                    }
                    if ui.small_button(t!("mod.Uninstall")).clicked() {
                        self.for_each(false, true, &mut |m| {
                            mng.ops.uninstall(
                                m.volatile.path.clone(),
                                m.volatile.mod_download_url.is_empty(),
                            );
                        })
                    }
                });

                ui.end_row();
                ui.style_mut().wrap = Some(false);
                ui.group(|ui| {
                    ui.label(t!("mod.select.Select") + ":");
                    if ui.small_button(t!("mod.select.All")).clicked() {
                        self.for_each(true, false, &mut |m| {
                            m.volatile.selected.store(true, Relaxed)
                        })
                    }
                    if ui.small_button(t!("mod.select.None")).clicked() {
                        self.for_each(true, false, &mut |m| {
                            m.volatile.selected.store(false, Relaxed)
                        })
                    }
                });
                if ui.button(t!("mod.Fetch remote")).clicked() {
                    mng.ops.fetch_updates();
                }
            });
        }
        pub fn show_list(&self, ui: &mut Ui, mng: &mut ModMng) {
            mng.problems = false;
            ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                self.show_buttons_top(ui, mng);
                Grid::new(ui.next_auto_id())
                    .striped(true)
                    .min_col_width(0.0)
                    .show(ui, |ui| {
                        let mut column = |name: String, col: ModSort| {
                            // |ui: &mut Ui, name: &str, col: ModSort, mng: &mut ModMng| {
                            let name = match (col == mng.sort, mng.sort_rev) {
                                (true, true) => name + "⏶",
                                (true, false) => name + "⏷",
                                (false, _) => name,
                            };
                            //make buttons expand for full column width
                            ui.centered_and_justified(|ui| {
                                if ui.button(name).clicked() {
                                    mng.sort_rev = col == mng.sort && !mng.sort_rev;
                                    mng.sort = col;
                                };
                            });
                        };
                        column(t!("mod.Name"), ModSort::Name);
                        column("".to_string(), ModSort::Selected);
                        column("".to_string(), ModSort::Enabled);
                        column("".to_string(), ModSort::Update);
                        column(t!("mod.Mod type"), ModSort::Type);
                        ui.end_row();
                        if self._show(ui, 0, mng) {
                            ModSettingsJson::save();
                        }
                    })
            });
        }

        pub fn load_from_disk(path: &Path, mod_path: &ModPath) -> Self {
            let mut ret = Self::default();
            if let Ok(read_dir) = path.read_dir() {
                for entry in read_dir {
                    if let Ok(entry) = entry {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                let name = entry.file_name().to_string_lossy().to_lowercase();
                                if let Some(mod_) = Mod::load_from_disk(
                                    mod_path.clone(),
                                    &entry.path(),
                                    name.clone(),
                                ) {
                                    ret.0.insert(name, mod_);
                                }
                            }
                        }
                    }
                }
            }
            ret
        }
        pub fn mask(&mut self, mask: Self) {
            mask.0.into_iter().for_each(|(name, m)| {
                if let Some(mod_) = self.0.get_mut(&name) {
                    mod_.active = m.active;
                    mod_.checksum = m.checksum;
                    mod_.validated = m.validated;
                    mod_.mods.mask(m.mods);
                }
            });
        }

        pub fn get_mod<'a>(&'a self, path: &ModPath) -> Option<&'a Mod> {
            if let Some(mod_) = self.0.get(&path.0[0]) {
                path.0
                    .iter()
                    .skip(1)
                    .try_fold(mod_, |mod_, name| match mod_.mods.0.get(name) {
                        Some(mod_) => Some(mod_),
                        None => None,
                    })
            } else {
                None
            }
        }
        pub fn conflicts_update(&self) {
            self.for_each(false, false, &mut |m| m.conflicts_update());
        }
        pub fn sort(&mut self) {
            self.0
                .sort_unstable_by(|_, x, _, y| x.get_name().cmp(&y.get_name()));
        }
        pub fn for_each<OP>(&self, recursive: bool, selected_only: bool, op: &mut OP)
        where
            OP: FnMut(&Mod),
        {
            self.0.iter().for_each(|(_, m)| {
                if m.volatile.selected.load(Relaxed) || !selected_only {
                    op(m);
                }
                if recursive {
                    m.mods.for_each(recursive, selected_only, op)
                }
            });
        }
    }

    impl Serialize for Mods {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut map = serializer.serialize_map(None)?;
            for (k, v) in self
                .0
                .iter()
                .filter(|(_, v)| v.active.load(Relaxed) != ModTriState::Uninstalled)
            //we are skipping uninstalled mods
            {
                map.serialize_entry(k, v)?;
            }
            map.end()
        }
    }
    impl Mod {
        pub fn new(name: &String, online_mod: &ModUpdatesListElem) -> Self {
            let mod_file = online_mod.mod_file.clone().unwrap();
            let conflict_vcmi = !mod_file.compatibility.satisfied();
            let conflicts = ModRelations::new(&mod_file.conflicts);
            let depends = ModRelations::new(&mod_file.conflicts);
            Self {
                volatile: ModVolatile {
                    path: ModPath(vec![name.clone()]),
                    depends,
                    conflicts,
                    conflict_vcmi,
                    mod_file,
                    mod_download_url: online_mod.download.clone(),
                    screenshots: online_mod.screenshots.clone(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }
        fn state_enabled(&self) -> ModStateEnabled {
            if self.active.installed() {
                if !&self.active.enabled() {
                    ModStateEnabled::Disabled
                } else if self.conflicted() {
                    ModStateEnabled::Conflict
                } else if self.conflicted_submods() {
                    ModStateEnabled::SubModConflict
                } else {
                    ModStateEnabled::Enabled
                }
            } else {
                ModStateEnabled::None
            }
        }
        fn state_update(&self) -> ModStateUpdate {
            if self.volatile.ongoing_op.load(Relaxed) {
                ModStateUpdate::Processing
            } else if !self.active.installed() {
                ModStateUpdate::Install
            } else if self.volatile.mod_file_update.is_some() {
                ModStateUpdate::Update
            } else {
                ModStateUpdate::None
            }
        }
        pub fn show_list_elem(&self, ui: &mut Ui, indent_level: usize, mng: &mut ModMng) -> bool {
            let mut ret = false;
            ui.horizontal(|ui| {
                let mod_name = self.get_name();
                for _ in 0..indent_level {
                    ui.separator();
                }
                if !self.mods.is_empty() {
                    let icon = if self.volatile.unfolded.load(Relaxed) {
                        "⏷"
                    } else {
                        "⏵"
                    };
                    if ui
                        .add_enabled(
                            self.active.enabled(),
                            Button::new(icon).frame(false).small(),
                        )
                        .clicked()
                    {
                        self.volatile.unfolded.fetch_xor(true, Relaxed);
                    }
                }

                let highlighted = if let Some(highlighted_mod) = &mng.highlighted_mod {
                    *highlighted_mod == self.volatile.path
                } else {
                    false
                };
                if highlighted && mng.highlighted_scroll {
                    ui.scroll_to_cursor(Some(egui::Align::Center));
                    mng.highlighted_scroll = false;
                }
                if ui.selectable_label(highlighted, mod_name).clicked() {
                    mng.selected_mod = Some(self.volatile.path.clone());
                }
            });

            let mut val = self.volatile.selected.load(Relaxed);
            if Checkbox::without_text(&mut val).ui(ui).changed() {
                self.volatile.selected.store(val, Relaxed);
            }

            let state_enabled = self.state_enabled();
            mng.problems |= state_enabled == ModStateEnabled::Conflict
                || state_enabled == ModStateEnabled::SubModConflict;
            let toggle = match state_enabled {
                ModStateEnabled::Disabled => icon!(ui, "../icons/mod-disabled.png"),
                ModStateEnabled::Conflict => icon!(ui, "../icons/mod-invalid.png"),
                ModStateEnabled::SubModConflict => icon!(ui, "../icons/mod-invalid-sub.png"),
                ModStateEnabled::Enabled => icon!(ui, "../icons/mod-enabled.png"),
                ModStateEnabled::None => ui.label(""),
            };
            if toggle.clicked() {
                self.toggle();
                ret = true;
            }

            match self.state_update() {
                ModStateUpdate::Install => {
                    if icon!(ui, "../icons/mod-download.png").clicked() {
                        mng.ops.install(self.volatile.path.clone());
                    }
                }
                ModStateUpdate::Update => {
                    if icon!(ui, "../icons/mod-update.png").clicked() {
                        mng.ops.update(self.volatile.path.clone());
                    }
                }
                ModStateUpdate::Processing => _ = ui.spinner(),
                ModStateUpdate::None => _ = ui.label(""),
            }

            let mod_file = &self.volatile.mod_file;
            ui.label(mod_file.mod_type.to_string_i18n());
            ui.label(mod_file.version.clone());

            ui.end_row();

            if !self.mods.is_empty() && self.volatile.unfolded.load(Relaxed) {
                ret |= self.mods._show(ui, indent_level + 1, mng);
            }
            ret
        }

        pub fn show_desc<'a>(&'a self, ui: &'a mut Ui, mng: &mut ModMng) {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .drag_to_scroll(true)
                .show(ui, |ui| {
                    ui.with_layout(Layout::top_down(egui::Align::RIGHT), |ui| {
                        if ui.button("🗙").clicked() || ui.input(|i| i.key_pressed(Key::Escape))
                        //add also vertical swipe here
                        {
                            mng.selected_mod = None;
                        }
                    });
                    let transl = self.get_translated();
                    let mf = &self.volatile.mod_file;
                    transl.name.show(ui, &t!("mod.Name"));
                    mf.version.show(ui, &t!("mod.Version"));
                    mf.mod_type.to_string_i18n().show(ui, &t!("mod.Mod type"));
                    self.volatile
                        .src
                        .to_string_i18n()
                        .show(ui, &t!("mod.Mod source"));
                    transl.author.show(ui, &t!("mod.Author"));
                    mf.contact.show(ui, &t!("mod.Contact"));
                    format!("{:2}", transl.size).show(ui, &t!("mod.Download size [MB]"));
                    transl.description.show(ui, &t!("mod.Description"));
                    if !mf.license_name.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong(t!("mod.License name") + ":");
                            ui.hyperlink_to(&mf.license_name, &mf.licenseURL);
                        });
                    }

                    //conflicts & deps
                    {
                        if self.volatile.conflict_vcmi {
                            ui.colored_label(Color32::RED, t!("mod.Incompatible VCMI version!"));
                        }
                        let red = Some(Color32::RED);
                        let conf = &self.volatile.conflicts.0.read().active;
                        conf.show3(ui, t!("mod.Conflicting mods"), red, mng);

                        let deps = &self.volatile.depends.0.read();
                        deps.inactive
                            .show3(ui, t!("mod.Missing dependencies"), red, mng);
                        deps.active.show3(ui, t!("mod.Dependencies"), None, mng);
                    }

                    mf.maps
                        .mod_translations
                        .keys()
                        .map(|x| x.translated().to_owned())
                        .show(ui, t!("mod.Available languages"));

                    //buttons
                    ui.horizontal_wrapped(|ui| {
                        if self.active.installed() {
                            let toggle_label = if self.active.enabled() {
                                t!("_common.Disable")
                            } else {
                                t!("_common.Enable")
                            };
                            if ui.button(toggle_label).clicked() {
                                self.toggle();
                                ModSettingsJson::save();
                            }

                            if self.volatile.ongoing_op.load(Relaxed) {
                                ui.spinner();
                            } else {
                                if self.volatile.mod_file_update.is_some()
                                    && ui.button(t!("_common.Update")).clicked()
                                {
                                    mng.ops.update(self.volatile.path.clone());
                                }

                                if self.volatile.path.is_top()
                                    && ui.button(t!("mod.Uninstall")).clicked()
                                {
                                    mng.ops.uninstall(
                                        self.volatile.path.clone(),
                                        self.volatile.mod_download_url.is_empty(),
                                    );
                                }
                            }
                        } else {
                            if self.volatile.ongoing_op.load(Relaxed) {
                                ui.spinner();
                            } else if ui.button(t!("mod.Install")).clicked() {
                                mng.ops.install(self.volatile.path.clone());
                            }
                        }
                    });

                    transl.changelog.show(ui, &t!("mod.Changelog"));
                    let width = ui.available_size()[0];
                    self.volatile.screenshots.iter().for_each(|url| {
                        ui.separator();
                        let img = Image::from_uri(url)
                            .show_loading_spinner(true)
                            .max_width(width)
                            .fit_to_original_size(1.0);
                        if MOBILE_VIEW.load(Relaxed) {
                            img.ui(ui);
                        } else {
                            if img.sense(Sense::click()).ui(ui).clicked() {
                                mng.zoomed_screenshot = Some(url.clone());
                            }
                        }
                    });
                });
        }
        pub fn toggle(&self) {
            self.active.toggle();
            self.volatile.unfolded.store(false, Relaxed);
            self.conflicts_update();
        }
        pub fn conflicts_update(&self) {
            {
                //serch for dependencies
                let mut deps = self.volatile.depends.0.write();
                let mut active = Vec::new();
                deps.inactive.retain(|dep| {
                    if let Ok(mod_) = dep.get_mod() {
                        if mod_.active.enabled() {
                            active.push(dep.clone());
                            false //dependency is satisfied
                        } else {
                            true //retain in inactive, as dependency is inactive
                        }
                    } else {
                        true //retain in inactive, as dependency is not available
                    }
                });
                deps.active.extend(active);
            }
            if self.active.enabled() {
                self.conflicts_update_enable();
            } else {
                self.conflicts_update_disable();
            }
        }
        pub fn conflicts_update_disable(&self) {
            let s = &self.volatile;
            s.depends.for_each(|m| m.dependants.move2inactive(&s.path));
            s.dependants.for_each(|m| m.depends.move2inactive(&s.path));
            s.conflicts.for_each(|m| m.conflicts.move2inactive(&s.path));
            self.mods.for_each(false, false, &mut |submod| {
                submod.conflicts_update_disable()
            });
        }
        pub fn conflicts_update_enable(&self) {
            let s = &self.volatile;
            s.depends.for_each(|m| m.dependants.move2active(&s.path));
            s.dependants.for_each(|m| m.depends.move2active(&s.path));
            s.conflicts.for_each(|m| m.conflicts.move2active(&s.path));
            self.mods.conflicts_update();
        }

        pub fn get_name<'a>(&'a self) -> &str {
            let mod_file = &self.volatile.mod_file;
            let mod_name = if mod_file.name.is_empty() {
                self.volatile.path.top()
            } else {
                mod_file.name.as_str()
            };
            if let Some(translations) = mod_file.maps.mod_translations.get(&LANGUAGE.get()) {
                if translations.name.is_empty() {
                    mod_name
                } else {
                    translations.name.as_str()
                }
            } else {
                mod_name
            }
        }
        pub fn get_translated<'a>(&'a self) -> ModFileTranslatedRef {
            let mod_file = if let Some(update_file) = &self.volatile.mod_file_update {
                update_file
            } else {
                &self.volatile.mod_file
            };
            let mut ret = ModFileTranslatedRef {
                name: &mod_file.name,
                description: &mod_file.description,
                author: &mod_file.author,
                changelog: &mod_file.changelog,
                size: mod_file.download_size,
            };
            if let Some(translations) = mod_file.maps.mod_translations.get(&LANGUAGE.get()) {
                if translations.name.is_empty() {
                    ret.name = &translations.name;
                };
                if translations.description.is_empty() {
                    ret.description = &translations.description;
                };
                if translations.author.is_empty() {
                    ret.author = &translations.author;
                };
                if translations.changelog.is_empty() {
                    ret.changelog = &translations.changelog;
                };
            }
            ret
        }
        pub fn conflicted(&self) -> bool {
            self.volatile.conflict_vcmi
                || !self.volatile.depends.0.read_recursive().inactive.is_empty()
                || !self.volatile.conflicts.0.read_recursive().active.is_empty()
        }
        pub fn conflicted_submods(&self) -> bool {
            self.conflicted()
                || self
                    .mods
                    .0
                    .iter()
                    .filter(|(_, mod_)| mod_.active.enabled())
                    .any(|(_, mod_)| mod_.conflicted_submods())
        }
        pub fn load_from_disk(mod_path: ModPath, path: &Path, name: String) -> Option<Self> {
            let path_json = path.join("mod.json");
            if !path_json.exists() {
                log::error!("Unable to load mod json: {};", path_json.to_string_lossy(),);
                return None;
            }
            let mut mod_: Mod = Default::default();
            let mod_file: ModFile = load_file_mod(&path_json);
            mod_.active = bool::into(!mod_file.keep_disabled);
            mod_.mods = {
                let mv = &mut mod_.volatile;
                mv.conflict_vcmi = !mod_file.compatibility.satisfied();
                mv.conflicts.0.get_mut().inactive = mod_file.conflicts.clone();
                mv.depends.0.get_mut().inactive = mod_file.depends.clone();

                // // use this if download link is stored in modfile (this will allow to download updates without refreshing repos)
                // let upath = &entry.path().join("UPDATE_mod.json");
                // if upath.exists() {
                //     let mod_file_update: ModFile = load_file_mod(&upath);
                //     if mod_file_update.update_available(&mod_file.version) {
                //         mv.mod_file_update = Some(mod_file_update);
                //     }
                // }
                if ModType::Translation == mod_file.mod_type {
                    let lang = mod_file.language.to_string();
                    GAME_LANGUAGES.write().entry(lang.clone()).or_insert(lang);
                }
                mv.mod_file = mod_file;

                let mut submods_path = path.join("mods");
                if !submods_path.exists() {
                    submods_path = path.join("Mods");
                }
                mv.path = mod_path.clone();
                mv.path.0.push(name.clone());

                Mods::load_from_disk(&submods_path, &mv.path)
            };
            Some(mod_)
        }
    }

    impl AtomicModTriState {
        pub fn enabled(&self) -> bool {
            self.load(Relaxed) == Enabled
        }
        pub fn installed(&self) -> bool {
            self.load(Relaxed) != Uninstalled
        }
        pub fn toggle(&self) {
            let new = match self.load(Relaxed) {
                Enabled => Disabled,
                Disabled => Enabled,
                Uninstalled => Uninstalled,
            };
            self.store(new, Relaxed);
        }
    }
    impl Default for AtomicModTriState {
        fn default() -> Self {
            Self(Default::default())
        }
    }
    impl<'de> Deserialize<'de> for AtomicModTriState {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let b = bool::deserialize(deserializer)?;
            Ok(b.into())
        }
    }
    impl Serialize for AtomicModTriState {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            (self.load(Relaxed) == Enabled).serialize(serializer)
        }
    }
    impl From<bool> for AtomicModTriState {
        fn from(value: bool) -> Self {
            AtomicModTriState::new(if value { Enabled } else { Disabled })
        }
    }

    impl ModRelations {
        fn new(v: &IndexSet<ModPath>) -> Self {
            ModRelations(RwLock::new(_ModRelations {
                active: IndexSet::new(),
                inactive: v.clone(),
            }))
        }
        fn for_each<F: FnMut(&ModVolatile)>(&self, mut f: F) {
            let mods = &MODS.read_recursive().active_mods;
            let s = self.0.read_recursive();
            s.active.iter().chain(s.inactive.iter()).for_each(|path| {
                if let Some(mod_) = mods.get_mod(path) {
                    f(&mod_.volatile);
                }
            });
        }
        fn move2active(&self, path: &ModPath) {
            let mut s = self.0.write();
            s.inactive.shift_remove(path);
            if s.active.insert(path.clone()) {
                s.active
                    .sort_unstable_by(|x, y| x.to_string().cmp(&y.to_string()));
            }
        }
        fn move2inactive(&self, path: &ModPath) {
            let mut s = self.0.write();
            s.active.shift_remove(path);
            if s.inactive.insert(path.clone()) {
                s.inactive
                    .sort_unstable_by(|x, y| x.to_string().cmp(&y.to_string()));
            }
        }
    }
    impl ModPath {
        pub fn new(from: &str) -> Self {
            ModPath([from.to_string()].to_vec())
        }
        pub fn get_mod(&self) -> anyhow::Result<parking_lot::MappedRwLockReadGuard<'_, Mod>> {
            RwLockReadGuard::try_map(MODS.read_recursive(), |rwg| rwg.active_mods.get_mod(&self))
                .map_err(|_| anyhow::Error::msg("Mod with requested mod path not found!"))
        }
        pub fn is_top(&self) -> bool {
            self.0.len() == 1
        }
        pub fn top(&self) -> &str {
            if let Some(s) = self.0.first() {
                s.as_ref()
            } else {
                ""
            }
        }
        pub fn unfold_ancestors(&self) {
            let mut mods = &MODS.read_recursive().active_mods;
            for segment in &self.0 {
                if let Some(mod_) = mods.0.get(segment) {
                    mods = &mod_.mods;
                    mod_.volatile.unfolded.store(true, Relaxed);
                } else {
                    return;
                }
            }
        }
    }
    impl Display for ModPath {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mods = MODS.read_recursive();
            let mut mods = &mods.active_mods;
            let path: Vec<String> = self
                .0
                .iter()
                .map(|name| match mods.0.get(name) {
                    Some(mod_) => {
                        mods = &mod_.mods;
                        mod_.get_name().to_string()
                    }
                    None => name.clone(),
                })
                .collect();
            write!(f, "{}", path.join("::"))
        }
    }
    impl<'de> Deserialize<'de> for ModPath {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let string = String::deserialize(deserializer)?;
            Ok(ModPath(
                string.split('.').map(|x| x.to_lowercase()).collect(),
            ))
        }
    }
    impl Serialize for ModPath {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.0.join(".").serialize(serializer)
        }
    }
}
pub use local::*;
mod ops {

    use super::*;

    #[derive(Debug, Default)]
    pub struct ModOpsQueue(Vec<ModOp>);

    #[derive(Debug, Default)]
    pub struct ModOp {
        pub op_type: ModOpType,
        pub path: ModPath,
        pub handle: AsyncHandle<(), ModOpProgress>,
    }

    #[derive(Clone, Debug, Default, PartialEq, ToStringI18N)]
    #[module(mod)]
    pub enum ModOpType {
        #[default]
        InitMods,
        Install,
        Update,
        Uninstall,
        FindUpdates,
    }
    use reqwest::IntoUrl;
    use ModOpType::*;

    #[atomic_enum]
    #[derive(Default, PartialEq, ToStringI18N)]
    #[module(mod)]
    pub enum ModSubOp {
        #[default]
        Downloading,
        Unpacking,
        Processing,
    }
    #[derive(Debug)]
    pub struct ModOpProgress {
        downloaded: AtomicUsize,
        to_download: AtomicUsize,
        sub_op: AtomicModSubOp,
    }

    impl ModOpProgress {
        pub fn new(to_download: f32) -> Arc<Self> {
            Arc::new(Self {
                sub_op: AtomicModSubOp::new(Default::default()),
                downloaded: Default::default(),
                to_download: AtomicUsize::new((to_download * 1_000_000.0) as usize),
            })
        }
        pub fn dummy() -> Arc<Self> {
            Self::new(0.0)
        }
        pub fn show(&self, ui: &mut Ui) {
            let downloaded = self.downloaded.load(Relaxed) as f32;
            let max = self.to_download.load(Relaxed) as f32;
            let sub_op = self.sub_op.load(Relaxed);
            ui.horizontal(|ui| {
                ui.label(sub_op.to_string_i18n());
                match sub_op {
                    ModSubOp::Downloading => {
                        ui.label(format!(
                            "{:>5.2}/{:>5.2} MB",
                            downloaded / 1_000_000.,
                            max / 1_000_000.
                        ));
                        ProgressBar::new(downloaded / max).animate(true).ui(ui);
                    }
                    _ => _ = ui.spinner(),
                }
            });
        }
        pub fn add_downloaded(&self, rhs: usize) {
            let downloaded = self.downloaded.load(Relaxed) + rhs;
            let max = self.to_download.load(Relaxed);
            if max < downloaded {
                self.to_download.store(downloaded, Relaxed);
            }
            self.downloaded.store(downloaded, Relaxed);
        }
    }

    impl ModOp {
        pub fn show(&mut self, ui: &mut Ui) {
            self.handle.fetch_handle();
            ui.label(self.path.to_string());
            match &self.handle {
                Uninit => _ = ui.spinner(), //it should be rather imposible to be here
                Running(_, progress) => progress.show(ui),
                Finished(Ok(())) => _ = ui.colored_label(Color32::GREEN, t!("_common.Finished")),
                Finished(Err(err)) => {
                    _ = ui
                        .colored_label(Color32::RED, t!("_common.Error"))
                        .on_hover_text(format!("{:#}", err))
                }
            };
            if ui.button("🗙").clicked() {
                if let Running(handle, _) = &self.handle {
                    handle.abort();
                }
                self.handle = Uninit;
            }
            ui.end_row();
        }
    }
    impl ModOpsQueue {
        fn run<F>(
            &mut self,
            op: ModOpType,
            mod_path: ModPath,
            progress: Arc<ModOpProgress>,
            success_msg: String,
            err_toast: String,
            future: F,
        ) where
            F: Future<Output = anyhow::Result<()>> + Send + 'static,
        {
            self.push(Default::default());
            self.last_mut().unwrap().op_type = op;
            self.last_mut().unwrap().path = mod_path;
            self.last_mut().unwrap().handle.run(progress, async move {
                match future.await {
                    Ok(_) => {
                        log::info!("{}", success_msg);
                        Ok(())
                    }
                    Err(err) => {
                        Toast::error(err_toast.clone());
                        log::error!("{:#}", err);
                        Err(err)
                    }
                }
            });
        }
        fn run_simple<F>(&mut self, op: ModOpType, err_toast: String, future: F)
        where
            F: Future<Output = anyhow::Result<()>> + Send + 'static,
        {
            self.run(
                op.clone(),
                Default::default(),
                ModOpProgress::dummy(),
                format!("Mod op: {:?} finished succesfully!", op),
                err_toast,
                future,
            );
        }
        fn run_mod<F, FC>(
            &mut self,
            op: ModOpType,
            mod_path: ModPath,
            progress: Arc<ModOpProgress>,
            ok_toast: String,
            err_toast: String,
            checks: FC,
            future: F,
        ) where
            FC: Fn(&Mod) -> bool,
            F: Future<Output = anyhow::Result<()>> + Send + 'static,
        {
            if let Some(mod_) = mod_path.clone().get_mod().ok() {
                if mod_.volatile.ongoing_op.swap(true, Relaxed) {
                    //there is already some op ongoing on this mod
                    return;
                } else if !checks(&mod_) {
                    //checks failed
                    mod_.volatile.ongoing_op.store(false, Relaxed);
                    return;
                }
                mod_.conflicts_update_disable();
                self.run(
                    op.clone(),
                    mod_path.clone(),
                    progress,
                    format!("Mod op: {:?} on {} finished succesfully!", op, mod_path),
                    err_toast,
                    async move {
                        future.await?;
                        if let Some(mod_) = mod_path.get_mod().ok() {
                            mod_.volatile.ongoing_op.store(false, Relaxed);
                        }
                        Toast::success(ok_toast + &format!("({})", mod_path.to_string()));
                        Ok(())
                    },
                );
            }
        }
        pub fn init_mods(&mut self, update_on_start: bool) {
            self.run_simple(
                InitMods,
                t!("toasts.mod.Mod initialization failed!"),
                async move {
                    let file: ModSettingsJson =
                        load_file_settings(&get_dirs().settings_mod.clone());
                    let mut loaded = ModSettingsJson {
                        active_mods: Mods::load_from_disk(
                            &get_dirs().mods.clone(),
                            &Default::default(),
                        ),
                        extra: file.extra,
                    };
                    loaded.active_mods.mask(file.active_mods);

                    loaded.sort();
                    *MODS.write() = loaded;
                    MODS.read().conflicts_update();
                    log::info!("Mod list loaded from disk!",);
                    if update_on_start {
                        Self::_fetch_updates().await?
                    }
                    Ok(())
                },
            );
        }
        pub fn fetch_updates(&mut self) {
            self.run_simple(
                FindUpdates,
                t!("toasts.mod.Mod updates check failed!"),
                Self::_fetch_updates(),
            );
        }
        pub fn uninstall(&mut self, mod_path: ModPath, full: bool) {
            self.run_mod(
                Uninstall,
                mod_path.clone(),
                ModOpProgress::dummy(),
                t!("toasts.mod.Mod uninstalled"),
                t!("toasts.mod.Mod uninstall failed"),
                |m| m.active.installed(),
                async move {
                    let name_mod = mod_path.0.first().unwrap();
                    if full {
                        MODS.write().0.shift_remove(name_mod);
                    } else {
                        _ = mod_path
                            .get_mod()
                            .map(|m| m.active.store(ModTriState::Uninstalled, Relaxed));
                    }
                    Self::_remove_mod(name_mod);
                    Ok(())
                },
            );
        }
        pub fn install(&mut self, mod_path: ModPath) {
            self._install(
                Install,
                mod_path,
                t!("toasts.mod.Mod installed"),
                t!("toasts.mod.Mod install failed"),
                |m| {
                    !m.active.installed()
                        && m.volatile.path.is_top()
                        && !m.volatile.mod_download_url.is_empty()
                },
            );
        }
        pub fn update(&mut self, mod_path: ModPath) {
            self._install(
                Update,
                mod_path,
                t!("toasts.mod.Mod updated"),
                t!("toasts.mod.Mod update failed"),
                |m| {
                    m.active.installed()
                        && m.volatile.path.is_top()
                        && !m.volatile.mod_download_url.is_empty()
                        && m.volatile.mod_file_update.is_some()
                },
            );
        }
        fn _install<FC>(
            &mut self,
            op: ModOpType,
            mod_path: ModPath,
            ok_toast: String,
            err_toast: String,
            checks: FC,
        ) where
            FC: Fn(&Mod) -> bool,
        {
            let name_mod = mod_path.0.first().unwrap().clone();
            let (progress, url);
            {
                if let Ok(mod_) = mod_path.get_mod() {
                    progress = ModOpProgress::new(mod_.volatile.mod_file.download_size);
                    url = mod_.volatile.mod_download_url.clone();
                } else {
                    return;
                }
            };
            self.run_mod(
                op,
                mod_path.clone(),
                progress.clone(),
                ok_toast,
                err_toast,
                checks,
                async move {
                    //Download
                    let response = REQWEST.get(url.clone()).send().await;
                    let mut response =
                        response.context(format!("Unable to download file from: {}", url))?;
                    if let Some(len) = response.content_length() {
                        progress.to_download.store(len as usize, Relaxed);
                    }
                    let mut downloaded = Vec::with_capacity(progress.to_download.load(Relaxed));
                    while let Ok(Some(chunk)) = response.chunk().await {
                        progress.add_downloaded(chunk.len());
                        downloaded.extend_from_slice(&chunk);
                    }

                    //Extract
                    progress.sub_op.store(ModSubOp::Unpacking, Relaxed);
                    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(downloaded))?;
                    let name_in_zip: &Path = zip.file_names().next().unwrap_or_default().as_ref();
                    let name_in_zip = name_in_zip.iter().next().unwrap();
                    let target_dir = get_dirs().mods.join(name_mod.clone());
                    let extracted_dir_top = get_dirs().mods.join(name_in_zip);
                    let mut extracted_dir_mod = extracted_dir_top.clone();

                    zip.extract(get_dirs().mods.clone()).context(format!(
                        "Unable to extract archive into: {}",
                        get_dirs().mods.to_string_lossy(),
                    ))?;

                    //rename & move extracted
                    progress.sub_op.store(ModSubOp::Processing, Relaxed);
                    Self::_remove_mod(&name_mod); //mainly used when updating, but also usefull if there is some junk left from previous installs

                    if let Ok(read_dir) = extracted_dir_top.read_dir() {
                        for entry in read_dir {
                            if let Ok(entry) = entry {
                                if entry.file_name() == "mod.json" {
                                    extracted_dir_mod = extracted_dir_top.clone();
                                    break;
                                }
                                if entry.path().join("mod.json").exists() {
                                    extracted_dir_mod = entry.path();
                                }
                            }
                        }
                    }
                    std::fs::rename(extracted_dir_mod.clone(), target_dir.clone()).context(
                        format!(
                            "Failed to rename extracted: {} into: {}",
                            extracted_dir_mod.to_string_lossy(),
                            target_dir.to_string_lossy(),
                        ),
                    )?;
                    _ = std::fs::remove_dir_all(extracted_dir_top);

                    //load mod data
                    if let Some(mut loaded) =
                        Mod::load_from_disk(ModPath::default(), &target_dir, name_mod.clone())
                    {
                        {
                            let mut mods = MODS.write();
                            let m = mods.0.get_mut(&name_mod).unwrap();
                            loaded.volatile.mod_download_url = m.volatile.mod_download_url.clone();
                            loaded.volatile.screenshots = m.volatile.screenshots.clone();
                            if m.active.installed() {
                                loaded.active = std::mem::take(&mut m.active);
                                loaded.mods.mask(std::mem::take(&mut m.mods));
                            }
                            *m = loaded;
                        }
                        MODS.read().conflicts_update();
                    }
                    Ok(())
                },
            );
        }

        fn _remove_mod(name_mod: &String) {
            if let Ok(read_dir) = get_dirs().mods.read_dir() {
                for entry in read_dir {
                    if let Ok(entry) = entry {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                                if dir_name == *name_mod {
                                    _ = std::fs::remove_dir_all(entry.path());
                                }
                            }
                        }
                    }
                }
            }
        }
        async fn _fetch_updates() -> anyhow::Result<()> {
            const MAIN_REPO: &'static str =
                "https://raw.githubusercontent.com/vcmi/vcmi-mods-repository/develop/vcmi-1.4.json"; //TODO gen from launcher version

            Self::_fetch_updates_single(MAIN_REPO, ModSource::MainRepository).await?;
            let extra = EXTRA_REPO.read().clone();
            if extra.extra_repository_enabled {
                Self::_fetch_updates_single(extra.extra_repository_url, ModSource::ExtraRepository)
                    .await?;
            }

            Toast::info(t!("toasts.mod.Mod updates list downloaded!"));
            Ok(())
        }
        async fn _fetch_updates_single(
            url: impl IntoUrl + Display,
            source: ModSource,
        ) -> anyhow::Result<()> {
            let toast = t!("toasts.mod.Mod updates check failed!");
            let online_mods: ModUpdatesList = get_file_from_url(url, &toast).await?;

            let m = online_mods.into_iter().map(|(name, mut mod_)| {
                let toast = toast.clone();
                tokio::spawn(async move {
                    mod_.mod_file = get_file_from_url(mod_.mod_json.clone(), &toast).await.ok();
                    (name, mod_)
                })
            });
            let mut online_mods = futures::future::join_all(m).await;
            let mut mods = MODS.write();
            online_mods.iter_mut().for_each(|x| {
                if let Ok((name, online_mod)) = x {
                    if let Some(online_file) = &mut online_mod.mod_file {
                        online_file.download_size = online_mod.download_size;

                        if let Some(mod_) = mods.0.get_mut(name) {
                            if online_file.update_available(&mod_.volatile.mod_file.version) {
                                // // use this if download link is stored in modfile
                                // save_file(
                                //     &get_dirs()
                                //         .mods
                                //         .join(name.to_lowercase())
                                //         .join("UPDATE_mod.json"),
                                //     online_file,
                                // );
                                mod_.volatile.mod_file_update = Some(online_file.clone());
                            }
                            mod_.volatile.mod_file.download_size = online_file.download_size;
                            mod_.volatile.src = source.clone();
                            mod_.volatile.mod_download_url = online_mod.download.clone();
                            mod_.volatile.screenshots = online_mod.screenshots.clone();
                        } else {
                            let entry = mods
                                .0
                                .entry(name.clone())
                                .or_insert(Mod::new(name, online_mod));
                            entry.volatile.src = source.clone();
                        }
                    }
                }
            });
            mods.sort();
            log::info!("Mod updates list downloaded {:?}!", source);
            Ok(())
        }
    }
    impl Deref for ModOpsQueue {
        type Target = Vec<ModOp>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl DerefMut for ModOpsQueue {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}
pub use ops::*;
mod updates_json {
    use super::*;
    // ModList file eg.
    // https://raw.githubusercontent.com/vcmi/vcmi-mods-repository/develop/vcmi-1.4.json
    // (ModListEntry is single mod from it)
    //
    // {
    //     "hota" : {
    //         "mod" : "https://raw.githubusercontent.com/vcmi-mods/horn-of-the-abyss/vcmi-1.4/hota/mod.json",
    //         "download" : "https://github.com/vcmi-mods/horn-of-the-abyss/archive/refs/heads/vcmi-1.4.zip",
    //         "screenshots" : [
    //             "https://raw.githubusercontent.com/vcmi-mods/horn-of-the-abyss/vcmi-1.4/screenshots/01.png",
    //             "https://raw.githubusercontent.com/vcmi-mods/horn-of-the-abyss/vcmi-1.4/screenshots/02.png"
    //         ],
    //         "downloadSize" : 109.587
    //     },
    // },

    pub type ModUpdatesList = IndexMap<String, ModUpdatesListElem>;

    #[derive(Default, Clone, Deserialize)]
    #[serde(default, rename_all = "camelCase")]

    pub struct ModUpdatesListElem {
        #[serde(rename = "mod")]
        pub mod_json: String,
        pub download: String,
        pub screenshots: Vec<String>,
        pub download_size: f32,
        #[serde(skip)]
        pub mod_file: Option<ModFile>,
    }
}
pub use updates_json::*;
