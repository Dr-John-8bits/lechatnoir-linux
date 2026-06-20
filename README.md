# Le Chat Noir — application de bureau Linux

Client d'écoute natif de la webradio **Le Chat Noir** pour Linux (Ubuntu/GNOME et
Linux Mint/Cinnamon). GTK4 + libadwaita, en Rust. Même flux et mêmes données éditoriales
que l'app macOS et le site (sources mutualisées).

## Structure (workspace Cargo)

```
crates/
  lcn-core/   # logique PURE (sans GTK) : config, modèles, parsing, état d'antenne,
              #   reconnexion, horloge. Tous les tests unitaires. Compile partout.
  lcn-app/    # binaire `lechatnoir-player` : GTK4/libadwaita/Relm4, GStreamer, MPRIS.
data/         # .desktop, metainfo AppStream, schéma GSettings, icônes hicolor 48→512
flatpak/      # manifeste Flatpak
snapshots/    # (dans lcn-core) JSON éditoriaux figés, embarqués pour le repli hors-ligne
```

## Construire & tester

Prérequis : Rust (rustup), et les libs de dev **GTK4 + libadwaita + GStreamer (+ plugins)**.
- macOS (dev) : `brew install gtk4 libadwaita gstreamer gst-plugins-{base,good,bad} gst-libav`.
- Ubuntu : `sudo apt install libgtk-4-dev libadwaita-1-dev libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-{base,good,bad} gstreamer1.0-libav`.

```
cargo test          # logique pure (lcn-core)
cargo clippy --workspace
cargo run -p lcn-app
```

## Configuration (un seul point)

Tous les endpoints sont dans `crates/lcn-core/src/config.rs`.
- Temps réel (`stream.lechatnoirradio.fr/…`) : **figé**, ne change jamais.
- Contenu éditorial : `CONTENT_BASE_URL` = **préprod** aujourd'hui. Bascule prod = remplacer
  cette **seule** constante par `PRODUCTION_BASE_URL`. ⚠️ Ne basculer que lorsque
  `schedule.json` **et** `voices.json` répondent **200** en prod (404 au 13/06/2026).

## Hooks de dev/test (variables d'environnement)

| Variable | Effet |
|---|---|
| `LCN_AUTOPLAY=1` | démarre la lecture au lancement (vérif pipeline) |
| `LCN_FORCE_THEME=light\|dark\|auto` | force le thème (captures, CI) |
| `LCN_START_SECTION=home\|news\|history\|schedule\|voices\|about` | ouvre directement un écran |
| `LCN_STREAM_URL=…` | surcharge l'URL du flux (tester la reconnexion sur une URL morte) |
| `RUST_LOG=lechatnoir_player=debug` | logs détaillés |

## Packaging

- **Flatpak / Flathub** (canal unique de distribution) : `flatpak/fr.lechatnoirradio.Player.yml`.
  - Build **hors-ligne** conforme Flathub : crates Rust vendorées dans `flatpak/cargo-sources.json`
    (régénérer après tout changement de `Cargo.lock` via `flatpak-cargo-generator`), `cargo --offline`,
    aucun réseau au build (réseau conservé au runtime pour le flux).
  - Test local : `flatpak-builder --user --force-clean --install-deps-from=flathub --install
    build-flatpak flatpak/fr.lechatnoirradio.Player.yml` puis `flatpak run fr.lechatnoirradio.Player`.
  - Soumission Flathub : voir `flatpak/FLATHUB-CHECKLIST.md`.
- **App-id** : `fr.lechatnoirradio.Player` (conditionne .desktop, MPRIS, icônes, GSettings).
- **Licence** : GPL-3.0-only.

> Règle d'or packaging : valider sur l'**artefact final** (le Flatpak) lancé dans un
> environnement **propre** (sandbox), jamais seulement `cargo run`. Les assets internes
> (logo, CSS, snapshots) sont **embarqués dans le binaire** (`include_bytes!`), sans aucun
> chemin du système de fichiers à l'exécution.

## Plateformes

Cible v1 : Ubuntu 24.04+ (GNOME) et Linux Mint 21/22+ (Cinnamon), x86_64, Wayland + X11.
Crédit photo du logo et de l'icône : **Yirmi June** (utilisée avec autorisation —
voir [`ASSETS.md`](ASSETS.md) pour les droits des visuels).
