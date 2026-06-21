# Dossier de soumission Flathub

Ces **2 fichiers** constituent la proposition à déposer sur Flathub :

- `fr.lechatnoirradio.Player.yml` — le manifeste (source = dépôt Git au tag `v26.6.20`, build hors-ligne)
- `cargo-sources.json` — les dépendances Rust vendorées (généré depuis `Cargo.lock`)

## Comment les utiliser (étape « Soumettre »)

1. Forker `github.com/flathub/flathub` (décocher « Copy the master branch only »,
   pour récupérer la branche `new-pr`).
2. Dans ton fork, créer une branche **à partir de `new-pr`** (nom libre, p.ex. `fr.lechatnoirradio.Player`).
3. Copier **ces 2 fichiers** à la **racine** de cette branche.
4. Commit + push, puis ouvrir une **Pull Request** **contre la branche `new-pr`**
   de `flathub/flathub` (⚠️ surtout pas `master`).
   Titre de la PR : **`Add fr.lechatnoirradio.Player`**.

Un robot construit la PR ; des relecteurs valident ; à la fusion, l'app est publiée et
un dépôt dédié `flathub/fr.lechatnoirradio.Player` est créé pour les mises à jour.

## En cas de nouvelle version

1. Créer un nouveau tag (ex. `v26.07.05`) sur `lechatnoir-linux`.
2. Régénérer `cargo-sources.json` si `Cargo.lock` a changé.
3. Mettre à jour `tag:` + `commit:` dans le manifeste, et l'envoyer dans le dépôt
   `flathub/fr.lechatnoirradio.Player`.
