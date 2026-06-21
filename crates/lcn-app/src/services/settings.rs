//! Réglages persistants (volume, thème, géométrie de fenêtre) dans un petit fichier
//! clé=valeur sous `$XDG_CONFIG_HOME/lechatnoir/settings.conf`. Volontairement portable
//! (dev, AppImage, Flatpak) : aucune dépendance à un schéma GSettings installé.

use std::collections::BTreeMap;
use std::path::PathBuf;

// Clés.
pub const VOLUME: &str = "volume";
pub const THEME: &str = "theme"; // "auto" | "light" | "dark"
pub const WINDOW_WIDTH: &str = "window_width";
pub const WINDOW_HEIGHT: &str = "window_height";
pub const WINDOW_MAXIMIZED: &str = "window_maximized";

fn file_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("lechatnoir").join("settings.conf"))
}

fn read_all() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if let Some(content) = file_path().and_then(|p| std::fs::read_to_string(p).ok()) {
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    map
}

fn write_all(map: &BTreeMap<String, String>) {
    if let Some(path) = file_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let body: String = map.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
        let _ = std::fs::write(path, body);
    }
}

/// Lit une valeur brute.
pub fn get(key: &str) -> Option<String> {
    read_all().get(key).cloned()
}

/// Écrit une valeur (lit-modifie-réécrit le fichier, qui reste minuscule).
pub fn set(key: &str, value: &str) {
    let mut map = read_all();
    map.insert(key.to_string(), value.to_string());
    write_all(&map);
}

pub fn get_f64(key: &str, default: f64) -> f64 {
    get(key).and_then(|s| s.parse().ok()).unwrap_or(default)
}

pub fn get_i32(key: &str, default: i32) -> i32 {
    get(key).and_then(|s| s.parse().ok()).unwrap_or(default)
}

pub fn get_bool(key: &str, default: bool) -> bool {
    get(key).map(|s| s == "true").unwrap_or(default)
}

pub fn set_i32(key: &str, value: i32) {
    set(key, &value.to_string());
}

pub fn set_bool(key: &str, value: bool) {
    set(key, if value { "true" } else { "false" });
}
