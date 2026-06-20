//! Logo de marque, **embarqué dans le binaire** via `include_bytes!` (aucun chemin du
//! système de fichiers à l'exécution → conforme au REX packaging). Crédit photo : Yirmi June.

use std::path::PathBuf;

use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::gtk::{gdk, glib};

const LOGO_BYTES: &[u8] = include_bytes!("../../assets/logo.png");

fn logo_texture() -> Option<gdk::Texture> {
    gdk::Texture::from_bytes(&glib::Bytes::from_static(LOGO_BYTES)).ok()
}

/// Image du logo à la taille demandée (depuis l'asset embarqué).
pub fn logo_image(pixel_size: i32) -> gtk::Image {
    let image = match logo_texture() {
        Some(texture) => gtk::Image::from_paintable(Some(&texture)),
        None => gtk::Image::new(),
    };
    image.set_pixel_size(pixel_size);
    image.add_css_class("lcn-logo");
    image
}

/// Écrit le logo embarqué dans le cache XDG et renvoie un `file://` (pour `mpris:artUrl`).
/// Emplacement standard, jamais un chemin de build. Utilisé uniquement par MPRIS (Linux).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn logo_file_uri() -> Option<String> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    let path = base.join("lechatnoir").join("logo.png");
    if !path.exists() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if std::fs::write(&path, LOGO_BYTES).is_err() {
            return None;
        }
    }
    Some(format!("file://{}", path.to_str()?))
}
