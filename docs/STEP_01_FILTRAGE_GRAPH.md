# STEP 08.1 - Filtrage du graph (auteur/date/message/path)

**Priorite**: Haute  
**Effort estime**: 3h  
**Impact**: Eleve

## Objectif

Permettre le filtrage des commits affiches dans le graph selon plusieurs criteres combinables.

## Portee

- Filtres supportes: auteur, date debut, date fin, chemin, texte dans le message.
- Ouverture d'un popup de filtre via `f`.
- Application instantanee (ou au submit) sur la source des commits.
- Affichage d'un indicateur visuel quand un filtre est actif.

## Fichiers impactes (proposition)

- `src/state.rs` (etat des filtres)
- `src/app.rs` (actions + navigation)
- `src/event.rs` (raccourci `f`)
- `src/git/repo.rs` (log filtre)
- `src/ui/` (popup + badge filtre actif)

## Plan d'implementation

1. Ajouter une structure `GraphFilter` dans l'etat applicatif.
2. Ajouter les actions `OpenFilter`, `UpdateFilterField`, `ApplyFilter`, `ClearFilter`.
3. Introduire un composant UI de popup avec validation simple.
4. Adapter la recuperation des commits avec post-filtrage (ou filtrage git2 partiel + post-filtrage).
5. Recalculer proprement la selection apres application des filtres.
6. Afficher un badge dans la barre d'etat: `Filtre actif`.

## Tests

- Unitaires sur la logique de matching (`author`, `date range`, `path`, `message`).
- Test d'integration sur un repo de test avec plusieurs auteurs/dates.
- Test UX: ouverture popup, appliquer, reset.

## Criteres d'acceptation

- L'utilisateur peut filtrer et retirer les filtres sans redemarrer l'app.
- Le nombre de commits affiches diminue selon les criteres.
- Le curseur ne sort jamais des bornes apres un filtrage.
