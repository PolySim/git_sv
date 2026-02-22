use git2::Repository;
use std::collections::VecDeque;
use std::io::Write;

use crate::error::{GitSvError, Result};

/// Source d'une ligne dans le résultat résolu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineSource {
    /// Ligne de contexte (inchangée).
    Context,
    /// Ligne provenant de "ours".
    Ours,
    /// Ligne provenant de "theirs".
    Theirs,
    /// Marqueur de conflit non résolu.
    ConflictMarker,
}

/// Ligne résolue avec sa provenance.
#[derive(Debug, Clone)]
pub struct ResolvedLine {
    pub content: String,
    pub source: LineSource,
}

/// Mode de résolution des conflits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictResolutionMode {
    File,
    Block,
    Line,
}

/// Type de conflit sur un fichier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictType {
    /// Conflit classique : modifié des deux côtés.
    BothModified,
    /// Supprimé dans ours, modifié dans theirs.
    DeletedByUs,
    /// Modifié dans ours, supprimé dans theirs.
    DeletedByThem,
    /// Ajouté dans les deux branches avec des contenus différents.
    BothAdded,
}

/// Résolution par ligne dans une section (ancienne structure, gardée pour compatibilité).
#[derive(Debug, Clone, PartialEq)]
pub struct LineResolution {
    pub line_index: usize,
    pub source: ConflictResolution,
}

/// Résolution au niveau ligne - permet de choisir individuellement quelles lignes inclure.
#[derive(Debug, Clone, PartialEq)]
pub struct LineLevelResolution {
    /// true = cette ligne ours est dans le résultat
    pub ours_lines_included: Vec<bool>,
    /// true = cette ligne theirs est dans le résultat
    pub theirs_lines_included: Vec<bool>,
    /// Indique si l'utilisateur a modifié manuellement les sélections
    pub touched: bool,
}

impl LineLevelResolution {
    /// Crée une nouvelle résolution ligne par ligne avec toutes les lignes ours incluses par défaut.
    pub fn new(ours_count: usize, theirs_count: usize) -> Self {
        Self {
            ours_lines_included: vec![true; ours_count],
            theirs_lines_included: vec![false; theirs_count],
            touched: false,
        }
    }

    /// Vérifie si au moins une ligne est sélectionnée de chaque côté.
    pub fn has_selection(&self) -> bool {
        let has_ours = self.ours_lines_included.iter().any(|&b| b);
        let has_theirs = self.theirs_lines_included.iter().any(|&b| b);
        has_ours || has_theirs
    }
}

/// Résolution possible pour une section de conflit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictResolution {
    Ours,
    Theirs,
    Both,
}

/// Côté de résolution (déterminé par le panneau actif).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolutionSide {
    Ours,
    Theirs,
}

/// Section de conflit enrichie.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictSection {
    pub context_before: Vec<String>,
    pub ours: Vec<String>,
    pub theirs: Vec<String>,
    pub context_after: Vec<String>,
    pub resolution: Option<ConflictResolution>,
    pub line_resolutions: Vec<LineResolution>,
    /// Résolution au niveau ligne (mode Ligne)
    pub line_level_resolution: Option<LineLevelResolution>,
}

/// Un fichier en conflit.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictFile {
    pub path: String,
    pub conflicts: Vec<ConflictSection>,
    pub is_resolved: bool,
    pub conflict_type: ConflictType,
}

/// Fichier dans un merge (en conflit ou non).
#[derive(Debug, Clone)]
pub struct MergeFile {
    pub path: String,
    pub has_conflicts: bool,
    pub conflicts: Vec<ConflictSection>,
    pub is_resolved: bool,
    pub conflict_type: Option<ConflictType>, // None si pas de conflit
}

/// Résultat d'une opération de merge.
#[derive(Debug)]
pub enum MergeResult {
    Success,
    FastForward,
    UpToDate,
    Conflicts(Vec<ConflictFile>),
}

