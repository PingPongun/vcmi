cargo expand --all-features --bin vcmilauncherv2 | out-file translate/expanded.rs -encoding utf8
cargo i18n ./translate
