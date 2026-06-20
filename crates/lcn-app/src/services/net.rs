//! Polling réseau : threads `reqwest::blocking` (fetch + parse hors thread UI) qui
//! poussent des [`Update`] via un canal async ; un récepteur sur le thread principal
//! les applique au [`DataStore`]. Évite tout mélange d'exécuteurs tokio/GLib.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use relm4::gtk::glib;
use reqwest::blocking::Client;
use reqwest::header::RANGE;

use lcn_core::config;
use lcn_core::services::history::{self, HistoryEntry};
use lcn_core::services::{current_show, news, now_playing, schedule, voices};

use super::data_store::{DataStore, Update};

/// Démarre les boucles de polling et le récepteur UI. À appeler une fois au démarrage.
pub fn start_polling(store: DataStore) {
    let (tx, rx) = async_channel::unbounded::<Update>();

    let client = Arc::new(
        Client::builder()
            .timeout(config::HTTP_REQUEST_TIMEOUT)
            .user_agent(config::USER_AGENT)
            .build()
            .expect("construction du client HTTP"),
    );

    spawn_loop(client.clone(), tx.clone(), config::POLL_NOWPLAYING, |client, tx| {
        match fetch_text(client, config::NOWPLAYING_URL).and_then(|t| now_playing::parse(&t)) {
            Some(np) => send(tx, Update::NowPlaying(np)),
            None => send(tx, Update::MetaFailed),
        }
    });

    spawn_loop(client.clone(), tx.clone(), config::POLL_CURRENT_SHOW, |client, tx| {
        match fetch_text(client, config::CURRENT_SHOW_URL).and_then(|t| current_show::parse(&t)) {
            Some(cs) => send(tx, Update::CurrentShow(cs)),
            None => send(tx, Update::MetaFailed),
        }
    });

    spawn_loop(client.clone(), tx.clone(), config::POLL_HISTORY, |client, tx| {
        match fetch_history(client) {
            Some(entries) if !entries.is_empty() => send(tx, Update::History(entries)),
            Some(_) => {} // vide : ne pas écraser le repli
            None => send(tx, Update::HistoryFailed),
        }
    });

    // Contenu éditorial (600 s) : ne pousser qu'en cas de succès non vide (sinon snapshot/repli).
    spawn_loop(client.clone(), tx.clone(), config::POLL_NEWS, |client, tx| {
        if let Some(text) = fetch_text(client, &config::news_url(config::CONTENT_BASE_URL)) {
            let items = news::parse(&text);
            if !items.is_empty() {
                send(tx, Update::News(items));
            }
        }
    });
    spawn_loop(client.clone(), tx.clone(), config::POLL_SCHEDULE, |client, tx| {
        if let Some(text) = fetch_text(client, &config::schedule_url(config::CONTENT_BASE_URL)) {
            let schedule = schedule::parse(&text);
            if !schedule.days.is_empty() {
                send(tx, Update::Schedule(schedule));
            }
        }
    });
    spawn_loop(client.clone(), tx.clone(), config::POLL_VOICES, |client, tx| {
        if let Some(text) = fetch_text(client, &config::voices_url(config::CONTENT_BASE_URL)) {
            let voices = voices::parse(&text, config::CONTENT_BASE_URL);
            if !voices.producers.is_empty() || !voices.shows.is_empty() {
                send(tx, Update::Voices(voices));
            }
        }
    });

    // Récepteur sur le thread principal : applique chaque mise à jour au store.
    glib::spawn_future_local(async move {
        while let Ok(update) = rx.recv().await {
            store.apply(update);
        }
    });
}

fn send(tx: &async_channel::Sender<Update>, update: Update) {
    let _ = tx.send_blocking(update);
}

/// Lance un thread qui exécute `task` immédiatement puis toutes les `interval`.
fn spawn_loop<F>(client: Arc<Client>, tx: async_channel::Sender<Update>, interval: Duration, task: F)
where
    F: Fn(&Client, &async_channel::Sender<Update>) + Send + 'static,
{
    thread::spawn(move || loop {
        task(&client, &tx);
        thread::sleep(interval);
    });
}

fn fetch_text(client: &Client, url: &str) -> Option<String> {
    let response = client.get(url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.text().ok()
}

/// Historique : Range sur la fin (~512 Ko) en priorité, repli téléchargement complet.
fn fetch_history(client: &Client) -> Option<Vec<HistoryEntry>> {
    if let Some(entries) = fetch_history_range(client) {
        return Some(entries);
    }
    let text = fetch_text(client, config::HISTORY_CSV_URL)?;
    Some(history::parse_recent(&text))
}

fn fetch_history_range(client: &Client) -> Option<Vec<HistoryEntry>> {
    let response = client
        .get(config::HISTORY_CSV_URL)
        .header(RANGE, format!("bytes=-{}", config::HISTORY_RANGE_TAIL_BYTES))
        .send()
        .ok()?;
    let status = response.status();
    if !(status.is_success() || status.as_u16() == 206) {
        return None;
    }
    let bytes = response.bytes().ok()?;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    // Sur 206 (Partial Content), la 1re ligne est partielle → l'ignorer.
    if status.as_u16() == 206 {
        if let Some(idx) = text.find('\n') {
            text = text[idx + 1..].to_string();
        }
    }
    Some(history::parse_recent(&text))
}
