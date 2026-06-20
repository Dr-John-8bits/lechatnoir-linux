//! Écran À propos — contenu VERBATIM (`05-EDITORIAL-ET-MARQUE.md`, variante app macOS) :
//! manifeste 5 lignes, chips, section « Participer & contact » (3 cartes mailto pré-remplies),
//! lien site, et « Mentions et confidentialité » (6 mentions).

use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::gtk::glib;

use lcn_core::config;

use super::{body, card, page_scaffold, section_header};

pub struct AboutScreen {
    root: gtk::ScrolledWindow,
}

impl AboutScreen {
    pub fn new() -> Self {
        let (root, sections) = page_scaffold("À propos");

        sections.append(&manifesto());
        sections.append(&chips());
        sections.append(&participer());
        sections.append(&site_link());
        sections.append(&mentions());

        Self { root }
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.root
    }
}

const MANIFESTO: [&str; 5] = [
    "Le Chat Noir est une webradio lilloise, artisanale, indépendante et autogérée, dédiée aux créations sonores et musicales.",
    "Elle diffuse en continu des créations libres : paysages sonores, field recordings, expérimentations radiophoniques, émissions et musiques de tous horizons, sans cloisonnement rigide.",
    "La radio assume une écoute lente entre fiction et réel, et respecte les dynamiques des œuvres sans compression globale imposée à l'antenne.",
    "Le catalogue est le fruit d'une curation humaine, patiente et sensible : ici, pas d'algorithme de recommandation, pas d'IA, seulement des choix d'écoute, des essais, des intuitions et du temps passé à chercher.",
    "Tout est fait maison, hébergé, programmé et maintenu localement. Une radio de proximité cosmique, née dans un coin de la tête, tournée vers l'espace.",
];

const CHIPS: [&str; 6] = [
    "Autogérée",
    "Créations sonores",
    "Écoute lente",
    "Scène locale",
    "Diffusion continue",
    "Auto-hébergée",
];

const MENTIONS: [(&str, &str); 6] = [
    ("Éditeur.", "Le Chat Noir Radio est édité et maintenu par un particulier, hébergé à titre non commercial sur un serveur personnel."),
    ("Contenu et diffusion.", "Tous les morceaux diffusés sont des œuvres libres de droits ou créées par leurs auteur·ices respectif·ves, dans le respect de leurs choix de diffusion. Si tu constates une erreur ou une diffusion non souhaitée, signale-la par le bouton de contact."),
    ("Données personnelles.", "On ne trace pas les auditeur·ices. Les seules données collectées sont celles que tu fournis volontairement pour nous écrire ; elles servent uniquement à répondre à ta demande et ne sont ni stockées ni partagées. Les statistiques d'écoute sont agrégées et anonymes."),
    ("Open source.", "L'intégralité de la webradio repose sur des outils open source comme Ubuntu, Icecast et Liquidsoap, et nous encourageons chaleureusement le soutien à cette communauté qui rend cette aventure possible."),
    ("Crédit photo.", "La photo utilisée pour le logo du Chat Noir a été prise par Yirmi June."),
    ("Responsabilité.", "L'éditeur ne saurait être tenu responsable d'une interruption temporaire du flux, ni de tout dommage indirect lié à l'usage de l'application ou à la diffusion en ligne."),
];

fn manifesto() -> gtk::Box {
    let block = gtk::Box::new(gtk::Orientation::Vertical, 10);

    let logo = crate::design::brand::logo_image(112);
    logo.set_halign(gtk::Align::Start);
    logo.update_property(&[gtk::accessible::Property::Label("Logo Le Chat Noir")]);
    block.append(&logo);

    let h1 = gtk::Label::new(Some("Un laboratoire radiophonique indépendant"));
    h1.add_css_class("lcn-page-title");
    h1.set_wrap(true);
    h1.set_xalign(0.0);
    h1.set_halign(gtk::Align::Start);
    block.append(&h1);

    for (i, line) in MANIFESTO.iter().enumerate() {
        let label = body(line);
        // 1re ligne en avant (ink), les suivantes en secondaire (inkSoft).
        if i > 0 {
            label.add_css_class("lcn-secondary");
        }
        block.append(&label);
    }
    block
}

fn chips() -> gtk::FlowBox {
    let flow = gtk::FlowBox::new();
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_max_children_per_line(6);
    flow.set_column_spacing(8);
    flow.set_row_spacing(8);
    flow.set_hexpand(true);
    for text in CHIPS {
        let chip = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        chip.add_css_class("lcn-chip");
        chip.append(&gtk::Label::new(Some(text)));
        flow.insert(&chip, -1);
    }
    flow
}

