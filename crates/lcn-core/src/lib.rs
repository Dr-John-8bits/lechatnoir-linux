//! `lcn-core` — cœur logique **pur** de l'app Le Chat Noir (Linux).
//!
//! Ce crate ne dépend ni de GTK, ni de GStreamer, ni du réseau : il contient les
//! endpoints ([`config`]), les modèles de données, et la logique testable (parsing
//! JSON/CSV, détection DIRECT, `stripEndcap`, `aliasMap`, créneau courant, machine
//! d'état d'antenne, `reconnect_step`). Il compile et se teste sur n'importe quelle
//! plateforme — y compris la machine de dev macOS — ce qui garantit un « build vert »
//! continu indépendamment de la pile GTK Linux.
//!
//! Le binaire UI vit dans le crate `lcn-app` (GTK4/libadwaita/Relm4 + GStreamer + MPRIS).

pub mod antenne;
pub mod clock;
pub mod config;
pub mod content;
pub mod player;
pub mod services;
