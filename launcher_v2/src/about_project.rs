/*
 * vcmi_launcher.rs, part of VCMI engine
 * About project view
 *
 * Authors: listed in file AUTHORS in main folder
 *
 * License: GNU General Public License v2.0 or later
 * Full text of license available in license.txt file, in main folder
 *
 */

use egui::{Color32, Ui};
use egui_toast::Toast;
use rust_i18n::t;
use std::{collections::HashMap, sync::Arc};

use crate::gui_primitives::EguiUiExt;
use crate::vcmi_launcher::*;

impl VCMILauncher {
    fn version() -> String {
        #[cfg(feature = "enable_gitversion")]
        let mut m = ["VCMI ", env!("CARGO_PKG_VERSION")].join("");
        {
            m.push('.');
            m.push_str(option_env!("GIT_SHA1").unwrap_or_default());
        }
        m
    }
    pub fn show_about(&mut self, ui: &mut Ui) {
        show_join_us(ui);

        ui.add_space(6.0);
        ui.heading(t!("about.Build Information"));
        ui.group_wrapped(|ui| {
            ui.strong(t!("about.Game Version"));
            ui.label(VCMILauncher::version());
        });
        ui.group_wrapped(|ui| {
            let _ = self.update_fetch.vcmi.if_running( &mut |_| {
                ui.spinner();
            })
            || //using OR, couse it's shortcircutting so if_running is true, if success will not be executed
            self.update_fetch.vcmi.if_success( &mut |json| {
                if json.update_available() {
                    let color = match json.update_type {
                        VcmiUpdatesType::Minor => Color32::GRAY,
                        VcmiUpdatesType::Major => Color32::from_rgb(255, 127, 0),
                        VcmiUpdatesType::Critical => Color32::RED,
                    };
                    ui.colored_label(color, t!("about.VCMI update available!"));
                    ui.hyperlink_to(t!("about.Download"), json.get_download_link());
                } else {
                    ui.colored_label(Color32::GREEN, t!("about.VCMI is up-to-date!"));
                }
            });
            if ui.button(t!("about.Check for updates")).clicked() {
                self.spawn_update_check_vcmi()
            }
        });
        ui.group_wrapped(|ui| {
            ui.label(t!("about.Operating System"));
            ui.label(os_info::get().to_string());
        });

        ui.add_space(6.0);
        ui.heading(t!("about.Data Directories"));
        ui.group_wrapped(|ui| {
            ui.label(t!("about.Game data directory"));
            ui.label(self.dirs.internal.to_string_lossy());
        });
        ui.group_wrapped(|ui| {
            ui.label(t!("about.User data directory"));
            ui.label(self.dirs.user_data.to_string_lossy());
        });
        ui.group_wrapped(|ui| {
            ui.label(t!("about.Log files directory"));
            ui.label(self.dirs.log.parent().unwrap().to_string_lossy());
        });
    }

