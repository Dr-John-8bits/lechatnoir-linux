//! Modèles du contenu éditorial mutualisé (news, grille, voix) + résolution d'image.
//! Structures pures ; le parsing vit dans `services::{news,schedule,voices}`.

/// Image résolue : URL absolue (si présente) + nom de base (pour un repli embarqué).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentImage {
    /// URL absolue prête à charger (None si chemin vide).
    pub url: Option<String>,
    /// Nom de base sans extension (pour retrouver une image embarquée hors-ligne).
    pub base_name: Option<String>,
    /// `imageFit == "contain"` → affichage *fit* ; sinon *fill*.
    pub fit_contain: bool,
}

impl ContentImage {
    /// Résout un chemin d'image (§3.5) contre la base de contenu.
    /// - vide → pas d'image ; absolu `http(s)://` → tel quel ; relatif → `base_url + chemin`.
    pub fn resolve(raw: &str, base_url: &str, fit_contain: bool) -> Self {
        let raw = raw.trim();
        if raw.is_empty() {
            return Self { url: None, base_name: None, fit_contain };
        }
        let url = if raw.starts_with("http://") || raw.starts_with("https://") {
            raw.to_string()
        } else {
            format!("{base_url}{raw}")
        };
        Self { base_name: base_name_without_ext(raw), url: Some(url), fit_contain }
    }
}

/// Dernier composant de chemin, sans extension (ex. `assets/media/x/foo.webp` → `foo`).
fn base_name_without_ext(path: &str) -> Option<String> {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(file);
    (!stem.is_empty()).then(|| stem.to_string())
}

/// Une actualité (champs consommés ; le HTML est ignoré, on prend le texte brut).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewsEntry {
    pub stable_id: String,
    pub title: String,
    pub sort_key: String,
    pub date_label: String,
    pub lead: String,
    pub body: String,
}

impl NewsEntry {
    /// Année extraite de `sortKey` (`AAAA-MM-...`).
    pub fn year(&self) -> Option<&str> {
        self.sort_key.get(0..4).filter(|y| y.chars().all(|c| c.is_ascii_digit()))
    }

    /// Mois (1..12) extrait de `sortKey` (`AAAA-MM-...`).
    pub fn month(&self) -> Option<u32> {
        self.sort_key.get(5..7).and_then(|m| m.parse::<u32>().ok()).filter(|m| (1..=12).contains(m))
    }
}

/// Un créneau de la grille.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScheduleSlot {
    /// Heure d'affichage (`"07h00"`, `"Puis"`…).
    pub time: String,
    pub title: String,
    pub desc: String,
    pub meta: bool,
    pub highlight: bool,
    pub badge: Option<String>,
    pub kind: String,
    /// Début en minutes depuis minuit (Paris), si déterminable.
    pub start_min: Option<u32>,
    /// Fin en minutes depuis minuit (Paris), si déterminable.
    pub end_min: Option<u32>,
}

/// Un jour de la grille.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScheduleDay {
    pub id: String,
    pub name: String,
    pub short_name: String,
    pub summary: String,
    pub slots: Vec<ScheduleSlot>,
}

/// Une voix (producteur·ice / projet).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProducerProfile {
    pub name: String,
    pub role: String,
    pub bio: String,
    pub image: ContentImage,
    pub href: Option<String>,
}

/// Une émission / format.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StationShow {
    pub title: String,
    pub meta: String,
    pub text: String,
    pub image: ContentImage,
    pub href: Option<String>,
    pub action_label: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_resolution() {
        let base = "https://host/preprod/";
        let rel = ContentImage::resolve("assets/media/shows/lautrenuit.webp", base, true);
        assert_eq!(rel.url.as_deref(), Some("https://host/preprod/assets/media/shows/lautrenuit.webp"));
        assert_eq!(rel.base_name.as_deref(), Some("lautrenuit"));
        assert!(rel.fit_contain);

        let abs = ContentImage::resolve("https://x/y/z.png", base, false);
        assert_eq!(abs.url.as_deref(), Some("https://x/y/z.png"));
        assert_eq!(abs.base_name.as_deref(), Some("z"));

        let empty = ContentImage::resolve("", base, false);
        assert_eq!(empty.url, None);
        assert_eq!(empty.base_name, None);
    }

    #[test]
    fn news_year_month() {
        let n = NewsEntry { sort_key: "2026-05-24-001".into(), ..Default::default() };
        assert_eq!(n.year(), Some("2026"));
        assert_eq!(n.month(), Some(5));
        let bad = NewsEntry { sort_key: "".into(), ..Default::default() };
        assert_eq!(bad.year(), None);
        assert_eq!(bad.month(), None);
    }
}
