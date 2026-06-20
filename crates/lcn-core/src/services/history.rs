//! Parse `history/nowplaying.csv` — historique des titres (§3.7). Tokenizer **RFC 4180**
//! (guillemets, `""`, virgules et retours-ligne dans les champs), détection d'en-tête par
//! la date, `parseDate` multi-format (fuseau Europe/Paris), cap, tri, recherche de créneau.
//!
//! ⚠️ JAMAIS de `split(',')` ni `split('\n')` naïf : un titre peut contenir des virgules,
//! des guillemets et des retours-ligne (cf. test « Death by Horsecock »).

use chrono::{DateTime, NaiveDateTime, Timelike, Utc};
use chrono_tz::{Europe::Paris, Tz};

use crate::config;

/// Une diffusion de l'historique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub timestamp_iso: String,
    pub artist: String,
    pub title: String,
    pub album: String,
    pub year: String,
    /// Instant analysé (UTC), pour tri et affichage en heure de Paris.
    pub at: DateTime<Utc>,
}

impl HistoryEntry {
    /// Instant en heure de Paris.
    pub fn paris(&self) -> DateTime<Tz> {
        self.at.with_timezone(&Paris)
    }

    /// Libellé d'heure « HH:MM » en heure de Paris.
    pub fn time_label(&self) -> String {
        self.paris().format("%H:%M").to_string()
    }

    /// Ligne secondaire : `[artist, album, year]` non vides joints par ` • `.
    pub fn metadata_line(&self) -> String {
        [&self.artist, &self.album, &self.year]
            .into_iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" • ")
    }

    fn minute_of_day_paris(&self) -> i64 {
        let t = self.paris();
        i64::from(t.hour()) * 60 + i64::from(t.minute())
    }

    /// Identité (déduplication) = champs joints par `|`.
    pub fn identity(&self) -> String {
        [
            self.timestamp_iso.as_str(),
            self.artist.as_str(),
            self.title.as_str(),
            self.album.as_str(),
            self.year.as_str(),
        ]
        .join("|")
    }
}

/// Parse le CSV et renvoie les entrées triées par date **décroissante**.
/// `limit` borne le nombre d'entrées conservées (les plus récentes) ; `None` = tout (recherche archive).
pub fn parse_csv(data: &str, limit: Option<usize>) -> Vec<HistoryEntry> {
    let mut rows = tokenize(data);

    // a) En-tête détecté par la date : si la 1re cellule de la 1re ligne ne parse pas → en-tête.
    if let Some(first) = rows.first() {
        let first_cell = first.first().map(String::as_str).unwrap_or("");
        if parse_date(first_cell).is_none() {
            rows.remove(0);
        }
    }

    let mut entries: Vec<HistoryEntry> = rows
        .into_iter()
        .filter_map(|row| row_to_entry(&row))
        .collect();

    // c) Tri par date décroissante, puis cap aux N plus récentes.
    entries.sort_by_key(|e| std::cmp::Reverse(e.at));
    if let Some(max) = limit {
        entries.truncate(max);
    }
    entries
}

/// Variante liste (cap 240, parité §3.7.c).
pub fn parse_recent(data: &str) -> Vec<HistoryEntry> {
    parse_csv(data, Some(config::HISTORY_MAX_ROWS))
}

fn row_to_entry(row: &[String]) -> Option<HistoryEntry> {
    // Ligne rejetée si < 4 colonnes.
    if row.len() < 4 {
        return None;
    }
    let timestamp_iso = row[0].trim().to_string();
    let artist = row.get(2).cloned().unwrap_or_default();
    let title = row.get(3).cloned().unwrap_or_default();
    let album = row.get(4).cloned().unwrap_or_default();
    let year = row.get(5).cloned().unwrap_or_default();

    // Rejet : ts_iso vide, ou title ET artist vides.
    if timestamp_iso.is_empty() || (title.trim().is_empty() && artist.trim().is_empty()) {
        return None;
    }
    let at = parse_date(&timestamp_iso)?;

    Some(HistoryEntry { timestamp_iso, artist, title, album, year, at })
}

