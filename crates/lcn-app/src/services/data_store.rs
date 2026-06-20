//! `DataStore` — agrège tout l'état (temps réel + éditorial) et le diffuse à l'UI.
//! Vit sur le thread principal (Rc) ; les threads de polling lui poussent des [`Update`]
//! via un canal (voir `net`). Politique : **garder la dernière valeur connue** en cas
//! d'échec, ne jamais écraser avec du vide. Initialisé depuis le snapshot embarqué pour
//! un premier rendu instantané.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use lcn_core::config;
use lcn_core::content::NewsEntry;
use lcn_core::services::current_show::CurrentShow;
use lcn_core::services::history::HistoryEntry;
use lcn_core::services::now_playing::NowPlaying;
use lcn_core::services::schedule::Schedule;
use lcn_core::services::snapshot;
use lcn_core::services::voices::Voices;

/// État de chargement de l'historique (consommé par l'écran Historique).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    Loading,
    Ready,
    Offline,
}

/// Catégorie de mise à jour, pour rafraîchir uniquement les écrans concernés.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateKind {
    /// nowplaying / current-show (état d'antenne, titre).
    Meta,
    History,
    News,
    Schedule,
    Voices,
}

/// Message poussé par un thread de polling vers le thread UI.
#[derive(Debug)]
pub enum Update {
    NowPlaying(NowPlaying),
    CurrentShow(CurrentShow),
    History(Vec<HistoryEntry>),
    News(Vec<NewsEntry>),
    Schedule(Schedule),
    Voices(Voices),
    MetaFailed,
    HistoryFailed,
}

struct State {
    now_playing: Option<NowPlaying>,
    current_show: Option<CurrentShow>,
    history: Vec<HistoryEntry>,
    news: Vec<NewsEntry>,
    schedule: Option<Schedule>,
    voices: Option<Voices>,
    last_meta_success: Option<Instant>,
    history_loaded: bool,
    history_offline: bool,
}

type UpdateCallback = Rc<RefCell<Option<Rc<dyn Fn(UpdateKind)>>>>;

/// Poignée clonable vers l'état partagé.
#[derive(Clone)]
pub struct DataStore {
    state: Rc<RefCell<State>>,
    on_update: UpdateCallback,
}

impl DataStore {
    /// Initialise depuis le snapshot embarqué (premier rendu instantané + repli hors-ligne).
    pub fn new() -> Self {
        let state = State {
            now_playing: None,
            current_show: None,
            history: Vec::new(),
            news: snapshot::news(),
            schedule: Some(snapshot::schedule()),
            voices: Some(snapshot::voices()),
            last_meta_success: None,
            history_loaded: false,
            history_offline: false,
        };
        Self {
            state: Rc::new(RefCell::new(state)),
            on_update: Rc::new(RefCell::new(None)),
        }
    }

    /// Callback invoqué après chaque mise à jour, avec sa catégorie.
    pub fn set_on_update(&self, callback: impl Fn(UpdateKind) + 'static) {
        *self.on_update.borrow_mut() = Some(Rc::new(callback));
    }

    /// Applique une mise à jour, puis notifie l'UI avec la catégorie concernée.
    pub fn apply(&self, update: Update) {
        let kind = {
            let mut s = self.state.borrow_mut();
            match update {
                Update::NowPlaying(np) => {
                    s.now_playing = Some(np);
                    s.last_meta_success = Some(Instant::now());
                    UpdateKind::Meta
                }
                Update::CurrentShow(cs) => {
                    s.current_show = Some(cs);
                    s.last_meta_success = Some(Instant::now());
                    UpdateKind::Meta
                }
                Update::History(entries) => {
                    if !entries.is_empty() {
                        s.history = entries;
                    }
                    s.history_loaded = true;
                    s.history_offline = false;
                    UpdateKind::History
                }
                Update::News(items) => {
                    if !items.is_empty() {
                        s.news = items;
                    }
                    UpdateKind::News
                }
                Update::Schedule(schedule) => {
                    if !schedule.days.is_empty() {
                        s.schedule = Some(schedule);
                    }
                    UpdateKind::Schedule
                }
                Update::Voices(voices) => {
                    if !voices.producers.is_empty() || !voices.shows.is_empty() {
                        s.voices = Some(voices);
                    }
                    UpdateKind::Voices
                }
                Update::MetaFailed => UpdateKind::Meta,
                Update::HistoryFailed => {
                    if !s.history_loaded {
                        s.history_offline = true;
                    }
                    UpdateKind::History
                }
            }
        };
        self.notify(kind);
    }

    fn notify(&self, kind: UpdateKind) {
        let cb = self.on_update.borrow().clone();
        if let Some(cb) = cb {
            cb(kind);
        }
    }

    pub fn now_playing(&self) -> Option<NowPlaying> {
        self.state.borrow().now_playing.clone()
    }

    pub fn current_show(&self) -> Option<CurrentShow> {
        self.state.borrow().current_show.clone()
    }

    pub fn history(&self) -> Vec<HistoryEntry> {
        self.state.borrow().history.clone()
    }

    pub fn news(&self) -> Vec<NewsEntry> {
        self.state.borrow().news.clone()
    }

    pub fn schedule(&self) -> Option<Schedule> {
        self.state.borrow().schedule.clone()
    }

    pub fn voices(&self) -> Option<Voices> {
        self.state.borrow().voices.clone()
    }

    /// Fusion en OU des deux sources de direct.
    pub fn is_live(&self) -> bool {
        let s = self.state.borrow();
        s.current_show.as_ref().is_some_and(|c| c.is_live)
            || s.now_playing.as_ref().is_some_and(|n| n.is_live)
    }

    /// Métadonnées fraîches = un fetch a réussi il y a moins de 90 s.
    pub fn metadata_fresh(&self) -> bool {
        self.state
            .borrow()
            .last_meta_success
            .is_some_and(|t| t.elapsed() < config::METADATA_STALE_AFTER)
    }

    pub fn history_state(&self) -> LoadState {
        let s = self.state.borrow();
        if s.history_loaded {
            LoadState::Ready
        } else if s.history_offline {
            LoadState::Offline
        } else {
            LoadState::Loading
        }
    }
}
