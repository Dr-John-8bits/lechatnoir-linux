//! Parse `current-show.json` — émission/bloc à l'antenne (§3.3). Tolérant : payload
//! objet OU simple string (= nom du show), alias de clés, booléen tolérant, `stripEndcap`.

use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

/// Émission/bloc courant. `show` est déjà passé par `stripEndcap`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CurrentShow {
    pub show: String,
    /// `kind` normalisé (trim + minuscules) : `music_block`, `editorial_event`, …, `live`.
    pub kind: String,
    pub is_live: bool,
    /// Début du bloc (epoch s), 0 si absent.
    pub since: i64,
}

impl CurrentShow {
    /// « depuis X » à partir de `since` (§3.3.c). `now_unix` injecté pour la testabilité.
    pub fn elapsed_text(&self, now_unix: i64) -> Option<String> {
        if self.since <= 0 {
            return None;
        }
        let seconds = now_unix - self.since;
        if seconds < 60 {
            return Some("depuis quelques instants".to_string());
        }
        let minutes = seconds / 60;
        if minutes < 60 {
            return Some(format!("depuis {minutes} min"));
        }
        let hours = minutes / 60;
        let rest = minutes % 60;
        Some(if rest == 0 {
            format!("depuis {hours} h")
        } else {
            format!("depuis {hours} h {rest}")
        })
    }
}

pub fn parse(data: &str) -> Option<CurrentShow> {
    let root: Value = serde_json::from_str(data).ok()?;

    // Payload = simple string → nom du show.
    if let Value::String(s) = &root {
        let show = strip_endcap(s);
        return (!show.is_empty()).then(|| CurrentShow {
            show,
            ..Default::default()
        });
    }

    let candidates: Vec<&Value> = match root.as_array() {
        Some(arr) => arr.iter().collect(),
        None => vec![&root],
    };
    candidates.into_iter().find_map(parse_object)
}

fn parse_object(value: &Value) -> Option<CurrentShow> {
    let show = strip_endcap(&string_at(value, &["show", "name", "title", "label"]));
    let kind = normalize_kind(&string_at(value, &["kind", "show_kind", "showKind", "type"]));
    let since = int_at(value, &["since", "started_at", "startedAt"]).filter(|n| *n > 0).unwrap_or(0);
    let is_live = bool_tolerant(value, &["is_live", "isLive", "show_is_live"]) || kind == "live";

    // Valide si au moins un champ est renseigné.
    if show.is_empty() && kind.is_empty() && !is_live && since <= 0 {
        return None;
    }
    Some(CurrentShow { show, kind, is_live, since })
}

fn string_at(value: &Value, keys: &[&str]) -> String {
    for &key in keys {
        match value.get(key) {
            Some(Value::String(s)) if !s.trim().is_empty() => return s.trim().to_string(),
            Some(Value::Number(n)) => return n.to_string(),
            _ => {}
        }
    }
    String::new()
}

fn int_at(value: &Value, keys: &[&str]) -> Option<i64> {
    for &key in keys {
        match value.get(key) {
            Some(Value::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    return Some(i);
                }
                if let Some(f) = n.as_f64() {
                    return Some(f as i64);
                }
            }
            Some(Value::String(s)) => {
                if let Ok(i) = s.trim().parse::<i64>() {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Booléen tolérant : `true`/`false`, nombre > 0, ou string `true`/`1`/`yes`/`oui`.
fn bool_tolerant(value: &Value, keys: &[&str]) -> bool {
    for &key in keys {
        match value.get(key) {
            Some(Value::Bool(b)) => return *b,
            Some(Value::Number(n)) => return n.as_f64().map(|f| f > 0.0).unwrap_or(false),
            Some(Value::String(s)) => {
                return matches!(s.trim().to_lowercase().as_str(), "true" | "1" | "yes" | "oui")
            }
            _ => {}
        }
    }
    false
}

fn normalize_kind(raw: &str) -> String {
    raw.trim().to_lowercase()
}

/// Retire systématiquement un suffixe « endcap » (insensible à la casse) puis trim.
fn strip_endcap(raw: &str) -> String {
    re_endcap().replace(raw, "").trim().to_string()
}

fn re_endcap() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)\s+endcap$").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echantillon_reel() {
        let cs = parse(r#"{"show":"La table du chat","kind":"music_block","is_live":false,"since":1781950842}"#).unwrap();
        assert_eq!(cs.show, "La table du chat");
        assert_eq!(cs.kind, "music_block");
        assert!(!cs.is_live);
        assert_eq!(cs.since, 1781950842);
    }

    #[test]
    fn direct_et_kind_live() {
        assert!(parse(r#"{"show":"DIRECT","kind":"live","is_live":true}"#).unwrap().is_live);
        // is_live déduit de kind == live même sans champ is_live.
        assert!(parse(r#"{"kind":"live"}"#).unwrap().is_live);
    }

    #[test]
    fn payload_string_et_endcap() {
        assert_eq!(parse(r#""Beats & Flow""#).unwrap().show, "Beats & Flow");
        assert_eq!(parse(r#"{"show":"Matinée Fragments endcap"}"#).unwrap().show, "Matinée Fragments");
    }

    #[test]
    fn booleen_tolerant() {
        assert!(parse(r#"{"show":"X","is_live":"oui"}"#).unwrap().is_live);
        assert!(parse(r#"{"show":"X","is_live":1}"#).unwrap().is_live);
        assert!(!parse(r#"{"show":"X","is_live":"non"}"#).unwrap().is_live);
    }

    #[test]
    fn elapsed_paliers() {
        let cs = CurrentShow { since: 1000, ..Default::default() };
        assert_eq!(cs.elapsed_text(1000), Some("depuis quelques instants".into())); // 0 s
        assert_eq!(cs.elapsed_text(1030), Some("depuis quelques instants".into()));
        assert_eq!(cs.elapsed_text(1000 + 120), Some("depuis 2 min".into()));
        assert_eq!(cs.elapsed_text(1000 + 3700), Some("depuis 1 h 1".into()));
        assert_eq!(cs.elapsed_text(1000 + 7200), Some("depuis 2 h".into()));
        let none = CurrentShow { since: 0, ..Default::default() };
        assert_eq!(none.elapsed_text(99999), None);
    }

    #[test]
    fn invalide() {
        assert!(parse("{}").is_none());
    }
}
