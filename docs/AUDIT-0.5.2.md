# Audit de fiabilité, mémoire et performances — git_sv 0.5.2

Audit du 6 novembre 2026, réalisé depuis le tag publié `v0.5.1` (`d18e0bd`).
Le fetch et la vérification des tags distants ont précédé les modifications.
Les douze commits déjà présents localement et publiés dans ce tag ont été conservés.
Le manifeste de cette ancienne version indiquait encore `0.5.0` ; la version est
maintenant cohérente dans Cargo.toml, Cargo.lock et le tag de livraison.

## Méthode et couverture

Revue des couches Git, handlers, état, CLI, cache, rendu, entrées clavier/souris,
watcher, terminal, configuration et livraison. Les corrections s'appuient sur des
cas reproductibles et sur de vrais dépôts temporaires, plutôt que sur le seul
passage des tests existants. La suite initiale comptait 412 tests unitaires et
5 tests d'intégration, tous verts malgré les défauts ci-dessous.

L'audit combine lecture du code, tests de régression, Clippy, builds avec et sans
images, audit RustSec, mesures en release et exercices de la TUI via Rex. Il ne
constitue pas une preuve d'absence de tous les défauts possibles ni un test de
charge prolongé sur tous les types de dépôts et systèmes de fichiers.

## Conservation des données Git

| Problème confirmé | Correction et preuve principale |
| --- | --- |
| Discard d'un fichier partiellement stagé rétablissait HEAD et pouvait perdre le staging | Restauration depuis l'index ; tests comparent le contenu et l'arbre index avant/après, y compris avant le premier commit |
| Merge possible en fast-forward utilisait un checkout forcé | Checkout sûr avant mutation de HEAD ; tests de collision refusée et de conservation des changements indépendants |
| Résolution de conflit perdait le contexte au-delà des quelques lignes affichées | Reconstruction du fichier complet ; régressions contexte long, diff3, CRLF et fin sans saut de ligne |
| Écriture des conflits relative au répertoire de lancement | Chemins ancrés au worktree ; refus des traversées et liens symboliques ; sauvegarde manuelle et stratégies utilisent le même garde-fou |
| Index libgit2 périmé après rebase/pull externe | Relecture explicite de l'index ; reproduction d'un résultat `Conflicts([])` alors que Git avait bien enregistré un conflit |
| Finalisation/abort traitaient toute opération comme un merge | Reprise/annulation native de rebase, cherry-pick, revert ou am ; conservation des parents du merge et des changements indépendants |
| Staging partiel : hunk mal ciblé, EOF modifiée, faux échec après mutation | Validation avant application, offsets et chemins littéraux, préservation des fins de fichiers ; tests hunks suivants, lignes et noms spéciaux |
| Déstaging impossible dans un dépôt sans premier commit | Gestion explicite de HEAD unborn pour fichier, ensemble et hunk |
| Hunk partiellement masqué par la limite d'affichage pouvait être stagé entièrement | Refus des actions partielles sur un diff tronqué ; test 20 100 lignes vérifie l'index inchangé ; action fichier complet conservée |
| Séquence de stashes temporaires pouvait toucher un stash préexistant | Opération native unique avec conservation de l'index ; tests du stash existant et des fichiers non suivis |
| Confirmation de suppression stash liée à un index mouvant | OID mémorisé et contrôlé lors de la confirmation |
| Worktree créé sans respecter la branche demandée ou supprimé malgré des changements | Branche transmise à l'API et suppression Git native refusant un worktree sale |
| Push natif ciblait le nom local plutôt que l'upstream réel | Résolution de la destination et traitement des rejets serveur ; tests upstream de nom différent |
| Undo reflog après checkout pouvait réinitialiser une autre branche | Vérification de la branche cible avant reset |
| Amend conservait l'ancien committer | Auteur conservé, identité/date du committer actualisées |

Les conflits binaires et ceux portant sur des liens symboliques sont explicitement
refusés lorsqu'une résolution sûre n'est pas prise en charge ; ils restent
résolubles avec Git. Les opérations destructives conservent leurs confirmations.

## Mémoire, ressources et processus

Aucune fuite permanente ni cycle Arc n'a été confirmé dans le code applicatif
inspecté. Plusieurs rétentions et allocations insuffisamment bornées ont en
revanche été corrigées.

