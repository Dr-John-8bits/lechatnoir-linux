//! Composant racine Relm4 (MVU) : la coquille de l'app.
//!
//! Disposition : `Box` vertical = [ header bar | corps (sidebar | stack) | barre de
//! lecture ]. La barre de lecture est ancrée en bas et persiste sur tous les écrans.
//!
//! La `Chrome` recompose l'**état d'antenne** (§2.1) à partir du lecteur (santé, lecture)
//! ET des données (direct, fraîcheur), puis met à jour la sidebar et la barre. Elle est
//! rafraîchie sur changement du lecteur, sur mise à jour des données, et périodiquement
//! (la fraîcheur se périme avec le temps).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use relm4::gtk::glib;
use relm4::{adw, gtk, ComponentParts, ComponentSender, SimpleComponent};
use relm4::adw::prelude::*;

use lcn_core::antenne::{self, AntenneState, Tint};
use lcn_core::config;
use lcn_core::player::StreamHealth;

use crate::services::settings;

use super::screens::about::AboutScreen;
use super::screens::history::HistoryScreen;
use super::screens::home::HomeScreen;
use super::screens::news::NewsScreen;
use super::screens::schedule::ScheduleScreen;
use super::screens::voices::VoicesScreen;
use super::section::Section;
use super::sidebar::StatusHandles;
use super::{player_bar, sidebar};
use super::player_bar::PlayerBarHandles;
use crate::design::theme::{self, ThemePreference};
use crate::player::controller::PlayerController;
use crate::player::mpris::{self, MprisHandle};
use crate::services::data_store::{DataStore, UpdateKind};
use crate::services::net;

/// Conteneur des 6 écrans, pour les rafraîchir de façon ciblée.
struct Screens {
    home: HomeScreen,
    news: NewsScreen,
    history: HistoryScreen,
    schedule: ScheduleScreen,
    voices: VoicesScreen,
    about: AboutScreen,
}

/// État de la coquille : la rubrique active. Le lecteur et le store y sont conservés
/// pour rester vivants pendant toute la durée de l'app.
pub struct RootModel {
    active: Section,
    #[allow(dead_code)]
    player: PlayerController,
    #[allow(dead_code)]
    data: DataStore,
    #[allow(dead_code)]
    screens: Rc<Screens>,
    #[allow(dead_code)]
    mpris: MprisHandle,
}

#[derive(Debug)]
pub enum RootInput {
    Navigate(Section),
}

pub struct RootWidgets {
    stack: gtk::Stack,
}

/// Widgets de chrome (sidebar + barre) mis à jour selon l'état d'antenne courant.
struct Chrome {
    status: StatusHandles,
    bar: PlayerBarHandles,
    /// Dernier titre notifié (évite de re-notifier à chaque tick / au 1er chargement).
    last_notified: RefCell<Option<String>>,
}

impl Chrome {
    fn refresh(&self, player: &PlayerController, data: &DataStore) {
        let state = antenne::resolve(
            player.stream_health(),
            data.is_live(),
            player.is_playing(),
            data.metadata_fresh(),
        );

        // Sidebar : kicker (braise en direct) + pastille de signal.
        let kicker_classes: &[&str] = if state == AntenneState::Direct {
            &["lcn-kicker", "live"]
        } else {
            &["lcn-kicker"]
        };
        self.status.kicker.set_css_classes(kicker_classes);
        self.status.kicker.set_text(state.kicker());
        apply_pill(&self.status.pill, state);
        self.status.pill_label.set_text(state.label());
        self.status
            .pill
            .update_property(&[gtk::accessible::Property::Label(state.accessibility_label())]);

        // Texte conditionnel sous la pastille.
        let conditional = if data.is_live() {
            match data.current_show().and_then(|c| c.elapsed_text(now_unix())) {
                Some(elapsed) => format!("Direct en cours {elapsed}"),
                None => "Direct en cours".to_string(),
            }
        } else if player.stream_health() == StreamHealth::Failed && player.is_playing() {
            "Problème de lecture du flux audio.".to_string()
        } else {
            String::new()
        };
        self.status.conditional.set_visible(!conditional.is_empty());
        self.status.conditional.set_text(&conditional);

        // Barre : bloc titre (now-playing).
        match data.now_playing() {
            Some(np) => {
                let title = np.display_title();
                self.bar.title.set_text(title);
                let line = np.metadata_line();
                self.bar.subtitle.set_text(if line.is_empty() {
                    "programmation en cours"
                } else {
                    &line
                });
                self.notify_on_track_change(title, &line, player.is_playing());
            }
            None => {
                self.bar.title.set_text("Chargement des métadonnées…");
                self.bar.subtitle.set_text("programmation en cours");
            }
        }

        // Barre : indicateur d'état — texte = playbackStateText pendant « connexion », sinon label.
        let indicator_text = if state == AntenneState::Connexion {
            player.playback_text()
        } else {
            state.label()
        };
        self.bar.state_label.set_text(indicator_text);
        apply_tint(&self.bar.state_indicator, state.tint());
        self.bar
            .state_indicator
            .update_property(&[gtk::accessible::Property::Label(state.accessibility_label())]);

        // Pulsation du point uniquement en « à l'antenne » (§4.6).
        let pulse = state == AntenneState::ALAntenne;
        for dot in [&self.status.pill_dot, &self.bar.state_dot] {
            if pulse {
                dot.add_css_class("lcn-pulse");
            } else {
                dot.remove_css_class("lcn-pulse");
            }
        }

        // Bouton lecture (icône + libellé selon l'intention).
        let playing = player.is_playing();
        self.bar.play_button.set_icon_name(if playing {
            player_bar::PAUSE_ICON
        } else {
            player_bar::PLAY_ICON
        });
        let play_label = if playing {
            "Mettre le direct en pause"
        } else {
            "Lancer le direct"
        };
        self.bar.play_button.set_tooltip_text(Some(play_label));
        self.bar
            .play_button
            .update_property(&[gtk::accessible::Property::Label(play_label)]);

        tracing::debug!(
            "chrome: antenne={:?} live={} titre={:?}",
            state,
            data.is_live(),
            data.now_playing().map(|n| n.title)
        );
    }