/// Les 10 diffusions (ou `limit`) les plus proches d'un créneau donné (même jour Paris),
/// triées par écart de minutes croissant ; à égalité, la plus récente d'abord (§2.5.3).
pub fn search_closest(
    entries: &[HistoryEntry],
    target_paris: DateTime<Tz>,
    limit: usize,
) -> Vec<HistoryEntry> {
    let target_day = target_paris.date_naive();
    let target_minute = i64::from(target_paris.hour()) * 60 + i64::from(target_paris.minute());

    let mut matches: Vec<&HistoryEntry> = entries
        .iter()
        .filter(|e| e.paris().date_naive() == target_day)
        .collect();

    matches.sort_by(|a, b| {
        let gap_a = (a.minute_of_day_paris() - target_minute).abs();
        let gap_b = (b.minute_of_day_paris() - target_minute).abs();
        gap_a.cmp(&gap_b).then(b.at.cmp(&a.at))
    });

    matches.into_iter().take(limit).cloned().collect()
}

/// `parseDate` — ordre d'essai (§3.7) : ISO 8601 (avec/sans fractions), puis formats
/// `DateFormatter` interprétés en Europe/Paris.
pub fn parse_date(raw: &str) -> Option<DateTime<Utc>> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    // 1 & 2 : ISO 8601 (rfc3339 gère le suffixe Z et les fractions de seconde).
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // 3 : formats locaux, interprétés en heure de Paris.
    const FORMATS: [&str; 4] = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%d/%m/%Y %H:%M:%S",
        "%d/%m/%Y %H:%M",
    ];
    for fmt in FORMATS {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            if let Some(dt) = naive.and_local_timezone(Paris).single() {
                return Some(dt.with_timezone(&Utc));
            }
        }
    }
    None
}

/// Tokenizer RFC 4180 → lignes de champs. Gère `"` ouvrant/fermant, `""` littéral,
/// `,` séparateur hors guillemets, `\n`/`\r`/`\r\n` fin de ligne hors guillemets,
/// ligne entièrement vide ignorée, dernière ligne sans saut final émise.
fn tokenize(input: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut any = false;

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_quotes {
            if c == '"' {
                if chars.get(i + 1) == Some(&'"') {
                    field.push('"');
                    i += 2;
                } else {
                    in_quotes = false;
                    i += 1;
                }
            } else {
                field.push(c);
                i += 1;
            }
            continue;
        }
        match c {
            '"' => {
                in_quotes = true;
                any = true;
                i += 1;
            }
            ',' => {
                record.push(std::mem::take(&mut field));
                any = true;
                i += 1;
            }
            '\r' | '\n' => {
                flush(&mut rows, &mut record, &mut field, &mut any);
                if c == '\r' && chars.get(i + 1) == Some(&'\n') {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                field.push(c);
                any = true;
                i += 1;
            }
        }
    }
    flush(&mut rows, &mut record, &mut field, &mut any);
    rows
}

