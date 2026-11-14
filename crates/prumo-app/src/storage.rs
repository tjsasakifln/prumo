//! Host-owned persistence paths and recovery policy. Frontends only receive IDs.

use super::{AccessMode, AppError, MAX_PROJECT_BYTES, ProjectService, reject_non_regular};
use prumo_domain::storage::{
    CandidateKind, PackageEntrySummary, PackageSummary, StorageAction, StorageActionResult,
    StorageCandidate, StorageDiagnostic, StorageFormat, StorageState, StorageStatus,
};
use prumo_domain::{PROJECT_SCHEMA_VERSION, Project, canonical::hash_bytes};
use prumo_storage::{
    CommitPhase, CommitRecord, DecodedPackage, EncodedPackage, MAX_DATABASE_BYTES,
    MAX_PACKAGE_BYTES, PackageOrigin, SQLiteWorkingStore, decode_package, encode_validated_package,
    restore_database,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAX_ORIGINALS: usize = 32;

pub(super) struct LoadedFile {
    pub project: Project,
    raw: Vec<u8>,
    package: Option<DecodedPackage>,
}

impl LoadedFile {
    pub(super) fn sha256(&self) -> String {
        hash_bytes(&self.raw)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SavePhase {
    ObjectsInserted,
    RevisionInserted,
    HeadUpdated,
    BeforeCommit,
    DatabaseCommitted,
    AutosavePublished,
    BackupPublished,
    PrimaryPublished,
    RecoveryPreserved,
    RecoveryAutosavePublished,
    RecoveryDatabaseRestored,
    RecoveryRecorded,
    RecoveryPrimaryPublished,
}

struct WorkPaths {
    root: PathBuf,
    database: PathBuf,
    autosave: PathBuf,
    backup: PathBuf,
}

#[derive(Clone)]
enum CandidateSource {
    File(PathBuf),
    Database,
}

struct Inspection {
    status: StorageStatus,
    sources: BTreeMap<String, CandidateSource>,
}

impl ProjectService {
    pub fn storage_status(&self) -> Result<StorageStatus, AppError> {
        Ok(self.inspect_storage()?.status)
    }

    pub fn execute_storage(&self, action: StorageAction) -> Result<StorageActionResult, AppError> {
        self.require_write()?;
        let _lock = self.lock()?;
        let inspection = self.inspect_storage()?;
        let supplied = match &action {
            StorageAction::Migrate {
                inspection_sha256, ..
            }
            | StorageAction::Backup {
                inspection_sha256, ..
            }
            | StorageAction::Recover {
                inspection_sha256, ..
            } => inspection_sha256,
        };
        if supplied != &inspection.status.inspection_sha256 {
            return Err(AppError::StorageConflict);
        }
        if inspection.status.state == StorageState::Unsupported {
            return Err(AppError::RecoveryRequired);
        }
        let (project, changed, message) = match action {
            StorageAction::Migrate {
                expected_revision,
                confirm,
                ..
            } => {
                if !confirm {
                    return Err(AppError::ConfirmationRequired);
                }
                let original = self.load_primary()?;
                check_revision(&original.project, expected_revision)?;
                if inspection.status.state == StorageState::RecoveryAvailable {
                    return Err(AppError::RecoveryRequired);
                }
                if original.package.is_some() {
                    (
                        original.project,
                        false,
                        "O arquivo já usa o pacote atual.".into(),
                    )
                } else {
                    self.preserve_original(&original.raw)?;
                    let store = self.rebuild_from(&original)?;
                    let bytes = encode(&store)?;
                    self.publish_save(&bytes, Some(&original.raw))?;
                    (
                        original.project,
                        true,
                        "Migração concluída; o original foi preservado.".into(),
                    )
                }
            }
            StorageAction::Backup {
                expected_revision, ..
            } => {
                let original = self.load_primary()?;
                check_revision(&original.project, expected_revision)?;
                let paths = self.ensure_work_paths()?;
                let changed = read_optional(&paths.backup, MAX_PACKAGE_BYTES)?
                    .is_none_or(|bytes| bytes != original.raw);
                if changed {
                    atomic_write(&paths.backup, &original.raw)?;
                }
                // Verify the published backup through the same bounded reader.
                let backup = load_file(&paths.backup)?;
                if backup.raw != original.raw || backup.project != original.project {
                    return Err(AppError::InvalidProject(
                        "O backup diverge do projeto.".into(),
                    ));
                }
                (original.project, changed, "Backup verificado.".into())
            }
            StorageAction::Recover {
                candidate_id,
                confirm,
                ..
            } => {
                if !confirm {
                    return Err(AppError::ConfirmationRequired);
                }
                let candidate = inspection
                    .status
                    .candidates
                    .iter()
                    .find(|candidate| candidate.id == candidate_id && candidate.recoverable)
                    .ok_or(AppError::StorageConflict)?;
                let source = inspection
                    .sources
                    .get(&candidate_id)
                    .ok_or(AppError::StorageConflict)?;
                let bytes = match source {
                    CandidateSource::File(path) => load_file(path)?.raw,
                    CandidateSource::Database => {
                        let store =
                            SQLiteWorkingStore::open_read_only(&self.work_paths().database)?;
                        encode(&store)?.into_bytes()
                    }
                };
                let selected = decode_file(bytes)?;
                let selected_hash = match source {
                    CandidateSource::File(_) => hash_bytes(&selected.raw),
                    CandidateSource::Database => hash_bytes(
                        &selected
                            .package
                            .as_ref()
                            .ok_or(AppError::StorageConflict)?
                            .database_bytes,
                    ),
                };
                if selected_hash != candidate.sha256
                    || selected.project.id != candidate.project_id
                    || selected.project.revision != candidate.revision
                {
                    return Err(AppError::StorageConflict);
                }
                // Keep a byte-exact incident copy before replacing a damaged or
                // older primary, and keep the chosen source before any DB move.
                if let Some(bytes) = read_optional(&self.path, MAX_PACKAGE_BYTES)? {
                    self.preserve_incident(&bytes)?;
                }
                self.checkpoint(SavePhase::RecoveryPreserved);
                let package_bytes = if selected.package.is_some() {
                    selected.raw.clone()
                } else {
                    self.preserve_original(&selected.raw)?;
                    let temp = tempfile::tempdir()?;
                    let mut store = SQLiteWorkingStore::open(&temp.path().join("state.sqlite"))?;
                    store.commit(
                        None,
                        &selected.project,
                        self.baseline_record("storage.import")?,
                    )?;
                    encode(&store)?.into_bytes()
                };
                let paths = self.ensure_work_paths()?;
                atomic_write(&paths.autosave, &package_bytes)?;
                self.checkpoint(SavePhase::RecoveryAutosavePublished);
                let selected_package = decode_package(&package_bytes)?;
                self.quarantine_database()?;
                restore_database(&paths.database, &selected_package.database_bytes)?;
                self.checkpoint(SavePhase::RecoveryDatabaseRestored);
                let mut store = SQLiteWorkingStore::open(&paths.database)?;
                let mut record = self.baseline_record("storage.recover")?;
                record.command = Some(
                    json!({"operation":"storage.recover", "candidate_id":candidate_id,
                    "source_sha256":candidate.sha256, "inspection_sha256":inspection.status.inspection_sha256}),
                );
                store.commit(Some(selected.project.revision), &selected.project, record)?;
                self.checkpoint(SavePhase::RecoveryRecorded);
                let recovered = encode(&store)?;
                drop(store);
                atomic_write(&paths.autosave, recovered.as_bytes())?;
                atomic_write(&self.path, recovered.as_bytes())?;
                self.checkpoint(SavePhase::RecoveryPrimaryPublished);
                (
                    selected.project,
                    true,
                    "Revisão recuperada; origem e histórico preservados.".into(),
                )
            }
        };
        Ok(StorageActionResult {
            project,
            status: self.inspect_storage()?.status,
            changed,
            message,
        })
    }

    pub(super) fn load_primary(&self) -> Result<LoadedFile, AppError> {
        load_file(&self.path)
    }

    pub(super) fn check_write_basis(&self, original: &LoadedFile) -> Result<(), AppError> {
        if let Some((project_id, hash)) = &self.write_basis
            && (*project_id != original.project.id || hash != &hash_bytes(&original.raw))
        {
            return Err(AppError::StorageConflict);
        }
        Ok(())
    }

    pub(super) fn save_new_locked(
        &self,
        project: &Project,
        command: Value,
    ) -> Result<(), AppError> {
        check_project_size(project)?;
        let status = self.inspect_storage()?.status;
        if !status.candidates.is_empty() || status.state == StorageState::Unsupported {
            return Err(AppError::RecoveryRequired);
        }
        let paths = self.ensure_work_paths()?;
        self.quarantine_database()?;
        let mut store = SQLiteWorkingStore::open(&paths.database)?;
        self.commit(&mut store, None, project, command)?;
        let bytes = encode(&store)?;
        drop(store);
        self.publish_save(&bytes, None)
    }

    pub(super) fn save_changed_locked(
        &self,
        original: &LoadedFile,
        project: &Project,
        command: Value,
    ) -> Result<(), AppError> {
        check_project_size(project)?;
        let status = self.inspect_storage_with_primary(Some(original))?.status;
        if status.primary_sha256.as_deref() != Some(hash_bytes(&original.raw).as_str()) {
            return Err(AppError::StorageConflict);
        }
        if status.state == StorageState::Unsupported {
            return Err(AppError::RecoveryRequired);
        }
        let mut store = if let Some(package) = &original.package {
            let paths = self.ensure_work_paths()?;
            if paths.database.try_exists()? {
                match SQLiteWorkingStore::open_with_project(&paths.database) {
                    Ok((store, working)) => match working {
                        Some(working) if working == original.project => {
                            if store.backup_bytes()? != package.database_bytes {
                                return Err(AppError::RecoveryRequired);
                            }
                            store
                        }
                        Some(working)
                            if working.id == original.project.id
                                && working.revision >= original.project.revision =>
                        {
                            return Err(AppError::RecoveryRequired);
                        }
                        _ => {
                            drop(store);
                            self.rebuild_from(original)?
                        }
                    },
                    Err(_) => return Err(AppError::RecoveryRequired),
                }
            } else {
                self.rebuild_from(original)?
            }
        } else {
            if status.state == StorageState::RecoveryAvailable {
                return Err(AppError::RecoveryRequired);
            }
            self.preserve_original(&original.raw)?;
            self.rebuild_from(original)?
        };
        self.commit(
            &mut store,
            Some(original.project.revision),
            project,
            command,
        )?;
        let bytes = encode(&store)?;
        drop(store);
        self.publish_save(&bytes, Some(&original.raw))
    }

    fn commit(
        &self,
        store: &mut SQLiteWorkingStore,
        expected: Option<u64>,
        project: &Project,
        command: Value,
    ) -> Result<(), AppError> {
        let last = project
            .history
            .last()
            .ok_or_else(|| AppError::InvalidProject("Histórico ausente.".into()))?;
        let record = CommitRecord {
            command_id: Uuid::new_v4(),
            actor: self.actor.clone(),
            operation: last.operation.clone(),
            reason: None,
            timestamp_unix_ms: last.timestamp_unix_ms,
            command: Some(command),
        };
        store.commit_with_hook(expected, project, record, |phase| {
            self.checkpoint(match phase {
                CommitPhase::ObjectsInserted => SavePhase::ObjectsInserted,
                CommitPhase::RevisionInserted => SavePhase::RevisionInserted,
                CommitPhase::HeadUpdated => SavePhase::HeadUpdated,
                CommitPhase::BeforeCommit => SavePhase::BeforeCommit,
            });
            Ok(())
        })?;
        self.checkpoint(SavePhase::DatabaseCommitted);
        Ok(())
    }

    fn publish_save(
        &self,
        package: &EncodedPackage,
        previous: Option<&[u8]>,
    ) -> Result<(), AppError> {
        // Only the storage encoder can construct this verified native output.
        // Received bytes always take the complete decoder path instead.
        let bytes = package.as_bytes();
        let paths = self.ensure_work_paths()?;
        atomic_write(&paths.autosave, bytes)?;
        self.checkpoint(SavePhase::AutosavePublished);
        if let Some(previous) = previous {
            atomic_write(&paths.backup, previous)?;
        }
        self.checkpoint(SavePhase::BackupPublished);
        atomic_write(&self.path, bytes)?;
        self.checkpoint(SavePhase::PrimaryPublished);
        Ok(())
    }

    fn rebuild_from(&self, original: &LoadedFile) -> Result<SQLiteWorkingStore, AppError> {
        let paths = self.ensure_work_paths()?;
        self.quarantine_database()?;
        if let Some(package) = &original.package {
            restore_database(&paths.database, &package.database_bytes)?;
            Ok(SQLiteWorkingStore::open(&paths.database)?)
        } else {
            let mut store = SQLiteWorkingStore::open(&paths.database)?;
            store.commit(
                None,
                &original.project,
                self.baseline_record("storage.import")?,
            )?;
            Ok(store)
        }
    }

    fn baseline_record(&self, operation: &str) -> Result<CommitRecord, AppError> {
        Ok(CommitRecord {
            command_id: Uuid::new_v4(),
            actor: self.actor.clone(),
            operation: operation.into(),
            reason: Some("Preservação de um estado existente, sem execução de cálculo.".into()),
            timestamp_unix_ms: self.change(1, operation)?.timestamp_unix_ms,
            command: None,
        })
    }

    fn work_paths(&self) -> WorkPaths {
        let name = self.path.file_name().expect("validated project filename");
        let root = self
            .path
            .with_file_name(format!(".prumo-{}", hash_bytes(name.as_encoded_bytes())));
        WorkPaths {
            database: root.join("state.sqlite"),
            autosave: root.join("autosave.prumo"),
            backup: root.join("backup.prumo"),
            root,
        }
    }

    fn ensure_work_paths(&self) -> Result<WorkPaths, AppError> {
        let paths = self.work_paths();
        check_directory(&paths.root)?;
        if !paths.root.try_exists()? {
            fs::create_dir(&paths.root)?;
            sync_parent(&paths.root)?;
        }
        Ok(paths)
    }

    fn quarantine_database(&self) -> Result<(), AppError> {
        let paths = self.ensure_work_paths()?;
        let sources: Vec<_> = [
            "state.sqlite",
            "state.sqlite-journal",
            "state.sqlite-wal",
            "state.sqlite-shm",
        ]
        .into_iter()
        .map(|name| paths.root.join(name))
        .collect();
        for source in &sources {
            reject_non_regular(source)?;
        }
        if !sources.iter().any(|path| path.exists()) {
            return Ok(());
        }
        let retired = paths.root.join(format!("retired-{}", Uuid::new_v4()));
        fs::create_dir(&retired)?;
        for source in sources {
            if source.try_exists()? {
                fs::rename(
                    &source,
                    retired.join(source.file_name().expect("fixed filename")),
                )?;
            }
        }
        sync_parent(&retired)?;
        Ok(())
    }

    fn preserve_original(&self, bytes: &[u8]) -> Result<(), AppError> {
        let paths = self.ensure_work_paths()?;
        let hash = hash_bytes(bytes);
        immutable_write(&paths.root.join(format!("original-{hash}.prumo")), bytes)?;
        // Keep the PRU-004 byte-exact legacy backup contract on first v1 write.
        if serde_json::from_slice::<Value>(bytes)
            .ok()
            .and_then(|value| value.get("schema_version").and_then(Value::as_u64))
            == Some(1)
        {
            immutable_write(
                &self.path.with_file_name(format!("prumo-v1-{hash}.backup")),
                bytes,
            )?;
        }
        Ok(())
    }

    fn preserve_incident(&self, bytes: &[u8]) -> Result<(), AppError> {
        let paths = self.ensure_work_paths()?;
        immutable_write(
            &paths
                .root
                .join(format!("incident-{}.bytes", hash_bytes(bytes))),
            bytes,
        )
    }

    fn inspect_storage(&self) -> Result<Inspection, AppError> {
        self.inspect_storage_with_primary(None)
    }

    fn inspect_storage_with_primary(
        &self,
        known: Option<&LoadedFile>,
    ) -> Result<Inspection, AppError> {
        let mut diagnostics = Vec::new();
        let mut fingerprints = BTreeMap::<String, String>::new();
        let primary_bytes = read_optional(&self.path, MAX_PACKAGE_BYTES)?;
        let primary_sha256 = primary_bytes.as_ref().map(|bytes| hash_bytes(bytes));
        fingerprints.insert(
            "primary".into(),
            primary_sha256.clone().unwrap_or_else(|| "missing".into()),
        );
        let reusable =
            known.filter(|file| primary_sha256.as_deref() == Some(hash_bytes(&file.raw).as_str()));
        let (decoded_primary, mut state, format) = if let Some(file) = reusable {
            (
                None,
                if file.package.is_some() {
                    StorageState::Ready
                } else {
                    StorageState::Legacy
                },
                file_format(&file.raw),
            )
        } else {
            match primary_bytes {
                None => (None, StorageState::Missing, StorageFormat::Missing),
                Some(bytes) => {
                    let format = file_format(&bytes);
                    match decode_file(bytes) {
                        Ok(file) => {
                            let state = if file.package.is_some() {
                                StorageState::Ready
                            } else {
                                StorageState::Legacy
                            };
                            (Some(file), state, format)
                        }
                        Err(error) => {
                            let state = if is_unsupported(&error) {
                                StorageState::Unsupported
                            } else {
                                StorageState::Invalid
                            };
                            diagnostics.push(diagnostic(error.code(), error.to_string()));
                            (None, state, format)
                        }
                    }
                }
            }
        };
        let primary = reusable.or(decoded_primary.as_ref());
        let mut sources = BTreeMap::new();
        let mut candidates = Vec::new();
        if let Some(primary) = &primary {
            add_candidate(
                &mut candidates,
                &mut sources,
                CandidateKind::Original,
                "Revisão publicada",
                &primary.project,
                hash_bytes(&primary.raw),
                CandidateSource::File(self.path.clone()),
            );
        }
        let paths = self.work_paths();
        if let Err(error) = check_directory(&paths.root) {
            diagnostics.push(diagnostic("unsafe_working_directory", error.to_string()));
            fingerprints.insert("working_directory".into(), "unsafe".into());
        } else if paths.root.try_exists()? {
            let mut files = vec![
                (
                    paths.autosave.clone(),
                    CandidateKind::Autosave,
                    "Salvamento automático",
                ),
                (
                    paths.backup.clone(),
                    CandidateKind::Backup,
                    "Backup anterior",
                ),
            ];
            let mut originals = Vec::new();
            for entry in fs::read_dir(&paths.root)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("original-") && name.ends_with(".prumo") {
                    originals.push(entry.path());
                    if originals.len() > MAX_ORIGINALS {
                        break;
                    }
                }
            }
            originals.sort();
            if originals.len() > MAX_ORIGINALS {
                diagnostics.push(diagnostic("original_limit", "Há mais de 32 originais preservados; consulte os arquivos com suporte técnico."));
                originals.truncate(MAX_ORIGINALS);
            }
            files.extend(
                originals
                    .into_iter()
                    .map(|path| (path, CandidateKind::Original, "Original de migração")),
            );
            for (path, kind, label) in files {
                let key = path
                    .file_name()
                    .expect("fixed filename")
                    .to_string_lossy()
                    .into_owned();
                match read_optional(&path, MAX_PACKAGE_BYTES) {
                    Ok(Some(bytes)) => {
                        let hash = hash_bytes(&bytes);
                        fingerprints.insert(key.clone(), hash.clone());
                        // Identical bytes have already passed the complete reader.
                        // Reuse that validated value within this inspection only.
                        if let Some(primary) = &primary
                            && primary_sha256.as_ref() == Some(&hash)
                        {
                            add_candidate(
                                &mut candidates,
                                &mut sources,
                                kind,
                                label,
                                &primary.project,
                                hash,
                                CandidateSource::File(path),
                            );
                            continue;
                        }
                        match decode_file(bytes) {
                            Ok(file) => add_candidate(
                                &mut candidates,
                                &mut sources,
                                kind,
                                label,
                                &file.project,
                                hash,
                                CandidateSource::File(path),
                            ),
                            Err(error) => {
                                if is_unsupported(&error) {
                                    state = StorageState::Unsupported;
                                }
                                diagnostics.push(diagnostic(
                                    "invalid_candidate",
                                    format!("{label}: {error}"),
                                ));
                            }
                        }
                    }
                    Ok(None) => {
                        fingerprints.insert(key, "missing".into());
                    }
                    Err(error) => {
                        fingerprints.insert(key, error.to_string());
                        diagnostics
                            .push(diagnostic("invalid_candidate", format!("{label}: {error}")));
                    }
                }
            }
            match read_optional(&paths.database, MAX_DATABASE_BYTES) {
                Ok(Some(bytes)) => {
                    fingerprints.insert("database".into(), hash_bytes(&bytes));
                    match inspect_working_database(&paths.database, primary) {
                        Ok((Some(project), hash)) => {
                            if primary.as_ref().is_some_and(|primary| {
                                primary.project.id == project.id
                                    && (primary.project.revision < project.revision
                                        || (primary.project.revision == project.revision
                                            && primary.package.as_ref().is_some_and(|package| {
                                                hash_bytes(&package.database_bytes) != hash
                                            })))
                            }) {
                                diagnostics.push(diagnostic("unpublished_commit", "Há uma revisão confirmada no banco que ainda não foi publicada no arquivo. Escolha a revisão a recuperar."));
                                if state != StorageState::Unsupported {
                                    state = StorageState::RecoveryAvailable;
                                }
                            }
                            add_candidate(
                                &mut candidates,
                                &mut sources,
                                CandidateKind::WorkingDatabase,
                                "Banco de trabalho",
                                &project,
                                hash,
                                CandidateSource::Database,
                            );
                        }
                        Ok((None, _)) => {}
                        Err(error) => {
                            if matches!(
                                error,
                                prumo_storage::StorageError::UnsupportedVersion { .. }
                                    | prumo_storage::StorageError::UnsupportedVersionValue { .. }
                                    | prumo_storage::StorageError::UnsupportedRepresentation(_)
                            ) {
                                state = StorageState::Unsupported;
                            }
                            diagnostics
                                .push(diagnostic("invalid_working_database", error.to_string()));
                        }
                    }
                }
                Ok(None) => {
                    fingerprints.insert("database".into(), "missing".into());
                }
                Err(error) => {
                    fingerprints.insert("database".into(), error.to_string());
                    diagnostics.push(diagnostic("invalid_working_database", error.to_string()));
                }
            }
            for name in [
                "state.sqlite-journal",
                "state.sqlite-wal",
                "state.sqlite-shm",
            ] {
                match read_optional(&paths.root.join(name), MAX_DATABASE_BYTES) {
                    Ok(Some(bytes)) => {
                        fingerprints.insert(name.into(), hash_bytes(&bytes));
                    }
                    Ok(None) => {
                        fingerprints.insert(name.into(), "missing".into());
                    }
                    Err(error) => {
                        fingerprints.insert(name.into(), error.to_string());
                    }
                }
            }
        }
        let identities: BTreeSet<_> = candidates
            .iter()
            .map(|candidate| candidate.project_id)
            .collect();
        let allowed_id = primary
            .as_ref()
            .map(|primary| primary.project.id)
            .or_else(|| {
                (identities.len() == 1).then(|| *identities.first().expect("one identity"))
            });
        if primary.is_none() && identities.len() > 1 {
            diagnostics.push(diagnostic("ambiguous_project_identity", "Os candidatos pertencem a projetos diferentes. Nenhum será recuperado automaticamente."));
        }
        let writable = self.access == AccessMode::ReadWrite;
        for candidate in &mut candidates {
            candidate.recoverable = writable
                && state != StorageState::Unsupported
                && allowed_id == Some(candidate.project_id);
        }
        let can_recover = candidates.iter().any(|candidate| candidate.recoverable);
        if primary.is_none() && !candidates.is_empty() && state != StorageState::Unsupported {
            state = StorageState::RecoveryAvailable;
        }
        if state == StorageState::Legacy {
            diagnostics.push(diagnostic("legacy_json", "A leitura preserva o original. A primeira edição ou a migração confirmada cria o pacote e mantém uma cópia exata do JSON."));
        }
        let package = primary
            .as_ref()
            .and_then(|primary| primary.package.as_ref())
            .map(package_summary);
        let inspection_sha256 = hash_bytes(&serde_json::to_vec(
            &json!({"files":fingerprints,"candidates":candidates,"state":state}),
        )?);
        let status = StorageStatus {
            state,
            format,
            project_id: primary.as_ref().map(|file| file.project.id),
            revision: primary.as_ref().map(|file| file.project.revision),
            primary_sha256,
            inspection_sha256,
            package,
            candidates,
            diagnostics,
            can_recover,
            can_migrate: writable
                && state == StorageState::Legacy
                && primary.as_ref().is_some_and(|file| file.package.is_none()),
            can_backup: writable && state != StorageState::Unsupported && primary.is_some(),
        };
        Ok(Inspection { status, sources })
    }

    pub(super) fn checkpoint(&self, phase: SavePhase) {
        #[cfg(test)]
        if self.fault == Some(phase) {
            let mut stderr = std::io::stderr().lock();
            writeln!(stderr, "[prumo-test-crash:{phase:?}]").expect("write crash marker");
            stderr.flush().expect("flush crash marker");
            std::process::abort();
        }
        let _ = phase;
    }
}

fn inspect_working_database(
    path: &Path,
    primary: Option<&LoadedFile>,
) -> prumo_storage::Result<(Option<Project>, String)> {
    if let Some(primary) = primary
        && let Some(package) = &primary.package
    {
        // This API promises a consistent image, not validated objects.
        // Exact byte equality reuses only the validation from this read.
        let image = SQLiteWorkingStore::backup_read_only(path)?;
        if image == package.database_bytes {
            return Ok((Some(primary.project.clone()), hash_bytes(&image)));
        }
    }
    let snapshot = SQLiteWorkingStore::snapshot_read_only(path)?;
    Ok((snapshot.project, hash_bytes(&snapshot.database_bytes)))
}

fn load_file(path: &Path) -> Result<LoadedFile, AppError> {
    let bytes = read_optional(path, MAX_PACKAGE_BYTES)?.ok_or(AppError::NotFound)?;
    decode_file(bytes)
}

fn file_format(bytes: &[u8]) -> StorageFormat {
    if bytes.starts_with(b"PK") {
        StorageFormat::NativePackage
    } else if bytes.iter().find(|byte| !byte.is_ascii_whitespace()) == Some(&b'{') {
        StorageFormat::LegacyJson
    } else {
        StorageFormat::Unknown
    }
}

fn decode_file(bytes: Vec<u8>) -> Result<LoadedFile, AppError> {
    if file_format(&bytes) == StorageFormat::NativePackage {
        let package = decode_package(&bytes)?;
        if package
            .manifest
            .entries
            .get("project.json")
            .is_none_or(|entry| entry.size > MAX_PROJECT_BYTES)
        {
            return Err(AppError::InvalidProject(
                "O modelo excede o limite de 16 MiB.".into(),
            ));
        }
        Ok(LoadedFile {
            project: package.project.clone(),
            raw: bytes,
            package: Some(package),
        })
    } else {
        if bytes.len() as u64 > MAX_PROJECT_BYTES {
            return Err(AppError::InvalidProject(
                "JSON maior que o limite de 16 MiB.".into(),
            ));
        }
        prumo_domain::canonical::validate_json_representation(&bytes).map_err(
            |error| match error {
                prumo_domain::canonical::JsonRepresentationError::ReservedNamespace => {
                    AppError::UnsupportedRepresentation(error.to_string())
                }
                prumo_domain::canonical::JsonRepresentationError::InvalidJson(error) => {
                    AppError::Serialization(error)
                }
            },
        )?;
        let value: Value = serde_json::from_slice(&bytes)?;
        if let Some(version) = value.get("schema_version").and_then(Value::as_u64)
            && version != 1
            && version != u64::from(PROJECT_SCHEMA_VERSION)
        {
            return Err(AppError::UnsupportedSchema { found: version });
        }
        if let Some(version) = value.get("schema_version").and_then(Value::as_number)
            && version.as_u64().is_none()
        {
            return Err(AppError::UnsupportedSchemaValue {
                value: version.to_string(),
            });
        }
        let project = prumo_domain::model::decode_project(value)
            .map_err(|error| AppError::InvalidProject(error.to_string()))?;
        Ok(LoadedFile {
            project,
            raw: bytes,
            package: None,
        })
    }
}

fn read_optional(path: &Path, maximum: u64) -> Result<Option<Vec<u8>>, AppError> {
    reject_non_regular(path)?;
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if file.metadata()?.len() > maximum {
        return Err(AppError::InvalidProject(
            "Arquivo excede o limite de leitura.".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(AppError::InvalidProject(
            "Arquivo excede o limite de leitura.".into(),
        ));
    }
    Ok(Some(bytes))
}

fn check_project_size(project: &Project) -> Result<(), AppError> {
    if serde_json::to_vec(project)?.len() as u64 > MAX_PROJECT_BYTES {
        return Err(AppError::InvalidProject(
            "O modelo excede o limite de 16 MiB.".into(),
        ));
    }
    Ok(())
}

fn encode(store: &SQLiteWorkingStore) -> Result<EncodedPackage, AppError> {
    Ok(encode_validated_package(
        store,
        &BTreeMap::new(),
        PackageOrigin {
            application: "Prumo".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
        },
    )?)
}

fn check_revision(project: &Project, expected: u64) -> Result<(), AppError> {
    if project.revision != expected {
        Err(AppError::Conflict {
            expected,
            actual: project.revision,
        })
    } else {
        Ok(())
    }
}

fn check_directory(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            Err(AppError::InvalidInput(
                "A pasta de trabalho deve ser um diretório regular, sem links simbólicos.".into(),
            ))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    reject_non_regular(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| AppError::InvalidInput("Pasta ausente.".into()))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    verify_temporary(&temp, bytes)?;
    temp.persist(path)
        .map_err(|error| AppError::Io(error.error))?;
    sync_parent(path)
}

fn immutable_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    reject_non_regular(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| AppError::InvalidInput("Pasta ausente.".into()))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    verify_temporary(&temp, bytes)?;
    match temp.persist_noclobber(path) {
        Ok(_) => {}
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_optional(path, MAX_PACKAGE_BYTES)?.as_deref() != Some(bytes) {
                return Err(AppError::InvalidProject(
                    "A cópia preservada diverge dos bytes originais.".into(),
                ));
            }
        }
        Err(error) => return Err(error.error.into()),
    }
    sync_parent(path)
}

fn verify_temporary(file: &tempfile::NamedTempFile, expected: &[u8]) -> Result<(), AppError> {
    let actual = File::open(file.path())?;
    if actual.metadata()?.len() != expected.len() as u64 {
        return Err(AppError::InvalidProject(
            "Tamanho gravado diverge do conteúdo preparado.".into(),
        ));
    }
    let mut reader = actual.take(expected.len() as u64 + 1);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    let actual_hash: [u8; 32] = digest.finalize().into();
    let expected_hash: [u8; 32] = Sha256::digest(expected).into();
    if actual_hash != expected_hash {
        return Err(AppError::InvalidProject(
            "O hash do arquivo temporário diverge do conteúdo preparado.".into(),
        ));
    }
    Ok(())
}

fn sync_parent(path: &Path) -> Result<(), AppError> {
    #[cfg(unix)]
    File::open(
        path.parent()
            .ok_or_else(|| AppError::InvalidInput("Pasta ausente.".into()))?,
    )?
    .sync_all()?;
    let _ = path;
    Ok(())
}

fn is_unsupported(error: &AppError) -> bool {
    matches!(
        error,
        AppError::UnsupportedSchema { .. }
            | AppError::UnsupportedSchemaValue { .. }
            | AppError::UnsupportedRepresentation(_)
            | AppError::Storage(
                prumo_storage::StorageError::UnsupportedVersion { .. }
                    | prumo_storage::StorageError::UnsupportedVersionValue { .. }
                    | prumo_storage::StorageError::UnsupportedRepresentation(_)
            )
    )
}

fn diagnostic(code: impl Into<String>, message: impl Into<String>) -> StorageDiagnostic {
    StorageDiagnostic {
        code: code.into(),
        message: message.into(),
    }
}

fn add_candidate(
    candidates: &mut Vec<StorageCandidate>,
    sources: &mut BTreeMap<String, CandidateSource>,
    kind: CandidateKind,
    label: &str,
    project: &Project,
    sha256: String,
    source: CandidateSource,
) {
    let id = hash_bytes(format!("{kind:?}:{label}:{sha256}").as_bytes());
    sources.insert(id.clone(), source);
    candidates.push(StorageCandidate {
        id,
        kind,
        label: label.into(),
        project_id: project.id,
        revision: project.revision,
        sha256,
        recoverable: false,
    });
}

fn package_summary(package: &DecodedPackage) -> PackageSummary {
    let manifest = &package.manifest;
    PackageSummary {
        format_version: manifest.format_version,
        database_version: manifest.database_version,
        model_version: manifest.model_version,
        created_unix_ms: manifest.created_unix_ms,
        origin: format!(
            "{} {} ({})",
            manifest.origin.application, manifest.origin.version, manifest.origin.platform
        ),
        project_sha256: manifest.project_sha256.clone(),
        head_sha256: manifest.head_sha256.clone(),
        entries: manifest
            .entries
            .iter()
            .map(|(name, entry)| PackageEntrySummary {
                name: name.clone(),
                size: entry.size,
                sha256: entry.sha256.clone(),
            })
            .collect(),
    }
}