    /// Notification bureau au CHANGEMENT de titre (jamais au 1er chargement, uniquement en
    /// lecture). Le dernier titre est mémorisé pour ne pas re-notifier à chaque tick.
    fn notify_on_track_change(&self, title: &str, line: &str, playing: bool) {
        let mut last = self.last_notified.borrow_mut();
        if last.as_deref() == Some(title) {
            return;
        }
        let had_previous = last.is_some();
        *last = Some(title.to_string());
        if playing && had_previous && !title.is_empty() {
            let notif = relm4::gtk::gio::Notification::new("À l'antenne");
            let body = if line.is_empty() {
                title.to_string()
            } else {
                format!("{title} — {line}")
            };
            notif.set_body(Some(&body));
            if let Some(app) = relm4::gtk::gio::Application::default() {
                app.send_notification(Some("lcn-now-playing"), &notif);
            }
        }
    }
}

impl SimpleComponent for RootModel {
    type Init = ();
    type Input = RootInput;
    type Output = ();
    type Root = adw::ApplicationWindow;
    type Widgets = RootWidgets;

    fn init_root() -> Self::Root {
        adw::ApplicationWindow::builder()
            .title("Le Chat Noir")
            .default_width(settings::get_i32(settings::WINDOW_WIDTH, 1320))
            .default_height(settings::get_i32(settings::WINDOW_HEIGHT, 860))
            .width_request(1024)
            .height_request(720)
            .build()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let style_manager = adw::StyleManager::default();
        theme::install(&style_manager);
        // Hook captures/tests : forcer un thème (clair/sombre/auto) sans toucher au système.
        if let Ok(forced) = std::env::var("LCN_FORCE_THEME") {
            let pref = match forced.as_str() {
                "light" => Some(ThemePreference::Light),
                "dark" => Some(ThemePreference::Dark),
                "auto" => Some(ThemePreference::Auto),
                _ => None,
            };
            if let Some(pref) = pref {
                theme::set_preference(&style_manager, pref);
            }
        }
        // Hook de test : simule un clic « Sombre » à chaud (vérifie en headless que la
        // bascule recharge bien le CSS au runtime — cf. theme::connect_color_scheme_notify).
        if std::env::var("LCN_TEST_TOGGLE_DARK").is_ok() {
            let sm = style_manager.clone();
            relm4::gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(2500), move || {
                theme::set_preference(&sm, ThemePreference::Dark);
            });
        }
        install_quit_accelerator();

        // Géométrie de fenêtre : restaure l'état maximisé, et sauvegarde taille + maximisé
        // à la fermeture (la taille par défaut est déjà restaurée dans init_root).
        if settings::get_bool(settings::WINDOW_MAXIMIZED, false) {
            root.maximize();
        }
        root.connect_close_request(move |w| {
            settings::set_bool(settings::WINDOW_MAXIMIZED, w.is_maximized());
            if !w.is_maximized() {
                settings::set_i32(settings::WINDOW_WIDTH, w.default_width());
                settings::set_i32(settings::WINDOW_HEIGHT, w.default_height());
            }
            relm4::gtk::glib::Propagation::Proceed
        });

