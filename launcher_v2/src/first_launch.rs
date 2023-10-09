/*
 * first_launch.rs, part of VCMI engine
 * First launch view/GUI generation & related data structures
 *
 * Authors: listed in file AUTHORS in main folder
 *
 * License: GNU General Public License v2.0 or later
 * Full text of license available in license.txt file, in main folder
 *
 */
use atomic_enum::atomic_enum;
use egui::{Context, Grid, RichText, Ui};
use egui_toast::Toast;
use rust_i18n::{t, ToStringI18N};
use std::sync::{atomic::Ordering, Arc};

use crate::gui_primitives::{DisplayGUI, EguiUiExt};
use crate::mod_manager::ModPath;
use crate::utils::*;
use crate::vcmi_launcher::*;

impl VCMILauncher {
    pub fn show_first_launch(&mut self, ctx: &Context) {
        if let AsyncHandle::Uninit = self.first_launch.homm_data_cpy {
            self.first_launch_spawn_homm_data_search()
        }
        self.first_launch_show_stage_top_panel(ctx);

        egui::CentralPanel::default().show(ctx, |ui| match self.first_launch.init_state {
            InitializationState::Unknown => {
                self.first_launch.init_state = InitializationState::SetLanguage;
                self.first_launch_spawn_internal_data_cpy();
            }
            InitializationState::SetLanguage => self.first_launch_show_language_set(ui),
            InitializationState::GetHoMMData => self.first_launch_show_homm_data_get(ui),
            InitializationState::PresetMods => self.first_launch_show_preset_mods(ui),
            InitializationState::ProcessingData => {
                ui.heading(t!("first_launch.Almost there..."));
                ui.label(t!("first_launch.VCMI prepares necessary files."));
                ui.centered_and_justified(|ui| ui.spinner());
                if self.first_launch.internal_data_cpy.is_success()
                    && self.first_launch.homm_data_cpy.is_success()
                {
                    self.first_launch.init_state = InitializationState::Finished;
                    self.settings.launcher.setup_completed = true;
                    self.save_settings();
                }
            }
            InitializationState::Finished => (),
        });
    }

    /////////////////////////////////////////////////////////////////
    /////////////////////Manage async tasks//////////////////////////
    /////////////////////////////////////////////////////////////////

    fn first_launch_spawn_internal_data_cpy(&mut self) {
        let extract_dest = get_dirs().internal.clone();
        self.first_launch
            .internal_data_cpy
            .run(Arc::new(()), async move {
                if cfg!(target_os = "android") {
                    //TODO check internal data hash at each launch
                    //TODO ? is this neaded on iOS
                    let zipped_data = include_bytes!("../assets/internalData.zip");
                    let extract_result = zip::ZipArchive::new(std::io::Cursor::new(zipped_data))
                        .unwrap()
                        .extract(extract_dest.clone());
                    if let Err(err) = extract_result {
                        Toast::error(t!("toasts.error.Prepare internal data failed!"));
                        log::error!(
                            "Unpack internal data to {} failed!; Error: {}",
                            extract_dest.display(),
                            err
                        );
                        return Err(err.into());
                    }
                    Toast::success(t!("toasts.success.Internal data ready!"));
                    log::info!(
                        "Unpack internal data to {} finished!",
                        extract_dest.display(),
                    );
                    Ok(())
                } else {
                    Ok(()) //internal data needs to be unpacked only on android
                }
            })
    }

    #[cfg(all(not(target_os = "android"), not(target_os = "ios")))]
    fn first_launch_spawn_homm_data_cpy(&mut self) {
        let progress = Arc::new(AtomicHOMMDataState::new(HOMMDataState::NotSelected));
        self.first_launch
            .homm_data_cpy
            .run(progress.clone(), async move {
                if let Some(src) = rfd::AsyncFileDialog::new().pick_folder().await {
                    progress.store(HOMMDataState::CheckingSelectedPath, Ordering::Relaxed);
                    let src = src.path();
                    if let Err(err) = check_data_dir_valid(src) {
                        progress.store(HOMMDataState::NotFound, Ordering::Relaxed);
                        Toast::error(t!("toasts.error.Valid HoMM data not found!"));
                        log::error!(
                            "Selected path does not contain valid HoMM data!; Error: {}",
                            err
                        );
                        return Err(err.into());
                    }
                    Toast::success(t!("toasts.success.Valid HoMM data found!"));
                    log::info!("Valid HoMM data found!");
                    progress.store(HOMMDataState::Found, Ordering::Relaxed);
                    let cpy_resoult = fs_extra::copy_items(
                        &[src.join("data"), src.join("maps"), src.join("mp3")],
                        get_dirs().user_data.clone(),
                        &fs_extra::dir::CopyOptions::new().overwrite(true),
                    );
                    if let Err(err) = cpy_resoult {
                        Toast::error(t!("toasts.error.HoMM data copy failed!"));
                        log::error!("HoMM data copy failed!; Error: {}", err);
                        return Err(err.into());
                    }
                    Toast::success(t!("toasts.success.HoMM data imported!"));
                    log::info!("HoMM data imported!");
                    Ok(())
                } else {
                    anyhow::bail!("Failed to create dialog!")
                }
            })
    }