/// Parser les marqueurs de conflit dans un fichier.
pub fn parse_conflict_file(path: &str) -> Result<Vec<ConflictSection>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        GitSvError::Other(format!("Impossible de lire le fichier '{}': {}", path, e))
    })?;

    let mut sections = Vec::new();
    let mut lines = content.lines().peekable();
    let mut context_before: VecDeque<String> = VecDeque::new();

    while let Some(line) = lines.next() {
        if line.starts_with("<<<<<<<") {
            // Début d'une section de conflit
            let mut ours: Vec<String> = Vec::new();
            let mut theirs: Vec<String> = Vec::new();
            let mut in_ours = true;

            // Parser jusqu'au séparateur ou fin
            while let Some(line) = lines.next() {
                if line == "=======" {
                    in_ours = false;
                } else if line.starts_with(">>>>>>>") {
                    // Fin de la section de conflit
                    let context_after = collect_context_after(&mut lines, 3);

                    // Initialiser la résolution ligne par ligne avec ours inclus par défaut
                    let line_resolution = LineLevelResolution::new(ours.len(), theirs.len());

                    sections.push(ConflictSection {
                        context_before: context_before.iter().cloned().collect(),
                        ours,
                        theirs,
                        context_after,
                        resolution: None,
                        line_resolutions: Vec::new(),
                        line_level_resolution: Some(line_resolution),
                    });

                    context_before.clear();
                    break;
                } else if in_ours {
                    ours.push(line.to_string());
                } else {
                    theirs.push(line.to_string());
                }
            }
        } else {
            // Garder les lignes de contexte (max 3 avant le prochain conflit)
            context_before.push_back(line.to_string());
            if context_before.len() > 3 {
                context_before.pop_front();
            }
        }
    }

    Ok(sections)
}

/// Collecte les lignes de contexte après un conflit.
fn collect_context_after(
    lines: &mut std::iter::Peekable<std::str::Lines>,
    max: usize,
) -> Vec<String> {
    let mut context = Vec::new();
    for _ in 0..max {
        if let Some(line) = lines.peek() {
            if !line.starts_with("<<<<<<<") {
                context.push(line.to_string());
                lines.next();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    context
}

/// Lit le contenu d'un blob depuis l'index git (pour les fichiers supprimés localement).
fn read_blob_content(repo: &Repository, entry: &git2::IndexEntry) -> Result<Vec<String>> {
    let blob = repo
        .find_blob(entry.id)
        .map_err(|e| GitSvError::Other(format!("Impossible de trouver le blob: {}", e)))?;
    let content = std::str::from_utf8(blob.content())
        .map_err(|_| GitSvError::Other("Contenu du blob invalide".into()))?;
    Ok(content.lines().map(|l| l.to_string()).collect())
}

/// Lit les lignes d'un fichier local.
fn read_file_lines(path: &str) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        GitSvError::Other(format!("Impossible de lire le fichier '{}': {}", path, e))
    })?;
    Ok(content.lines().map(|l| l.to_string()).collect())
}

/// Liste tous les fichiers en conflit dans le repository.
pub fn list_conflict_files(repo: &Repository) -> Result<Vec<ConflictFile>> {
    let index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;

    let conflicts = index
        .conflicts()
        .map_err(|e| GitSvError::Other(format!("Impossible de lister les conflits: {}", e)))?;

    let mut files = Vec::new();

    for conflict in conflicts {
        let conflict = conflict
            .map_err(|e| GitSvError::Other(format!("Erreur lors du parsing du conflit: {}", e)))?;

        // Déterminer le type de conflit et le chemin
        let (path, conflict_type) = match (&conflict.our, &conflict.their, &conflict.ancestor) {
            // Cas classique : modifié des deux côtés
            (Some(ours), Some(_theirs), Some(_ancestor)) => {
                let p = std::str::from_utf8(&ours.path)
                    .map_err(|_| GitSvError::Other("Chemin de fichier invalide".into()))?
                    .to_string();
                (p, ConflictType::BothModified)
            }
            // Modifié dans ours, supprimé dans theirs
            (Some(ours), None, _) => {
                let p = std::str::from_utf8(&ours.path)
                    .map_err(|_| GitSvError::Other("Chemin de fichier invalide".into()))?
                    .to_string();
                (p, ConflictType::DeletedByThem)
            }
            // Supprimé dans ours, modifié dans theirs
            (None, Some(theirs), _) => {
                let p = std::str::from_utf8(&theirs.path)
                    .map_err(|_| GitSvError::Other("Chemin de fichier invalide".into()))?
                    .to_string();
                (p, ConflictType::DeletedByUs)
            }
            // Pas d'ancêtre commun, ajouté des deux côtés
            (Some(ours), Some(_theirs), None) => {
                let p = std::str::from_utf8(&ours.path)
                    .map_err(|_| GitSvError::Other("Chemin de fichier invalide".into()))?
                    .to_string();
                (p, ConflictType::BothAdded)
            }
            _ => continue, // Cas impossible en théorie
        };

        // Créer les sections selon le type de conflit
        let sections = match conflict_type {
            ConflictType::BothModified | ConflictType::BothAdded => {
                // Parser les marqueurs de conflit dans le fichier
                parse_conflict_file(&path)?
            }
            ConflictType::DeletedByUs => {
                // Le fichier n'existe pas en local (supprimé par nous), lire depuis theirs
                let theirs_content = if let Some(ref their_entry) = conflict.their {
                    read_blob_content(repo, their_entry)?
                } else {
                    vec![]
                };
                vec![ConflictSection {
                    context_before: vec![],
                    ours: vec![], // Supprimé
                    theirs: theirs_content.clone(),
                    context_after: vec![],
                    resolution: None,
                    line_resolutions: vec![],
                    line_level_resolution: Some(LineLevelResolution::new(0, theirs_content.len())),
                }]
            }
            ConflictType::DeletedByThem => {
                // Le fichier existe en local (nous l'avons gardé), theirs est vide
                let ours_content = read_file_lines(&path).unwrap_or_default();
                vec![ConflictSection {
                    context_before: vec![],
                    ours: ours_content.clone(),
                    theirs: vec![], // Supprimé
                    context_after: vec![],
                    resolution: None,
                    line_resolutions: vec![],
                    line_level_resolution: Some(LineLevelResolution::new(ours_content.len(), 0)),
                }]
            }
        };

        let is_resolved = sections.iter().all(|s| s.resolution.is_some());

        files.push(ConflictFile {
            path,
            conflicts: sections,
            is_resolved,
            conflict_type,
        });
    }

    Ok(files)
}