        let player = PlayerController::new();
        let data = DataStore::new();

        // Raccourci clavier : Espace = lecture/pause. Contrôleur posé sur la fenêtre (phase
        // « bubble ») : il ne se déclenche que si aucun widget focalisé (bouton, champ texte,
        // liste déroulante…) n'a déjà consommé la touche — pas de double-action ni de conflit
        // avec la saisie.
        let key_controller = gtk::EventControllerKey::new();
        {
            let player = player.clone();
            key_controller.connect_key_pressed(move |_, keyval, _, _| {
                if keyval == gtk::gdk::Key::space {
                    player.toggle();
                    gtk::glib::Propagation::Stop
                } else {
                    gtk::glib::Propagation::Proceed
                }
            });
        }
        root.add_controller(key_controller);

        // Section initiale (défaut Accueil ; surchageable via LCN_START_SECTION pour les captures).
        let initial = std::env::var("LCN_START_SECTION")
            .ok()
            .and_then(|id| Section::from_id(&id))
            .unwrap_or(Section::Home);

        // Les 6 écrans (initialisés depuis le snapshot via le data_store).
        let screens = Rc::new(Screens {
            home: HomeScreen::new(data.clone()),
            news: NewsScreen::new(data.clone()),
            history: HistoryScreen::new(data.clone()),
            schedule: ScheduleScreen::new(data.clone()),
            voices: VoicesScreen::new(data.clone()),
            about: AboutScreen::new(),
        });

        // Zone de contenu : un stack avec une page par rubrique.
        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.add_named(screens.home.widget(), Some(Section::Home.id()));
        stack.add_named(screens.news.widget(), Some(Section::News.id()));
        stack.add_named(screens.history.widget(), Some(Section::History.id()));
        stack.add_named(screens.schedule.widget(), Some(Section::Schedule.id()));
        stack.add_named(screens.voices.widget(), Some(Section::Voices.id()));
        stack.add_named(screens.about.widget(), Some(Section::About.id()));
        stack.set_visible_child_name(initial.id());

        // Sidebar + barre de lecture (widgets exposés à la chrome).
        let sidebar_parts = sidebar::build(sender.clone());
        if let Some(row) = sidebar_parts.nav_list.row_at_index(initial.index()) {
            sidebar_parts.nav_list.select_row(Some(&row));
        }
        // Action « app.show-news » : l'accroche actu de l'accueil sélectionne la rubrique
        // Actualités via la sidebar (la navigation reste ainsi synchronisée avec la liste).
        {
            let nav = sidebar_parts.nav_list.clone();
            let action = relm4::gtk::gio::SimpleAction::new("show-news", None);
            action.connect_activate(move |_, _| {
                if let Some(row) = nav.row_at_index(Section::News.index()) {
                    nav.select_row(Some(&row));
                }
            });
            relm4::main_application().add_action(&action);
        }
        let (player_bar_widget, bar_handles) = player_bar::build(&player);

        let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        body.set_vexpand(true);
        body.append(&sidebar_parts.container);
        body.append(&stack);

        let header = adw::HeaderBar::new();
        let website = gtk::LinkButton::with_label(config::WEBSITE_URL, "Site web");
        header.pack_end(&website);
        header.pack_start(&sleep_timer_button(player.clone()));

        let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        outer.append(&header);
        outer.append(&body);
        outer.append(&player_bar_widget);

        root.set_content(Some(&outer));

        // Serveur MPRIS2 (touches média + centre de contrôle). No-op hors Linux.
        let mpris = mpris::start(player.clone(), data.clone());

        // Chrome + rafraîchissement unifié (lecteur, données, MPRIS, timer de fraîcheur).
        let chrome = Rc::new(Chrome {
            status: sidebar_parts.status,
            bar: bar_handles,
            last_notified: RefCell::new(None),
        });
        let refresh: Rc<dyn Fn()> = {
            let chrome = chrome.clone();
            let player = player.clone();
            let data = data.clone();
            let mpris = mpris.clone();
            Rc::new(move || {
                chrome.refresh(&player, &data);
                mpris.notify();
            })
        };
        player.set_on_change({
            let refresh = refresh.clone();
            move || refresh()
        });
        data.set_on_update({
            let refresh = refresh.clone();
            let screens = screens.clone();
            move |kind| {
                refresh(); // la chrome (état d'antenne, titre) se met à jour à chaque tick
                match kind {
                    UpdateKind::Meta => {}
                    UpdateKind::History => {
                        screens.home.refresh();
                        screens.history.refresh();
                    }
                    UpdateKind::News => {
                        screens.home.refresh();
                        screens.news.refresh();
                    }
                    UpdateKind::Schedule => {
                        screens.home.refresh();
                        screens.schedule.refresh();
                    }
                    UpdateKind::Voices => screens.voices.refresh(),
                }
            }
        });
        // Chrome (fraîcheur de l'état d'antenne) toutes les 5 s.
        glib::timeout_add_local(Duration::from_secs(5), {
            let refresh = refresh.clone();
            move || {
                refresh();
                glib::ControlFlow::Continue
            }
        });
        // Écrans sensibles à l'heure (créneau courant) toutes les 60 s.
        glib::timeout_add_local(Duration::from_secs(60), {
            let screens = screens.clone();
            move || {
                screens.home.refresh();
                screens.schedule.refresh();
                glib::ControlFlow::Continue
            }
        });
        refresh();

