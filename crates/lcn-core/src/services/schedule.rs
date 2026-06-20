//! Parse `schedule.json` (§3.4) : jours/créneaux, `aliasMap` (table embarquée fusionnée
//! avec `aliases` du JSON), `normalizeComparableText`, calcul du créneau courant (via
//! `start`/`end`), et rapprochement nom d'antenne ↔ titre de grille.

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

use crate::content::{ScheduleDay, ScheduleSlot};

/// Grille analysée : jours + table d'alias normalisée (variante → canonique).
#[derive(Debug, Clone)]
pub struct Schedule {
    pub days: Vec<ScheduleDay>,
    pub alias_map: HashMap<String, String>,
}

/// Table de repli embarquée (déjà normalisée), pour le mode hors-ligne (§3.4).
const EMBEDDED_ALIASES: &[(&str, &str)] = &[
    ("blocsonic", "blocsonic"),
    ("blocsonic mixtapes", "blocsonic"),
    ("instinct mode", "l instinct mode"),
    ("l instinct mode", "l instinct mode"),
    ("autre nuit", "l autre nuit"),
    ("l autre nuit", "l autre nuit"),
    ("pseudodocumentaire de l espace", "le pseudodocumentaire de l espace"),
    ("le pseudodocumentaire de l espace", "le pseudodocumentaire de l espace"),
    ("fragments", "matinee fragments"),
    ("matinee fragments", "matinee fragments"),
    ("trajectoires", "matinee trajectoires"),
    ("matinee trajectoires", "matinee trajectoires"),
    ("immersion", "matinee immersion"),
    ("matinee immersion", "matinee immersion"),
    ("traversees", "matinee traversees"),
    ("matinee traversees", "matinee traversees"),
    ("transmissions du dr john", "les transmissions du dr john"),
    ("les transmissions du dr john", "les transmissions du dr john"),
    ("ondes du chat noir", "les ondes du chat noir"),
    ("les ondes du chat noir", "les ondes du chat noir"),
];

const VALID_DAY_IDS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

impl Schedule {
    pub fn day(&self, id: &str) -> Option<&ScheduleDay> {
        self.days.iter().find(|d| d.id == id)
    }

    /// Forme canonique d'un texte déjà normalisé (via `alias_map`, sinon lui-même).
    pub fn canonical(&self, normalized: &str) -> String {
        self.alias_map
            .get(normalized)
            .cloned()
            .unwrap_or_else(|| normalized.to_string())
    }

    /// Index du créneau courant pour `minute_of_day` (Paris) : le slot dont l'heure de
    /// début est ≤ minute et la plus grande (les slots se suivent ; le 1er couvre 00h00).
    pub fn current_slot_index(day: &ScheduleDay, minute_of_day: u32) -> Option<usize> {
        let mut best: Option<(usize, u32)> = None;
        for (i, slot) in day.slots.iter().enumerate() {
            if let Some(start) = slot.start_min {
                if start <= minute_of_day && best.map_or(true, |(_, b)| start >= b) {
                    best = Some((i, start));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// Rapproche un nom d'antenne **déjà normalisé** d'un créneau du jour, via `aliasMap`.
    pub fn match_slot_by_name(&self, day: &ScheduleDay, normalized_show: &str) -> Option<usize> {
        let target = self.canonical(normalized_show);
        day.slots
            .iter()
            .position(|s| self.canonical(&normalize_comparable(&s.title)) == target)
    }
}

pub fn parse(data: &str) -> Schedule {
    let root: Value = serde_json::from_str(data).unwrap_or(Value::Null);

    // alias_map : table embarquée + aliases du JSON (les deux normalisés).
    let mut alias_map: HashMap<String, String> = EMBEDDED_ALIASES
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    if let Some(aliases) = root.get("aliases").and_then(Value::as_object) {
        for (variant, canonical) in aliases {
            if let Some(canonical) = canonical.as_str() {
                alias_map.insert(normalize_comparable(variant), normalize_comparable(canonical));
            }
        }
    }

    let days = root
        .get("days")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(parse_day).collect())
        .unwrap_or_default();

    Schedule { days, alias_map }
}

fn parse_day(value: &Value) -> Option<ScheduleDay> {
    let id = string_field(value, "id");
    if !VALID_DAY_IDS.contains(&id.as_str()) {
        return None; // id requis et valide, sinon jour rejeté
    }
    let name = string_field(value, "name");
    let short_name = {
        let s = string_field(value, "shortName");
        if s.is_empty() { id.clone() } else { s }
    };
    let summary = string_field(value, "summary");

    let slots = value
        .get("slots")
        .and_then(Value::as_array)
        .map(|arr| parse_slots(arr))
        .unwrap_or_default();

    Some(ScheduleDay { id, name, short_name, summary, slots })
}

fn parse_slots(arr: &[Value]) -> Vec<ScheduleSlot> {
    let mut slots = Vec::with_capacity(arr.len());
    let mut last_start: Option<u32> = None;
    for value in arr {
        let time = string_field(value, "time");
        let title = string_field(value, "title");
        if time.is_empty() || title.is_empty() {
            continue; // time et title requis
        }
        // start : champ `start` (HH:MM), sinon parse de `time`, sinon « Puis » hérite.
        let start_min = parse_hhmm(&string_field(value, "start"))
            .or_else(|| parse_time_label(&time))
            .or(if time.eq_ignore_ascii_case("puis") { last_start } else { None });
        if let Some(s) = start_min {
            last_start = Some(s);
        }
        let end_min = parse_hhmm(&string_field(value, "end"));

        slots.push(ScheduleSlot {
            time,
            title,
            desc: string_field(value, "desc"),
            meta: bool_field(value, "meta"),
            highlight: bool_field(value, "highlight"),
            badge: {
                let b = string_field(value, "badge");
                (!b.is_empty()).then_some(b)
            },
            kind: string_field(value, "kind"),
            start_min,
            end_min,
        });
    }
    slots
}

fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.trim().split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    (h < 24 && m < 60).then_some(h * 60 + m)
}

fn parse_time_label(time: &str) -> Option<u32> {
    let caps = re_time().captures(time.trim())?;
    let h: u32 = caps.get(1)?.as_str().parse().ok()?;
    let m: u32 = caps.get(2)?.as_str().parse().ok()?;
    (h < 24 && m < 60).then_some(h * 60 + m)
}

fn re_time() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^(\d{1,2})\s*[h:]\s*(\d{2})$").unwrap())
}

fn string_field(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

fn bool_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// `normalizeComparableText` (§3.4) : trim → minuscules → repli diacritique FR →
/// apostrophes→espace → non-alphanumériques→espace → espaces compactés → trim.
pub fn normalize_comparable(input: &str) -> String {
    let lowered = input.trim().to_lowercase();
    let folded = fold_diacritics(&lowered);
    let mut out = String::with_capacity(folded.len());
    let mut pending_space = false;
    for ch in folded.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(ch);
        } else {
            pending_space = true;
        }
    }
    out
}