/// Vérifie si le repository a des conflits non résolus.
pub fn has_conflicts(repo: &Repository) -> Result<bool> {
    let index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;

    Ok(index.has_conflicts())
}

/// Récupère le nom court de la branche courante (HEAD).
/// Retourne le nom de la branche ou un fallback si HEAD est détaché.
pub fn get_current_branch_name(repo: &Repository) -> String {
    // Essayer d'obtenir la référence HEAD
    match repo.head() {
        Ok(head) => {
            // Si c'est une branche, retourner son nom court
            if let Some(name) = head.shorthand() {
                name.to_string()
            } else {
                // HEAD détaché - utiliser le SHA court
                head.target()
                    .map(|oid| format!("{:.7}", oid))
                    .unwrap_or_else(|| "HEAD".to_string())
            }
        }
        Err(_) => {
            // Pas de HEAD (repo vide) - fallback
            "HEAD".to_string()
        }
    }
}

/// Récupère le nom de la branche mergée depuis MERGE_HEAD ou un message d'opération.
/// Pour un merge : nom de la branche source.
/// Pour un cherry-pick : SHA court du commit.
/// Fallback : "MERGE_HEAD" si non disponible.
pub fn get_merge_branch_name(repo: &Repository, operation_msg: Option<&str>) -> String {
    // D'abord essayer de lire le message d'opération s'il contient le nom
    if let Some(msg) = operation_msg {
        // Extraire le nom de branche depuis des patterns comme "Merge de 'branch' dans 'other'"
        if let Some(start) = msg.find('\'') {
            if let Some(end) = msg[start + 1..].find('\'') {
                return msg[start + 1..start + 1 + end].to_string();
            }
        }
    }

    // Essayer de lire MERGE_HEAD depuis le filesystem
    let merge_head_path = repo.path().join("MERGE_HEAD");
    if let Ok(merge_head_content) = std::fs::read_to_string(&merge_head_path) {
        let merge_head_oid = merge_head_content.trim();
        if let Ok(oid) = git2::Oid::from_str(merge_head_oid) {
            // Chercher une branche qui pointe vers ce commit
            if let Ok(branches) = repo.branches(None) {
                for branch_result in branches {
                    if let Ok((branch, _)) = branch_result {
                        if let Some(target) = branch.get().target() {
                            if target == oid {
                                if let Some(name) = branch.name().ok().flatten() {
                                    return name.to_string();
                                }
                            }
                        }
                    }
                }
            }
            // Aucune branche trouvée - utiliser le SHA court
            return format!("{:.7}", oid);
        }
    }

    // Fallback final
    operation_msg
        .map(|s| s.to_string())
        .unwrap_or_else(|| "MERGE_HEAD".to_string())
}