| Structure ou opération | Borne / changement |
| --- | --- |
| Texte d'un diff affiché | 20 000 lignes et 4 MiB de contenu ; le nombre de lignes seul ne protégeait pas contre une ligne géante |
| Entrée texte libgit2 | Seuil 20 MiB ; blobs et lectures disque contrôlés avant matérialisation |
| Cache de diffs | 50 entrées / 64 MiB estimés ; entrée trop grosse ignorée sans vider les entrées utiles |
| Undo/redo | 8 MiB de texte par champ, en plus du nombre maximal de snapshots |
| Blame | Source 4 MiB, 100 000 lignes, 32 MiB estimés d'annotations ; suppression des copies intermédiaires et recherches de commit par ligne |
| Sorties de processus | Drainage simultané stdout/stderr, limite de 8 MiB par flux, erreur explicite en cas de dépassement |
| Durée des processus | Délai couvrant aussi la fermeture des pipes ; nettoyage du processus et de son groupe en cas d'erreur/dépassement |
| Cache d'images | Arc de la source conservé dans la clé et format vérifié ; fin de réutilisation possible d'une adresse mémoire recyclée |
| Images hors vue | Protocole libéré dans les vues sans diff et lors d'un changement de dépôt |
| SVG | Pas de chargement de fichiers externes via href ni de décompression SVGZ implicite |
| Lecture au presse-papier | 20 MiB maximum et refus des contenus binaires |

Les commandes de fond ne lisent plus le clavier de la TUI. Un test produit
256 KiB sur chacun des deux pipes pour vérifier l'absence de blocage. Un autre
laisse un descendant conserver les pipes ouverts et vérifie l'application du délai.

Le terminal tente toutes les étapes de restauration même si une séquence échoue,
et une garde restaure le mode raw si l'initialisation échoue avant la création
de la session. Les erreurs ordinaires d'action Git deviennent des messages dans
la TUI, sans fermer l'application ni effacer la saisie.

Ces bornes ne forment pas un plafond de RSS global : libgit2, les bibliothèques
natives, le moteur SVG, les caches et le système peuvent consommer davantage.

## Recherche, rafraîchissement et interface

- Le log filtré utilise un seul revwalk, au lieu de recommencer le parcours depuis
  le début pour chaque lot. Les limites numériques extrêmes n'imposent plus une
  allocation immédiate ni un dépassement arithmétique.
- La recherche CLI parcourt l'historique jusqu'au nombre de résultats demandé ;
  elle ne se limite plus arbitrairement à trois fois ce nombre de commits.
- HEAD détachée est incluse même sans référence de branche ; son repère est affiché.
- La recherche floue respecte les frontières Unicode et borne le calcul de
  distance d'édition à la tolérance utile, avec réutilisation des buffers.
- Le parcours d'ascendance devient un DFS itératif, sans requête d'ascendance
  répétée pour chaque commit.
- Le watcher compare les valeurs des références, notamment distantes et imbriquées.
  Le mtime d'un dossier ne change pas lorsqu'une référence existante avance.
  La découverte libgit2 identifie correctement les worktrees imbriqués.
- Le refresh manuel invalide les diffs du working tree. Les recherches sont
  recalculées après effacement, changement de type et refresh.
- Les sélections sont préservées par identité ; changer de fichier réinitialise
  ligne et défilement, même lors d'un accès au cache. Le clic sur un autre stash
  recharge immédiatement le panneau de détails.
- Les clics tiennent compte du défilement. Les modales absorbent les événements.
  Une confirmation destructive exige un clic sur la zone visible du bouton,
  au lieu d'une ligne approximative pouvant inclure une zone vide.
- Navigation des conflits vides, édition d'un fichier vide et petites dimensions
  du terminal couvertes par régressions.
- `--until` inclut toute la journée UTC ; dates et formats invalides sont refusés.
  `--format` fonctionne aussi après la sous-commande. Les pipes JSON fermés ne
  provoquent plus de panic. Une configuration invalide n'est plus silencieusement
  remplacée par les valeurs par défaut.
- Le diagnostic des hooks tient compte de core.hooksPath et des worktrees liés.
- Les timestamps antérieurs à 1970 ne provoquent plus de dépassement dans le blame.

## Mesures reproductibles

Machine macOS arm64, Rust 1.99.1. Comparaison du binaire macOS publié en v0.5.1
avec la build locale v0.5.2 `--release --features vendored-ssl`. Dépôt synthétique
linéaire de 10 000 commits, un message « needle » uniquement sur le premier commit.
Une exécution de chauffe, puis cinq exécutions alternées par version ; médianes
mesurées avec time.perf_counter, sortie capturée et code de retour vérifié.

