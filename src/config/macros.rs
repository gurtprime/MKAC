use std::path::PathBuf;

use super::{config_dir, ensure_dirs, sanitize_name};
use crate::engine::macros::Macro;

pub fn macros_dir() -> Option<PathBuf> {
    config_dir().map(|d| d.join("macros"))
}

pub fn ensure_macros_dir() -> std::io::Result<()> {
    if let Some(d) = macros_dir() {
        std::fs::create_dir_all(d)?;
    }
    Ok(())
}

pub fn macro_path(name: &str) -> Option<PathBuf> {
    macros_dir().map(|d| d.join(format!("{}.json", sanitize_name(name))))
}

pub fn list_macros() -> Vec<String> {
    let Some(dir) = macros_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .filter_map(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .collect();
    names.sort_unstable();
    names
}

pub fn save_macro(m: &Macro) -> anyhow::Result<()> {
    ensure_dirs()?;
    ensure_macros_dir()?;
    let path = macro_path(&m.name).ok_or_else(|| anyhow::anyhow!("no macros dir"))?;
    let data = serde_json::to_string_pretty(m)?;
    std::fs::write(&path, data)?;
    Ok(())
}

pub fn load_macro(name: &str) -> anyhow::Result<Macro> {
    let path = macro_path(name).ok_or_else(|| anyhow::anyhow!("no macros dir"))?;
    let data = std::fs::read_to_string(&path)?;
    let m: Macro = serde_json::from_str(&data)?;
    Ok(m)
}

pub fn delete_macro(name: &str) -> anyhow::Result<()> {
    if let Some(path) = macro_path(name) {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