fn fold_diacritics(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'ç' => 'c',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ÿ' | 'ý' => 'y',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation() {
        assert_eq!(normalize_comparable("L'instinct mode"), "l instinct mode");
        assert_eq!(normalize_comparable("Matinée : Fragments"), "matinee fragments");
        assert_eq!(normalize_comparable("  Les   Ondes  "), "les ondes");
    }

    #[test]
    fn creneau_courant_et_puis() {
        let data = r#"{"days":[{"id":"mon","name":"Lundi","slots":[
            {"time":"00h00","title":"Nuit","start":"00:00","end":"07:00"},
            {"time":"07h00","title":"Réveil","start":"07:00","end":"07:05"},
            {"time":"Puis","title":"Matinée : Fragments","start":"07:05","end":"12:00"}
        ]}]}"#;
        let sch = parse(data);
        let day = sch.day("mon").unwrap();
        assert_eq!(day.slots.len(), 3);
        // 09:30 = 570 min → créneau « Matinée : Fragments » (index 2).
        assert_eq!(Schedule::current_slot_index(day, 9 * 60 + 30), Some(2));
        // 07:02 → « Réveil » (index 1).
        assert_eq!(Schedule::current_slot_index(day, 7 * 60 + 2), Some(1));
    }

    #[test]
    fn alias_map_embarque_et_json() {
        let data = r#"{"aliases":{"La table du chat (repère)":"La table du chat"},"days":[]}"#;
        let sch = parse(data);
        // Embarqué : « fragments » → « matinee fragments ».
        assert_eq!(sch.canonical("fragments"), "matinee fragments");
        // JSON : variante normalisée → canonique normalisée.
        assert_eq!(sch.canonical("la table du chat repere"), "la table du chat");
    }

    #[test]
    fn rapprochement_nom_grille() {
        let data = r#"{"days":[{"id":"mon","name":"L","slots":[
            {"time":"07h05","title":"Matinée : Fragments","start":"07:05"}
        ]}]}"#;
        let sch = parse(data);
        let day = sch.day("mon").unwrap();
        // Nom d'antenne court « Fragments » → rapproché du créneau « Matinée : Fragments ».
        let normalized = normalize_comparable("Fragments");
        assert_eq!(sch.match_slot_by_name(day, &normalized), Some(0));
    }

    #[test]
    fn jour_sans_id_rejete() {
        let data = r#"{"days":[{"name":"SansId","slots":[]},{"id":"tue","name":"Mardi","slots":[]}]}"#;
        let sch = parse(data);
        assert_eq!(sch.days.len(), 1);
        assert_eq!(sch.days[0].id, "tue");
    }
}
