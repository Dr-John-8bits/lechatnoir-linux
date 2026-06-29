//! Écran Historique : liste des dernières diffusions OU recherche par créneau (les 10
//! plus proches). La recherche télécharge l'archive complète sur un thread (ponctuel).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::thread;

use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::gtk::glib;

use lcn_core::services::history::{self, HistoryEntry};
use lcn_core::{clock, config};

use super::{clear, mono, page_scaffold, section_header, time_label, MONTHS_FR};
use crate::services::data_store::{DataStore, LoadState};

struct Inner {
    root: gtk::ScrolledWindow,
    results: gtk::Box,
    data: DataStore,
    calendar: gtk::Calendar,
    hour: gtk::DropDown,
    minute: gtk::DropDown,
    search_button: gtk::Button,
    latest_button: gtk::Button,
    search_active: Cell<bool>,
    searching: Cell<bool>,
    search_error: Cell<bool>,
    header_label: RefCell<String>,
    found: RefCell<Vec<HistoryEntry>>,
}

#[derive(Clone)]
pub struct HistoryScreen {
    inner: Rc<Inner>,
}

impl HistoryScreen {
    pub fn new(data: DataStore) -> Self {
        let (root, sections) = page_scaffold("Historique");

        let hours: Vec<String> = (0..24).map(|h| format!("{h:02}h")).collect();
        let minutes: Vec<String> = (0..60).map(|m| format!("{m:02}")).collect();
        let hour = dropdown(&hours);
        hour.update_property(&[gtk::accessible::Property::Label("Heure de recherche")]);
        let minute = dropdown(&minutes);
        minute.update_property(&[gtk::accessible::Property::Label("Minute de recherche")]);
        let now = clock::paris_now();

        let calendar = gtk::Calendar::new();
        let date_button = gtk::MenuButton::new();
        date_button.set_label(&format_calendar_date(&calendar));
        date_button.update_property(&[gtk::accessible::Property::Label("Date de recherche")]);
        let popover = gtk::Popover::new();
        popover.set_child(Some(&calendar));
        date_button.set_popover(Some(&popover));
        {
            let date_button = date_button.clone();
            calendar.connect_day_selected(move |cal| {
                date_button.set_label(&format_calendar_date(cal));
            });
        }

        let search_button = gtk::Button::with_label("Rechercher");
        search_button.add_css_class("suggested-action");
        let latest_button = gtk::Button::with_label("Voir les dernières");

        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        controls.set_halign(gtk::Align::Start);
        controls.append(&labeled("Date de recherche", &date_button));
        controls.append(&labeled("Heure de recherche", &hour));
        controls.append(&labeled("Minute de recherche", &minute));
        controls.append(&labeled(" ", &search_button));
        controls.append(&labeled(" ", &latest_button));
        sections.append(&controls);

        let results = gtk::Box::new(gtk::Orientation::Vertical, 10);
        sections.append(&results);

        let inner = Rc::new(Inner {
            root,
            results,
            data,
            calendar,
            hour,
            minute,
            search_button,
            latest_button,
            search_active: Cell::new(false),
            searching: Cell::new(false),
            search_error: Cell::new(false),
            header_label: RefCell::new(String::new()),
            found: RefCell::new(Vec::new()),
        });
        // Heure/minute par défaut = maintenant (Paris).
        inner.hour.set_selected(now.format("%H").to_string().parse::<u32>().unwrap_or(0));
        inner.minute.set_selected(now.format("%M").to_string().parse::<u32>().unwrap_or(0));

        let screen = Self { inner };
        screen.wire();
        screen.refresh();
        screen
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.inner.root
    }

    fn wire(&self) {
        let s = self.clone();
        self.inner.search_button.connect_clicked(move |_| s.start_search());
        let s = self.clone();
        self.inner.latest_button.connect_clicked(move |_| {
            s.inner.search_active.set(false);
            s.refresh();
        });
    }

    /// Reconstruit la zone de résultats (les contrôles persistent).
    pub fn refresh(&self) {
        let inner = &self.inner;
        let busy = inner.searching.get();
        inner.search_button.set_sensitive(!busy);
        inner.latest_button.set_visible(inner.search_active.get());
        inner.latest_button.set_sensitive(!busy);
        inner.calendar.set_sensitive(!busy);
        inner.hour.set_sensitive(!busy);
        inner.minute.set_sensitive(!busy);

        clear(&inner.results);

        if inner.search_active.get() {
            self.render_search();
        } else {
            self.render_list();
        }
    }

