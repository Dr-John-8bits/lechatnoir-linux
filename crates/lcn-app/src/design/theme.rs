//! Pose la palette LCN (tokens `04-DESIGN-SYSTEM.md` §4.1) dans un `CssProvider`
//! GTK via `@define-color`, et la fait suivre le thème clair/sombre du système
//! grâce au `StyleManager` libadwaita (portail XDG `color-scheme`, cross-DE).
//!
//! Le voile d'accent change d'opacité selon le thème (0.10 clair / 0.14 sombre) :
//! les deux valeurs sont dans la table, jamais une valeur unique.

use relm4::adw;
use relm4::gtk;

/// Un token de couleur de la charte, en versions claire et sombre.
struct Token {
    /// Nom de la couleur CSS GTK (préfixe `lcn_` pour ne pas heurter les couleurs Adwaita).
    name: &'static str,
    light: &'static str,
    dark: &'static str,
}

/// Palette complète (hex exacts du cahier des charges). Ordre = §4.1.
const TOKENS: &[Token] = &[
    Token { name: "lcn_bg", light: "#f4eee0", dark: "#14110d" },
    Token { name: "lcn_surface", light: "#faf6ec", dark: "#1c1813" },
    Token { name: "lcn_surface_raised", light: "#fdfbf4", dark: "#241f18" },
    Token { name: "lcn_ink", light: "#1b1813", dark: "#ece4d2" },
    Token { name: "lcn_ink_soft", light: "#4a4336", dark: "#c9bfa9" },
    Token { name: "lcn_muted", light: "#6d644f", dark: "#938973" },
    Token { name: "lcn_line", light: "#d9cfba", dark: "#322c22" },
    Token { name: "lcn_line_strong", light: "#b8ac92", dark: "#4c4434" },
    Token { name: "lcn_accent", light: "#0c84cc", dark: "#36a6df" },
    Token { name: "lcn_accent_ink", light: "#0f5c8c", dark: "#4cb4e6" },
    Token { name: "lcn_accent_fill", light: "#0d5685", dark: "#2f9fd8" },
    Token { name: "lcn_accent_contrast", light: "#f6efe2", dark: "#14110d" },
    // Voile d'accent : opacité différente selon le thème (0.10 clair / 0.14 sombre).
    Token { name: "lcn_accent_veil", light: "rgba(12,132,204,0.10)", dark: "rgba(54,166,223,0.14)" },
    // BRAISE — réservée à la pastille DIRECT pleine (règle des deux accents).
    Token { name: "lcn_direct", light: "#b5371a", dark: "#ef6a40" },
    Token { name: "lcn_warn", light: "#8a5e0c", dark: "#cfa14a" },
];

/// Règles statiques de la charte (référencent les `@define-color` ci-dessus).
const BASE_CSS: &str = include_str!("style.css");

/// Construit la feuille CSS complète pour le mode demandé.
fn full_css(dark: bool) -> String {
    let mut css = String::with_capacity(BASE_CSS.len() + 1024);
    for token in TOKENS {
        let value = if dark { token.dark } else { token.light };
        css.push_str("@define-color ");
        css.push_str(token.name);
        css.push(' ');
        css.push_str(value);
        css.push_str(";\n");
    }
    css.push('\n');
    css.push_str(BASE_CSS);
    css
}

/// Installe le provider CSS sur le display courant et l'actualise à chaque
/// changement clair/sombre du système. À appeler une fois au `startup` de l'app.
pub fn install(style_manager: &adw::StyleManager) {
    let display = gtk::gdk::Display::default().expect("aucun display GDK disponible");
    let provider = gtk::CssProvider::new();
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Hook captures/CI : LCN_FORCE_THEME force le choix clair/sombre au niveau CSS,
    // indépendamment du portail (déterministe en headless). Vide = suit le système.
    let forced = std::env::var("LCN_FORCE_THEME").ok();
    // `load_from_string` est derrière la feature gtk `v4_12` ; `load_from_data(&str)` est
    // inconditionnel et équivalent (voir `reload_fn`).
    let reload = std::rc::Rc::new(reload_fn(provider, forced));
    reload(style_manager);
    // On recharge sur les DEUX signaux : `dark` (bascule du thème système) ET `color-scheme`
    // (bascule explicite via le sélecteur in-app `set_color_scheme`). Sans le second, cliquer
    // « Sombre » assombrissait la chrome libadwaita SANS recharger nos @lcn_* → encre claire
    // sur fond sombre = texte illisible. (bug remonté par le test GNOME du 21/06/2026.)
    let r = reload.clone();
    style_manager.connect_dark_notify(move |sm| r(sm));
    style_manager.connect_color_scheme_notify(move |sm| reload(sm));
}

/// Fabrique la fonction de (re)chargement du CSS pour un provider + un éventuel thème forcé.
fn reload_fn(
    provider: gtk::CssProvider,
    forced: Option<String>,
) -> impl Fn(&adw::StyleManager) {
    move |sm: &adw::StyleManager| {
        let dark = match forced.as_deref() {
            Some("dark") => true,
            Some("light") => false,
            _ => sm.is_dark(),
        };
        provider.load_from_data(&full_css(dark));
    }
}

/// Applique la préférence de thème (3 modes, défaut Auto = suit le système).
pub fn set_preference(style_manager: &adw::StyleManager, pref: ThemePreference) {
    style_manager.set_color_scheme(match pref {
        ThemePreference::Auto => adw::ColorScheme::Default,
        ThemePreference::Light => adw::ColorScheme::ForceLight,
        ThemePreference::Dark => adw::ColorScheme::ForceDark,
    });
}

/// Préférence de thème persistée (clé GSettings `lcn-theme-preference`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreference {
    Auto,
    Light,
    Dark,
}
