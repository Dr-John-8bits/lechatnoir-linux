//! Parse `news.json` (§3.6) : `items[]` (ou `entries[]`), champs texte bruts (le HTML
//! est ignoré), filtrage des titres vides, tri par `sortKey` décroissant.

use serde_json::Value;

use crate::content::NewsEntry;

pub fn parse(data: &str) -> Vec<NewsEntry> {
    let Ok(root) = serde_json::from_str::<Value>(data) else {
        return Vec::new();
    };
    let items = root
        .get("items")
        .or_else(|| root.get("entries"))
        .and_then(Value::as_array);
    let Some(items) = items else {
        return Vec::new();
    };

    let mut entries: Vec<NewsEntry> = items.iter().filter_map(parse_item).collect();
    // Tri par sortKey décroissant (format AAAA-MM-NNN triable lexicographiquement).
    entries.sort_by(|a, b| b.sort_key.cmp(&a.sort_key));
    entries
}

fn parse_item(value: &Value) -> Option<NewsEntry> {
    let title = string_field(value, "title");
    if title.is_empty() {
        return None; // item au titre vide supprimé
    }
    let mut stable_id = first_non_empty(value, &["id", "slug"]);
    let sort_key = first_non_empty(value, &["sortKey", "publishedOn"]);
    let date_label = first_non_empty(value, &["dateLabel", "publishedOn"]);
    let lead = string_field(value, "lead");
    let body = string_field(value, "body");

    if stable_id.is_empty() {
        stable_id = format!("{sort_key}{title}");
    }
    Some(NewsEntry { stable_id, title, sort_key, date_label, lead, body })
}

fn string_field(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

fn first_non_empty(value: &Value, keys: &[&str]) -> String {
    for &key in keys {
        let s = string_field(value, key);
        if !s.is_empty() {
            return s;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tri_desc_et_filtrage_titre_vide() {
        let data = r#"{"items":[
            {"id":"a","title":"Ancien","sortKey":"2026-01-01-001","lead":"l","body":"b"},
            {"id":"b","title":"","sortKey":"2026-09-09-009"},
            {"slug":"c","title":"Récent","sortKey":"2026-08-01-002"}
        ]}"#;
        let entries = parse(data);
        assert_eq!(entries.len(), 2); // le titre vide est filtré
        assert_eq!(entries[0].title, "Récent"); // sortKey desc
        assert_eq!(entries[1].title, "Ancien");
        assert_eq!(entries[1].stable_id, "a");
    }

    #[test]
    fn replis_stableid_datelabel() {
        // dateLabel absent → publishedOn ; id absent → slug.
        let data = r#"{"items":[{"slug":"s","title":"T","publishedOn":"2026-03-02"}]}"#;
        let e = &parse(data)[0];
        assert_eq!(e.stable_id, "s");
        assert_eq!(e.sort_key, "2026-03-02");
        assert_eq!(e.date_label, "2026-03-02");
    }

    #[test]
    fn cle_entries_en_repli() {
        let data = r#"{"entries":[{"id":"x","title":"T","sortKey":"2026-01-01-001"}]}"#;
        assert_eq!(parse(data).len(), 1);
    }

    #[test]
    fn json_invalide() {
        assert!(parse("nope").is_empty());
        assert!(parse("{}").is_empty());
    }
}
