//! MPRIS2 — touches média + centre de contrôle (GNOME/Cinnamon/KDE). Implémenté avec
//! `mpris-server` (LocalServer, mono-thread) lancé sur la boucle GLib : les handlers
//! tournent sur le thread principal et pilotent directement le `PlayerController` (Rc),
//! sans marshaling. Hors Linux (macOS), un stub no-op (pas de bus D-Bus).
//!
//! Flux live → Next/Previous/Seek désactivés (parité macOS `MPRemoteCommandCenter`).

use crate::player::controller::PlayerController;
use crate::services::data_store::DataStore;

#[cfg(target_os = "linux")]
pub use linux::{start, MprisHandle};

#[cfg(not(target_os = "linux"))]
pub use stub::{start, MprisHandle};

/// Stub no-op (plateformes sans bus D-Bus de session, ex. macOS de dev).
#[cfg(not(target_os = "linux"))]
mod stub {
    use super::{DataStore, PlayerController};

    #[derive(Clone)]
    pub struct MprisHandle;

    impl MprisHandle {
        pub fn notify(&self) {}
    }

    pub fn start(_player: PlayerController, _data: DataStore) -> MprisHandle {
        MprisHandle
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::cell::RefCell;
    use std::rc::Rc;

    use mpris_server::zbus::{fdo, Result};
    use mpris_server::{
        LocalPlayerInterface, LocalRootInterface, LocalServer, LoopStatus, Metadata, PlaybackRate,
        PlaybackStatus, Property, Time, TrackId, Volume,
    };
    use relm4::gtk::glib;
    use relm4::gtk::prelude::*;

    use lcn_core::config;
    use lcn_core::player::StreamHealth;

    use super::{DataStore, PlayerController};

    type ServerCell = Rc<RefCell<Option<Rc<LocalServer<LcnMpris>>>>>;

    /// Implémentation des interfaces MPRIS, partagée (Rc) avec l'UI.
    struct LcnMpris {
        player: PlayerController,
        data: DataStore,
    }

    /// Poignée clonable : permet d'émettre `PropertiesChanged` quand l'état change.
    #[derive(Clone)]
    pub struct MprisHandle {
        server: ServerCell,
        player: PlayerController,
        data: DataStore,
    }

    impl MprisHandle {
        /// Émet `PropertiesChanged` (statut, métadonnées, volume) si le serveur est prêt.
        pub fn notify(&self) {
            let server = self.server.clone();
            let status = playback_status(&self.player);
            let volume = self.player.volume();
            let metadata = build_metadata(&self.data);
            glib::spawn_future_local(async move {
                let server = server.borrow().clone();
                if let Some(server) = server {
                    let _ = server
                        .properties_changed([
                            Property::PlaybackStatus(status),
                            Property::Metadata(metadata),
                            Property::Volume(volume),
                        ])
                        .await;
                }
            });
        }
    }

    /// Démarre le serveur MPRIS (création asynchrone sur la boucle GLib).
    pub fn start(player: PlayerController, data: DataStore) -> MprisHandle {
        let server: ServerCell = Rc::new(RefCell::new(None));
        let imp = LcnMpris { player: player.clone(), data: data.clone() };

        let server_for_task = server.clone();
        glib::spawn_future_local(async move {
            match LocalServer::new(config::APP_ID, imp).await {
                Ok(srv) => {
                    let srv = Rc::new(srv);
                    glib::spawn_future_local(srv.run());
                    *server_for_task.borrow_mut() = Some(srv);
                    tracing::info!("MPRIS prêt (org.mpris.MediaPlayer2.{})", config::APP_ID);
                }
                Err(err) => tracing::warn!("MPRIS indisponible: {err}"),
            }
        });

        MprisHandle { server, player, data }
    }

    fn playback_status(player: &PlayerController) -> PlaybackStatus {
        if player.is_playing() {
            PlaybackStatus::Playing
        } else if player.stream_health() == StreamHealth::Failed {
            PlaybackStatus::Stopped
        } else {
            PlaybackStatus::Paused
        }
    }

    fn build_metadata(data: &DataStore) -> Metadata {
        let trackid = TrackId::try_from(config::MPRIS_TRACK_ID).unwrap_or(TrackId::NO_TRACK);
        let mut builder = Metadata::builder().trackid(trackid);
        if let Some(uri) = crate::design::brand::logo_file_uri() {
            builder = builder.art_url(uri);
        }
        if let Some(np) = data.now_playing() {
            builder = builder.title(np.display_title().to_string());
            if !np.artist.is_empty() {
                builder = builder.artist([np.artist.clone()]);
            }
            if !np.album.is_empty() {
                builder = builder.album(np.album.clone());
            }
        }
        builder.build()
    }

    impl LocalRootInterface for LcnMpris {
        async fn raise(&self) -> fdo::Result<()> {
            if let Some(window) = relm4::main_application().active_window() {
                window.present();
            }
            Ok(())
        }

        async fn quit(&self) -> fdo::Result<()> {
            relm4::main_application().quit();
            Ok(())
        }

        async fn can_quit(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn fullscreen(&self) -> fdo::Result<bool> {
            Ok(false)
        }

        async fn set_fullscreen(&self, _fullscreen: bool) -> Result<()> {
            Ok(())
        }

        async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
            Ok(false)
        }

        async fn can_raise(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn has_track_list(&self) -> fdo::Result<bool> {
            Ok(false)
        }

        async fn identity(&self) -> fdo::Result<String> {
            Ok("Le Chat Noir".to_string())
        }

        async fn desktop_entry(&self) -> fdo::Result<String> {
            Ok(config::APP_ID.to_string())
        }

        async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
            Ok(vec![])
        }

        async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
            Ok(vec![])
        }
    }

