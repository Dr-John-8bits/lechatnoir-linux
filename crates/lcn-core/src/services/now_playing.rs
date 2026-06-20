//! Parse `nowplaying.json` — tolérant aux variantes de schéma, reproduit à l'identique
//! les règles macOS (§3.2) : alias de clés, détection DIRECT sur champs **bruts** avant
//! split, split artiste/titre, extraction album/année depuis le titre.

use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

/// Titre en cours, après normalisation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NowPlaying {
    pub artist: String,
    pub title: String,
    pub album: String,
    pub year: String,
    /// Direct détecté (convention BUTT) sur les champs bruts.
    pub is_live: bool,
}

impl NowPlaying {
    /// Titre à afficher : repli « Titre indisponible pour l'instant » si vide mais artiste présent.
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            "Titre indisponible pour l'instant"
        } else {
            &self.title
        }
    }

    /// Ligne secondaire : `[artist, album, year]` non vides joints par ` • ` (peut être vide).
    pub fn metadata_line(&self) -> String {
        [&self.artist, &self.album, &self.year]
            .into_iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" • ")
    }
}

/// Parse le payload (objet, ou tableau dont on prend le premier élément décodable).
pub fn parse(data: &str) -> Option<NowPlaying> {
    let root: Value = serde_json::from_str(data).ok()?;
    let candidates: Vec<&Value> = match root.as_array() {
        Some(arr) => arr.iter().collect(),
        None => vec![&root],
    };
    candidates.into_iter().find_map(parse_object)
}

fn parse_object(value: &Value) -> Option<NowPlaying> {
    // Portées de recherche : racine d'abord, puis sous-objets connus, puis now_playing.song.
    let mut scopes: Vec<&Value> = vec![value];
    for key in ["now_playing", "nowPlaying", "song", "track", "current", "data"] {
        if let Some(sub) = value.get(key) {
            if sub.is_object() {
                scopes.push(sub);
            }
        }
    }
    if let Some(song) = value.get("now_playing").and_then(|np| np.get("song")) {
        if song.is_object() {
            scopes.push(song);
        }
    }

    let artist_raw = lookup(&scopes, &["artist", "artist_name", "creator", "author", "performer", "dj", "host"]);
    let title_raw = lookup(&scopes, &["title", "name", "track", "song", "now_playing"]);
    let mut album = lookup(&scopes, &["album", "release", "record"]);
    let mut year = normalize_year(&lookup(&scopes, &["year", "date", "released", "release_year"]));

    // a) Détection DIRECT sur les champs BRUTS, avant tout split (convention BUTT).
    let is_live =
        re_title_direct().is_match(&title_raw) || re_artist_direct().is_match(&artist_raw);

    // b) Split artiste/titre si artiste vide : premier séparateur ` — ` (cadratin) puis ` - `.
    let mut artist = artist_raw;
    let mut title = title_raw;
    if artist.is_empty() && !title.is_empty() {
        for sep in [" — ", " - "] {
            if let Some(idx) = title.find(sep) {
                artist = title[..idx].trim().to_string();
                title = title[idx + sep.len()..].trim().to_string();
                break;
            }
        }
    }

    // c) Album/année depuis le titre (dernier groupe entre parenthèses), si encore vides.
    if album.is_empty() || year.is_empty() {
        if let Some((extracted_album, extracted_year)) = extract_album_year(&title) {
            if album.is_empty() {
                album = extracted_album;
            }
            if year.is_empty() {
                year = extracted_year;
            }
        }
    }

    // d) Validité : titre ET artiste vides → payload invalide.
    if artist.is_empty() && title.is_empty() {
        return None;
    }

    Some(NowPlaying { artist, title, album, year, is_live })
}

/// Premier champ non vide (portées dans l'ordre, puis clés dans l'ordre). Strings trimés,
/// nombres convertis en string.
fn lookup(scopes: &[&Value], keys: &[&str]) -> String {
    for scope in scopes {
        for &key in keys {
            if let Some(s) = string_at(scope, key) {
                return s;
            }
        }
    }
    String::new()
}