fn flush(rows: &mut Vec<Vec<String>>, record: &mut Vec<String>, field: &mut String, any: &mut bool) {
    record.push(std::mem::take(field));
    if *any {
        rows.push(std::mem::take(record));
    } else {
        record.clear();
    }
    *any = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_date_iso_et_formats() {
        // Format réel (ISO Z).
        assert!(parse_date("2026-06-13T12:30:59Z").is_some());
        // Avec fractions de seconde.
        assert!(parse_date("2026-06-13T12:30:59.250Z").is_some());
        // Format local interprété en Paris.
        assert!(parse_date("2026-06-13 12:30:59").is_some());
        assert!(parse_date("13/06/2026 12:30").is_some());
        // En-tête non daté → None (sert à détecter l'en-tête).
        assert!(parse_date("ts_iso").is_none());
        assert!(parse_date("").is_none());
    }

    #[test]
    fn entete_detecte_et_retire() {
        let csv = "ts_iso,ts_unix,artist,title,album,year\n\
                   2026-06-13T12:30:59Z,1781950859,Artiste,Titre,Album,2020\n";
        let entries = parse_csv(csv, None);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].artist, "Artiste");
        assert_eq!(entries[0].title, "Titre");
    }

    #[test]
    fn champ_avec_virgule_et_guillemets() {
        // Le cas qui casse un split(',') naïf : virgule DANS le champ titre (entre guillemets).
        let csv = "ts_iso,ts_unix,artist,title,album,year\n\
                   2026-06-13T12:30:59Z,1781950859,Death by Horsecock,\"À table (Déjeuner avec DBH | Lunch with DBH, 2020)\",EX1,2020\n";
        let entries = parse_csv(csv, None);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "À table (Déjeuner avec DBH | Lunch with DBH, 2020)");
        assert_eq!(entries[0].album, "EX1");
    }

    #[test]
    fn guillemet_litteral_et_retour_ligne_dans_champ() {
        // "" = guillemet littéral ; retour-ligne à l'intérieur d'un champ entre guillemets.
        let csv = "ts_iso,ts_unix,artist,title,album,year\n\
                   2026-06-13T12:30:59Z,1,A,\"Ligne1\nLigne2 \"\"citée\"\"\",Alb,2021\n";
        let entries = parse_csv(csv, None);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Ligne1\nLigne2 \"citée\"");
    }

    #[test]
    fn rejet_lignes_invalides() {
        let csv = "2026-06-13T12:30:59Z,1,Artiste,Titre,Album,2020\n\
                   ,2,A,B,C,D\n\
                   2026-06-13T13:00:00Z,3,,,X,Y\n\
                   2026-06-13T13:05:00Z,4\n";
        // 1re ligne valide (1re cellule = date → pas d'en-tête, tout gardé) ;
        // ligne ts vide rejetée ; ligne title+artist vides rejetée ; ligne < 4 colonnes rejetée.
        let entries = parse_csv(csv, None);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Titre");
    }

    #[test]
    fn tri_decroissant_et_cap() {
        let csv = "ts_iso,ts_unix,artist,title,album,year\n\
                   2026-06-13T10:00:00Z,1,A,Vieux,,\n\
                   2026-06-13T12:00:00Z,2,B,Recent,,\n\
                   2026-06-13T11:00:00Z,3,C,Milieu,,\n";
        let entries = parse_csv(csv, None);
        assert_eq!(entries.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), ["Recent", "Milieu", "Vieux"]);
        // Cap.
        assert_eq!(parse_csv(csv, Some(2)).len(), 2);
    }

    #[test]
    fn recherche_creneau_le_plus_proche() {
        let csv = "ts_iso,ts_unix,artist,title,album,year\n\
                   2026-06-13T12:00:00Z,1,A,Midi,,\n\
                   2026-06-13T14:30:00Z,2,B,DeuxTrente,,\n\
                   2026-06-13T13:00:00Z,3,C,Treize,,\n";
        let entries = parse_csv(csv, None);
        // En heure de Paris (été, UTC+2) : Midi=14:00, Treize=15:00, DeuxTrente=16:30.
        // Cible 15:10 Paris → le plus proche est « Treize » (15:00, écart 10 min).
        let target = chrono::NaiveDate::from_ymd_opt(2026, 6, 13)
            .unwrap()
            .and_hms_opt(15, 10, 0)
            .unwrap()
            .and_local_timezone(Paris)
            .unwrap();
        let closest = search_closest(&entries, target, 10);
        assert_eq!(closest.first().unwrap().title, "Treize");
        assert_eq!(closest.len(), 3);
    }
}