        // Démarre les boucles de polling temps réel.
        net::start_polling(data.clone());

        if std::env::var_os("LCN_AUTOPLAY").is_some() {
            player.play();
        }

        tracing::info!("shell prêt ({})", config::APP_ID);
        ComponentParts {
            model: RootModel { active: initial, player, data, screens, mpris },
            widgets: RootWidgets { stack },
        }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            RootInput::Navigate(section) => self.active = section,
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        widgets.stack.set_visible_child_name(self.active.id());
    }
}

fn apply_pill(pill: &gtk::Box, state: AntenneState) {
    let classes: &[&str] = match state.tint() {
        Tint::Braise => &["lcn-signal-pill", "live"],
        Tint::Muted => &["lcn-signal-pill", "offline"],
        Tint::Azur => &["lcn-signal-pill"],
    };
    pill.set_css_classes(classes);
}

fn apply_tint(widget: &gtk::Box, tint: Tint) {
    let class = match tint {
        Tint::Azur => "lcn-tint-azur",
        Tint::Braise => "lcn-tint-braise",
        Tint::Muted => "lcn-tint-muted",
    };
    widget.set_css_classes(&[class]);
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `Ctrl+Q` quitte l'app (parité macOS `Cmd+Q` ; pas de bouton « Quitter »).
fn install_quit_accelerator() {
    use relm4::gtk::gio;
    let app = relm4::main_application();
    let quit = gio::SimpleAction::new("quit", None);
    let app_for_quit = app.clone();
    quit.connect_activate(move |_, _| app_for_quit.quit());
    app.add_action(&quit);
    app.set_accels_for_action("app.quit", &["<Primary>q"]);
}

/// Bouton « minuteur de veille » (header) : met le direct en pause après 15/30/60 min.
/// Un seul minuteur actif à la fois (toute nouvelle sélection annule le précédent).
fn sleep_timer_button(player: PlayerController) -> gtk::MenuButton {
    let handle: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    let btn = gtk::MenuButton::new();
    btn.set_icon_name("alarm-symbolic");
    btn.set_tooltip_text(Some("Minuteur de veille"));

    let popover = gtk::Popover::new();
    let list = gtk::Box::new(gtk::Orientation::Vertical, 2);
    list.set_margin_top(6);
    list.set_margin_bottom(6);
    list.set_margin_start(6);
    list.set_margin_end(6);

    for (label, mins) in [
        ("Désactivé", 0u64),
        ("Pause dans 15 min", 15),
        ("Pause dans 30 min", 30),
        ("Pause dans 60 min", 60),
    ] {
        let item = gtk::Button::with_label(label);
        item.add_css_class("flat");
        item.set_halign(gtk::Align::Fill);

        let handle = handle.clone();
        let player = player.clone();
        let popover_weak = popover.downgrade();
        let btn_weak = btn.downgrade();
        item.connect_clicked(move |_| {
            if let Some(id) = handle.borrow_mut().take() {
                id.remove();
            }
            if mins > 0 {
                let player = player.clone();
                let handle_for_fire = handle.clone();
                let btn_weak_for_fire = btn_weak.clone();
                let id = glib::timeout_add_local_once(Duration::from_secs(mins * 60), move || {
                    player.pause();
                    *handle_for_fire.borrow_mut() = None;
                    if let Some(b) = btn_weak_for_fire.upgrade() {
                        b.set_tooltip_text(Some("Minuteur de veille"));
                    }
                });
                *handle.borrow_mut() = Some(id);
                if let Some(b) = btn_weak.upgrade() {
                    b.set_tooltip_text(Some(&format!("Veille programmée ({label})")));
                }
            } else if let Some(b) = btn_weak.upgrade() {
                b.set_tooltip_text(Some("Minuteur de veille"));
            }
            if let Some(p) = popover_weak.upgrade() {
                p.popdown();
            }
        });
        list.append(&item);
    }

    popover.set_child(Some(&list));
    btn.set_popover(Some(&popover));
    btn
}
