//! Configuration centrale — **le SEUL endroit** où vivent les endpoints et les
//! constantes réseau. Règle d'or n°1 du cahier des charges (`03-CONTRATS-DONNEES.md`) :
//! aucune URL en dur ailleurs dans le code.
//!
//! Deux familles d'endpoints, une seule bascule :
//!  - **Temps réel** (`stream.lechatnoirradio.fr`) : infra Icecast/Liquidsoap/Nginx,
//!    **ne change JAMAIS**.
//!  - **Contenu éditorial** (`CONTENT_BASE_URL`) : bascule préprod → prod **en une
//!    seule ligne**, le jour J (voir `switch_to_production`).

// ───────────────────────────── A. Temps réel (figé) ─────────────────────────────

/// Base des endpoints temps réel. Infra Icecast/Liquidsoap/Nginx — **ne bascule jamais**.
pub const REALTIME_BASE_URL: &str = "https://stream.lechatnoirradio.fr";

/// Flux audio MP3 (192 kbps stéréo 44,1 kHz) — la source lue en v1.
pub const STREAM_MP3_URL: &str = "https://stream.lechatnoirradio.fr/stream.mp3";

/// Flux audio Opus (96 kbps) — alternative, **non utilisée en v1**.
pub const STREAM_OPUS_URL: &str = "https://stream.lechatnoirradio.fr/stream";

/// Titre en cours (JSON). Rafraîchi toutes les [`POLL_NOWPLAYING`].
pub const NOWPLAYING_URL: &str = "https://stream.lechatnoirradio.fr/nowplaying.json";

/// Émission/bloc à l'antenne (JSON). Rafraîchi toutes les [`POLL_CURRENT_SHOW`].
pub const CURRENT_SHOW_URL: &str = "https://stream.lechatnoirradio.fr/current-show.json";

/// Historique des titres (CSV RFC 4180, ~6,3 Mo). Rafraîchi toutes les [`POLL_HISTORY`].
/// Le serveur renvoie `Accept-Ranges: bytes` → privilégier une requête Range sur la fin.
pub const HISTORY_CSV_URL: &str = "https://stream.lechatnoirradio.fr/history/nowplaying.csv";

// ──────────────────────── B. Contenu éditorial (bascule) ────────────────────────

/// Base du contenu éditorial mutualisé (news / schedule / voices / médias).
///
/// **BASCULÉ EN PRODUCTION le 21/06/2026.** Précondition vérifiée ce jour-là : `news.json`,
/// `schedule.json` ET `voices.json` répondent **200** sur la prod, avec une **structure
/// identique** à la préprod (contenu plus à jour). La **préprod va être vidée** → ne plus la
/// cibler. Les chemins relatifs `assets/data/*.json` et `assets/media/*` restent identiques ;
/// les endpoints temps réel ne bougent pas.
pub const CONTENT_BASE_URL: &str = "https://lechatnoirradio.fr/";

/// Base de contenu en production. Depuis le 21/06/2026, identique à [`CONTENT_BASE_URL`].
pub const PRODUCTION_BASE_URL: &str = "https://lechatnoirradio.fr/";

/// Chemins relatifs des JSON éditoriaux (joints à [`CONTENT_BASE_URL`], **sans** slash initial).
pub const NEWS_PATH: &str = "assets/data/news.json";
pub const SCHEDULE_PATH: &str = "assets/data/schedule.json";
pub const VOICES_PATH: &str = "assets/data/voices.json";
/// Préfixe des médias éditoriaux (les chemins relatifs des JSON s'y résolvent).
pub const MEDIA_PATH_PREFIX: &str = "assets/media/";

/// Construit une URL absolue de contenu à partir d'un chemin relatif et de la base courante.
/// `base` doit se terminer par `/` (les chemins n'ont pas de slash initial).
pub fn content_url(base: &str, relative_path: &str) -> String {
    format!("{base}{relative_path}")
}

#[must_use]
pub fn news_url(base: &str) -> String {
    content_url(base, NEWS_PATH)
}
#[must_use]
pub fn schedule_url(base: &str) -> String {
    content_url(base, SCHEDULE_PATH)
}
#[must_use]
pub fn voices_url(base: &str) -> String {
    content_url(base, VOICES_PATH)
}

// ──────────────────────────── Liens externes / marque ───────────────────────────

/// Site vitrine (lien externe « Site web »).
pub const WEBSITE_URL: &str = "https://lechatnoirradio.fr/";
/// E-mail de contact (cartes `mailto` de la page À propos).
pub const CONTACT_EMAIL: &str = "radio@lechatnoirradio.fr";
/// Compte Instagram.
pub const INSTAGRAM_URL: &str = "https://www.instagram.com/lechatnoirradio/";

