//! Contrôleur audio GStreamer (`playbin3`) : lecture/pause, volume persistant,
//! suivi d'état via le bus, et **reconnexion plafonnée** (parité `PlayerController.swift`).
//! La logique de décision pure (délais, libellés) vit dans `lcn_core::player` (testée) ;
//! ici on pilote le pipeline réel et on ordonnance les reconnexions.
//!
//! M1a : play/pause + volume. M1b (ici) : reconnexion (backoff, « hors ligne » honnête,
//! garde-fou génération, auto-guérison). MPRIS = à venir, sur Linux (pas de bus D-Bus sur Mac).

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::time::Duration;

use gstreamer as gst;
use gstreamer::glib;
use gstreamer::prelude::*;
use gstreamer_audio::{StreamVolume, StreamVolumeFormat};

use lcn_core::config;
use lcn_core::player::{playback_text, reconnect_step, reconnecting_indicators, StreamHealth};

type ChangeCallback = dyn Fn();

struct Inner {
    /// Référence faible sur soi-même (pour planifier des timers qui se réfèrent au contrôleur).
    weak_self: Weak<Inner>,
    playbin: gst::Element,
    /// Nom du playbin, pour filtrer les `StateChanged` venant du pipeline lui-même.
    name: String,
    /// URI du flux (surcharge possible via `LCN_STREAM_URL` pour les tests).
    uri: String,
    /// Intention utilisateur (≠ état réel du flux) = « should resume » macOS.
    is_playing: Cell<bool>,
    /// Valeur « cubique » 0..1 du slider (telle qu'affichée), avant courbe.
    volume: Cell<f64>,
    /// Dernière santé de flux publiée (lue par la chrome pour l'état d'antenne).
    health: Cell<StreamHealth>,
    /// Dernier `playbackStateText` publié.
    text: Cell<&'static str>,
    /// Callback « quelque chose a changé » → la chrome se rafraîchit (lit les getters).
    on_change: RefCell<Option<Rc<ChangeCallback>>>,
    /// Garde la surveillance du bus vivante (la perdre retire la watch).
    bus_guard: RefCell<Option<gst::bus::BusWatchGuard>>,
    /// Numéro de tentative de reconnexion en cours (0 = nominal).
    reconnect_attempts: Cell<u32>,
    /// « Génération » : incrémentée à chaque play/pause pour invalider une reconnexion
    /// programmée devenue obsolète (anti Pause→Play rapide).
    generation: Cell<u64>,
    /// Source GLib de la reconnexion en attente (pour l'annuler proprement).
    reconnect_source: RefCell<Option<glib::SourceId>>,
}

impl Inner {
    /// Publie un nouvel état (santé + texte) et déclenche le rafraîchissement de la chrome.
    fn set_state(&self, health: StreamHealth, text: &'static str) {
        self.health.set(health);
        self.text.set(text);
        if let Some(cb) = self.on_change.borrow().as_ref() {
            cb();
        }
    }

    /// Applique la valeur du slider au pipeline via la courbe cubique→linéaire.
    fn apply_volume(&self) {
        let linear = StreamVolume::convert_volume(
            StreamVolumeFormat::Cubic,
            StreamVolumeFormat::Linear,
            self.volume.get(),
        );
        self.playbin.set_property("volume", linear);
    }

    /// Annule toute reconnexion en attente et invalide celles déjà programmées (génération).
    fn invalidate_pending_reconnect(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        if let Some(id) = self.reconnect_source.borrow_mut().take() {
            id.remove();
        }
    }

