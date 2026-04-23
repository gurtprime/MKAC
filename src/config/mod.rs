pub mod macros;
pub mod schema;
pub mod settings;

use std::path::PathBuf;

/// `%APPDATA%\MKAC\MKAC\config` — the double "MKAC" matches what the app used
/// when it still depended on the `directories` crate, so existing user configs
/// and macro libraries on disk keep resolving without a migration step.
pub fn config_dir() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(appdata)
            .join("MKAC")
            .join("MKAC")
            .join("config"),
    )
}

pub fn settings_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("settings.json"))
}

pub fn ensure_dirs() -> std::io::Result<()> {
    if let Some(d) = config_dir() {
        std::fs::create_dir_all(d)?;
    }
    Ok(())
}

/// Strip characters that Windows forbids in filenames and any control chars.
pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    cleaned.trim().to_string()
}
