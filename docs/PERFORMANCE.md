# Performance et ressources

Ce document décrit les garde-fous de `git_sv` concernant le poids du binaire,
la mémoire et l'activité CPU au repos. Les chiffres sont des mesures locales,
pas des garanties identiques sur toutes les plateformes.

## Mesure de référence

Mesure du 14 juillet 2026 sur macOS arm64, Rust 1.96.1, profil release par
défaut :

| Mesure | Résultat |
|--------|----------|
| `cargo build --release` avec cache chaud | 13,33 s |
| Taille de `target/release/git_sv` | 5 807 680 octets (5,54 MiB) |
| `git_sv --format plain graph -n 50` | 0,42 s réel sur ce dépôt |

Le binaire de départ de l'audit pesait environ 13,5 MiB. La release courante
est donc environ 59 % plus petite. Les facteurs principaux sont `strip`, Thin
LTO, un seul codegen unit et la désactivation des codecs inutilisés dans les
dépendances d'images.

## Bornes mémoire

- Le cache LRU de diffs est limité à 50 entrées **et** 64 MiB estimés. Une
  entrée trop lourde est refusée sans évincer les autres entrées.
- Les diffs sont partagés avec `Arc<FileDiff>` entre l'état et le cache, ce qui
  évite de dupliquer leurs lignes et leurs images.
- Un diff matérialise au maximum 20 000 lignes et 4 MiB de texte.
- Le seuil des entrées texte libgit2 est de 20 MiB.
- Undo/redo est limité à 8 MiB de texte par champ.
- Les sorties de processus sont limitées à 8 MiB par flux et drainées simultanément.
- La lecture d'un fichier non suivi est limitée à 1 MiB pour le diff texte.
- Une prévisualisation d'image est refusée au-delà de 20 MiB.
- L'historique charge 200 commits au démarrage, puis double progressivement
  jusqu'à une limite de sécurité de 10 000 commits.
- Une cellule du graphe occupe un octet, y compris dans `Option<GraphCell>`.

Ces bornes ne représentent pas le RSS total : libgit2, le terminal, le graphe
chargé et les buffers du système consomment aussi de la mémoire. Elles empêchent
cependant les principales structures applicatives de croître sans contrôle.

## CPU et énergie au repos

La TUI n'utilise plus de boucle de rendu à fréquence fixe :

- un redraw se produit après une entrée, une mutation d'état, un résultat de
  tâche de fond, une expiration de message ou pendant l'animation du spinner ;
- hors animation, l'attente est bloquante jusqu'au prochain événement utile ;
- les métadonnées Git sont vérifiées toutes les 2 secondes ;
- le scan plus coûteux du working tree est séparé et limité à toutes les 5
  secondes ;
- un debounce de 500 ms regroupe les rafales de modifications ;
- `push`, `pull`, `fetch` et la lecture de PR GitHub s'exécutent hors du thread
  de rendu ; les éditeurs, difftools et commandes utilisateur suspendent la TUI.

Cette architecture réduit fortement les réveils CPU lorsque l'utilisateur ne
fait rien. Le coût résiduel dépend surtout de la taille du working tree au
moment du scan effectué toutes les cinq secondes. Depuis la version 0.5.3,
ce scan s'exécute sur un worker dédié, sans bloquer les entrées du terminal.

## Reproduire les mesures

```bash
cargo build --release
ls -lh target/release/git_sv
stat -f "%z bytes" target/release/git_sv # macOS
stat -c "%s bytes" target/release/git_sv # Linux

/usr/bin/time -p target/release/git_sv --format plain graph -n 50
```

Pour mesurer le RSS de la TUI, ouvrez-la sur un dépôt représentatif et relevez
le pic après avoir parcouru plusieurs gros diffs. Sous Linux,
`/usr/bin/time -v git_sv` fournit `Maximum resident set size`; sous macOS,
utilisez `time -l` ou Activity Monitor.

## Garde-fous de contribution

Avant de fusionner une modification sensible aux performances :

1. exécuter les tests et Clippy avec toutes les features ;
2. vérifier aussi `cargo check --no-default-features` ;
3. reconstruire en release et noter toute croissance significative du binaire ;
4. tester un dépôt avec beaucoup de commits et un gros diff ;
5. éviter tout timer de redraw permanent ou cache non borné.