    /// Programme une reconnexion selon le n° de tentative (parité macOS `scheduleReconnect`).
    fn schedule_reconnect(&self) {
        // Seulement si l'utilisateur veut écouter, et si rien n'est déjà programmé.
        if !self.is_playing.get() || self.reconnect_source.borrow().is_some() {
            return;
        }

        let attempt = self.reconnect_attempts.get() + 1;
        self.reconnect_attempts.set(attempt);

        let step = reconnect_step(attempt);
        let (health, text) = reconnecting_indicators(attempt);
        self.set_state(health, text);

        // Logger UNE fois au passage en « hors ligne » honnête.
        if attempt == config::RECONNECT_GRACE_ATTEMPTS + 1 {
            tracing::warn!(
                "Flux indisponible après {} tentatives — affichage « hors ligne », sonde espacée à {}s.",
                config::RECONNECT_GRACE_ATTEMPTS,
                config::SLOW_RECONNECT_DELAY as u64
            );
        }

        let generation = self.generation.get();
        let weak = self.weak_self.clone();
        let id = glib::timeout_add_local(Duration::from_secs_f64(step.delay_secs), move || {
            if let Some(inner) = weak.upgrade() {
                // La source est consommée : on l'oublie pour ne pas la re-supprimer.
                inner.reconnect_source.replace(None);
                // Un play()/pause() pendant l'attente a changé de génération → obsolète.
                if generation == inner.generation.get() && inner.is_playing.get() {
                    inner.restart_pipeline();
                }
            }
            glib::ControlFlow::Break
        });
        self.reconnect_source.replace(Some(id));
    }

    /// Recrée un flux frais (Null → uri → Playing). En cas de nouvel échec, le bus
    /// redéclenchera `schedule_reconnect` (la tentative s'incrémente, le backoff progresse).
    fn restart_pipeline(&self) {
        tracing::info!("reconnexion : nouveau flux (tentative {})", self.reconnect_attempts.get());
        let _ = self.playbin.set_state(gst::State::Null);
        self.playbin.set_property("uri", &self.uri);
        if let Err(err) = self.playbin.set_state(gst::State::Playing) {
            tracing::warn!("relance du flux échouée: {err:?}");
        }
    }

    fn handle_message(&self, msg: &gst::Message) {
        match msg.view() {
            gst::MessageView::Buffering(b) => {
                let percent = b.percent();
                tracing::debug!("buffering {percent}%");
                if self.is_playing.get() && percent >= 100 {
                    let _ = self.playbin.set_state(gst::State::Playing);
                }
            }
            gst::MessageView::StateChanged(sc) => {
                let from_pipeline = msg
                    .src()
                    .map(|s| self.name == s.name().as_str())
                    .unwrap_or(false);
                if from_pipeline {
                    let current = sc.current();
                    tracing::info!("état pipeline -> {current:?}");
                    if current == gst::State::Playing && self.is_playing.get() {
                        // Lecture effective rétablie : on remet le compteur à zéro.
                        self.reconnect_attempts.set(0);
                        self.set_state(StreamHealth::Active, playback_text::PLAYING);
                    }
                }
            }
            gst::MessageView::Error(e) => {
                tracing::warn!("erreur flux: {} ({:?})", e.error(), e.debug());
                if self.is_playing.get() {
                    self.schedule_reconnect();
                }
            }
            gst::MessageView::Eos(_) => {
                tracing::info!("EOS — le serveur a probablement coupé le flux");
                if self.is_playing.get() {
                    self.schedule_reconnect();
                }
            }
            _ => {}
        }
    }
}

/// Poignée clonable vers le contrôleur (interne partagé en `Rc`).
#[derive(Clone)]
pub struct PlayerController {
    inner: Rc<Inner>,
}

impl PlayerController {
    /// Construit le pipeline `playbin3` et installe la surveillance du bus.
    /// `gstreamer::init()` doit avoir été appelé au préalable (dans `main`).
    pub fn new() -> Self {
        // Surcharge d'URI pour les tests (URL morte → observer la reconnexion).
        let uri = std::env::var("LCN_STREAM_URL")
            .unwrap_or_else(|_| config::STREAM_MP3_URL.to_string());

        let playbin = gst::ElementFactory::make("playbin3")
            .property("uri", &uri)
            .build()
            .expect("création de playbin3");
        playbin.set_property("buffer-duration", config::PLAYBIN_BUFFER_DURATION_NS);

        // playbin3 n'expose plus notify::source : on règle souphttpsrc dans source-setup.
        playbin.connect("source-setup", false, |values| {
            if let Ok(source) = values[1].get::<gst::Element>() {
                configure_source(&source);
            }
            None
        });

        let name = playbin.name().to_string();
        let inner = Rc::new_cyclic(|weak| Inner {
            weak_self: weak.clone(),
            playbin,
            name,
            uri,
            is_playing: Cell::new(false),
            volume: Cell::new(load_volume()),
            health: Cell::new(StreamHealth::Active),
            text: Cell::new(playback_text::READY),
            on_change: RefCell::new(None),
            bus_guard: RefCell::new(None),
            reconnect_attempts: Cell::new(0),
            generation: Cell::new(0),
            reconnect_source: RefCell::new(None),
        });
        inner.apply_volume();

        if let Some(bus) = inner.playbin.bus() {
            let weak: Weak<Inner> = Rc::downgrade(&inner);
            let guard = bus
                .add_watch_local(move |_, msg| {
                    if let Some(inner) = weak.upgrade() {
                        inner.handle_message(msg);
                    }
                    glib::ControlFlow::Continue
                })
                .expect("surveillance du bus GStreamer");
            *inner.bus_guard.borrow_mut() = Some(guard);
        }

        Self { inner }
    }

