//! Machine d'état d'antenne (vocabulaire FIGÉ, §2.1). Calculée **une seule fois** et
//! partagée par la sidebar ET la barre de lecture — c'est le cœur de l'« univers unifié ».
//! Tous les libellés sont verbatim et ne doivent jamais varier.

use crate::player::StreamHealth;

/// État d'antenne (≠ état du lecteur).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntenneState {
    /// Direct réel — la seule exception à l'accent azur (braise « on air »).
    Direct,
    ALAntenne,
    Connexion,
    HorsLigne,
}

/// Teinte sémantique (mappée en couleurs concrètes par l'UI). Braise = DIRECT uniquement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Azur,
    Braise,
    Muted,
}

impl AntenneState {
    /// Libellé de la pastille (verbatim §2.1).
    pub fn label(self) -> &'static str {
        match self {
            AntenneState::Direct => "direct — on air",
            AntenneState::ALAntenne => "à l'antenne",
            AntenneState::Connexion => "connexion…",
            AntenneState::HorsLigne => "hors ligne",
        }
    }

    /// Kicker mono (« // direct » en live, sinon « // en ce moment »).
    pub fn kicker(self) -> &'static str {
        match self {
            AntenneState::Direct => "// direct",
            _ => "// en ce moment",
        }
    }

    /// Teinte : braise pour le direct, muted pour hors-ligne, azur sinon.
    pub fn tint(self) -> Tint {
        match self {
            AntenneState::Direct => Tint::Braise,
            AntenneState::HorsLigne => Tint::Muted,
            _ => Tint::Azur,
        }
    }

    /// La pastille n'est « pleine » que pour le direct.
    pub fn is_filled(self) -> bool {
        matches!(self, AntenneState::Direct)
    }

    /// Libellé d'accessibilité (verbatim §2.1).
    pub fn accessibility_label(self) -> &'static str {
        match self {
            AntenneState::Direct => "En direct, à l'antenne",
            AntenneState::ALAntenne => "À l'antenne",
            AntenneState::Connexion => "Connexion au flux en cours",
            AntenneState::HorsLigne => "Hors ligne",
        }
    }
}

/// Résolution dans l'ordre EXACT (§2.1). `is_live` prime sur tout, même sans écoute locale.
pub fn resolve(
    health: StreamHealth,
    is_live: bool,
    is_playing: bool,
    metadata_fresh: bool,
) -> AntenneState {
    if is_live {
        AntenneState::Direct
    } else if health == StreamHealth::Failed {
        AntenneState::HorsLigne
    } else if health == StreamHealth::Connecting && is_playing {
        AntenneState::Connexion
    } else if is_playing || metadata_fresh {
        AntenneState::ALAntenne
    } else {
        AntenneState::HorsLigne
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::StreamHealth::*;

    #[test]
    fn direct_prime_sur_tout() {
        // Même flux échoué et sans écoute, un direct reste « direct — on air ».
        assert_eq!(resolve(Failed, true, false, false), AntenneState::Direct);
        assert_eq!(resolve(Active, true, true, true), AntenneState::Direct);
    }

    #[test]
    fn ordre_de_resolution() {
        // failed (hors live) → hors ligne.
        assert_eq!(resolve(Failed, false, true, true), AntenneState::HorsLigne);
        // connecting + lecture → connexion.
        assert_eq!(resolve(Connecting, false, true, false), AntenneState::Connexion);
        // connecting sans lecture mais métadonnées fraîches → à l'antenne.
        assert_eq!(resolve(Connecting, false, false, true), AntenneState::ALAntenne);
        // actif + lecture → à l'antenne.
        assert_eq!(resolve(Active, false, true, false), AntenneState::ALAntenne);
        // actif, pas de lecture, métadonnées fraîches → à l'antenne.
        assert_eq!(resolve(Active, false, false, true), AntenneState::ALAntenne);
        // rien : pas de lecture, métadonnées périmées → hors ligne.
        assert_eq!(resolve(Active, false, false, false), AntenneState::HorsLigne);
    }

    #[test]
    fn sorties_verbatim() {
        assert_eq!(AntenneState::Direct.label(), "direct — on air");
        assert_eq!(AntenneState::Direct.kicker(), "// direct");
        assert_eq!(AntenneState::Direct.tint(), Tint::Braise);
        assert!(AntenneState::Direct.is_filled());

        assert_eq!(AntenneState::ALAntenne.label(), "à l'antenne");
        assert_eq!(AntenneState::ALAntenne.kicker(), "// en ce moment");
        assert_eq!(AntenneState::ALAntenne.tint(), Tint::Azur);
        assert!(!AntenneState::ALAntenne.is_filled());

        assert_eq!(AntenneState::Connexion.label(), "connexion…");
        assert_eq!(AntenneState::Connexion.tint(), Tint::Azur);

        assert_eq!(AntenneState::HorsLigne.label(), "hors ligne");
        assert_eq!(AntenneState::HorsLigne.tint(), Tint::Muted);
    }
}
