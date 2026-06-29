//! Écran Accueil : dernière actualité + deux colonnes (derniers titres / aujourd'hui).

use relm4::gtk;
use relm4::gtk::prelude::*;

use lcn_core::clock;
use lcn_core::services::schedule::Schedule;

use super::{body, card, clear, mono, page_scaffold, section_header, time_label};
use crate::services::data_store::{DataStore, LoadState};

pub struct HomeScreen {
    root: gtk::ScrolledWindow,
    sections: gtk::Box,
    data: DataStore,
}

impl HomeScreen {
    pub fn new(data: DataStore) -> Self {
        let (root, sections) = page_scaffold("Accueil");
        let screen = Self { root, sections, data };
        screen.refresh();
        screen
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.root
    }

    pub fn refresh(&self) {
        clear(&self.sections);
        self.sections.append(&self.latest_news());

        let columns = gtk::Box::new(gtk::Orientation::Horizontal, 18);
        columns.set_homogeneous(true);
        columns.append(&self.recent_tracks());
        columns.append(&self.today());
        self.sections.append(&columns);
    }

    fn latest_news(&self) -> gtk::Box {
        let news = self.data.news();
        let section = card();
        let first = news.first();
        section.append(&section_header(
            "Dernière actualité",
            first.map(|n| n.date_label.as_str()),
        ));
        match first {
            Some(n) => {
                let title = gtk::Label::new(Some(&n.title));
                title.add_css_class("lcn-section-title");
                title.set_wrap(true);
                title.set_xalign(0.0);
                title.set_halign(gtk::Align::Start);
                section.append(&title);
                // Accroche COURTE (chapô, sinon début du corps), tronquée à 2 lignes : l'accueil
                // donne envie d'aller lire, il n'affiche pas l'article entier (→ rubrique Actualités).
                let teaser_text = if !n.lead.is_empty() { n.lead.as_str() } else { n.body.as_str() };
                if !teaser_text.is_empty() {
                    let teaser = body(teaser_text);
                    teaser.set_lines(2);
                    teaser.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    section.append(&teaser);
                }
                // Appel à l'action vers la rubrique Actualités (via l'action app.show-news).
                let cta = gtk::Button::new();
                cta.add_css_class("flat");
                cta.set_halign(gtk::Align::Start);
                cta.set_margin_top(2);
                cta.set_action_name(Some("app.show-news"));
                let cta_label = gtk::Label::new(Some("Lire toutes les actualités  →"));
                cta_label.add_css_class("lcn-accent");
                cta.set_child(Some(&cta_label));
                section.append(&cta);
            }
            None => section.append(&body("Aucune actualité disponible pour le moment.")),
        }
        section
    }

    fn recent_tracks(&self) -> gtk::Box {
        let section = card();
        section.append(&section_header(
            "Derniers titres",
            Some("Les derniers passages diffusés à l'antenne."),
        ));
        match self.data.history_state() {
            LoadState::Loading => {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let spinner = gtk::Spinner::new();
                spinner.start();
                row.append(&spinner);
                row.append(&mono("Chargement des derniers passages…"));
                section.append(&row);
            }
            LoadState::Offline => {
                section.append(&mono("Derniers passages indisponibles — hors-ligne."));
            }
            LoadState::Ready => {
                let history = self.data.history();
                if history.is_empty() {
                    section.append(&mono("Aucun passage récent pour le moment."));
                } else {
                    for entry in history.iter().take(3) {
                        section.append(&track_row(
                            &entry.time_label(),
                            &entry.title,
                            &entry.metadata_line(),
                        ));
                    }
                }
            }
        }
        section
    }

    fn today(&self) -> gtk::Box {
        let section = card();
        section.append(&section_header(
            "Aujourd'hui",
            Some("Émissions en cours et à venir."),
        ));

        let Some(schedule) = self.data.schedule() else {
            section.append(&mono("La grille du jour sera disponible ici."));
            return section;
        };
        let Some(day) = schedule.day(clock::current_day_id()) else {
            section.append(&mono("La grille du jour sera disponible ici."));
            return section;
        };
        let current = Schedule::current_slot_index(day, clock::current_minute_of_day());

        // En ce moment.
        let now_header = gtk::Label::new(Some("En ce moment"));
        now_header.add_css_class("lcn-data");
        now_header.add_css_class("lcn-accent");
        now_header.set_halign(gtk::Align::Start);
        section.append(&now_header);
        match current.and_then(|i| day.slots.get(i)) {
            Some(slot) => {
                let title = gtk::Label::new(Some(&slot.title));
                title.add_css_class("lcn-body");
                title.set_wrap(true);
                title.set_xalign(0.0);
                title.set_halign(gtk::Align::Start);
                section.append(&title);
                section.append(&mono(&format!("{} • {}", slot.time, slot.desc)));
            }
            None => section.append(&mono("Rien à l'antenne pour le moment.")),
        }

        // À venir (jusqu'à 3 créneaux suivants).
        if let Some(i) = current {
            let upcoming: Vec<_> = day.slots.iter().skip(i + 1).take(3).collect();
            if !upcoming.is_empty() {
                let header = gtk::Label::new(Some("À venir"));
                header.add_css_class("lcn-section-title");
                header.set_halign(gtk::Align::Start);
                header.set_margin_top(6);
                section.append(&header);
                for slot in upcoming {
                    section.append(&track_row(&slot.time, &slot.title, &slot.desc));
                }
            }
        }
        section
    }
}

/// Ligne « heure (azur) + titre + ligne secondaire ».
fn track_row(time: &str, title: &str, secondary: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_margin_top(4);
    let time = time_label(time, true);
    time.set_width_request(52);
    row.append(&time);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    text.set_hexpand(true);
    let title_label = gtk::Label::new(Some(title));
    title_label.add_css_class("lcn-body");
    title_label.set_wrap(true);
    title_label.set_xalign(0.0);
    title_label.set_halign(gtk::Align::Start);
    text.append(&title_label);
    if !secondary.is_empty() {
        text.append(&mono(secondary));
    }
    row.append(&text);
    row
}