    impl LocalPlayerInterface for LcnMpris {
        // Flux live : pas de navigation par piste ni de scrubbing.
        async fn next(&self) -> fdo::Result<()> {
            Ok(())
        }

        async fn previous(&self) -> fdo::Result<()> {
            Ok(())
        }

        async fn pause(&self) -> fdo::Result<()> {
            self.player.pause();
            Ok(())
        }

        async fn play_pause(&self) -> fdo::Result<()> {
            self.player.toggle();
            Ok(())
        }

        async fn stop(&self) -> fdo::Result<()> {
            self.player.pause();
            Ok(())
        }

        async fn play(&self) -> fdo::Result<()> {
            self.player.play();
            Ok(())
        }

        async fn seek(&self, _offset: Time) -> fdo::Result<()> {
            Ok(())
        }

        async fn set_position(&self, _track_id: TrackId, _position: Time) -> fdo::Result<()> {
            Ok(())
        }

        async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
            Ok(())
        }

        async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
            Ok(playback_status(&self.player))
        }

        async fn loop_status(&self) -> fdo::Result<LoopStatus> {
            Ok(LoopStatus::None)
        }

        async fn set_loop_status(&self, _loop_status: LoopStatus) -> Result<()> {
            Ok(())
        }

        async fn rate(&self) -> fdo::Result<PlaybackRate> {
            Ok(1.0)
        }

        async fn set_rate(&self, _rate: PlaybackRate) -> Result<()> {
            Ok(())
        }

        async fn shuffle(&self) -> fdo::Result<bool> {
            Ok(false)
        }

        async fn set_shuffle(&self, _shuffle: bool) -> Result<()> {
            Ok(())
        }

        async fn metadata(&self) -> fdo::Result<Metadata> {
            Ok(build_metadata(&self.data))
        }

        async fn volume(&self) -> fdo::Result<Volume> {
            Ok(self.player.volume())
        }

        async fn set_volume(&self, volume: Volume) -> Result<()> {
            self.player.set_volume(volume);
            Ok(())
        }

        async fn position(&self) -> fdo::Result<Time> {
            Ok(Time::from_micros(0))
        }

        async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
            Ok(1.0)
        }

        async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
            Ok(1.0)
        }

        async fn can_go_next(&self) -> fdo::Result<bool> {
            Ok(false)
        }

        async fn can_go_previous(&self) -> fdo::Result<bool> {
            Ok(false)
        }

        async fn can_play(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn can_pause(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn can_seek(&self) -> fdo::Result<bool> {
            Ok(false)
        }

        async fn can_control(&self) -> fdo::Result<bool> {
            Ok(true)
        }
    }
}