fn string_at(scope: &Value, key: &str) -> Option<String> {
    match scope.get(key) {
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

/// Extrait une année `(19|20)\d{2}` d'une valeur (ex. d'une date) ; sinon renvoie la valeur trimée.
fn normalize_year(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    re_year()
        .find(raw)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| raw.trim().to_string())
}

/// Depuis le titre : dernier groupe `(...)`, année = `(19|20)\d{2}`, album = le reste
/// (ponctuation finale `[,;/\-]` retirée).
fn extract_album_year(title: &str) -> Option<(String, String)> {
    let last = re_paren().captures_iter(title).last()?;
    let content = last.get(1)?.as_str().trim();
    let year = re_year().find(content).map(|m| m.as_str().to_string()).unwrap_or_default();

    let mut album = if year.is_empty() {
        content.to_string()
    } else {
        content.replacen(&year, "", 1)
    };
    album = re_trailing_punct().replace(album.trim(), "").trim().to_string();

    Some((album, year))
}

fn re_title_direct() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)^DIRECT\s*-").unwrap())
}
fn re_artist_direct() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)\(\s*DIRECT\s*\)").unwrap())
}
fn re_year() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(19|20)\d{2}").unwrap())
}
fn re_paren() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\(([^()]*)\)").unwrap())
}
fn re_trailing_punct() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"[,;/\-]\s*$").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echantillon_reel() {
        let np = parse(r#"{"artist":"Dried Beans","title":"Will you marry it?","album":"EX1","year":"2018","ts":1781951278}"#).unwrap();
        assert_eq!(np.artist, "Dried Beans");
        assert_eq!(np.title, "Will you marry it?");
        assert_eq!(np.album, "EX1");
        assert_eq!(np.year, "2018");
        assert!(!np.is_live);
        assert_eq!(np.metadata_line(), "Dried Beans • EX1 • 2018");
    }

    #[test]
    fn direct_sur_titre_brut() {
        // Tiret OBLIGATOIRE, ancré au début.
        assert!(parse(r#"{"title":"DIRECT - Session","artist":"X"}"#).unwrap().is_live);
        // Faux positif évité : « Direct Hit » sans tiret ancré.
        assert!(!parse(r#"{"title":"Direct Hit","artist":"X"}"#).unwrap().is_live);
    }

    #[test]
    fn direct_sur_artiste_brut() {
        assert!(parse(r#"{"artist":"Le Chat (DIRECT)","title":"X"}"#).unwrap().is_live);
    }

    #[test]
    fn split_artiste_titre() {
        let np = parse(r#"{"title":"Foo — Bar"}"#).unwrap();
        assert_eq!(np.artist, "Foo");
        assert_eq!(np.title, "Bar");
        let np2 = parse(r#"{"title":"Foo - Bar"}"#).unwrap();
        assert_eq!(np2.artist, "Foo");
        assert_eq!(np2.title, "Bar");
    }

    #[test]
    fn album_annee_depuis_titre() {
        let np = parse(r#"{"title":"Song (Great Album, 2019)"}"#).unwrap();
        assert_eq!(np.year, "2019");
        assert_eq!(np.album, "Great Album");
    }

    #[test]
    fn objet_imbrique_et_tableau() {
        let np = parse(r#"{"now_playing":{"song":{"artist":"A","title":"B"}}}"#).unwrap();
        assert_eq!((np.artist.as_str(), np.title.as_str()), ("A", "B"));
        let np2 = parse(r#"[{"artist":"A","title":"B"}]"#).unwrap();
        assert_eq!((np2.artist.as_str(), np2.title.as_str()), ("A", "B"));
    }

    #[test]
    fn annee_numerique_et_repli_titre() {
        let np = parse(r#"{"artist":"A","title":"B","year":2024}"#).unwrap();
        assert_eq!(np.year, "2024");
        // Titre vide mais artiste présent → valide, titre affiché = repli.
        let np2 = parse(r#"{"artist":"A","title":""}"#).unwrap();
        assert_eq!(np2.display_title(), "Titre indisponible pour l'instant");
    }

    #[test]
    fn payloads_invalides() {
        assert!(parse("{}").is_none());
        assert!(parse(r#"{"ts":1}"#).is_none());
        assert!(parse("pas du json").is_none());
    }
}
