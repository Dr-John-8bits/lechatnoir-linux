//! Helpers d'horloge en fuseau **Europe/Paris** (jour courant, minute du jour, instant).
//! Regroupés ici pour que le reste du code n'appelle pas `now()` un peu partout.

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::{Europe::Paris, Tz};

/// Instant courant en heure de Paris.
pub fn paris_now() -> DateTime<Tz> {
    Utc::now().with_timezone(&Paris)
}

/// Horodatage epoch (s) courant.
pub fn now_unix() -> i64 {
    Utc::now().timestamp()
}

/// Identifiant de jour de grille (`mon`..`sun`) pour un jour de la semaine.
pub fn weekday_id(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
        Weekday::Sun => "sun",
    }
}

/// Jour de grille courant (Paris).
pub fn current_day_id() -> &'static str {
    weekday_id(paris_now().weekday())
}

/// Minute du jour courante (Paris), 0..1439.
pub fn current_minute_of_day() -> u32 {
    let now = paris_now();
    now.hour() * 60 + now.minute()
}

/// Construit un instant Paris depuis des composants (pour la recherche d'historique).
pub fn paris_datetime(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Option<DateTime<Tz>> {
    Paris
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekday_ids() {
        assert_eq!(weekday_id(Weekday::Mon), "mon");
        assert_eq!(weekday_id(Weekday::Sun), "sun");
    }

    #[test]
    fn paris_datetime_construit() {
        let dt = paris_datetime(2026, 6, 13, 15, 10).unwrap();
        assert_eq!(dt.hour(), 15);
        assert_eq!(dt.minute(), 10);
    }
}
