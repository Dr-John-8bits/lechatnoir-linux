//! Sidebar : en-tête marque + 6 rubriques + carte de statut (kicker, pastille,
//! version, sélecteur de thème). M0 : la carte de statut est un placeholder
//! statique ; elle sera pilotée par l'état d'antenne au jalon M2.

use relm4::adw;
use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::ComponentSender;

use super::root::{RootInput, RootModel};
use super::section::Section;
use crate::design::theme::{self, ThemePreference};
use crate::services::settings;

/// Widgets de la sidebar exposés au composant racine.
pub struct SidebarParts {
    pub container: gtk::Box,
    /// Conservée pour une future synchro de sélection programmatique.
    #[allow(dead_code)]
    pub nav_list: gtk::ListBox,
    pub status: StatusHandles,
}

/// Widgets de la carte de statut, mis à jour par la chrome selon l'état d'antenne.
pub struct StatusHandles {
    pub kicker: gtk::Label,
    pub pill: gtk::Box,
    pub pill_dot: gtk::Label,
    pub pill_label: gtk::Label,
    pub conditional: gtk::Label,
}

pub fn build(sender: ComponentSender<RootModel>) -> SidebarParts {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 12);
    container.add_css_class("lcn-sidebar");
    container.set_width_request(280);
    container.set_margin_top(16);
    container.set_margin_bottom(16);
    container.set_margin_start(12);
    container.set_margin_end(12);

    container.append(&brand_header());

    let nav_list = build_nav_list(&sender);
    nav_list.set_vexpand(true);
    container.append(&nav_list);

    let (status_widget, status) = build_status_card();
    container.append(&status_widget);

    SidebarParts { container, nav_list, status }
}

fn brand_header() -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    header.set_margin_start(8);
    header.set_margin_end(8);
    header.set_margin_top(4);
    header.set_margin_bottom(8);

    let logo = crate::design::brand::logo_image(40);
    logo.set_valign(gtk::Align::Start);
    logo.update_property(&[gtk::accessible::Property::Label("Logo Le Chat Noir")]);
    header.append(&logo);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text.set_valign(gtk::Align::Center);
    text.set_hexpand(true);

    let name = gtk::Label::new(Some("Le Chat Noir"));
    name.add_css_class("lcn-brand-name");
    name.set_halign(gtk::Align::Start);
    name.set_xalign(0.0);

    let tagline = gtk::Label::new(Some("Laboratoire radiophonique indépendant"));
    tagline.add_css_class("lcn-brand-tagline");
    tagline.set_halign(gtk::Align::Start);
    // xalign 0 : sinon les lignes d'un label qui passe à la ligne se centrent (effet « flotte à droite »).
    tagline.set_xalign(0.0);
    tagline.set_wrap(true);

    text.append(&name);
    text.append(&tagline);
    header.append(&text);
    header
}

fn build_nav_list(sender: &ComponentSender<RootModel>) -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("navigation-sidebar");

    let mut first_row: Option<gtk::ListBoxRow> = None;
    for section in Section::ALL {
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let icon = gtk::Image::from_icon_name(section.icon_name());
        let label = gtk::Label::new(Some(section.title()));
        label.add_css_class("lcn-nav-row");
        label.set_halign(gtk::Align::Start);
        row_box.append(&icon);
        row_box.append(&label);

        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&row_box));
        list.append(&row);
        if first_row.is_none() {
            first_row = Some(row);
        }
    }

    let click_sender = sender.clone();
    list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            if let Some(section) = Section::from_index(row.index()) {
                click_sender.input(RootInput::Navigate(section));
            }
        }
    });

    if let Some(row) = first_row {
        list.select_row(Some(&row));
    }
    list
}

fn build_status_card() -> (gtk::Box, StatusHandles) {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    card.add_css_class("lcn-card");
    card.set_margin_start(4);
    card.set_margin_end(4);

    let kicker = gtk::Label::new(Some("// en ce moment"));
    kicker.add_css_class("lcn-kicker");
    kicker.set_halign(gtk::Align::Start);

    // Pastille de signal (état initial « hors ligne », pilotée par la chrome).
    let pill = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    pill.add_css_class("lcn-signal-pill");
    pill.add_css_class("offline");
    pill.set_halign(gtk::Align::Start);
    let dot = gtk::Label::new(Some("●"));
    let pill_label = gtk::Label::new(Some("hors ligne"));
    pill.append(&dot);
    pill.append(&pill_label);

    // Texte conditionnel (direct en cours / problème de lecture) — masqué par défaut.
    let conditional = gtk::Label::new(None);
    conditional.add_css_class("lcn-data");
    conditional.set_halign(gtk::Align::Start);
    conditional.set_wrap(true);
    conditional.set_xalign(0.0);
    conditional.set_visible(false);

    // Version affichée = version du paquet (Cargo.toml [workspace.package]), calculée au build.
    let version_text = format!("Version {}", env!("CARGO_PKG_VERSION"));
    let version = gtk::Label::new(Some(&version_text));
    version.add_css_class("lcn-version");
    version.set_halign(gtk::Align::Start);

    card.append(&kicker);
    card.append(&pill);
    card.append(&conditional);
    card.append(&version);
    card.append(&theme_selector());

    (card, StatusHandles { kicker, pill, pill_dot: dot, pill_label, conditional })
}

fn theme_selector() -> gtk::Box {
    let segmented = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    segmented.add_css_class("linked");
    segmented.set_halign(gtk::Align::Start);
    segmented.set_margin_top(4);

    let style_manager = adw::StyleManager::default();

    let auto = gtk::ToggleButton::with_label("Auto");
    let light = gtk::ToggleButton::with_label("Clair");
    let dark = gtk::ToggleButton::with_label("Sombre");
    light.set_group(Some(&auto));
    dark.set_group(Some(&auto));

    let sm = style_manager.clone();
    auto.connect_toggled(move |b| {
        if b.is_active() {
            theme::set_preference(&sm, ThemePreference::Auto);
            settings::set(settings::THEME, "auto");
        }
    });
    let sm = style_manager.clone();
    light.connect_toggled(move |b| {
        if b.is_active() {
            theme::set_preference(&sm, ThemePreference::Light);
            settings::set(settings::THEME, "light");
        }
    });
    let sm = style_manager.clone();
    dark.connect_toggled(move |b| {
        if b.is_active() {
            theme::set_preference(&sm, ThemePreference::Dark);
            settings::set(settings::THEME, "dark");
        }
    });

    // Restaure la préférence sauvegardée (LCN_FORCE_THEME prioritaire pour les tests/captures).
    // Activer le bouton déclenche le handler → applique le thème + ré-enregistre (sans effet).
    let initial = std::env::var("LCN_FORCE_THEME")
        .ok()
        .or_else(|| settings::get(settings::THEME))
        .unwrap_or_else(|| "auto".to_string());
    match initial.as_str() {
        "light" => light.set_active(true),
        "dark" => dark.set_active(true),
        _ => auto.set_active(true),
    }

    segmented.append(&auto);
    segmented.append(&light);
    segmented.append(&dark);
    segmented
}
