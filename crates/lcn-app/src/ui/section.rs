//! Les 6 rubriques de la sidebar, dans l'ordre figé du cahier des charges (§2.0).

/// Une rubrique de navigation. L'ordre de déclaration EST l'ordre d'affichage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Home,
    News,
    History,
    Schedule,
    Voices,
    About,
}

impl Section {
    /// Les 6 rubriques dans l'ordre exact (Accueil → À propos).
    pub const ALL: [Section; 6] = [
        Section::Home,
        Section::News,
        Section::History,
        Section::Schedule,
        Section::Voices,
        Section::About,
    ];

    /// Identifiant technique (nom de page du `gtk::Stack`).
    pub fn id(self) -> &'static str {
        match self {
            Section::Home => "home",
            Section::News => "news",
            Section::History => "history",
            Section::Schedule => "schedule",
            Section::Voices => "voices",
            Section::About => "about",
        }
    }

    /// Libellé affiché (verbatim app macOS).
    pub fn title(self) -> &'static str {
        match self {
            Section::Home => "Accueil",
            Section::News => "Actualités",
            Section::History => "Historique",
            Section::Schedule => "Grille",
            Section::Voices => "Voix & Formats",
            Section::About => "À propos",
        }
    }

    /// Icône symbolique GTK (équivalents du cahier des charges §2.0).
    pub fn icon_name(self) -> &'static str {
        match self {
            Section::Home => "go-home-symbolic",
            Section::News => "mail-unread-symbolic",
            Section::History => "document-open-recent-symbolic",
            Section::Schedule => "x-office-calendar-symbolic",
            Section::Voices => "audio-input-microphone-symbolic",
            Section::About => "help-about-symbolic",
        }
    }

    /// Retrouve une rubrique depuis l'index de ligne de la sidebar.
    pub fn from_index(index: i32) -> Option<Section> {
        usize::try_from(index).ok().and_then(|i| Section::ALL.get(i).copied())
    }

    /// Retrouve une rubrique depuis son identifiant technique.
    pub fn from_id(id: &str) -> Option<Section> {
        Section::ALL.into_iter().find(|s| s.id() == id)
    }

    /// Index de la rubrique dans l'ordre figé (= index de ligne sidebar).
    pub fn index(self) -> i32 {
        Section::ALL.iter().position(|s| *s == self).unwrap_or(0) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_and_roundtrip_are_stable() {
        // L'ordre figé : Accueil, Actualités, Historique, Grille, Voix & Formats, À propos.
        let ids: Vec<&str> = Section::ALL.iter().map(|s| s.id()).collect();
        assert_eq!(ids, ["home", "news", "history", "schedule", "voices", "about"]);
        for (i, section) in Section::ALL.iter().enumerate() {
            assert_eq!(Section::from_index(i as i32), Some(*section));
        }
        assert_eq!(Section::from_index(6), None);
        assert_eq!(Section::from_index(-1), None);
    }
}
