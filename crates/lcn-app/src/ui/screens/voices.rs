//! Écran Voix & Formats : deux grilles (producteur·ices, émissions) avec images
//! distantes (cache → réseau) et placeholder à initiales.

use relm4::gtk;
use relm4::gtk::prelude::*;

use lcn_core::content::{ContentImage, ProducerProfile, StationShow};

use super::{body, card, clear, mono, page_scaffold, section_header};
use crate::services::data_store::DataStore;
use crate::ui::image;

pub struct VoicesScreen {
    root: gtk::ScrolledWindow,
    sections: gtk::Box,
    data: DataStore,
}

impl VoicesScreen {
    pub fn new(data: DataStore) -> Self {
        let (root, sections) = page_scaffold("Les voix qui fabriquent la radio");
        let screen = Self { root, sections, data };
        screen.refresh();
        screen
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.root
    }

    pub fn refresh(&self) {
        clear(&self.sections);
        let Some(voices) = self.data.voices() else {
            self.sections.append(&mono("Les voix seront disponibles ici."));
            return;
        };

        self.sections.append(&section_header(
            "Les voix",
            Some("Les personnes et projets qui incarnent la station."),
        ));
        let producers: Vec<gtk::Box> = voices.producers.iter().map(producer_card).collect();
        self.sections.append(&grid3(producers));

        self.sections.append(&section_header(
            "Émissions et formats",
            Some("Les émissions, fictions et objets sonores de la radio."),
        ));
        let shows: Vec<gtk::Box> = voices.shows.iter().map(show_card).collect();
        self.sections.append(&grid3(shows));
    }
}

/// Grille déterministe à 3 colonnes homogènes (fiable, contrairement à FlowBox ici).
fn grid3(cards: Vec<gtk::Box>) -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.set_column_spacing(14);
    grid.set_row_spacing(14);
    grid.set_column_homogeneous(true);
    grid.set_hexpand(true);
    for (i, card) in cards.into_iter().enumerate() {
        grid.attach(&card, (i % 3) as i32, (i / 3) as i32, 1, 1);
    }
    grid
}

fn producer_card(producer: &ProducerProfile) -> gtk::Box {
    let c = card();
    c.set_width_request(200);
    c.set_hexpand(true);
    c.set_valign(gtk::Align::Start);

    c.append(&image_widget(&producer.image, 118, 118, &initials(&producer.name)));
    c.append(&name_widget(&producer.name, producer.href.as_deref()));

    if !producer.role.is_empty() {
        let role = gtk::Label::new(Some(&producer.role));
        role.add_css_class("lcn-data");
        role.add_css_class("lcn-accent");
        role.set_halign(gtk::Align::Start);
        role.set_xalign(0.0);
        c.append(&role);
    }
    if !producer.bio.is_empty() {
        c.append(&clamped_body(&producer.bio));
    }
    c
}

fn show_card(show: &StationShow) -> gtk::Box {
    let c = card();
    c.set_width_request(200);
    c.set_hexpand(true);
    c.set_valign(gtk::Align::Start);

    c.append(&image_widget(&show.image, 220, 132, &initials(&show.title)));

    let title = gtk::Label::new(Some(&show.title));
    title.add_css_class("lcn-section-title");
    title.set_wrap(true);
    title.set_xalign(0.0);
    title.set_halign(gtk::Align::Start);
    title.set_max_width_chars(24);
    c.append(&title);

    if !show.meta.is_empty() {
        let meta = gtk::Label::new(Some(&show.meta));
        meta.add_css_class("lcn-data");
        meta.add_css_class("lcn-accent");
        meta.set_halign(gtk::Align::Start);
        meta.set_xalign(0.0);
        c.append(&meta);
    }
    if !show.text.is_empty() {
        c.append(&clamped_body(&show.text));
    }

    // Action : lien si href, sinon simple texte.
    match &show.href {
        Some(href) => {
            let link = gtk::LinkButton::with_label(href, &format!("{} ↗", show.action_label));
            link.set_halign(gtk::Align::Start);
            c.append(&link);
        }
        None => {
            let action = mono(&show.action_label);
            c.append(&action);
        }
    }
    c
}

/// Nom cliquable (vers href, avec flèche) ou simple libellé.
fn name_widget(name: &str, href: Option<&str>) -> gtk::Widget {
    match href {
        Some(href) => {
            let link = gtk::LinkButton::with_label(href, &format!("{name} ↗"));
            link.set_halign(gtk::Align::Start);
            link.upcast()
        }
        None => {
            let label = gtk::Label::new(Some(name));
            label.add_css_class("lcn-section-title");
            label.set_wrap(true);
            label.set_xalign(0.0);
            label.set_halign(gtk::Align::Start);
            label.set_max_width_chars(24);
            label.upcast()
        }
    }
}

/// Image carrée/format ; placeholder à initiales si pas d'URL.
fn image_widget(image: &ContentImage, width: i32, height: i32, initials: &str) -> gtk::Widget {
    if image.url.is_some() {
        let picture = gtk::Picture::new();
        picture.set_size_request(width, height);
        picture.add_css_class("lcn-image");
        image::load(&picture, image);
        picture.upcast()
    } else {
        let placeholder = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        placeholder.set_size_request(width, height);
        placeholder.add_css_class("lcn-placeholder");
        let label = gtk::Label::new(Some(initials));
        label.set_hexpand(true);
        label.set_halign(gtk::Align::Center);
        label.set_valign(gtk::Align::Center);
        placeholder.append(&label);
        placeholder.upcast()
    }
}

fn clamped_body(text: &str) -> gtk::Label {
    let l = body(text);
    l.set_lines(4);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    // Plafonne la largeur naturelle pour que la FlowBox forme bien une grille (3 colonnes).
    l.set_max_width_chars(28);
    l.set_width_chars(20);
    l
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_uppercase()
}