| Scénario | v0.5.1 | v0.5.2 | Observation |
| --- | ---: | ---: | --- |
| `--format plain log -n 1 --message needle` | 732,2 ms | 45,1 ms | Environ 16,2× plus rapide |
| `--format plain graph -n 50` | 32,8 ms | 39,6 ms | Environ 6,8 ms supplémentaires ; ce chemin n'a pas été accéléré |
| Taille du binaire macOS avec OpenSSL embarqué | 8 897 440 octets | 8 036 896 octets | Environ −9,7 % |

Microbenchmark séparé du score fuzzy : 300 000 scores sur 10 000 chemins
synthétiques, trois requêtes, dix répétitions, compilation Rust optimisée.
Médianes 133,915 ms avant / 56,006 ms après, environ 2,39× plus rapide ; résultats
ASCII identiques. Ce microbenchmark ne mesure pas toute la latence de la TUI.

`leaks --atExit -- git_sv ... graph -n 50` : 0 fuite détectée, empreinte physique
12,8 MiB sur le scénario CLI mesuré. `leaks` sur la TUI après navigation et staging :
0 fuite détectée, 5 211 KiB d'allocations malloc actives. macOS a limité l'inspection
du contenu mémoire du binaire non débogable ; ces résultats sont des observations
sur ces scénarios, pas une garantie générale.

## Dépendances

L'audit RustSec a été exécuté sur la base mise à jour le 5 novembre 2026.
Mises à jour : git2 0.19 → 0.21.0 / libgit2 1.9.7, lru direct 0.12 → 0.18.5,
anyhow 1.0.104, libz-sys 1.1.31, clipboard-win 5.4.3.

- [git2 : Remote::list](https://rustsec.org/advisories/RUSTSEC-2026-0183.html),
  [BlameHunk](https://rustsec.org/advisories/RUSTSEC-2026-0184.html) et
  [Buf](https://rustsec.org/advisories/RUSTSEC-2026-0008.html) : versions corrigées.
- [anyhow downcast_mut](https://rustsec.org/advisories/RUSTSEC-2026-0190.html) : version corrigée.
- [libz-sys](https://rustsec.org/advisories/RUSTSEC-2026-0418.html) : version corrigée.
  Le verrouillage initial utilisait cc 1.2.56, antérieur à la condition cc ≥1.6.0
  décrite par l'avis ; aucune exposition de secrets n'a été constatée.
- [lru pop](https://rustsec.org/advisories/RUSTSEC-2026-0253.html) et
  [IterMut](https://rustsec.org/advisories/RUSTSEC-2026-0002.html) : cache applicatif mis à jour.

Résultat final de cargo audit : **0 vulnérabilité classée comme telle**, avec
avertissements toujours visibles. Ratatui 0.29 conserve une dépendance transitive
lru 0.12.5 signalée pour deux problèmes de sûreté. Le cache Layout inspecté
n'utilise pas les appels pop/IterMut concernés et sa clé n'a pas de Drop
personnalisé susceptible de paniquer ; l'avis n'est cependant pas supprimé.
La migration conjointe Ratatui/ratatui-image reste un chantier distinct de
compatibilité. paste, rustybuzz et ttf-parser sont signalés non maintenus.
Aucun de ces avertissements n'a été masqué dans la configuration d'audit.

## Vérification et livraison

Validation finale locale :

- `cargo build --locked` et build release avec `vendored-ssl` : réussis ;
- tests par défaut et toutes features : 497 unitaires + 5 intégration, soit 502 ;
- tests sans images : 488 unitaires + 5 intégration, soit 493 ;
- 85 tests unitaires supplémentaires par rapport à la base publiée ;
- rustfmt, Clippy toutes features/toutes cibles avec `-D warnings`, diff --check : réussis ;
- CLI réelle : version, JSON, recherche ancienne, limite numérique extrême,
  dates/formats invalides et pipe fermé vérifiés ;
- audit de dépendances et contrôle CLI/TUI décrits ci-dessus.

 La TUI a été exercée sur le dépôt de
10 000 commits : graphe, staging réel d'un fichier, arborescence, branches et
sortie avec code 0. Les opérations de test n'ont pas modifié le dépôt de travail.

La CI teste aussi la variante sans images. Le workflow release refuse désormais
un tag différent des versions Cargo.toml/Cargo.lock avant publication, puis produit
les archives macOS Intel/ARM, Linux x86_64 et Windows x86_64 ainsi que leurs SHA256.
Les formules Homebrew et Scoop sont mises à jour par les jobs de livraison existants.