/// Résout un fichier en appliquant les résolutions choisies.
pub fn resolve_file(repo: &Repository, file: &ConflictFile) -> Result<()> {
    let content = std::fs::read_to_string(&file.path).map_err(|e| {
        GitSvError::Other(format!(
            "Impossible de lire le fichier '{}': {}",
            file.path, e
        ))
    })?;

    let mut resolved_content = String::new();
    let mut conflict_idx = 0;
    let mut lines = content.lines();

    while let Some(line) = lines.next() {
        if line.starts_with("<<<<<<<") {
            // C'est une section de conflit
            let section = file
                .conflicts
                .get(conflict_idx)
                .ok_or_else(|| GitSvError::Other("Section de conflit non trouvée".into()))?;

            // Sauter toute la section de conflit
            while let Some(line) = lines.next() {
                if line.starts_with(">>>>>>>") {
                    break;
                }
            }

            // Appliquer la résolution
            match section.resolution {
                Some(ConflictResolution::Ours) => {
                    for l in &section.ours {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                }
                Some(ConflictResolution::Theirs) => {
                    for l in &section.theirs {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                }
                Some(ConflictResolution::Both) => {
                    for l in &section.ours {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                    for l in &section.theirs {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                }
                None => {
                    // Pas de résolution, garder le conflit tel quel
                    resolved_content.push_str(line);
                    resolved_content.push('\n');
                    for l in &section.ours {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                    resolved_content.push_str("=======\n");
                    for l in &section.theirs {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                    resolved_content.push_str(&format!(">>>>>>> {}\n", "HEAD"));
                }
            }

            conflict_idx += 1;
        } else {
            // Ligne normale
            resolved_content.push_str(line);
            resolved_content.push('\n');
        }
    }

    // Écrire le fichier résolu
    let mut file_handle = std::fs::File::create(&file.path).map_err(|e| {
        GitSvError::Other(format!(
            "Impossible d'écrire le fichier '{}': {}",
            file.path, e
        ))
    })?;
    file_handle
        .write_all(resolved_content.as_bytes())
        .map_err(|e| {
            GitSvError::Other(format!(
                "Erreur lors de l'écriture du fichier '{}': {}",
                file.path, e
            ))
        })?;

    // Mettre à jour l'index pour marquer le conflit comme résolu
    let mut index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;

    // Supprimer explicitement toutes les entrées pour ce chemin (stages 0, 1, 2, 3)
    // Cela nettoie les entrées de conflit (stages 1, 2, 3) si elles existent
    index.remove_path(std::path::Path::new(&file.path)).ok(); // Ignorer l'erreur si le chemin n'existe pas

    // Ajouter le fichier résolu à l'index (stage 0)
    index
        .add_path(std::path::Path::new(&file.path))
        .map_err(|e| {
            GitSvError::Other(format!("Impossible d'ajouter le fichier à l'index: {}", e))
        })?;
    index
        .write()
        .map_err(|e| GitSvError::Other(format!("Impossible d'écrire l'index: {}", e)))?;

    Ok(())
}

/// Annule le merge en cours (cleanup_state).
pub fn abort_merge(repo: &Repository) -> Result<()> {
    repo.cleanup_state()
        .map_err(|e| GitSvError::Other(format!("Impossible d'annuler le merge: {}", e)))?;

    // Réinitialiser les fichiers modifiés
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();
    repo.checkout_head(Some(&mut checkout_builder))
        .map_err(|e| GitSvError::Other(format!("Erreur lors du checkout: {}", e)))?;

    Ok(())
}

/// Finalise le merge en créant le commit de merge.
pub fn finalize_merge(repo: &Repository, message: &str) -> Result<()> {
    // Obtenir l'index pour vérification
    let mut index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;

    // Vérification détaillée : aucun conflit ne doit subsister dans l'index
    if index.has_conflicts() {
        // Lister les conflits restants pour un message d'erreur utile
        let remaining: Vec<String> = index
            .conflicts()
            .map_err(|e| GitSvError::Other(format!("Impossible de lister les conflits: {}", e)))?
            .filter_map(|c| c.ok())
            .filter_map(|c| {
                c.our
                    .or(c.their)
                    .or(c.ancestor)
                    .and_then(|e| String::from_utf8(e.path).ok())
            })
            .collect();

        return Err(GitSvError::Other(format!(
            "Des conflits non résolus subsistent dans l'index : {:?}. \
             Résolvez tous les fichiers avant de finaliser.",
            remaining
        )));
    }

    let signature = repo
        .signature()
        .map_err(|e| GitSvError::Other(format!("Impossible d'obtenir la signature: {}", e)))?;

    let head = repo
        .head()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à HEAD: {}", e)))?;
    let head_commit = head
        .peel_to_commit()
        .map_err(|e| GitSvError::Other(format!("Impossible de résoudre HEAD: {}", e)))?;

    // Écrire l'arbre
    let tree_id = index
        .write_tree()
        .map_err(|e| GitSvError::Other(format!("Impossible d'écrire l'arbre: {}", e)))?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|e| GitSvError::Other(format!("Impossible de trouver l'arbre: {}", e)))?;

    // Trouver les parents (HEAD + branche mergée si c'est un merge)
    // Vérifier s'il y a un MERGE_HEAD
    let merge_head_oid = std::fs::read_to_string(repo.path().join("MERGE_HEAD"))
        .ok()
        .and_then(|content| git2::Oid::from_str(content.trim()).ok());

    let merge_commit = merge_head_oid.and_then(|oid| repo.find_commit(oid).ok());

    // Créer le commit
    let _commit_oid = if let Some(ref merge_commit) = merge_commit {
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&head_commit, merge_commit],
        )
    } else {
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&head_commit],
        )
    }
    .map_err(|e| GitSvError::Other(format!("Impossible de créer le commit: {}", e)))?;

    // Nettoyer l'état de merge
    repo.cleanup_state()
        .map_err(|e| GitSvError::Other(format!("Impossible de nettoyer l'état de merge: {}", e)))?;

    Ok(())
}

/// Résout tous les conflits d'un fichier avec la même stratégie.
pub fn resolve_file_with_strategy(
    repo: &Repository,
    path: &str,
    strategy: ConflictResolution,
) -> Result<()> {
    let mut sections = parse_conflict_file(path)?;

    // Appliquer la stratégie à toutes les sections
    for section in &mut sections {
        section.resolution = Some(strategy);
    }

    let file = ConflictFile {
        path: path.to_string(),
        conflicts: sections,
        is_resolved: true,
        conflict_type: ConflictType::BothModified,
    };

    resolve_file(repo, &file)
}

/// Résout un fichier de conflit spécial (DeletedByUs, DeletedByThem, BothAdded).
/// Retourne true si le fichier doit être supprimé, false s'il doit être conservé.
pub fn resolve_special_file(
    repo: &Repository,
    file: &MergeFile,
    resolution: ConflictResolution,
) -> Result<bool> {
    use std::io::Write;

    let should_delete = match file.conflict_type {
        Some(ConflictType::DeletedByUs) => {
            // Supprimé chez nous, modifié chez eux
            // 'o' = garder la suppression (supprimer le fichier)
            // 't' ou 'b' = garder le fichier (version theirs)
            matches!(resolution, ConflictResolution::Ours)
        }
        Some(ConflictType::DeletedByThem) => {
            // Modifié chez nous, supprimé chez eux
            // 't' = garder la suppression (supprimer le fichier)
            // 'o' ou 'b' = garder le fichier (version ours)
            matches!(resolution, ConflictResolution::Theirs)
        }
        Some(ConflictType::BothAdded) => {
            // Ajouté des deux côtés
            // 'o' = version ours, 't' = version theirs, 'b' = les deux
            false // On écrit toujours un fichier, jamais de suppression
        }
        _ => {
            return Err(GitSvError::Other(
                "Type de conflit non supporté pour resolve_special_file".into(),
            ));
        }
    };

    let path = std::path::Path::new(&file.path);

    if should_delete {
        // Supprimer le fichier s'il existe
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| {
                GitSvError::Other(format!(
                    "Impossible de supprimer le fichier '{}': {}",
                    file.path, e
                ))
            })?;
        }

        // Supprimer les entrées de l'index (y compris les entrées de conflit stages 1, 2, 3)
        let mut index = repo
            .index()
            .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;

        // remove_path supprime toutes les entrées pour ce chemin (tous stages)
        index.remove_path(path).map_err(|e| {
            GitSvError::Other(format!(
                "Impossible de retirer le fichier de l'index: {}",
                e
            ))
        })?;
        index
            .write()
            .map_err(|e| GitSvError::Other(format!("Impossible d'écrire l'index: {}", e)))?;

        Ok(true)
    } else {
        // Garder le fichier - déterminer le contenu
        let content = match file.conflict_type {
            Some(ConflictType::DeletedByUs) => {
                // Garder la version theirs
                file.conflicts
                    .first()
                    .map(|s| s.theirs.join("\n"))
                    .unwrap_or_default()
            }
            Some(ConflictType::DeletedByThem) => {
                // Garder la version ours
                file.conflicts
                    .first()
                    .map(|s| s.ours.join("\n"))
                    .unwrap_or_default()
            }
            Some(ConflictType::BothAdded) => {
                // Selon la résolution choisie
                let section = file.conflicts.first();
                match resolution {
                    ConflictResolution::Ours => {
                        section.map(|s| s.ours.join("\n")).unwrap_or_default()
                    }
                    ConflictResolution::Theirs => {
                        section.map(|s| s.theirs.join("\n")).unwrap_or_default()
                    }
                    ConflictResolution::Both => section
                        .map(|s| {
                            let mut result = s.ours.clone();
                            result.extend(s.theirs.clone());
                            result.join("\n")
                        })
                        .unwrap_or_default(),
                }
            }
            _ => String::new(),
        };

        // Écrire le fichier
        let mut file_handle = std::fs::File::create(path).map_err(|e| {
            GitSvError::Other(format!(
                "Impossible de créer le fichier '{}': {}",
                file.path, e
            ))
        })?;
        file_handle.write_all(content.as_bytes()).map_err(|e| {
            GitSvError::Other(format!(
                "Erreur lors de l'écriture du fichier '{}': {}",
                file.path, e
            ))
        })?;

        // Mettre à jour l'index
        let mut index = repo
            .index()
            .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;

        // Supprimer explicitement toutes les entrées pour ce chemin (stages 0, 1, 2, 3)
        // Cela nettoie les entrées de conflit si elles existent
        index.remove_path(path).ok(); // Ignorer l'erreur si le chemin n'existe pas

        // Ajouter le fichier résolu à l'index (stage 0)
        index.add_path(path).map_err(|e| {
            GitSvError::Other(format!("Impossible d'ajouter le fichier à l'index: {}", e))
        })?;
        index
            .write()
            .map_err(|e| GitSvError::Other(format!("Impossible d'écrire l'index: {}", e)))?;

        Ok(false)
    }
}

