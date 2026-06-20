//! Parse `voices.json` (§3.5) : producteur·ices et émissions, résolution d'image contre
//! la base de contenu, repli d'`actionLabel`.

use serde_json::Value;

use crate::content::{ContentImage, ProducerProfile, StationShow};

/// Voix analysées.
#[derive(Debug, Clone)]
pub struct Voices {
    pub producers: Vec<ProducerProfile>,
    pub shows: Vec<StationShow>,
}

/// `base_url` = base de contenu courante (pour résoudre les images relatives).
pub fn parse(data: &str, base_url: &str) -> Voices {
    let root: Value = serde_json::from_str(data).unwrap_or(Value::Null);
    let producers = array(&root, "producers")
        .iter()
        .filter_map(|v| parse_producer(v, base_url))
        .collect();
    let shows = array(&root, "shows")
        .iter()
        .filter_map(|v| parse_show(v, base_url))
        .collect();
    Voices { producers, shows }
}

fn parse_producer(value: &Value, base_url: &str) -> Option<ProducerProfile> {
    let name = string_field(value, "name");
    if name.is_empty() {
        return None; // name requis, sinon rejeté
    }
    Some(ProducerProfile {
        name,
        role: string_field(value, "role"),
        bio: string_field(value, "bio"),
        image: ContentImage::resolve(&string_field(value, "image"), base_url, false),
        href: optional(value, "href"),
    })
}

fn parse_show(value: &Value, base_url: &str) -> Option<StationShow> {
    let title = string_field(value, "title");
    if title.is_empty() {
        return None; // title requis, sinon rejeté
    }
    let href = optional(value, "href");
    let fit_contain = string_field(value, "imageFit").eq_ignore_ascii_case("contain");

    // Repli actionLabel : donné si présent ; sinon « Découvrir » (href) / « En rotation… ».
    let action_label = {
        let given = string_field(value, "actionLabel");
        if !given.is_empty() {
            given
        } else if href.is_some() {
            "Découvrir".to_string()
        } else {
            "En rotation sur la radio".to_string()
        }
    };

    Some(StationShow {
        title,
        meta: string_field(value, "meta"),
        text: string_field(value, "text"),
        image: ContentImage::resolve(&string_field(value, "image"), base_url, fit_contain),
        href,
        action_label,
    })
}

fn array<'a>(root: &'a Value, key: &str) -> &'a [Value] {
    root.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn string_field(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

fn optional(value: &Value, key: &str) -> Option<String> {
    let s = string_field(value, key);
    (!s.is_empty()).then_some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://host/preprod/";

    #[test]
    fn parse_producteurs_et_emissions() {
        let data = r#"{
            "producers":[
                {"name":"Dr. John","role":"Production","image":"assets/media/producers/drjohn.webp","bio":"…"},
                {"role":"sans nom"}
            ],
            "shows":[
                {"title":"L'Autre Nuit","meta":"Émission","image":"assets/media/shows/x.webp","href":"https://x","actionLabel":"Écouter le podcast"},
                {"title":"Sans lien","meta":"Mixtapes","image":"y.webp","imageFit":"contain"}
            ]
        }"#;
        let v = parse(data, BASE);
        assert_eq!(v.producers.len(), 1); // le producteur sans nom est rejeté
        assert_eq!(v.producers[0].image.url.as_deref(), Some("https://host/preprod/assets/media/producers/drjohn.webp"));

        assert_eq!(v.shows.len(), 2);
        assert_eq!(v.shows[0].action_label, "Écouter le podcast");
        assert!(!v.shows[0].image.fit_contain);
        // Pas d'actionLabel + pas de href → « En rotation sur la radio ».
        assert_eq!(v.shows[1].action_label, "En rotation sur la radio");
        assert!(v.shows[1].image.fit_contain);
    }

    #[test]
    fn action_label_repli_decouvrir() {
        let data = r#"{"shows":[{"title":"T","href":"https://x"}]}"#;
        assert_eq!(parse(data, BASE).shows[0].action_label, "Découvrir");
    }
}