fn participer() -> gtk::Box {
    let block = gtk::Box::new(gtk::Orientation::Vertical, 12);
    block.append(&section_header(
        "Participer & contact",
        Some("Proposer une émission ou un son, demander un retrait, ou nous écrire : chaque mode ouvre un message déjà préparé."),
    ));

    block.append(&contact_card(
        "Contribution",
        "Proposer un son ou une émission",
        "Un morceau, une émission, un module ou un podcast diffusable. On cherche des propositions claires, diffusables, et accompagnées des bonnes infos.",
        &["Titre ou nom du projet", "Lien d'écoute ou fichier", "Infos de droits et nom ou pseudo"],
        "Le Chat Noir - Proposition sonore",
        "Bonjour,\n\nJe propose ce contenu pour diffusion :\n\n- Titre / Projet :\n- Lien d'écoute / téléchargement :\n- Droits / autorisation :\n- Nom / pseudo :\n\nMerci.",
    ));
    block.append(&contact_card(
        "Priorité",
        "Demander un retrait",
        "Les demandes de retrait sont traitées en priorité, sans friction inutile. Il suffit de nous donner les éléments permettant d'identifier le contenu concerné.",
        &["Titre ou contenu concerné", "Motif de la demande", "Lien ou plage horaire si possible"],
        "Le Chat Noir - Demande de retrait",
        "Bonjour,\n\nJe demande le retrait du contenu suivant :\n\n- Titre / Artiste :\n- Motif :\n- Lien / horaire de diffusion :\n- Contact :\n\nMerci.",
    ));
    block.append(&contact_card(
        "Contact",
        "Nous écrire",
        "Pour une question, une correction, un signalement, une idée d'émission ou toute autre prise de contact liée à la radio.",
        &["Objet du message", "Contexte en quelques lignes", "Retour attendu"],
        "Le Chat Noir - Contact",
        "Bonjour,\n\nObjet :\n\nMessage :\n\nNom / pseudo :\n\nMerci.",
    ));
    block
}

fn contact_card(
    kicker: &str,
    title: &str,
    text: &str,
    bullets: &[&str],
    subject: &str,
    mail_body: &str,
) -> gtk::Box {
    let c = card();

    let k = gtk::Label::new(Some(kicker));
    k.add_css_class("lcn-kicker");
    k.set_halign(gtk::Align::Start);
    c.append(&k);

    let t = gtk::Label::new(Some(title));
    t.add_css_class("lcn-section-title");
    t.set_wrap(true);
    t.set_xalign(0.0);
    t.set_halign(gtk::Align::Start);
    c.append(&t);

    c.append(&body(text));

    for bullet in bullets {
        let b = body(&format!("• {bullet}"));
        b.add_css_class("lcn-secondary");
        c.append(&b);
    }

    let link = gtk::LinkButton::with_label(&mailto(subject, mail_body), "Nous écrire");
    link.set_halign(gtk::Align::Start);
    c.append(&link);
    c
}

fn site_link() -> gtk::Box {
    let block = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let link = gtk::LinkButton::with_label(config::WEBSITE_URL, "Ouvrir le site de la radio");
    link.set_halign(gtk::Align::Start);
    block.append(&link);
    block
}

fn mentions() -> gtk::Box {
    let block = gtk::Box::new(gtk::Orientation::Vertical, 10);
    block.append(&section_header("Mentions et confidentialité", None));
    for (term, text) in MENTIONS {
        let label = gtk::Label::new(None);
        label.set_markup(&format!(
            "<b>{}</b> {}",
            glib::markup_escape_text(term),
            glib::markup_escape_text(text)
        ));
        label.set_wrap(true);
        label.set_xalign(0.0);
        label.set_halign(gtk::Align::Start);
        label.add_css_class("lcn-secondary");
        block.append(&label);
    }
    block
}

/// Construit un `mailto:` pré-rempli (sujet + corps percent-encodés via GLib).
fn mailto(subject: &str, mail_body: &str) -> String {
    let enc = |s: &str| glib::Uri::escape_string(s, None, false).to_string();
    format!(
        "mailto:{}?subject={}&body={}",
        config::CONTACT_EMAIL,
        enc(subject),
        enc(mail_body)
    )
}