    /// (Re)lance la lecture : annule toute reconnexion obsolète, repart à zéro.
    pub fn play(&self) {
        self.inner.invalidate_pending_reconnect();
        self.inner.reconnect_attempts.set(0);
        self.inner.is_playing.set(true);
        self.inner.set_state(StreamHealth::Connecting, playback_text::CONNECTING);
        if let Err(err) = self.inner.playbin.set_state(gst::State::Playing) {
            tracing::warn!("set_state(Playing) a échoué: {err:?}");
        }
        tracing::info!("play()");
    }

    /// Met en pause (annule l'intention de lecture et toute reconnexion en attente).
    pub fn pause(&self) {
        self.inner.invalidate_pending_reconnect();
        self.inner.is_playing.set(false);
        let _ = self.inner.playbin.set_state(gst::State::Paused);
        self.inner.set_state(StreamHealth::Active, playback_text::PAUSED);
        tracing::info!("pause()");
    }

    pub fn toggle(&self) {
        if self.inner.is_playing.get() {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn is_playing(&self) -> bool {
        self.inner.is_playing.get()
    }

    /// Volume « slider » courant (0..1, valeur cubique affichée).
    pub fn volume(&self) -> f64 {
        self.inner.volume.get()
    }

    /// Règle le volume (0..1), applique la courbe et persiste.
    pub fn set_volume(&self, value: f64) {
        let clamped = value.clamp(0.0, 1.0);
        self.inner.volume.set(clamped);
        self.inner.apply_volume();
        save_volume(clamped);
    }

    /// Enregistre le callback « état changé » (la chrome relit les getters et se rafraîchit).
    pub fn set_on_change(&self, callback: impl Fn() + 'static) {
        *self.inner.on_change.borrow_mut() = Some(Rc::new(callback));
    }

    /// Dernière santé de flux publiée.
    pub fn stream_health(&self) -> StreamHealth {
        self.inner.health.get()
    }

    /// Dernier `playbackStateText` publié (utile pendant « connexion »).
    pub fn playback_text(&self) -> &'static str {
        self.inner.text.get()
    }
}

/// Règle les propriétés de `souphttpsrc` (gardées : le serveur peut renvoyer une autre source).
fn configure_source(source: &gst::Element) {
    if source.has_property("is-live", None) {
        source.set_property("is-live", true);
    }
    if source.has_property("iradio-mode", None) {
        source.set_property("iradio-mode", true);
    }
    if source.has_property("timeout", None) {
        source.set_property("timeout", 10u32);
    }
    if source.has_property("retries", None) {
        // Le retry est géré par l'app (fenêtre de grâce), pas par souphttpsrc.
        source.set_property("retries", 0i32);
    }
    if source.has_property("user-agent", None) {
        source.set_property("user-agent", config::USER_AGENT);
    }
}

// ── Persistance du volume ────────────────────────────────────────────────────
// M1 : simple fichier sous $XDG_CONFIG_HOME/lechatnoir/volume (portable, sans schéma
// système). Le passage à GSettings (clé `lcn-player-volume`) est prévu au packaging (M5).

fn prefs_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("lechatnoir").join("volume"))
}

fn load_volume() -> f64 {
    prefs_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map(|v| v.clamp(0.0, 1.0))
        .unwrap_or(config::DEFAULT_VOLUME)
}

fn save_volume(value: f64) {
    if let Some(path) = prefs_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, format!("{value}"));
    }
}
