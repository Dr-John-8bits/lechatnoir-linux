//! Logique **pure** du lecteur : santé du flux, décision de reconnexion plafonnée,
//! libellés d'état. Parité exacte avec `PlayerController.swift` (app macOS).
//! Aucune dépendance GStreamer ici → entièrement testable sur n'importe quelle plateforme.

use crate::config;

/// Santé du flux (parité `StreamHealth` Swift).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamHealth {
    /// Lecture effective ou prêt.
    Active,
    /// En cours de connexion / mise en mémoire tampon / reconnexion (fenêtre de grâce).
    Connecting,
    /// Grâce épuisée : « hors ligne » honnête (on continue à sonder en fond).
    Failed,
}

/// Libellés d'état EXACTS (verbatim cahier des charges §2.3). L'ellipse est un seul
/// caractère « … » (U+2026), comme dans l'app macOS.
pub mod playback_text {
    /// Prêt, pas de lecture.
    pub const READY: &str = "Prêt à écouter";
    /// `play()` lancé / attente de buffer.
    pub const CONNECTING: &str = "Connexion au direct…";
    /// Pause.
    pub const PAUSED: &str = "En pause";
    /// Lecture effective.
    pub const PLAYING: &str = "En écoute";
    /// Reconnexion dans la fenêtre de grâce.
    pub const RECONNECTING: &str = "Reconnexion au direct…";
    /// Échec, grâce épuisée.
    pub const OFFLINE: &str = "hors ligne";
}

/// Décision pure de reconnexion pour une tentative donnée : combien attendre, et
/// faut-il déjà afficher « hors ligne ». Extraite pour être testable sans pipeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReconnectStep {
    pub delay_secs: f64,
    pub is_offline: bool,
}

/// Calcule le pas de reconnexion pour la `attempt`-ième tentative (≥ 1).
///
/// Phase rapide (1→5) : backoff `min(attempt × 2, 15)` → 2, 4, 6, 8, 10 s, on affiche
/// « Reconnexion… ». Au-delà de la fenêtre de grâce (> 5) : sonde lente fixe à 60 s et
/// affichage honnête « hors ligne » — on ne ment plus, on continue à guetter le flux.
pub fn reconnect_step(attempt: u32) -> ReconnectStep {
    let is_offline = attempt > config::RECONNECT_GRACE_ATTEMPTS;
    let delay_secs = if is_offline {
        config::SLOW_RECONNECT_DELAY
    } else {
        (f64::from(attempt) * config::FAST_RECONNECT_STEP).min(config::FAST_RECONNECT_CAP)
    };
    ReconnectStep { delay_secs, is_offline }
}

/// Indicateurs (santé, texte) affichés pendant une reconnexion — parité macOS
/// `applyReconnectingIndicators`.
pub fn reconnecting_indicators(attempt: u32) -> (StreamHealth, &'static str) {
    if reconnect_step(attempt).is_offline {
        (StreamHealth::Failed, playback_text::OFFLINE)
    } else {
        (StreamHealth::Connecting, playback_text::RECONNECTING)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_phase_backoff_matches_macos() {
        // Tentatives 1→5 : 2, 4, 6, 8, 10 s, jamais « hors ligne ».
        let expected = [2.0, 4.0, 6.0, 8.0, 10.0];
        for (i, &delay) in expected.iter().enumerate() {
            let step = reconnect_step(i as u32 + 1);
            assert_eq!(step.delay_secs, delay, "tentative {}", i + 1);
            assert!(!step.is_offline, "tentative {} ne doit pas être offline", i + 1);
        }
    }

    #[test]
    fn backoff_is_capped_then_slow_probe() {
        // Au-delà de la grâce (5) : 60 s fixes + « hors ligne ».
        for attempt in [6, 7, 20, 1000] {
            let step = reconnect_step(attempt);
            assert_eq!(step.delay_secs, 60.0, "tentative {attempt}");
            assert!(step.is_offline, "tentative {attempt} doit être offline");
        }
    }

    #[test]
    fn grace_boundary_is_five() {
        // La bascule se fait STRICTEMENT après la 5e tentative.
        assert!(!reconnect_step(config::RECONNECT_GRACE_ATTEMPTS).is_offline);
        assert!(reconnect_step(config::RECONNECT_GRACE_ATTEMPTS + 1).is_offline);
    }

    #[test]
    fn reconnecting_indicators_switch_to_offline_after_grace() {
        assert_eq!(
            reconnecting_indicators(5),
            (StreamHealth::Connecting, playback_text::RECONNECTING)
        );
        assert_eq!(
            reconnecting_indicators(6),
            (StreamHealth::Failed, playback_text::OFFLINE)
        );
    }

    #[test]
    fn fast_phase_never_exceeds_cap() {
        // Garde-fou : la phase rapide ne dépasse jamais 15 s même si la grâce changeait.
        for attempt in 1..=config::RECONNECT_GRACE_ATTEMPTS {
            assert!(reconnect_step(attempt).delay_secs <= config::FAST_RECONNECT_CAP);
        }
    }
}
