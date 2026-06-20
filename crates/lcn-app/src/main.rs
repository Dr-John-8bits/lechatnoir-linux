//! Le Chat Noir — point d'entrée de l'app Linux.
//!
//! Crée l'`adw::Application` (via Relm4, feature libadwaita) et lance la coquille
//! racine ([`ui::root::RootModel`]) : sidebar 6 rubriques + zone de contenu +
//! barre de lecture persistante, thème clair/sombre suivant le système.

mod design;
mod player;
mod services;
mod ui;

use relm4::RelmApp;

use ui::root::RootModel;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            // Le crate binaire s'appelle `lechatnoir_player` (nom du [[bin]]), pas `lcn_app`.
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lechatnoir_player=info,lcn_core=info".into()),
        )
        .init();

    // Doit précéder toute création d'élément GStreamer (le contrôleur audio).
    gstreamer::init().expect("échec de l'initialisation de GStreamer");

    let app = RelmApp::new(lcn_core::config::APP_ID);
    app.run::<RootModel>(());
}
