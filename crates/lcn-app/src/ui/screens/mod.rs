//! Les 6 écrans de contenu. Chaque écran est une struct (widget racine + état) avec une
//! méthode `refresh(...)` qui reconstruit sa zone de contenu depuis le `DataStore`, en
//! préservant ses contrôles interactifs (pills, recherche…).

use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk;

pub mod about;
pub mod home;
pub mod history;
pub mod news;
pub mod schedule;
pub mod voices;

/// Mois en français (index 0 = janvier).
pub const MONTHS_FR: [&str; 12] = [
    "Janvier", "Février", "Mars", "Avril", "Mai", "Juin", "Juillet", "Août", "Septembre",
    "Octobre", "Novembre", "Décembre",
];

/// Coquille d'écran scrollable : titre de page figé + zone de sections reconstructible.
/// Renvoie le `ScrolledWindow` racine et le `Box` de sections (à repeupler dans `refresh`).
pub fn page_scaffold(title: &str) -> (gtk::ScrolledWindow, gtk::Box) {
    let title_label = gtk::Label::new(Some(title));
    title_label.add_css_class("lcn-page-title");
    title_label.set_halign(gtk::Align::Start);
    title_label.set_xalign(0.0);

    let sections = gtk::Box::new(gtk::Orientation::Vertical, 22);

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 18);
    outer.set_margin_top(28);
    outer.set_margin_bottom(28);
    outer.set_margin_start(8);
    outer.set_margin_end(8);
    outer.append(&title_label);
    outer.append(&sections);

    let clamp = adw::Clamp::builder().maximum_size(760).build();
    clamp.set_child(Some(&outer));

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);
    scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroller.set_child(Some(&clamp));
    (scroller, sections)
}

/// Vide un conteneur de tous ses enfants.
pub fn clear(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

/// Aère l'interligne d'un label multi-ligne (facteur de hauteur de ligne Pango).
fn set_line_height(label: &gtk::Label, factor: f64) {
    let attrs = gtk::pango::AttrList::new();
    attrs.insert(gtk::pango::AttrFloat::new_line_height(factor));
    label.set_attributes(Some(&attrs));
}

/// En-tête de section : titre + sous-titre optionnel.
pub fn section_header(title: &str, subtitle: Option<&str>) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 5);
    let t = gtk::Label::new(Some(title));
    t.add_css_class("lcn-section-title");
    t.set_halign(gtk::Align::Start);
    t.set_xalign(0.0);
    b.append(&t);
    if let Some(sub) = subtitle {
        b.append(&mono(sub));
    }
    b
}

/// Carte (surface + bordure + rayon).
pub fn card() -> gtk::Box {
    let c = gtk::Box::new(gtk::Orientation::Vertical, 8);
    c.add_css_class("lcn-card");
    c
}

/// Texte rédactionnel (sans), retour à la ligne.
pub fn body(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class("lcn-body");
    l.set_wrap(true);
    l.set_xalign(0.0);
    l.set_halign(gtk::Align::Start);
    set_line_height(&l, 1.35);
    l
}

/// Donnée radio (mono), retour à la ligne.
pub fn mono(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class("lcn-data");
    l.set_wrap(true);
    l.set_xalign(0.0);
    l.set_halign(gtk::Align::Start);
    set_line_height(&l, 1.35);
    l
}

/// Étiquette d'heure mono (azur si `accent`).
pub fn time_label(text: &str, accent: bool) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class("lcn-data");
    if accent {
        l.add_css_class("lcn-accent");
    }
    l.set_xalign(0.0);
    l.set_valign(gtk::Align::Start);
    l
}

/// Badge capsule (azur par défaut ; ajouter `"live"` pour la braise).
pub fn badge(text: &str, extra: &[&str]) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    b.add_css_class("lcn-badge");
    for c in extra {
        b.add_css_class(c);
    }
    b.set_halign(gtk::Align::Start);
    b.set_valign(gtk::Align::Center);
    b.append(&gtk::Label::new(Some(text)));
    b
}
