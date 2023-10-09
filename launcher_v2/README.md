# VCMI Launcher v2

## Highlights

- single launcher for all platforms
- uses Rust + egui (instead of C++ + Qt)
  - as the rest of project is in C++ this may be counted also as disadvantage
  - requires more time & disc space to build
  - binary is larger (I guess it could be mitigated somehow, but does it realy that matters nowadays if total size is 20MB larger? (~5MB can be slashed by switching from "wgpu" to "glow" on eframe))
  - GUI code is closely bound to normal code (immediate mode GUI)
  - no longer object oriented, more procedural aproach (code is more linear & can be easly followed & understood)
  - non-native look&feel
  - all prons of Rust programing, including:
    - rustfmt: automatic code formater
    - cargo: zero-effort package managament
    - powerfull & ~simple macros
    - less error-prone
    - less code (-3.5kloc in Java (-45%, whats left is mostly SDL), launcher from 4.5k in C++(not including *.ui files) to 3.3k in Rust (! Rust version is not yet feature full, but I guess that to reach feature parity it should close slighlty above 4k, and some of it is from "duplicating" some functionalities from vcmi_lib & android code))
- low effort to add new settings to gui
- friendlier translations format
- dark mode

## TODO

### iOS

- link launcher_v2 in client
- change qt_main_wrapper in client/ios/main.m to launcher_main
- check if linking ios/main.m works
- make it work :)

### Feature parity with old launcher

- lobby
- [Android] [iOS] select homm data location for copying
- add missing settings
- detect homm lang (?)
- [Android] check internal data hash, or store app version in settings.json (needed for iOS?)
- better cmake integration:
  - don't require NDK env. variable when NDK installed from conan
  - use corrosion also for android
  - check iOS
  - integrate setup & java part
- hints on hover
- Test on mac & ios
- Check VDirs corectness & consistence with VCMI_dirs.cpp
- reuse VCMI_dirs.cpp ???
- advanced homm data verify (?)
- start client with args
- migrate translations

### Nice to have

- incremental mod update (git?, using sparse git (currently not supported by any library) can also enable instaling separete submods, which is usefull if they are dependencies of another mods)
- vcmi updater
- handle errors (not just ignore them)
- even further reduce android/java code
- change some settings serialization (e.g. extraRepositoryEnabled & extraRepositoryURL as single item; requires changes in client&server)
- Documentation & tests
- UI/visuals improvement
- refresh translations from build.rs
- Use system fonts (?)

## Developement

Building following Rust project is integrated into vcmi Cmake, but for the first time requires some setup steps mentioned in below section ([here](#first-time-configuration)). Into Cmake is integrated only Rust/C++ part of a project, after cmake invocation, java part needs to be manualy compiled using instruction from [here](#build-for-android). When cross-compiling with cmake for android it may look like build has freezen, but it's ok.
Instructions from following sections can be used to build launcher alone, outside Cmake, then out-files are in ./target/@profile@/ or ./target/@triple@/@profile@/ (exept for Android).

If there is need to modify Cargo manifest, Cargo.toml SHALL NOT BE MODIFIED DIRECTLY, but all changes should be made to file Cargo_TO_EDIT.toml, which on next cmake invocation would be used for Cargo.toml generation.

### First-time configuration

```bash
# 1. get rustup; on linux folowing command
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.bashrc # restart shell

# 2. add required targets
rustup target add $target 
# where $target is one of following: 
  # x86_64-apple-darwin, 
  # aarch64-apple-ios, 
  # aarch64-apple-darwin, 
  # x86_64-unknown-linux-gnu, 
  # x86_64-pc-windows-msvc, 
  # aarch64-linux-android

# 3. [Android only] install Android SDK & NDK and set env. variables
export ANDROID_NDK_HOME="path/to/ndk" # only if non AndroidStudio default 
export ANDROID_HOME="path/to/sdk" # required by gradle
cargo install cargo-ndk
```

### Run for native/host

```bash
# uses command aliases defined in .cargo/config.toml
cargo r 
# OR
cargo rr # RelWithDebInfo profile
# OR
cargo rrr # release profile with LTO (link time optimizations)
```

### Crosscompilation for Windows/Linux/macOS/IoS [not tested]

```bash
# build/rebuild
cargo bwin # OR blinux, bmac, bmacarm, bios, bandroid
# build release
cargo rwin # OR rlinux, ...
```

### Build for Android

```bash
# build/rebuild
cargo bandroid # skip if using Cmake
cd ../android_v2
./gradlew build # or build from AndroidStudio (manually starting gradlew throws errors & fails: on SDL part)

# run
./gradlew installDebug
adb shell am start -n eu.vcmi.vcmi/.MainActivity
```

### Translations

This project uses i18n crate for translations. To create new translated string add `t!` macro call e.g. `t!("Translated text key")`. This would look for line `Translated text key: Translated text` in locale related file in translate dir and (simplifying) evaluate to String `"Translated text"`.
Teoreticly translation key can be arbitrary, to introduce some order most of them follow these rules:

- Prefix translation key with some kind of pseudo-path indicating where it is used e.g. `about.` for file "about_project.rs" and about view; `toasts.error.` for msg. in toast with error severity.
- If translated text is short, translation key should be simply text in english e.g. `"about.Check for updates"`
- If text is long, use key that is short and describes "intent" of text, also add whole text to `translate/en.yml`. E.g. `first_launch.SelectHommDataLocation: Alternatively, you can provide the directory where Heroes III data is installed and VCMI will copy the existing data automatically.`
- Translation-key shall not contain any special characters ('.' are tolerable, but interfere with placing default translation)
- For generating translations from enum are available two macros:
  - `EnumComboboxI18N` which generates translated UI implementation (`.show_ui()` method) for this enum in form of combobox.

    ```Rust
    #[derive(Default, EnumComboboxI18N)]
    #[module(settings.SettingsServer)]
    enum AIAdventure {
        #[default]
        Nullkiller,
        VCAI,
    }
    ```

  - `ToStringI18N` which provides method `.to_string_i18n()` which returns translated string.
  
    ```Rust
    #[derive(ToStringI18N)]
    pub enum InitializationState {
        #[default]
        Unknown = 0,
        SetLanguage,
        GetHoMMData,
        PresetMods,
        ProcessingData,
        Finished,
    }
    ```

  - Both of them generates keys in form Module.EnumName.EnumVariant for example above following keys will be generated: `settings.SettingsServer.AIAdventure.Nullkiller` & `settings.SettingsServer.AIAdventure.VCAI`.
- All keys can be extracted using modified `cargo i18n`
  - Instalation: `cargo install --git "https://github.com/PingPongun/rust-i18n.git" --branch "develop" --bin cargo-i18n rust-i18n`
  - New exported keys are in files `TODO.@LOCALE@.yml` with default values taken from default locale(`en`) (if not available, from last part of a key)
  - After translating key add `DONE` on begining of its translation and on next `cargo i18n` invocation this value would be transfered to file `@LOCALE@.yml`
  - If key from file `@LOCALE@.yml` has been removed from code, it will be moved to file `REMOVED.@LOCALE@.yml` on `cargo i18n` invocation.
  - As some `t!()` calls may be hidden behind macros, all code should be expanded to `translate/expanded.rs` before calling `cargo i18n` (on powershell simply call refresh_translations.ps1, on other shells call equivalent cmds)