/// App-id reverse-DNS — conditionne Flatpak, MPRIS, .desktop, icônes, GSettings.
pub const APP_ID: &str = "fr.lechatnoirradio.Player";
/// Nom de l'exécutable installé (`command` du manifeste Flatpak / `Exec` du .desktop).
pub const BINARY_NAME: &str = "lechatnoir-player";
/// User-agent envoyé par `souphttpsrc` et le client HTTP.
pub const USER_AGENT: &str = "LeChatNoir-Linux/1.0";
/// `mpris:trackid` — chemin stable bidon (flux live, pas de vraie piste).
pub const MPRIS_TRACK_ID: &str = "/fr/lechatnoirradio/track/live";

// ───────────────────────────── Réseau (parité exacte) ────────────────────────────

use std::time::Duration;

/// Timeout d'une requête HTTP.
pub const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
/// Timeout de chargement complet d'une ressource.
pub const HTTP_RESOURCE_TIMEOUT: Duration = Duration::from_secs(15);
/// Au-delà de ce délai sans `nowplaying`/`current-show` frais → état « hors ligne »
/// même si la lecture continue (fraîcheur métadonnées).
pub const METADATA_STALE_AFTER: Duration = Duration::from_secs(90);

// ─────────────────────────── Cadences de polling (parité) ───────────────────────

pub const POLL_NOWPLAYING: Duration = Duration::from_secs(12);
pub const POLL_CURRENT_SHOW: Duration = Duration::from_secs(12);
pub const POLL_HISTORY: Duration = Duration::from_secs(20);
pub const POLL_NEWS: Duration = Duration::from_secs(600);
pub const POLL_VOICES: Duration = Duration::from_secs(600);
pub const POLL_SCHEDULE: Duration = Duration::from_secs(600);

// ───────────────────────────────── Historique ───────────────────────────────────

/// Nombre maximal de lignes d'historique conservées en mode liste.
pub const HISTORY_MAX_ROWS: usize = 240;
/// Taille de la requête Range sur la **fin** du CSV (~512 Ko) pour éviter de
/// re-télécharger les ~6,3 Mo toutes les 20 s. Repli = téléchargement complet.
pub const HISTORY_RANGE_TAIL_BYTES: u64 = 512 * 1024;
/// Nombre de diffusions « les plus proches » renvoyées par une recherche de créneau.
pub const HISTORY_SEARCH_RESULTS: usize = 10;

// ──────────────────────────── Reconnexion (parité macOS) ────────────────────────

/// Nombre de tentatives rapides avant de basculer en « hors ligne » honnête.
pub const RECONNECT_GRACE_ATTEMPTS: u32 = 5;
/// Cadence lente une fois la grâce épuisée (auto-guérison sans marteler le serveur).
pub const SLOW_RECONNECT_DELAY: f64 = 60.0;
/// Plafond du backoff de la phase rapide (2,4,6,8,10 → plafonné).
pub const FAST_RECONNECT_CAP: f64 = 15.0;
/// Multiplicateur du backoff rapide (`attempt × 2`).
pub const FAST_RECONNECT_STEP: f64 = 2.0;

// ───────────────────────────────── Audio / lecteur ──────────────────────────────

/// Volume par défaut (persistant, clé GSettings `lcn-player-volume`).
pub const DEFAULT_VOLUME: f64 = 0.72;
/// `buffer-duration` de `playbin3` en nanosecondes (2 s).
pub const PLAYBIN_BUFFER_DURATION_NS: i64 = 2_000_000_000;

// ──────────────────────────────────── Fuseau ────────────────────────────────────

/// Fuseau métier forcé pour tout calcul/affichage de date-heure.
pub const TIMEZONE: &str = "Europe/Paris";
/// Locale d'affichage.
pub const LOCALE: &str = "fr_FR";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_urls_join_without_double_slash() {
        // CONTENT_BASE_URL se termine par '/', les chemins n'ont pas de slash initial.
        assert!(CONTENT_BASE_URL.ends_with('/'));
        assert!(!NEWS_PATH.starts_with('/'));
        assert_eq!(
            news_url(CONTENT_BASE_URL),
            "https://lechatnoirradio.fr/assets/data/news.json"
        );
        assert_eq!(
            schedule_url(PRODUCTION_BASE_URL),
            "https://lechatnoirradio.fr/assets/data/schedule.json"
        );
    }

    #[test]
    fn realtime_endpoints_are_frozen() {
        // Garde-fou : les endpoints temps réel ne doivent jamais pointer ailleurs.
        assert!(STREAM_MP3_URL.starts_with(REALTIME_BASE_URL));
        assert!(NOWPLAYING_URL.starts_with(REALTIME_BASE_URL));
        assert!(CURRENT_SHOW_URL.starts_with(REALTIME_BASE_URL));
        assert!(HISTORY_CSV_URL.starts_with(REALTIME_BASE_URL));
    }

    #[test]
    fn content_is_on_production() {
        // Basculé en prod le 21/06/2026 (préprod vidée). CONTENT_BASE_URL == PRODUCTION_BASE_URL.
        assert_eq!(CONTENT_BASE_URL, PRODUCTION_BASE_URL);
        assert_eq!(CONTENT_BASE_URL, "https://lechatnoirradio.fr/");
    }
}
