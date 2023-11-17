/*
 * build.rs, part of VCMI engine
 * Build script:
 * - monitor "assets"
 * - icon (resources) for Windows
 * - compile foreign language code
 *
 * Authors: listed in file AUTHORS in main folder
 *
 * License: GNU General Public License v2.0 or later
 * Full text of license available in license.txt file, in main folder
 *
 */
fn main() {
    println!("cargo:rerun-if-changed=translate");
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=assets");
    embed_resource::compile("VCMI_launcher.rc", embed_resource::NONE);

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "ios" {
        cc::Build::new()
            .file("ios/main.m")
            .flag("-fobjc-arc")
            .flag("-std=c17")
            .compile("iosmain");
    }
    #[cfg(feature = "enable_gitversion")]
    git_sha1::GitSHA1::read().set("GIT_SHA1");
}
