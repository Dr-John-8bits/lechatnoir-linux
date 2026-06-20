//! Barre de lecture persistante, ancrée en bas. Le bouton play/pause et le slider
//! pilotent le contrôleur ; le bloc titre et l'indicateur d'état sont mis à jour par
//! la chrome (voir `ui::root::Chrome`) à partir de l'état d'antenne et du nowplaying.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk;
use relm4::gtk::glib;

use lcn_core::player::playback_text;

use crate::player::controller::PlayerController;

pub const PLAY_ICON: &str = "media-playback-start-symbolic";
pub const PAUSE_ICON: &str = "media-playback-pause-symbolic";

/// Widgets de la barre que la chrome doit pouvoir mettre à jour.
pub struct PlayerBarHandles {
    pub title: gtk::Label,
    pub subtitle: gtk::Label,
    pub state_indicator: gtk::Box,
    pub state_dot: gtk::Label,
    pub state_label: gtk::Label,
    pub play_button: gtk::Button,
}

pub fn build(player: &PlayerController) -> (gtk::Box, PlayerBarHandles) {
    let bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(14)
        .height_request(72)
        .build();
    bar.add_css_class("lcn-player-bar");

    // Bloc titre (now-playing). Le titre défile (marquee) s'il est trop long.
    let title = gtk::Label::new(Some("Chargement des métadonnées…"));
    title.add_css_class("lcn-track-title");
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);

    let subtitle = gtk::Label::new(Some("programmation en cours"));
    subtitle.add_css_class("lcn-data");
    subtitle.set_halign(gtk::Align::Start);
    subtitle.set_xalign(0.0);
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let title_block = gtk::Box::new(gtk::Orientation::Vertical, 2);
    title_block.set_hexpand(true);
    title_block.set_valign(gtk::Align::Center);
    title_block.append(&marquee(&title));
    title_block.append(&subtitle);

    // Bouton play/pause rond, fond azur.
    let play_button = gtk::Button::from_icon_name(PLAY_ICON);
    play_button.add_css_class("lcn-play-button");
    play_button.add_css_class("circular");
    play_button.set_valign(gtk::Align::Center);
    play_button.set_tooltip_text(Some("Lancer le direct"));
    play_button.update_property(&[gtk::accessible::Property::Label("Lancer le direct")]);
    {
        let player = player.clone();
        play_button.connect_clicked(move |_| player.toggle());
    }

    // Indicateur d'état : point teinté + texte (teinte gérée par la chrome).
    let state_indicator = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    state_indicator.add_css_class("lcn-tint-azur");
    state_indicator.set_valign(gtk::Align::Center);
    let dot = gtk::Label::new(Some("●"));
    let state_label = gtk::Label::new(Some(playback_text::READY));
    state_label.add_css_class("lcn-data");
    state_indicator.append(&dot);
    state_indicator.append(&state_label);

    // Volume.
    let volume_icon = gtk::Image::from_icon_name("audio-volume-high-symbolic");
    let volume_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.01);
    volume_scale.set_width_request(150);
    volume_scale.set_draw_value(false);
    volume_scale.set_value(player.volume());
    volume_scale.set_valign(gtk::Align::Center);
    volume_scale.add_css_class("lcn-accent");
    volume_scale.set_tooltip_text(Some("Volume"));
    volume_scale.update_property(&[gtk::accessible::Property::Label("Volume")]);
    {
        let player = player.clone();
        volume_scale.connect_value_changed(move |scale| player.set_volume(scale.value()));
    }

    let logo = crate::design::brand::logo_image(36);
    logo.set_valign(gtk::Align::Center);
    bar.append(&logo);
    bar.append(&title_block);
    bar.append(&play_button);
    bar.append(&state_indicator);
    bar.append(&volume_icon);
    bar.append(&volume_scale);

    let clamp = adw::Clamp::builder().maximum_size(1000).build();
    clamp.set_child(Some(&bar));

    let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
    wrapper.set_margin_top(12);
    wrapper.set_margin_bottom(16);
    wrapper.set_margin_start(16);
    wrapper.set_margin_end(16);
    wrapper.append(&clamp);

    (
        wrapper,
        PlayerBarHandles { title, subtitle, state_indicator, state_dot: dot, state_label, play_button },
    )
}

/// Enveloppe un label dans un défilement horizontal (marquee) qui s'active seulement si
/// le texte déborde, et reste figé si les animations système sont désactivées.
fn marquee(label: &gtk::Label) -> gtk::ScrolledWindow {
    label.set_wrap(false);
    label.set_ellipsize(gtk::pango::EllipsizeMode::None);

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_policy(gtk::PolicyType::External, gtk::PolicyType::Never);
    scroller.set_has_frame(false);
    scroller.set_hexpand(true);
    scroller.set_child(Some(label));

    let adjustment = scroller.hadjustment();
    let pause = Rc::new(Cell::new(0u32));
    glib::timeout_add_local(Duration::from_millis(33), move || {
        if animations_enabled() {
            let max = (adjustment.upper() - adjustment.page_size()).max(0.0);
            if max > 1.0 {
                if pause.get() > 0 {
                    pause.set(pause.get() - 1);
                } else {
                    let next = adjustment.value() + 0.6;
                    if next >= max {
                        adjustment.set_value(0.0);
                        pause.set(45); // ~1,5 s de pause au retour au début
                    } else {
                        adjustment.set_value(next);
                    }
                }
            }
        }
        glib::ControlFlow::Continue
    });
    scroller
}

fn animations_enabled() -> bool {
    gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations())
}