/// Compte le nombre de fichiers en conflit non résolus.
pub fn count_unresolved_files(files: &[ConflictFile]) -> usize {
    files.iter().filter(|f| !f.is_resolved).count()
}

/// Compte le nombre total de sections de conflit non résolues.
pub fn count_unresolved_sections(files: &[ConflictFile]) -> usize {
    files
        .iter()
        .flat_map(|f| &f.conflicts)
        .filter(|s| s.resolution.is_none())
        .count()
}

/// Met à jour le statut resolved d'un fichier basé sur ses sections.
pub fn update_file_resolved_status(file: &mut ConflictFile) {
    file.is_resolved = file.conflicts.iter().all(|s| s.resolution.is_some());
}

/// Liste tous les fichiers du merge (en conflit ou non).
pub fn list_all_merge_files(repo: &Repository) -> Result<Vec<MergeFile>> {
    let index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;

    let mut all_files: Vec<MergeFile> = Vec::new();

    // Collecter les informations de conflit avec leur type
    let mut conflict_map: std::collections::HashMap<String, ConflictType> =
        std::collections::HashMap::new();

    if let Ok(conflicts) = index.conflicts() {
        for conflict in conflicts.filter_map(|c| c.ok()) {
            let (path, conflict_type) = match (&conflict.our, &conflict.their, &conflict.ancestor) {
                // Cas classique : modifié des deux côtés
                (Some(ours), Some(_theirs), Some(_ancestor)) => {
                    if let Ok(p) = std::str::from_utf8(&ours.path) {
                        (p.to_string(), ConflictType::BothModified)
                    } else {
                        continue;
                    }
                }
                // Modifié dans ours, supprimé dans theirs
                (Some(ours), None, _) => {
                    if let Ok(p) = std::str::from_utf8(&ours.path) {
                        (p.to_string(), ConflictType::DeletedByThem)
                    } else {
                        continue;
                    }
                }
                // Supprimé dans ours, modifié dans theirs
                (None, Some(theirs), _) => {
                    if let Ok(p) = std::str::from_utf8(&theirs.path) {
                        (p.to_string(), ConflictType::DeletedByUs)
                    } else {
                        continue;
                    }
                }
                // Pas d'ancêtre commun, ajouté des deux côtés
                (Some(ours), Some(_theirs), None) => {
                    if let Ok(p) = std::str::from_utf8(&ours.path) {
                        (p.to_string(), ConflictType::BothAdded)
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };
            conflict_map.insert(path, conflict_type);
        }
    }

    // Parcourir tous les fichiers de l'index
    for i in 0..index.len() {
        if let Some(entry) = index.get(i) {
            let path_bytes = entry.path;
            if let Ok(path) = std::str::from_utf8(&path_bytes) {
                if let Some(&conflict_type) = conflict_map.get(path) {
                    // Fichier en conflit - créer les sections selon le type
                    let sections = match conflict_type {
                        ConflictType::BothModified | ConflictType::BothAdded => {
                            parse_conflict_file(path).unwrap_or_default()
                        }
                        ConflictType::DeletedByUs => {
                            // Lire le contenu depuis l'index (theirs)
                            let theirs_content = if let Ok(conflicts) = index.conflicts() {
                                conflicts
                                    .filter_map(|c| c.ok())
                                    .find(|c| {
                                        c.their.as_ref().map_or(false, |t| {
                                            std::str::from_utf8(&t.path).ok() == Some(path)
                                        })
                                    })
                                    .and_then(|c| c.their)
                                    .and_then(|entry| read_blob_content(repo, &entry).ok())
                                    .unwrap_or_default()
                            } else {
                                vec![]
                            };
                            vec![ConflictSection {
                                context_before: vec![],
                                ours: vec![],
                                theirs: theirs_content.clone(),
                                context_after: vec![],
                                resolution: None,
                                line_resolutions: vec![],
                                line_level_resolution: Some(LineLevelResolution::new(
                                    0,
                                    theirs_content.len(),
                                )),
                            }]
                        }
                        ConflictType::DeletedByThem => {
                            // Lire le contenu local (ours)
                            let ours_content = read_file_lines(path).unwrap_or_default();
                            vec![ConflictSection {
                                context_before: vec![],
                                ours: ours_content.clone(),
                                theirs: vec![],
                                context_after: vec![],
                                resolution: None,
                                line_resolutions: vec![],
                                line_level_resolution: Some(LineLevelResolution::new(
                                    ours_content.len(),
                                    0,
                                )),
                            }]
                        }
                    };

                    let is_resolved = sections.iter().all(|s| s.resolution.is_some());

                    all_files.push(MergeFile {
                        path: path.to_string(),
                        has_conflicts: true,
                        conflicts: sections,
                        is_resolved,
                        conflict_type: Some(conflict_type),
                    });
                } else {
                    // Fichier sans conflit
                    all_files.push(MergeFile {
                        path: path.to_string(),
                        has_conflicts: false,
                        conflicts: Vec::new(),
                        is_resolved: true,
                        conflict_type: None,
                    });
                }
            }
        }
    }

    // Ajouter les fichiers en conflit qui ne sont pas dans l'index (cas DeletedByUs)
    for (path, conflict_type) in &conflict_map {
        if !all_files.iter().any(|f| &f.path == path) {
            // Fichier supprimé chez nous mais modifié chez eux
            let sections = match conflict_type {
                ConflictType::DeletedByUs => {
                    // Lire le contenu depuis l'index (theirs)
                    let theirs_content = if let Ok(conflicts) = index.conflicts() {
                        conflicts
                            .filter_map(|c| c.ok())
                            .find(|c| {
                                c.their.as_ref().map_or(false, |t| {
                                    std::str::from_utf8(&t.path).ok().map(|s| s.to_string())
                                        == Some(path.clone())
                                })
                            })
                            .and_then(|c| c.their)
                            .and_then(|entry| read_blob_content(repo, &entry).ok())
                            .unwrap_or_default()
                    } else {
                        vec![]
                    };
                    vec![ConflictSection {
                        context_before: vec![],
                        ours: vec![],
                        theirs: theirs_content.clone(),
                        context_after: vec![],
                        resolution: None,
                        line_resolutions: vec![],
                        line_level_resolution: Some(LineLevelResolution::new(
                            0,
                            theirs_content.len(),
                        )),
                    }]
                }
                _ => vec![],
            };

            let is_resolved = sections.iter().all(|s| s.resolution.is_some());

            all_files.push(MergeFile {
                path: path.clone(),
                has_conflicts: !sections.is_empty(),
                conflicts: sections,
                is_resolved,
                conflict_type: Some(*conflict_type),
            });
        }
    }

    // Ajouter les fichiers non indexés (nouveaux fichiers non trackés)
    if let Ok(status) = repo.statuses(None) {
        for entry in status.iter() {
            if let Some(path) = entry.path() {
                if !all_files.iter().any(|f| f.path == path) {
                    let status = entry.status();
                    if status.is_wt_new() || status.is_index_new() {
                        all_files.push(MergeFile {
                            path: path.to_string(),
                            has_conflicts: false,
                            conflicts: Vec::new(),
                            is_resolved: true,
                            conflict_type: None,
                        });
                    }
                }
            }
        }
    }

    Ok(all_files)
}

/// Compte le nombre de fichiers en conflit non résolus dans MergeFile.
pub fn count_unresolved_merge_files(files: &[MergeFile]) -> usize {
    files
        .iter()
        .filter(|f| f.has_conflicts && !f.is_resolved)
        .count()
}

/// Génère le contenu résolu d'un fichier en fonction des résolutions.
pub fn generate_resolved_content(file: &MergeFile, mode: ConflictResolutionMode) -> Vec<String> {
    // Wrapper autour de la nouvelle fonction
    generate_resolved_content_with_source(file, mode)
        .into_iter()
        .map(|line| line.content)
        .collect()
}

/// Génère le contenu résolu avec provenance de chaque ligne.
pub fn generate_resolved_content_with_source(
    file: &MergeFile,
    mode: ConflictResolutionMode,
) -> Vec<ResolvedLine> {
    let mut result: Vec<ResolvedLine> = Vec::new();

    for section in &file.conflicts {
        // Contexte avant → LineSource::Context
        for line in &section.context_before {
            result.push(ResolvedLine {
                content: line.clone(),
                source: LineSource::Context,
            });
        }

        match mode {
            ConflictResolutionMode::File | ConflictResolutionMode::Block => {
                if let Some(resolution) = &section.resolution {
                    match resolution {
                        ConflictResolution::Ours => {
                            for line in &section.ours {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Ours,
                                });
                            }
                        }
                        ConflictResolution::Theirs => {
                            for line in &section.theirs {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Theirs,
                                });
                            }
                        }
                        ConflictResolution::Both => {
                            for line in &section.ours {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Ours,
                                });
                            }
                            for line in &section.theirs {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Theirs,
                                });
                            }
                        }
                    }
                } else {
                    // Non résolu → marqueurs de conflit
                    result.push(ResolvedLine {
                        content: "<<<<<<< HEAD".into(),
                        source: LineSource::ConflictMarker,
                    });
                    for line in &section.ours {
                        result.push(ResolvedLine {
                            content: line.clone(),
                            source: LineSource::Ours,
                        });
                    }
                    result.push(ResolvedLine {
                        content: "=======".into(),
                        source: LineSource::ConflictMarker,
                    });
                    for line in &section.theirs {
                        result.push(ResolvedLine {
                            content: line.clone(),
                            source: LineSource::Theirs,
                        });
                    }
                    result.push(ResolvedLine {
                        content: ">>>>>>>".into(),
                        source: LineSource::ConflictMarker,
                    });
                }
            }
            ConflictResolutionMode::Line => {
                // Résolution ligne par ligne avec line_level_resolution
                if let Some(ref lr) = section.line_level_resolution {
                    // Inclure les lignes ours marquées comme incluses
                    for (i, line) in section.ours.iter().enumerate() {
                        if lr.ours_lines_included.get(i) == Some(&true) {
                            result.push(ResolvedLine {
                                content: line.clone(),
                                source: LineSource::Ours,
                            });
                        }
                    }
                    // Puis les lignes theirs marquées comme incluses
                    for (i, line) in section.theirs.iter().enumerate() {
                        if lr.theirs_lines_included.get(i) == Some(&true) {
                            result.push(ResolvedLine {
                                content: line.clone(),
                                source: LineSource::Theirs,
                            });
                        }
                    }
                } else {
                    // Fallback : garder toutes les lignes ours par défaut
                    for line in &section.ours {
                        result.push(ResolvedLine {
                            content: line.clone(),
                            source: LineSource::Ours,
                        });
                    }
                }
            }
        }

        // Contexte après → LineSource::Context
        for line in &section.context_after {
            result.push(ResolvedLine {
                content: line.clone(),
                source: LineSource::Context,
            });
        }
    }

    result
}
