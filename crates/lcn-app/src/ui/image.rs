//! Chargement d'images éditoriales : cache disque (XDG) → réseau, sans bloquer l'UI.
//! Le fetch se fait sur un thread, l'affichage revient sur le thread principal. Si pas
//! d'URL ou en cas d'échec, la `Picture` garde son placeholder.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread;

use relm4::gtk;
use relm4::gtk::{gdk, glib};
use reqwest::blocking::Client;

use lcn_core::config;
use lcn_core::content::ContentImage;

fn client() -> &'static Client {
    static C: OnceLock<Client> = OnceLock::new();
    C.get_or_init(|| {
        Client::builder()
            .timeout(config::HTTP_RESOURCE_TIMEOUT)
            .user_agent(config::USER_AGENT)
            .build()
            .expect("client image")
    })
}

fn cache_path(url: &str) -> Option<PathBuf> {
    let name = url.rsplit('/').next().filter(|s| !s.is_empty())?;
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("lechatnoir").join("images").join(name))
}

/// Charge l'image dans la `Picture` (cache disque puis réseau). No-op si pas d'URL.
pub fn load(picture: &gtk::Picture, image: &ContentImage) {
    let Some(url) = image.url.clone() else {
        return;
    };
    let path = cache_path(&url);

    // Cache disponible → chargement immédiat.
    if let Some(p) = &path {
        if p.exists() {
            if let Ok(texture) = gdk::Texture::from_filename(p) {
                picture.set_paintable(Some(&texture));
                return;
            }
        }
    }

    // Sinon, fetch sur un thread puis affichage sur le thread principal.
    let (tx, rx) = async_channel::bounded::<bool>(1);
    let url_thread = url;
    let path_thread = path.clone();
    thread::spawn(move || {
        let ok = fetch_and_cache(&url_thread, path_thread.as_deref());
        let _ = tx.send_blocking(ok);
    });

    let picture = picture.clone();
    glib::spawn_future_local(async move {
        if let Ok(true) = rx.recv().await {
            if let Some(p) = &path {
                if let Ok(texture) = gdk::Texture::from_filename(p) {
                    picture.set_paintable(Some(&texture));
                }
            }
        }
    });
}

fn fetch_and_cache(url: &str, path: Option<&Path>) -> bool {
    let Ok(response) = client().get(url).send() else {
        return false;
    };
    if !response.status().is_success() {
        return false;
    }
    let Ok(bytes) = response.bytes() else {
        return false;
    };
    if let Some(p) = path {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        return std::fs::write(p, &bytes).is_ok();
    }
    false
}