    fn first_launch_spawn_homm_data_search(&mut self) {
        //check for homm data in vcmi dirs
        let progress = Arc::new(AtomicHOMMDataState::new(HOMMDataState::CheckingVCMIDirs));
        self.first_launch
            .homm_data_cpy
            .run(progress.clone(), async move {
                if check_data_dir_valid(&get_dirs().user_data.clone()).is_err()
                    && check_data_dir_valid(&get_dirs().internal.clone()).is_err()
                {
                    Toast::warning(t!("toasts.error.Valid HoMM data not found!"));
                    log::warn!("Valid HoMM data not found in VCMI dirs!",);
                    progress.store(HOMMDataState::NotSelected, Ordering::Relaxed);
                    anyhow::bail!("Valid HoMM data not found in VCMI dirs!")
                } else {
                    Toast::success(t!("toasts.success.Valid HoMM data found!"));
                    log::info!("Valid HoMM data found in VCMI dirs!");
                    progress.store(HOMMDataState::Found, Ordering::Relaxed);
                    Ok(())
                }
            });
    }

    /////////////////////////////////////////////////////////////////
    ////////////////////Display stage views//////////////////////////
    /////////////////////////////////////////////////////////////////

    fn first_launch_show_stage_top_panel(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("Top_init_panel").show(ctx, |ui: &mut egui::Ui| {
            let show_stage_button =
                |ui: &mut Ui, id: InitializationState, curr_stage: &mut InitializationState| {
                    ui.set_enabled(id <= *curr_stage);
                    let mut text = (id as usize).to_string();
                    if id == *curr_stage {
                        text.push_str(": ");
                        text.push_str(&id.to_string_i18n())
                    }
                    if ui
                        .selectable_label(id == *curr_stage, RichText::heading(text.into()))
                        .clicked()
                    {
                        *curr_stage = id;
                    }
                };
            ui.add_space(6.0);

            ui.horizontal_top(|ui| {
                [
                    InitializationState::SetLanguage,
                    InitializationState::GetHoMMData,
                    InitializationState::PresetMods,
                    InitializationState::ProcessingData,
                ]
                .into_iter()
                .for_each(|i| show_stage_button(ui, i, &mut self.first_launch.init_state))
            });
            ui.add_space(6.0)
        });
    }

    fn first_launch_show_language_set(&mut self, ui: &mut Ui) {
        ui.group_wrapped(|ui| {
            self.settings
                .general
                .language
                .show_ui(ui, &t!("first_launch.Select your language"));
        });
        ui.add_space(6.0);
        ui.label(t!("first_launch.intro_message"));
        ui.add_space(6.0);
        crate::about_project::show_join_us(ui);
        ui.add_space(6.0);
        if ui.button(t!("first_launch.Next")).clicked() {
            self.first_launch.init_state = InitializationState::GetHoMMData;
        }
    }