    pub fn spawn_update_check_vcmi(&mut self) {
        const CHECK_UPDATES_URL: &'static str =
            // "https://raw.githubusercontent.com/vcmi/vcmi-updates/master/vcmi-updates.json";
            "https://raw.githubusercontent.com/vcmi/vcmi-updates/vcmi-1.1.1/vcmi-updates.json";
        self.update_fetch.vcmi.run(Arc::new(()), async {
            match reqwest::get(CHECK_UPDATES_URL).await {
                Err(err) => {
                    Toast::error(t!("toasts.error.Update check failed!"));
                    log::error!(
                        "Unable to download vcmi-updates.json from: {}!; Error: {}",
                        CHECK_UPDATES_URL,
                        err
                    );
                    return Err(());
                }
                Ok(downloaded) => match downloaded.json::<VcmiUpdatesJson>().await {
                    Ok(json) => {
                        if json.update_available() {
                            Toast::warning(t!("about.VCMI update available!"));
                            log::info!("VCMI update available!",);
                        } else {
                            Toast::info(t!("about.VCMI is up-to-date!"));
                            log::info!("VCMI is up-to-date!",);
                        }
                        Ok(json)
                    }
                    Err(err) => {
                        Toast::error(t!("toasts.error.Update check failed!"));
                        log::error!("Unable to parse vcmi-updates.json!; Error: {}", err);
                        Err(())
                    }
                },
            }
        });
    }
}
pub fn show_join_us(ui: &mut Ui) {
    ui.heading(t!("about.Our Community"));
    ui.label(t!("about.join_us"));
    ui.group_wrapped(|ui| {
        ui.hyperlink_to("vcmi.eu", "https://vcmi.eu/");
        ui.hyperlink_to("Discord", "https://discord.com/invite/chBT42V");
        ui.hyperlink_to("Slack", "https://slack.vcmi.eu/");
        ui.hyperlink_to("GitHub", "https://github.com/vcmi/vcmi");
        ui.hyperlink_to(
            t!("about.Report a bug"),
            "https://github.com/vcmi/vcmi/issues",
        );
    });
}
#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum VcmiUpdatesType {
    Minor,
    Major,
    Critical,
}
#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VcmiUpdatesJson {
    update_type: VcmiUpdatesType,
    version: String,
    download_links: HashMap<String, String>,
    change_log: String,
    history: Vec<String>,
}

impl VcmiUpdatesJson {
    fn get_download_link(&self) -> &str {
        self.download_links
            .get(std::env::consts::OS)
            .map(|dl| dl.as_str())
            .unwrap_or("https://vcmi.eu")
    }
    fn update_available(&self) -> bool {
        //Simply follows update check logic from launcher_v1
        if self.version == VCMILauncher::version() {
            //the newest version already installed
            return false;
        }

        if self.history.contains(&VCMILauncher::version()) {
            //current version is outdated
            true
        } else {
            //current version is newer than upstream OR is custom build
            false
        }
    }
}
// {
// 	"updateType" : "critical",
// 	"version" : "VCMI 1.3.2",
// 	"downloadLinks" :
// 	{
// 		"macos" : "https://github.com/vcmi/vcmi/releases/tag/1.3.2",
// 		"windows" : "https://github.com/vcmi/vcmi/releases/tag/1.3.2",
// 		"android" : "https://github.com/vcmi/vcmi/releases/tag/1.3.2",
// 		"linux" : "https://wiki.vcmi.eu/Installation_on_Linux",
// 		"ios" : "https://github.com/vcmi/vcmi/releases/tag/1.3.2",
// 		"other" : "https://vcmi.eu"
// 	},
// 	"changeLog" :
// 		"VCMI 1.3.2 was released!\nStability improvements and fixes for issues found in previous release\nRead more on the downloads page."
// 	"history" :
// 	[
// 		"VCMI 1.0.0.cedc9a92ede66b3f9fff079022ea287a012330a5",
// 		"VCMI 1.1.0.3cd8da6a8becf3643c1d233b2b87ded9f6f5ac53",
// 		"VCMI 1.1.0",
// 		"VCMI 1.1.1.b429f0bfeb230de65c771e0edf6e1684a5eb653c",
// 		"VCMI 1.1.1",
// 		"VCMI 1.2.0.c125e040c305c88f66e7ba8a458ee22e4d455f30",
// 		"VCMI 1.2.0",
// 		"VCMI 1.2.1.25d5a1555cf51663bb8d8409dbc269dfaf511da8",
// 		"VCMI 1.2.1",
// 		"VCMI 1.3.0.b9729241f1018b4a07f880b319da32f94bf6409b",
// 		"VCMI 1.3.0",
// 		"VCMI 1.3.1.daa8a494fce938e32c842b0ad085d47a87d0c572",
// 		"VCMI 1.3.1",
// 		"VCMI 1.3.2.cfe87e46a92f90dcfbef487e2ce3b15a86d4eebe",
// 		"VCMI 1.3.2"
// 	]
// }
#[derive(Default)]
pub struct FetchUpdate {
    vcmi: AsyncHandle<VcmiUpdatesJson, ()>,
    // mod_list: AsyncHandle<ModListUpdatesJson, ()>,
}
