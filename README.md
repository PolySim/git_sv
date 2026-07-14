# git_sv - Visualisez Git dans votre terminal

[![CI](https://github.com/PolySim/git_sv/actions/workflows/ci.yml/badge.svg)](https://github.com/PolySim/git_sv/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`git_sv` est un client Git en terminal ecrit en Rust.

Il combine :

- une TUI pour naviguer dans l'historique, les diffs, le staging et les branches ;
- un mode CLI pour les usages rapides et les scripts ;
- une lecture du graphe git plus visuelle que les commandes terminal classiques.

En une phrase : `git_sv` cherche a apporter une experience proche d'un client Git graphique, sans quitter le terminal.

![Rust](https://img.shields.io/badge/Rust-2021-orange) ![ratatui](https://img.shields.io/badge/TUI-ratatui-green)

---

## Pourquoi git_sv ?

- pour lire un historique git complexe sans se battre avec un log brut ;
- pour stager, committer et naviguer plus vite sans sortir du terminal ;
- pour garder un outil scriptable grace au mode CLI ;
- pour offrir une experience terminal plus moderne autour de Git.

## Points forts

- graphe de commits avec couleurs stables par branche ;
- staging interactif par fichier, hunk ou ligne, avec diff et message de commit ;
- prévisualisation des images PNG, JPEG, GIF, WebP et SVG dans les diffs ;
- gestion des branches locales et distantes ;
- worktrees et stashes ;
- recherche de commits et filtres sur le graphe ;
- blame et resolution de conflits ;
- operations distantes `push`, `pull`, `fetch` ;
- rafraichissement automatique quand l'etat git change.

---

## Installation

Les releases officielles fournissent des binaires 64 bits pour Linux, Windows et macOS. Les gestionnaires de paquets sont la méthode recommandée : ils installent `git_sv` dans le `PATH` et simplifient les mises à jour.

### Linux

#### Homebrew (recommandé)

Homebrew fonctionne sur la plupart des distributions Linux récentes :

```bash
brew tap PolySim/homebrew-tap
brew install git_sv
git_sv --version
```

Mise à jour :

```bash
brew update
brew upgrade git_sv
```

#### Archive binaire x86_64

Cette méthode installe automatiquement le binaire de la dernière release dans `/usr/local/bin`. Elle nécessite `curl`, `tar` et une distribution utilisant glibc.

```bash
tmp_dir="$(mktemp -d)"
asset_url="$(curl -fsSL https://api.github.com/repos/PolySim/git_sv/releases/latest \
  | sed -n 's/.*"browser_download_url": "\([^"]*x86_64-unknown-linux-gnu.tar.gz\)".*/\1/p' \
  | head -n 1)"
test -n "$asset_url"
curl -fL "$asset_url" -o "$tmp_dir/git_sv.tar.gz"
tar -xzf "$tmp_dir/git_sv.tar.gz" -C "$tmp_dir"
sudo install -m 0755 "$tmp_dir/git_sv" /usr/local/bin/git_sv
rm -rf "$tmp_dir"
git_sv --version
```

Pour mettre à jour une installation par archive, relancez les mêmes commandes.

### Windows

#### Scoop (recommandé)

Depuis PowerShell :

```powershell
scoop bucket add git_sv https://github.com/PolySim/scoop-git_sv
scoop install git_sv
git_sv --version
```

Mise à jour :

```powershell
scoop update
scoop update git_sv
```

#### Archive binaire x86_64

Le script suivant télécharge la dernière release, installe `git_sv.exe` dans `%LOCALAPPDATA%\Programs\git_sv` et ajoute ce dossier au `PATH` utilisateur :

```powershell
$release = Invoke-RestMethod https://api.github.com/repos/PolySim/git_sv/releases/latest
$asset = $release.assets |
  Where-Object { $_.name -like '*x86_64-pc-windows-msvc.zip' } |
  Select-Object -First 1
if (-not $asset) { throw 'Archive Windows introuvable dans la dernière release' }

$installDir = Join-Path $env:LOCALAPPDATA 'Programs\git_sv'
$archive = Join-Path $env:TEMP 'git_sv.zip'
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Invoke-WebRequest $asset.browser_download_url -OutFile $archive
Expand-Archive -Path $archive -DestinationPath $installDir -Force
Remove-Item $archive

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (($userPath -split ';') -notcontains $installDir) {
  [Environment]::SetEnvironmentVariable('Path', "$userPath;$installDir", 'User')
}
$env:Path += ";$installDir"
git_sv --version
```

Pour mettre à jour une installation par archive, relancez le script.

### macOS

```bash
brew tap PolySim/homebrew-tap
brew install git_sv
```

### Compilation avec Cargo

Cette méthode fonctionne sur Linux, Windows et macOS. Elle nécessite Git, la toolchain Rust stable et un compilateur C. Sous Windows, utilisez la toolchain Rust MSVC et installez la charge de travail « Desktop development with C++ » des Build Tools Visual Studio.

```bash
cargo install --git https://github.com/PolySim/git_sv.git --features vendored-ssl
git_sv --version
```

Sous Debian/Ubuntu, les prérequis de compilation peuvent être installés avec :

```bash
sudo apt update
sudo apt install build-essential pkg-config perl
```

Sous Fedora :

```bash
sudo dnf install gcc pkgconf-pkg-config perl
```

Pour construire manuellement le dépôt :

```bash
git clone https://github.com/PolySim/git_sv.git
cd git_sv
cargo build --locked --release --features vendored-ssl
```

Le binaire se trouve ensuite dans `target/release/git_sv` ou `target\release\git_sv.exe` sous Windows.

### Vérification

```bash
git_sv --version
git_sv --help
```

---

## Utilisation

### Mode interactif

```bash
# Dans un repository git
git_sv

# Sur un repository specifique
git_sv --path /chemin/vers/repo
git_sv -p /chemin/vers/repo
```

### Mode CLI

```bash
# Historique
git_sv log
git_sv log -n 50
git_sv log --author "Alice"
git_sv log --message "fix"
git_sv log --since 2024-01-01 --until 2024-12-31
git_sv --format json log --author "Alice"

# Branches
git_sv branches
git_sv --format json branches

# Status
git_sv status
git_sv --format plain status

# Diagnostic local (signature de HEAD, hooks et sous-modules)
git_sv inspect
git_sv --format json inspect

# Recherche
git_sv search "fix bug"

# Graphe simplifie
git_sv graph -n 30
git_sv --format json graph
```

### Formats de sortie CLI

- `human` : sortie lisible avec couleurs ;
- `plain` : texte simple ;
- `json` : sortie structuree pour scripts.

`--format` est une option globale : elle se place avant la sous-commande.

La sous-commande `graph` limite actuellement la sortie a 50 commits maximum, meme si `-n` demande plus.

### Exemples

```bash
# Extraire les hashes d'un auteur
git_sv --format json log --author "Alice" | jq '.[].hash'

# Verifier rapidement les fichiers modifies
git_sv --format plain status | grep "^M"

# Trouver le dernier commit ayant touche un fichier
git_sv log -n 1 --path-filter src/main.rs
```

## Configuration

La configuration est chargee depuis `~/.config/git_sv/config.json`. Les themes
disponibles sont `dark`, `light` et `solarized`.

Pour afficher les thèmes et choisir interactivement :

```bash
git_sv theme
```

Un thème peut également être activé directement, même en dehors d'un dépôt
Git. L'alias `themes` est aussi disponible :

```bash
git_sv theme solarized
git_sv themes light
```

Le theme `solarized` reutilise le fond, le texte et les 16 couleurs ANSI du
profil terminal. Il s'adapte donc aux variantes Solarized Light et Dark sans
imposer un fond RGB different de celui du terminal :

```json
{
  "language": "fr",
  "theme": "solarized",
  "keybindings": {
    "graph.inspect": "ctrl+i",
    "diff.external": "alt+e",
    "staging.commit": "ctrl+c"
  },
  "custom_commands": [
    {
      "name": "Tests Rust",
      "key": "alt+t",
      "command": "cargo test",
      "confirm": true,
      "pause": true
    }
  ]
}
```

Les raccourcis configurés sont prioritaires sur les raccourcis intégrés quand
les deux utilisent la même combinaison. Les combinaisons acceptent notamment
`ctrl`, `alt`, `shift` et `super`, ainsi que `enter`, `esc`, `space`, `tab`, les
flèches, `home`, `end`, `pageup` et `pagedown`.

Identifiants d'action disponibles :

- globaux : `global.quit`, `global.refresh`, `global.help`, `global.copy`,
  `git.push`, `git.force_push`, `git.pull`, `git.fetch` ;
- vues : `view.graph`, `view.staging`, `view.branches`, `view.tree` ;
- graphe : `graph.commit`, `graph.stash`, `graph.merge`, `graph.search`,
  `graph.filter`, `graph.blame`, `graph.cherry_pick`, `graph.reset`,
  `graph.interactive_rebase`, `graph.undo`, `graph.create_tag`,
  `graph.delete_tag`, `graph.compare_head`, `graph.bisect`, `graph.inspect`,
  `graph.load_more` ;
- diff : `diff.external`, `diff.next_hunk`, `diff.previous_hunk`,
  `diff.toggle_view`, `diff.fullscreen` ;
- staging : `staging.stage_file`, `staging.unstage_file`, `staging.stage_all`,
  `staging.unstage_all`, `staging.stage_hunk`, `staging.unstage_hunk`,
  `staging.stage_line`, `staging.unstage_line`, `staging.commit`,
  `staging.discard_file`, `staging.discard_all` ;
- branches/arborescence : `branches.create`, `branches.delete`,
  `branches.rename`, `branches.checkout`, `tree.search`, `tree.compare`.

Une commande personnalisée s'exécute depuis la racine du dépôt avec le shell
utilisateur, après suspension propre de la TUI. `confirm` protège contre un
déclenchement accidentel et `pause` conserve sa sortie visible jusqu'à Entrée.
La variable `GIT_SV_REPO` contient le chemin du dépôt.

---

## Raccourcis clavier principaux

### Navigation globale

| Touche | Action |
|--------|--------|
| `1` | Vue graph |
| `2` | Vue staging |
| `3` | Vue branches |
| `4` | Vue arborescence |
| `5` | Vue conflits si active |
| `?` | Aide |
| `q` | Quitter |
| `Ctrl+c` | Quitter |

### Vue Graph

| Touche | Action |
|--------|--------|
| `j` / `k` | Naviguer dans les commits ou le panneau focalise |
| `g` / `G` | Aller au debut / a la fin du graphe ou du diff focalise |
| `Ctrl+d` / `Ctrl+u` | Page suivante / precedente dans le graphe, ou dans le diff si le diff a le focus |
| `Tab` | Changer de panneau |
| `Enter` | Depuis le graphe, ouvrir le panneau fichiers ; depuis fichiers/diff, basculer le diff plein ecran |
| `z` | Basculer le diff plein ecran depuis fichiers ou diff |
| `M` | Basculer le panneau bas-gauche |
| `r` | Rafraichir |
| `P` | Push |
| `Ctrl+p` | Force push |
| `p` | Pull |
| `f` | Fetch |
| `x` | Cherry-pick |
| `y` | Copier le contenu du panneau actif |
| `/` | Recherche |
| `F` | Filtres |
| `v` | Basculer le mode de diff quand le diff a le focus |
| `L` | Charger plus d'historique |
| `I` | Ouvrir un rebase interactif depuis le commit sélectionné |
| `Z` | Annuler la dernière transition de `HEAD` via le reflog (changements conservés) |
| `t` / `T` | Créer / supprimer un tag sur le commit sélectionné |
| `C` | Comparer le commit sélectionné à `HEAD` (commits et statistiques du diff) |
| `X` | Démarrer un bisect avec le commit sélectionné comme commit connu bon |
| `[` / `]` / `\` | Pendant un bisect : marquer bon / mauvais / terminer |
| `i` | Inspecter la signature du commit, les hooks et les sous-modules |
| `e` | Depuis un fichier ou son diff, ouvrir `git difftool` (la TUI est suspendue proprement) |
| `n` / `N` | Depuis le diff, aller au hunk suivant / précédent |

### Vue Staging

| Touche | Action |
|--------|--------|
| `j` / `k` | Naviguer dans les fichiers |
| `s` | Stage fichier |
| `u` | Unstage fichier |
| `a` | Stage tout |
| `U` | Unstage tout |
| `d` | Discard fichier |
| `D` | Discard tout |
| `c` | Editer le message de commit |
| `A` | Amend |
| `Tab` | Changer de focus |

Quand le panneau diff a le focus, `s`/`u` indexe ou désindexe le hunk
sélectionné et `S`/`U` applique uniquement la ligne ajoutée ou supprimée.
`n`/`N` navigue entre les hunks et `e` ouvre le fichier dans l'outil défini par
`git config diff.tool`.

### Vue Branches

| Touche | Action |
|--------|--------|
| `j` / `k` | Naviguer |
| `Tab` / `Shift+Tab` | Changer de section |
| `Enter` | Checkout |
| `n` | Creer branche ou worktree |
| `d` | Supprimer l'element courant |
| `r` | Renommer la branche |
| `R` | Afficher ou masquer les branches distantes |
| `m` | Merge |
| `s` | Sauver un stash |
| `a` | Appliquer un stash |
| `p` | Pop un stash |

### Vue Arborescence

| Touche | Action |
|--------|--------|
| `j` / `k` | Naviguer dans le panneau actif |
| `g` / `G` | Aller au debut / a la fin du panneau actif |
| `PageUp` / `PageDown` | Changer de page dans le panneau actif |
| `Enter` / `Espace` | Ouvrir ou fermer le dossier selectionne |
| `←` / `h` | Fermer le dossier ou selectionner son parent |
| `→` / `l` | Ouvrir le dossier selectionne |
| `/` | Rechercher rapidement un fichier ou dossier, avec tolerance aux fautes |
| `Tab` | Parcourir arbre, historique, fichiers touches et diff |
| `C` | Comparer les commits du chemin avec une autre branche |
| `Esc` | Fermer la comparaison de chemin active |
| `v` | Basculer le diff unifie / cote a cote |
| `e` | Depuis les fichiers touchés ou le diff, ouvrir `git difftool` |
| `n` / `N` | Depuis le diff, aller au hunk suivant / précédent |
| `r` | Rafraichir l'arborescence courante |
| `y` | Copier le chemin, le commit, l'etat du fichier au commit ou le diff actif |

La selection d'un chemin affiche les commits qui l'ont touche. La selection
d'un commit affiche tous ses fichiers modifies, puis le patch du fichier
selectionne. Dans le panneau « fichiers touches », `y` copie le contenu exact
du fichier tel qu'il existe dans le commit. La comparaison de branches affiche
uniquement les commits divergents qui ont touche le chemin : `+` indique la
branche courante et `-` la branche comparee. Le diff reste celui du commit
selectionne afin de parcourir les modifications une par une.

En mode CLI humain, `git_sv status` rend également les chemins cliquables dans
les terminaux compatibles OSC 8. Définissez `NO_HYPERLINK=1` pour les désactiver.

---

## Architecture

Structure actuelle simplifiee :

```text
src/
├── main.rs
├── app.rs
├── cli.rs
├── config.rs
├── error.rs
├── i18n.rs
├── terminal.rs
├── watcher.rs
├── git/
├── handler/
│   ├── dispatcher/
│   └── conflict/
├── state/
│   ├── action/
│   └── view/
├── test_utils/
└── ui/
    ├── common/
    └── input/
```

Pour le detail des modules et du flux d'execution, voir `docs/ARCHITECTURE.md`.

---

## Developpement

### Commandes de base

```bash
# Build
cargo build
cargo build --release

# Lancer
cargo run
cargo run -- --path /chemin/vers/repo
cargo run -- log -n 10

# Tests
cargo test
cargo test nom_du_test
cargo test module::
cargo test -- --nocapture

# Formatage
cargo fmt
cargo fmt -- --check

# Lint
cargo clippy
cargo clippy --all-features -- -D warnings

# Verification rapide
cargo check
```

### Conventions

- imports groupes : `std`, crates externes, modules internes ;
- commentaires en francais ;
- types en `PascalCase`, fonctions en `snake_case` ;
- `crate::error::Result` dans les modules, `anyhow::Result` au point d'entree.

---

## Pour communiquer sur le projet

- changelog : `CHANGELOG.md`
- kit de communication : `docs/COMMUNICATION.md`
- architecture : `docs/ARCHITECTURE.md`
- contribution : `docs/CONTRIBUTING.md`

---

## Licence

MIT - voir `LICENSE`.
