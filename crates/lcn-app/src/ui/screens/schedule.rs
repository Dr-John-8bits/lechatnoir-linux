//! Écran Grille : résumés (aujourd'hui / à l'antenne) + pills de jours + détail du jour
//! avec créneau courant mis en avant et badges (direct/à l'antenne).

use std::cell::RefCell;
use std::rc::Rc;

use relm4::gtk;
use relm4::gtk::prelude::*;

use lcn_core::clock;
use lcn_core::content::ScheduleSlot;
use lcn_core::services::schedule::Schedule;

use super::{badge, card, clear, mono, page_scaffold, section_header, time_label};
use crate::services::data_store::DataStore;

struct Inner {
    root: gtk::ScrolledWindow,
    sections: gtk::Box,
    data: DataStore,
    selected_day: RefCell<Option<String>>,
}

#[derive(Clone)]
pub struct ScheduleScreen {
    inner: Rc<Inner>,
}

impl ScheduleScreen {
    pub fn new(data: DataStore) -> Self {
        let (root, sections) = page_scaffold("Grille");
        let screen = Self {
            inner: Rc::new(Inner { root, sections, data, selected_day: RefCell::new(None) }),
        };
        screen.refresh();
        screen
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.inner.root
    }

    pub fn refresh(&self) {
        let sections = &self.inner.sections;
        clear(sections);

        let Some(schedule) = self.inner.data.schedule() else {
            sections.append(&mono("La grille du jour sera disponible ici."));
            return;
        };

        let today_id = clock::current_day_id();
        let today_minute = clock::current_minute_of_day();
        let is_live = self.inner.data.is_live();

        sections.append(&self.summaries(&schedule, today_id, today_minute));

        // Pills de jours (défaut = jour courant).
        let selected_id = self
            .inner
            .selected_day
            .borrow()
            .clone()
            .unwrap_or_else(|| today_id.to_string());
        sections.append(&section_header("Jour de la semaine", None));
        sections.append(&self.day_pills(&schedule, &selected_id));

        // Détail du jour sélectionné.
        if let Some(day) = schedule.day(&selected_id) {
            let detail = card();
            detail.append(&section_header(&day.name, Some(&day.summary)));
            let current = if selected_id == today_id {
                Schedule::current_slot_index(day, today_minute)
            } else {
                None
            };
            for (i, slot) in day.slots.iter().enumerate() {
                detail.append(&slot_row(slot, Some(i) == current, is_live));
            }
            sections.append(&detail);
        }
    }

    fn summaries(&self, schedule: &Schedule, today_id: &str, today_minute: u32) -> gtk::Box {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 18);
        row.set_homogeneous(true);

        // Aujourd'hui.
        let today_card = card();
        match schedule.day(today_id) {
            Some(day) => {
                today_card.append(&section_header("Aujourd'hui", Some(&day.name)));
                today_card.append(&mono(if day.summary.is_empty() {
                    "La grille du jour sera disponible ici."
                } else {
                    &day.summary
                }));
            }
            None => {
                today_card.append(&section_header("Aujourd'hui", Some("Jour inconnu")));
                today_card.append(&mono("La grille du jour sera disponible ici."));
            }
        }
        row.append(&today_card);

        // À l'antenne (créneau courant).
        let onair_card = card();
        onair_card.append(&section_header("À l'antenne", None));
        let current_slot = schedule
            .day(today_id)
            .and_then(|d| Schedule::current_slot_index(d, today_minute).and_then(|i| d.slots.get(i)));
        match current_slot {
            Some(slot) => {
                let title = gtk::Label::new(Some(&slot.title));
                title.set_wrap(true);
                title.set_xalign(0.0);
                title.set_halign(gtk::Align::Start);
                onair_card.append(&title);
                onair_card.append(&mono(if slot.desc.is_empty() {
                    "Programmation en cours"
                } else {
                    &slot.desc
                }));
            }
            None => {
                onair_card.append(&mono("Rien à l'antenne pour le moment."));
            }
        }
        row.append(&onair_card);
        row
    }

    fn day_pills(&self, schedule: &Schedule, selected_id: &str) -> gtk::FlowBox {
        let flow = gtk::FlowBox::new();
        flow.set_selection_mode(gtk::SelectionMode::None);
        flow.set_max_children_per_line(7);
        flow.set_column_spacing(8);
        flow.set_row_spacing(8);
        flow.set_hexpand(true);

        for day in &schedule.days {
            let pill = gtk::Button::with_label(&day.short_name);
            pill.add_css_class("lcn-pill");
            if day.id == selected_id {
                pill.add_css_class("selected");
            }
            let screen = self.clone();
            let id = day.id.clone();
            pill.connect_clicked(move |_| {
                *screen.inner.selected_day.borrow_mut() = Some(id.clone());
                screen.refresh();
            });
            flow.insert(&pill, -1);
        }
        flow
    }
}

fn slot_row(slot: &ScheduleSlot, is_current: bool, is_live: bool) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_margin_top(2);
    if is_current {
        row.add_css_class("lcn-current-slot");
    }

    let time = time_label(&slot.time, is_current);
    time.set_width_request(54);
    row.append(&time);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    text.set_hexpand(true);

    let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = gtk::Label::new(Some(&slot.title));
    title.set_wrap(true);
    title.set_xalign(0.0);
    title.set_halign(gtk::Align::Start);
    if slot.highlight || is_current {
        title.add_css_class("lcn-bold");
    }
    title_row.append(&title);
    if let Some(badge_text) = &slot.badge {
        title_row.append(&badge(badge_text, &[]));
    }
    if is_current {
        if is_live {
            title_row.append(&badge("direct", &["live"]));
        } else {
            title_row.append(&badge("à l'antenne", &[]));
        }
    }
    text.append(&title_row);

    if !slot.desc.is_empty() {
        text.append(&mono(&slot.desc));
    }
    row.append(&text);
    row
}
