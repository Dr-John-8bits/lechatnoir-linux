//! Services de parsing des flux (réseau→structs). Logique pure et testable :
//! ils reçoivent des octets/strings et renvoient des modèles, sans toucher au réseau.

pub mod current_show;
pub mod history;
pub mod news;
pub mod now_playing;
pub mod schedule;
pub mod snapshot;
pub mod voices;
