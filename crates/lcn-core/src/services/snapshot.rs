//! Snapshot embarqué : jeux de données éditoriaux figés (issus de la préprod) pour un
//! premier rendu instantané et le mode hors-ligne / avant bascule (§3.9). À régénérer
//! depuis les JSON préprod en remplaçant les fichiers de `snapshots/`.

use crate::config;
use crate::content::NewsEntry;
use crate::services::{news, schedule, voices};

const NEWS_JSON: &str = include_str!("../../snapshots/news.json");
const SCHEDULE_JSON: &str = include_str!("../../snapshots/schedule.json");
const VOICES_JSON: &str = include_str!("../../snapshots/voices.json");

/// Actualités figées.
pub fn news() -> Vec<NewsEntry> {
    news::parse(NEWS_JSON)
}

/// Grille figée.
pub fn schedule() -> schedule::Schedule {
    schedule::parse(SCHEDULE_JSON)
}

/// Voix figées (images résolues contre la base de contenu courante).
pub fn voices() -> voices::Voices {
    voices::parse(VOICES_JSON, config::CONTENT_BASE_URL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_news_se_decode() {
        let entries = news();
        assert_eq!(entries.len(), 24);
        // Tri décroissant : la 1re a le plus grand sortKey.
        assert!(entries[0].sort_key >= entries[1].sort_key);
        assert!(!entries[0].title.is_empty());
    }

    #[test]
    fn snapshot_schedule_se_decode() {
        let sch = schedule();
        assert_eq!(sch.days.len(), 7);
        let mon = sch.day("mon").expect("lundi présent");
        assert!(!mon.slots.is_empty());
        // Les créneaux ont des heures de début exploitables.
        assert!(mon.slots.iter().all(|s| s.start_min.is_some()));
        // Créneau courant à 09:30 calculable.
        assert!(schedule::Schedule::current_slot_index(mon, 9 * 60 + 30).is_some());
    }

    #[test]
    fn snapshot_voices_se_decode() {
        let v = voices();
        assert_eq!(v.producers.len(), 8);
        assert_eq!(v.shows.len(), 11);
        // Images résolues en URL absolues vers la préprod.
        assert!(v.producers[0].image.url.as_deref().unwrap().starts_with(config::CONTENT_BASE_URL));
        // Au moins une émission en mode « contain ».
        assert!(v.shows.iter().any(|s| s.image.fit_contain));
    }
}
