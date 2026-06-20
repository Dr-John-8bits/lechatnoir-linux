//! Écran Actualités : navigation par année puis mois, sinon les 3 plus récentes.

use std::cell::RefCell;
use std::rc::Rc;

use relm4::gtk;
use relm4::gtk::prelude::*;

use lcn_core::content::NewsEntry;

use super::{body, card, clear, mono, page_scaffold, section_header, MONTHS_FR};
use crate::services::data_store::DataStore;

#[derive(Default, Clone)]
struct Selection {
    year: Option<String>,
    month: Option<u32>,
}

struct Inner {
    root: gtk::ScrolledWindow,
    sections: gtk::Box,
    data: DataStore,
    selection: RefCell<Selection>,
}

#[derive(Clone)]
pub struct NewsScreen {
    inner: Rc<Inner>,
}

impl NewsScreen {
    pub fn new(data: DataStore) -> Self {
        let (root, sections) = page_scaffold("Actualités");
        let screen = Self {
            inner: Rc::new(Inner { root, sections, data, selection: RefCell::new(Selection::default()) }),
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

        let news = self.inner.data.news();
        let selection = self.inner.selection.borrow().clone();

        // Années (décroissant).
        let mut years: Vec<String> = news.iter().filter_map(|n| n.year().map(str::to_string)).collect();
        years.sort();
        years.dedup();
        years.reverse();

        sections.append(&section_header("Année", None));
        sections.append(&self.year_pills(&years, selection.year.as_deref()));

        if let Some(year) = &selection.year {
            sections.append(&section_header("Mois", None));
            sections.append(&self.month_pills(&news, year, selection.month));
        } else {
            sections.append(&mono(
                "Pour parcourir les actualités, choisis d'abord une année, puis un mois.",
            ));
        }

        match (&selection.year, selection.month) {
            (Some(year), Some(month)) => {
                let label = format!("{} {}", MONTHS_FR[(month - 1) as usize], year);
                sections.append(&section_header(&label, None));
                let filtered: Vec<&NewsEntry> = news
                    .iter()
                    .filter(|n| n.year() == Some(year.as_str()) && n.month() == Some(month))
                    .collect();
                if filtered.is_empty() {
                    sections.append(&mono("Aucune actualité pour cette période."));
                } else {
                    for entry in filtered {
                        sections.append(&entry_block(entry));
                    }
                }
            }
            (Some(_), None) => {
                sections.append(&mono(
                    "Choisis maintenant un mois pour afficher les actualités de cette année.",
                ));
            }
            (None, _) => {
                sections.append(&section_header(
                    "Dernières actualités",
                    Some("Les trois infos les plus récentes de la station."),
                ));
                for entry in news.iter().take(3) {
                    sections.append(&entry_block(entry));
                }
            }
        }
    }

    fn year_pills(&self, years: &[String], selected: Option<&str>) -> gtk::FlowBox {
        let flow = pill_flow();
        for year in years {
            let pill = pill(year, selected == Some(year.as_str()));
            let screen = self.clone();
            let year = year.clone();
            pill.connect_clicked(move |_| {
                let mut sel = screen.inner.selection.borrow_mut();
                sel.year = Some(year.clone());
                sel.month = None; // changer d'année réinitialise le mois
                drop(sel);
                screen.refresh();
            });
            flow.insert(&pill, -1);
        }
        flow
    }

    fn month_pills(&self, news: &[NewsEntry], year: &str, selected: Option<u32>) -> gtk::FlowBox {
        let mut months: Vec<u32> = news
            .iter()
            .filter(|n| n.year() == Some(year))
            .filter_map(NewsEntry::month)
            .collect();
        months.sort_unstable();
        months.dedup();

        let flow = pill_flow();
        for month in months {
            let pill = pill(MONTHS_FR[(month - 1) as usize], selected == Some(month));
            let screen = self.clone();
            pill.connect_clicked(move |_| {
                screen.inner.selection.borrow_mut().month = Some(month);
                screen.refresh();
            });
            flow.insert(&pill, -1);
        }
        flow
    }
}

fn entry_block(entry: &NewsEntry) -> gtk::Box {
    let block = card();
    let date = gtk::Label::new(Some(&entry.date_label.to_uppercase()));
    date.add_css_class("lcn-data");
    date.add_css_class("lcn-accent");
    date.set_halign(gtk::Align::Start);
    date.set_xalign(0.0);
    block.append(&date);

    let title = gtk::Label::new(Some(&entry.title));
    title.add_css_class("lcn-section-title");
    title.set_wrap(true);
    title.set_xalign(0.0);
    title.set_halign(gtk::Align::Start);
    block.append(&title);

    if !entry.lead.is_empty() {
        block.append(&body(&entry.lead));
    }
    if !entry.body.is_empty() {
        block.append(&body(&entry.body));
    }
    block
}

fn pill(label: &str, selected: bool) -> gtk::Button {
    let b = gtk::Button::with_label(label);
    b.add_css_class("lcn-pill");
    if selected {
        b.add_css_class("selected");
    }
    b
}

fn pill_flow() -> gtk::FlowBox {
    let flow = gtk::FlowBox::new();
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_max_children_per_line(12);
    flow.set_column_spacing(8);
    flow.set_row_spacing(8);
    flow.set_hexpand(true);
    flow
}
