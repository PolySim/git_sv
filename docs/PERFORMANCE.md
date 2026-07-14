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
  entrée trop lourde provoque immédiatement une éviction.
- Les diffs sont partagés avec `Arc<FileDiff>` entre l'état et le cache, ce qui
  évite de dupliquer leurs lignes et leurs images.
- Un diff matérialise au maximum 20 000 lignes.
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
moment du scan quinquennal.

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