    fn render_list(&self) {
        let inner = &self.inner;
        match inner.data.history_state() {
            LoadState::Loading => {
                inner.results.append(&section_header("Dernières diffusions", None));
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let spinner = gtk::Spinner::new();
                spinner.start();
                row.append(&spinner);
                row.append(&mono("Chargement de l'historique…"));
                inner.results.append(&row);
            }
            LoadState::Offline => {
                inner.results.append(&section_header("Dernières diffusions", None));
                inner.results
                    .append(&mono("Historique indisponible — vérifiez votre connexion."));
            }
            LoadState::Ready => {
                let history = inner.data.history();
                inner.results.append(&section_header(
                    "Dernières diffusions",
                    Some(&format!("{} lignes récentes affichées.", history.len())),
                ));
                if history.is_empty() {
                    inner.results.append(&mono("Aucune diffusion récente pour le moment."));
                } else {
                    for entry in &history {
                        inner.results.append(&entry_row(entry));
                    }
                }
            }
        }
    }

    fn render_search(&self) {
        let inner = &self.inner;
        let header = format!("Titres les plus proches du {}", inner.header_label.borrow());
        if inner.searching.get() {
            inner.results.append(&section_header(
                &header,
                Some("Recherche dans l'archive complète en cours…"),
            ));
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let spinner = gtk::Spinner::new();
            spinner.start();
            row.append(&spinner);
            row.append(&mono("Recherche…"));
            inner.results.append(&row);
            return;
        }
        if inner.search_error.get() {
            inner.results.append(&section_header(&header, None));
            inner.results
                .append(&mono("Impossible de charger l'archive complète pour la recherche."));
            return;
        }
        let found = inner.found.borrow();
        if found.is_empty() {
            inner.results.append(&section_header(
                &header,
                Some("Aucune diffusion trouvée pour ce jour-là."),
            ));
            inner.results.append(&mono("Aucun titre trouvé pour ce créneau."));
        } else {
            inner.results.append(&section_header(
                &header,
                Some(&format!(
                    "{} diffusion(s) la(s) plus proche(s) du créneau demandé.",
                    found.len()
                )),
            ));
            for entry in found.iter() {
                inner.results.append(&entry_row(entry));
            }
        }
    }

    fn start_search(&self) {
        let inner = &self.inner;
        let date = inner.calendar.date();
        let (year, month, day) = (date.year(), date.month(), date.day_of_month());
        let hour = inner.hour.selected();
        let minute = inner.minute.selected();

        let Some(target) = clock::paris_datetime(year, month as u32, day as u32, hour, minute) else {
            return;
        };

        *inner.header_label.borrow_mut() = format!(
            "{} {} {} à {:02}:{:02}",
            day,
            MONTHS_FR[(month as usize - 1).min(11)].to_lowercase(),
            year,
            hour,
            minute
        );
        inner.search_active.set(true);
        inner.searching.set(true);
        inner.search_error.set(false);
        self.refresh();

        let (tx, rx) = async_channel::bounded::<Option<Vec<HistoryEntry>>>(1);
        thread::spawn(move || {
            let result = fetch_full_archive().map(|text| {
                let all = history::parse_csv(&text, None);
                history::search_closest(&all, target, config::HISTORY_SEARCH_RESULTS)
            });
            let _ = tx.send_blocking(result);
        });

        let screen = self.clone();
        glib::spawn_future_local(async move {
            let outcome = rx.recv().await.ok().flatten();
            match outcome {
                Some(found) => {
                    *screen.inner.found.borrow_mut() = found;
                    screen.inner.search_error.set(false);
                }
                None => screen.inner.search_error.set(true),
            }
            screen.inner.searching.set(false);
            screen.refresh();
        });
    }
}

fn fetch_full_archive() -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(config::HTTP_RESOURCE_TIMEOUT)
        .user_agent(config::USER_AGENT)
        .build()
        .ok()?;
    let response = client.get(config::HISTORY_CSV_URL).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.text().ok()
}

fn entry_row(entry: &HistoryEntry) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_margin_top(2);
    let time = time_label(&entry.time_label(), true);
    time.set_width_request(54);
    row.append(&time);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    text.set_hexpand(true);
    let title = gtk::Label::new(Some(&entry.title));
    title.add_css_class("lcn-body");
    title.set_wrap(true);
    title.set_xalign(0.0);
    title.set_halign(gtk::Align::Start);
    text.append(&title);

    let secondary = entry.metadata_line();
    text.append(&mono(if secondary.is_empty() {
        "Diffusion sans métadonnées complètes"
    } else {
        &secondary
    }));
    row.append(&text);
    row
}

fn dropdown(items: &[String]) -> gtk::DropDown {
    let refs: Vec<&str> = items.iter().map(String::as_str).collect();
    gtk::DropDown::from_strings(&refs)
}

fn labeled(label: &str, child: &impl IsA<gtk::Widget>) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let l = gtk::Label::new(Some(label));
    l.add_css_class("lcn-kicker");
    l.set_halign(gtk::Align::Start);
    b.append(&l);
    b.append(child);
    b.set_valign(gtk::Align::End);
    b
}

fn format_calendar_date(calendar: &gtk::Calendar) -> String {
    let date = calendar.date();
    format!(
        "{} {} {}",
        date.day_of_month(),
        MONTHS_FR[(date.month() as usize - 1).min(11)].to_lowercase(),
        date.year()
    )
}