    fn first_launch_show_homm_data_get(&mut self, ui: &mut Ui) {
        ui.heading(t!("first_launch.Locate Heroes III data files"));
        ui.separator();
        ui.add_space(6.0);
        ui.group_wrapped(|ui| {
            ui.label(t!("first_launch.VCMI data directories"));
            ui.separator();
            ui.vertical(|ui| {
                ui.label(get_dirs().user_data.to_string_lossy());
                ui.label(get_dirs().internal.to_string_lossy());
            })
        });
        let homm_state = self.first_launch.homm_data_cpy.if_state(
            &mut |p| p.load(Ordering::Relaxed),
            &mut |_| HOMMDataState::Found,
            &mut || HOMMDataState::NotFound,
            &mut || HOMMDataState::NotSelected,
        );

        ui.group_wrapped(|ui| {
            ui.label(t!("first_launch.VCMI data state"));
            ui.label(homm_state.to_string_i18n());
        });
        match homm_state {
            HOMMDataState::CheckingVCMIDirs | HOMMDataState::CheckingSelectedPath => {
                ui.centered_and_justified(|ui| ui.spinner());
            }
            HOMMDataState::NotSelected | HOMMDataState::NotFound => {
                if cfg!(target_os = "linux") {
                    ui.group_wrapped(|ui| {
                        ui.label(t!("first_launch.HintVCMIBuilder"));
                        ui.hyperlink_to("wiki", "https://wiki.vcmi.eu/Installation_on_Linux#Install_data_using_vcmibuilder_script_.28recommended_for_non-Flatpak_installs.29");
                    });
                }
                ui.group_wrapped(|ui| {
                    ui.label(t!("first_launch.PleaseCopyHommData"));
                    if ui
                        .button(t!("first_launch.PleaseCopyHommDataBtn"))
                        .clicked()
                    {
                        self.first_launch_spawn_homm_data_search()
                    }
                });
                #[cfg(all(not(target_os = "android"), not(target_os = "ios")))]
                ui.group_wrapped(|ui| {
                    ui.label(t!("first_launch.SelectHommDataLocation"));
                    if ui
                        .button(t!("first_launch.SelectHommDataLocationBtn"))
                        .clicked()
                    {
                        self.first_launch_spawn_homm_data_cpy()
                    }
                });
            }
            HOMMDataState::Found => (),
        }
        //TODO select homm data lang.
        if homm_state == HOMMDataState::Found {
            ui.add_space(6.0);
            if ui.button(t!("first_launch.Next")).clicked() {
                self.first_launch.init_state = InitializationState::PresetMods;
            }
        }
    }

    fn first_launch_show_preset_mods(&mut self, ui: &mut Ui) {
        let mut all_installed = true;

        ui.heading(t!("first_launch.preset.Install some mods now"));
        ui.label(t!(
            "first_launch.preset.Or install them later from \"Mods\" tab"
        ));
        ui.add_space(6.0);

        if self.ongoing_ops() {
            //still downloading mod list
            ui.spinner();
            all_installed = false;
        } else {
            let s = &mut self.first_launch;
            // ui.horizontal_wrapped(|ui| {
            Grid::new(ui.next_auto_id())
                .striped(true)
                .min_col_width(0.0)
                .num_columns(4)
                .show(ui, |ui| {
                    let mut show_mod = |val: &mut bool, name, text| {
                        if let Ok(mod_) = ModPath::new(name).get_mod() {
                            if !mod_.active.installed() {
                                val.show_ui(ui, "");
                                ui.label(mod_.get_name());
                                ui.horizontal_wrapped(|ui| ui.label(text));
                                ui.end_row();
                                return false;
                            }
                        }
                        true
                    };

                    all_installed &=
                        show_mod(&mut s.hota, "hota", t!("first_launch.preset.hota_text"));
                    all_installed &= show_mod(
                        &mut s.wog,
                        "wake-of-gods",
                        t!("first_launch.preset.wog_text"),
                    );
                    all_installed &= show_mod(
                        &mut s.vcmi_extras,
                        "vcmi-extras",
                        t!("first_launch.preset.vcmi_extras_text"),
                    );
                });
            // });
        }

        ui.add_space(6.0);
        if all_installed
            || !*self.settings.launcher.auto_check_repositories
            || ui.button(t!("first_launch.Next")).clicked()
        {
            let s = &mut self.first_launch;

            let mut install_mod = |val, name| {
                if val {
                    self.mod_mng.ops.install(ModPath::new(name))
                }
            };

            install_mod(s.hota, "hota");
            install_mod(s.wog, "wake-of-gods");
            install_mod(s.vcmi_extras, "vcmi-extras");
            self.first_launch.init_state = InitializationState::ProcessingData;
        }
    }
}

/////////////////////////////////////////////////////////////////
///////////////////////Type definitions//////////////////////////
/////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct FirstLaunchState {
    init_state: InitializationState,
    homm_data_cpy: AsyncHandle<(), AtomicHOMMDataState>,
    internal_data_cpy: AsyncHandle<(), ()>,
    hota: bool,
    wog: bool,
    vcmi_extras: bool,
}

#[atomic_enum]
#[derive(Default, PartialEq, ToStringI18N)]
pub enum HOMMDataState {
    #[default]
    CheckingVCMIDirs = 0,
    NotSelected,
    CheckingSelectedPath,
    NotFound,
    Found,
}
impl Default for AtomicHOMMDataState {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

#[derive(Default, PartialEq, PartialOrd, Clone, Copy, ToStringI18N)]
pub enum InitializationState {
    #[default]
    Unknown = 0,
    SetLanguage,
    GetHoMMData,
    PresetMods,
    ProcessingData,
    Finished,
}