## Audit 0.5.2 — 6 novembre 2026

Le [rapport détaillé](AUDIT-0.5.2.md) décrit les bornes, les régressions testées et
les limites des observations mémoire. Sur 10 000 commits, le filtrage d'un
résultat ancien passe de 732,2 ms à 45,1 ms (médianes de cinq mesures alternées).
Le graphe de 50 commits passe de 32,8 à 39,6 ms ; le gain ne concerne donc pas
tous les chemins. Le binaire macOS avec OpenSSL embarqué passe de 8 897 440 à
8 036 896 octets. Ces tailles ne sont pas directement comparables à la build
sans OpenSSL embarqué de la mesure historique ci-dessus.

## Gros dépôt réel — version 0.5.3

Mesure du 6 novembre 2026 sur macOS arm64, avec 32 826 fichiers suivis,
11 206 commits et 250 références. Le dépôt est propre. Seuls les nombres et
durées sont conservés ; les essais ne modifient ni ses fichiers, ni son index,
ni ses références.

Les médianes suivantes portent sur trois ouvertures de la TUI via Rex, avec
la version publiée 0.5.2 puis la build release 0.5.3. Les changements d'onglet
sont mesurés jusqu'à l'apparition de leur contenu dans le terminal. Le coût
de pilotage et de capture de Rex est inclus ; les mesures ne correspondent
donc pas au temps CPU du rendu seul. Le cache disque est chaud après le
premier passage, sans purge forcée du système.

| Action | 0.5.2 | 0.5.3 |
| --- | ---: | ---: |
| Premier écran | 888 ms | 176 ms |
| Premier accès au staging | 750 ms | 37 ms |
| Premier accès aux branches | 823 ms | 70 ms |
| Premier accès à l'arborescence | 1 175 ms | 72 ms |
| Retour au graphe | 759 ms | 38 ms |
| Onglets déjà visités | 748–1 196 ms | 36–39 ms |

Le premier passage de chaque série prend respectivement 1 082 et 644 ms
pour le premier écran : la médiane ne représente pas un démarrage à froid.

Le statut complet arrive ensuite en arrière-plan : environ 604 ms après
le lancement dans ces mesures. L'interface reste utilisable pendant ce
scan et affiche « vérification… » ; elle n'annonce pas un dépôt propre
avant la première collecte. L'historique d'un fichier rare reste un calcul
coûteux (environ 0,6 s dans ce dépôt), exécuté lui aussi hors du thread TUI.
Sa mise en file prend 1,27 ms, et la récupération du résultat au plus
0,19 ms dans le profilage dédié.

Le profilage Rust isolé mesure environ 0,15–0,22 ms pour changer de vue
déjà chargée et la dessiner dans un `TestBackend` de 160 × 48 cellules.
Vingt déplacements dans le graphe, avec rendu à chaque étape, prennent
environ 4 ms. Ces mesures excluent le terminal réel.

Les causes corrigées sont les lectures globales sur chaque changement
d'onglet, le scan synchrone du watcher, puis un second scan de statut et
32 826 appels de métadonnées pour reconstruire l'arbre. Le helper qui
réutilise le statut construit désormais sa liste de chemins en 4–5 ms.
Le filtre par chemin évite aussi les statistiques de patches inutiles :
sur vingt commits et 101 fichiers modifiés, 225 ms deviennent 113 ms.

Les workers gardent au plus une demande en attente et un résultat. Les
générations et les références capturées empêchent l'application d'un résultat
périmé. Quitter n'attend pas un scan libgit2 déjà commencé : il termine,
annule le travail restant et libère ses ressources. Aucun thread n'est créé
à chaque tick ou changement de sélection.

Pour reproduire le profilage en lecture seule sur un dépôt local :

```bash
GIT_SV_PROFILE_REPO=/chemin/vers/grand-depot \
  cargo test --release --features profiling profile_large_repository -- --ignored --nocapture
GIT_SV_WATCHER_BENCH_REPO=/chemin/vers/grand-depot \
  cargo test --release benchmark_watcher_on_repository -- --ignored --nocapture
```
