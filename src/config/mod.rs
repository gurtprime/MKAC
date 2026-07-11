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

/// Write a small user-state file through a temporary sibling so a process
/// termination cannot leave a half-written JSON document behind.
pub fn atomic_write(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    let mut temp = path.to_path_buf();
    temp.set_extension("tmp");
    std::fs::write(&temp, data)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(temp, path)
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

#[cfg(test)]
mod tests {
    use super::sanitize_name;

    #[test]
    fn sanitizes_windows_path_characters() {
        assert_eq!(sanitize_name(r#" ..\bad:name?.json "#), ".._bad_name_.json");
    }

    #[test]
    fn removes_control_characters_and_outer_whitespace() {
        assert_eq!(sanitize_name("  hello\nworld  "), "hello_world");
    }
}
