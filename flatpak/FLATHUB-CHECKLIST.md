# Soumission Flathub — checklist

État : le manifeste `fr.lechatnoirradio.Player.yml` build **hors-ligne** (crates vendorées dans
`cargo-sources.json`), validé en sandbox. Voici les étapes pour publier sur Flathub.

## 1. Côté dépôt (à faire une fois)
- [ ] **Licence** : ajouter un fichier `LICENSE` **GPL-3.0-only** à la racine du dépôt
      (cohérent avec `Cargo.toml` et le `metainfo.xml`). Sur GitHub : « Add file → Create new file →
      LICENSE » propose le texte tout prêt.
- [ ] **Dépôt GitHub public** contenant le code (le propriétaire gère Git).
- [ ] **Screenshots** : héberger 1 à 3 captures (PNG) accessibles en HTTPS et mettre leurs URL dans
      `data/fr.lechatnoirradio.Player.metainfo.xml` (`<screenshots>`). Captures dispo dans
      `~/Desktop/lcn-screens/` (ex. `light-home.png`, `dark-schedule.png`).

## 2. Adapter le manifeste pour Flathub
Dans `flatpak/fr.lechatnoirradio.Player.yml`, remplacer la source locale par une source **git**
épinglée (Flathub n'accepte pas `type: dir`) :
```yaml
    sources:
      - type: git
        url: https://github.com/<org>/<repo>.git
        tag: v0.1.0
        commit: <sha du tag>
      - cargo-sources.json
```
> Après tout changement de `Cargo.lock`, régénérer le vendoring :
> `python3 flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json`

## 3. Validation locale (déjà OK, à refaire après modif)
- [ ] `flatpak-builder --user --force-clean --install-deps-from=flathub build flatpak/fr.lechatnoirradio.Player.yml`
      (build **hors-ligne** réussi).
- [ ] `flatpak run org.flatpak.Builder` → `flatpak-builder-lint manifest flatpak/fr.lechatnoirradio.Player.yml`
      et `flatpak-builder-lint repo <repo>` (zéro erreur — Flathub traite les warnings comme bloquants).
- [ ] `appstreamcli validate data/fr.lechatnoirradio.Player.metainfo.xml` (zéro erreur).

## 4. Soumettre
- [ ] Forker `github.com/flathub/flathub` (**décocher** « Copy the master branch only »).
- [ ] Créer une branche **à partir de `new-pr`** et y ajouter, **à la racine**, le manifeste +
      `cargo-sources.json` (les 2 fichiers de `flathub-submission/`).
- [ ] Ouvrir une **PR contre la branche `new-pr`** (⚠️ **pas** `master`), titre **`Add fr.lechatnoirradio.Player`**.
- [ ] Après acceptation : un dépôt `flathub/fr.lechatnoirradio.Player` est créé ; les builds sont gérés
      par l'infra Flathub.

## 5. Vérification éditeur (important pour Mint)
- [ ] Demander le statut **« Verified »** (preuve via le domaine `lechatnoirradio.fr` ou le compte
      GitHub). Sans ça, **Mint masque l'app par défaut**.

## Hors Flathub — rappel
- [ ] Le jour de la mise en prod du site : basculer `CONTENT_BASE_URL` → prod dans
      `crates/lcn-core/src/config.rs` (uniquement quand `schedule.json` **et** `voices.json` répondent
      **200** en prod). Sans rapport avec Flathub.
