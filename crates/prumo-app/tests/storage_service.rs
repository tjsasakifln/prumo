use prumo_app::{AccessMode, AppError, ProjectService};
use prumo_domain::{
    LengthUnit,
    canonical::{hash_bytes, model_snapshot},
    storage::{CandidateKind, StorageAction, StorageState},
};
use prumo_storage::{
    CommitRecord, OpaqueEntry, PackageOrigin, SQLiteWorkingStore, decode_package, encode_package,
};
use serde_json::json;
use std::{collections::BTreeMap, fs};
use uuid::Uuid;

fn open(directory: &tempfile::TempDir, mode: AccessMode) -> ProjectService {
    ProjectService::open(
        directory.path().join("project.prumo"),
        mode,
        "fixture:storage-service",
    )
    .unwrap()
}

#[test]
fn native_package_travels_offline_with_model_catalog_measurements_history_and_hashes() {
    let directory = tempfile::tempdir().unwrap();
    let service = open(&directory, AccessMode::ReadWrite);
    service
        .create("Pacote completo", LengthUnit::Metre)
        .unwrap();
    let package = service.available_normative_package().unwrap();
    service.pin_normative_package(1, &package.hash).unwrap();
    service
        .add_member(2, "Viga de ensaio", [0.0; 3], [3.0, 4.0, 0.0])
        .unwrap();
    let expected = service.inspect_model(3).unwrap();
    let expected_snapshot = model_snapshot(&expected).unwrap();
    let bytes = fs::read(service.project_path()).unwrap();
    assert!(bytes.starts_with(b"PK"));
    let decoded = decode_package(&bytes).unwrap();
    assert_eq!(decoded.project, expected);
    assert!(decoded.manifest.entries.contains_key("state.sqlite"));
    assert!(decoded.manifest.entries.contains_key("project.json"));
    assert!(decoded.manifest.entries.contains_key("commands.json"));
    let isolated = tempfile::tempdir().unwrap();
    let isolated_file = isolated.path().join("project.prumo");
    fs::write(&isolated_file, &bytes).unwrap();
    let read_only = open(&isolated, AccessMode::ReadOnly);
    assert_eq!(read_only.read().unwrap(), expected);
    assert_eq!(
        model_snapshot(&read_only.read().unwrap()).unwrap(),
        expected_snapshot
    );
    assert_eq!(fs::read(&isolated_file).unwrap(), bytes);
    let before_entries = fs::read_dir(isolated.path()).unwrap().count();
    let status = read_only.storage_status().unwrap();
    assert_eq!(status.state, StorageState::Ready);
    assert!(!status.can_migrate && !status.can_backup && !status.can_recover);
    assert_eq!(
        fs::read_dir(isolated.path()).unwrap().count(),
        before_entries
    );
    let writer = open(&isolated, AccessMode::ReadWrite);
    let changed = writer.rename_project(4, "Pacote reaberto").unwrap();
    assert_eq!(changed.normative, expected.normative);
    assert_eq!(changed.dependencies.results, expected.dependencies.results);
    assert_eq!(writer.read().unwrap(), changed);
}

#[test]
fn legacy_migration_is_explicit_idempotent_and_retains_the_exact_source() {
    let source = tempfile::tempdir().unwrap();
    let original = open(&source, AccessMode::ReadWrite)
        .create("Origem JSON", LengthUnit::Millimetre)
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let service = open(&directory, AccessMode::ReadWrite);
    let raw = serde_json::to_vec_pretty(&original).unwrap();
    fs::write(service.project_path(), &raw).unwrap();
    let status = service.storage_status().unwrap();
    assert_eq!(status.state, StorageState::Legacy);
    assert_eq!(service.read().unwrap(), original);
    assert_eq!(fs::read(service.project_path()).unwrap(), raw);
    assert!(matches!(
        service.execute_storage(StorageAction::Migrate {
            expected_revision: 1,
            inspection_sha256: status.inspection_sha256.clone(),
            confirm: false
        }),
        Err(AppError::ConfirmationRequired)
    ));
    let result = service
        .execute_storage(StorageAction::Migrate {
            expected_revision: 1,
            inspection_sha256: status.inspection_sha256,
            confirm: true,
        })
        .unwrap();
    assert!(result.changed);
    assert_eq!(result.project, original);
    let hash = hash_bytes(&raw);
    let root = directory
        .path()
        .join(format!(".prumo-{}", hash_bytes(b"project.prumo")));
    assert_eq!(
        fs::read(root.join(format!("original-{hash}.prumo"))).unwrap(),
        raw
    );
    let current = fs::read(service.project_path()).unwrap();
    let result = service
        .execute_storage(StorageAction::Migrate {
            expected_revision: 1,
            inspection_sha256: result.status.inspection_sha256,
            confirm: true,
        })
        .unwrap();
    assert!(!result.changed);
    assert_eq!(fs::read(service.project_path()).unwrap(), current);
}

#[test]
fn recovery_requires_fresh_identity_confirmation_and_preserves_the_damaged_file() {
    let directory = tempfile::tempdir().unwrap();
    let service = open(&directory, AccessMode::ReadWrite);
    let original = service.create("Anterior", LengthUnit::Metre).unwrap();
    let old_status = service.storage_status().unwrap();
    let current = service.rename_project(1, "Atual").unwrap();
    assert!(matches!(
        service.execute_storage(StorageAction::Backup {
            expected_revision: 1,
            inspection_sha256: old_status.inspection_sha256
        }),
        Err(AppError::StorageConflict)
    ));
    let damaged = b"bytes corrompidos, preservados no ensaio";
    fs::write(service.project_path(), damaged).unwrap();
    assert!(service.read().is_err());
    let status = service.storage_status().unwrap();
    assert_eq!(status.state, StorageState::RecoveryAvailable);
    let candidate = status
        .candidates
        .iter()
        .find(|candidate| candidate.kind == CandidateKind::Backup && candidate.revision == 1)
        .unwrap();
    let action = StorageAction::Recover {
        candidate_id: candidate.id.clone(),
        inspection_sha256: status.inspection_sha256.clone(),
        confirm: false,
    };
    assert!(matches!(
        service.execute_storage(action),
        Err(AppError::ConfirmationRequired)
    ));
    assert_eq!(fs::read(service.project_path()).unwrap(), damaged);
    let result = service
        .execute_storage(StorageAction::Recover {
            candidate_id: candidate.id.clone(),
            inspection_sha256: status.inspection_sha256,
            confirm: true,
        })
        .unwrap();
    assert_eq!(result.project, original);
    assert_eq!(service.read().unwrap(), original);
    assert_ne!(current, result.project);
    let root = directory
        .path()
        .join(format!(".prumo-{}", hash_bytes(b"project.prumo")));
    assert_eq!(
        fs::read(root.join(format!("incident-{}.bytes", hash_bytes(damaged)))).unwrap(),
        damaged
    );
}

#[test]
fn future_model_is_diagnosed_without_recovery_downgrade_or_rewriting_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let service = open(&directory, AccessMode::ReadWrite);
    let project = service.create("Atual", LengthUnit::Metre).unwrap();
    let mut future = serde_json::to_value(project).unwrap();
    future["schema_version"] = json!(u64::MAX);
    let bytes = serde_json::to_vec(&future).unwrap();
    fs::write(service.project_path(), &bytes).unwrap();
    let status = service.storage_status().unwrap();
    assert_eq!(status.state, StorageState::Unsupported);
    assert!(!status.can_recover && !status.can_migrate && !status.can_backup);
    assert!(
        status
            .candidates
            .iter()
            .all(|candidate| !candidate.recoverable)
    );
    assert!(service.rename_project(1, "Tentativa").is_err());
    let candidate_id = status.candidates[0].id.clone();
    assert!(
        service
            .execute_storage(StorageAction::Recover {
                candidate_id,
                inspection_sha256: status.inspection_sha256,
                confirm: true
            })
            .is_err()
    );
    assert_eq!(fs::read(service.project_path()).unwrap(), bytes);
}

#[test]
fn overflow_future_version_and_future_autosave_block_every_publication() {
    let directory = tempfile::tempdir().unwrap();
    let service = open(&directory, AccessMode::ReadWrite);
    let project = service
        .create("Versão conhecida", LengthUnit::Metre)
        .unwrap();
    let valid = fs::read(service.project_path()).unwrap();
    let mut value = serde_json::to_value(&project).unwrap();
    value["schema_version"] = json!("FUTURE_VERSION_MARKER");
    let future = serde_json::to_string(&value)
        .unwrap()
        .replace("\"FUTURE_VERSION_MARKER\"", "18446744073709551616")
        .into_bytes();
    fs::write(service.project_path(), &future).unwrap();
    assert_eq!(
        service.storage_status().unwrap().state,
        StorageState::Unsupported
    );
    assert_eq!(service.read().unwrap_err().code(), "unsupported_schema");
    fs::write(service.project_path(), &valid).unwrap();
    let root = directory
        .path()
        .join(format!(".prumo-{}", hash_bytes(b"project.prumo")));
    fs::write(root.join("autosave.prumo"), &future).unwrap();
    let status = service.storage_status().unwrap();
    assert_eq!(status.state, StorageState::Unsupported);
    assert!(!status.can_migrate && !status.can_backup && !status.can_recover);
    assert!(matches!(
        service.rename_project(1, "Sobrescrever futuro"),
        Err(AppError::RecoveryRequired)
    ));
    assert!(matches!(
        service.execute_storage(StorageAction::Backup {
            expected_revision: 1,
            inspection_sha256: status.inspection_sha256.clone()
        }),
        Err(AppError::RecoveryRequired)
    ));
    assert_eq!(fs::read(service.project_path()).unwrap(), valid);
    assert_eq!(fs::read(root.join("autosave.prumo")).unwrap(), future);
    fs::remove_file(service.project_path()).unwrap();
    assert!(matches!(
        service.create("Projeto novo", LengthUnit::Metre),
        Err(AppError::RecoveryRequired)
    ));
    assert_eq!(fs::read(root.join("autosave.prumo")).unwrap(), future);
}

fn package_with_opaque_extension(project: &prumo_domain::Project) -> Vec<u8> {
    let directory = tempfile::tempdir().unwrap();
    let mut store = SQLiteWorkingStore::open(&directory.path().join("opaque.sqlite")).unwrap();
    let entries = BTreeMap::from([(
        "extensions/future-mep/catalog.bin".into(),
        OpaqueEntry {
            media_type: "application/octet-stream".into(),
            version: 700,
            bytes: vec![0, 255, 0, 1, 2, 3, 255],
        },
    )]);
    let record = CommitRecord {
        command_id: Uuid::new_v4(),
        actor: "fixture:import".into(),
        operation: "storage.import".into(),
        reason: None,
        timestamp_unix_ms: 1,
        command: None,
    };
    store
        .commit_with_extensions(None, project, record, &entries)
        .unwrap();
    encode_package(
        &store,
        &entries,
        PackageOrigin {
            application: "Prumo fixture".into(),
            version: "1".into(),
            platform: "offline".into(),
        },
    )
    .unwrap()
}

#[test]
fn unknown_opaque_extensions_survive_edits_backup_and_recovery_without_becoming_engineering_data() {
    let source = tempfile::tempdir().unwrap();
    let mut project = open(&source, AccessMode::ReadWrite)
        .create("MEP futuro", LengthUnit::Metre)
        .unwrap();
    project.extensions.insert(
        "future.mep".into(),
        json!({"version":700, "binary":"extensions/future-mep/catalog.bin"}),
    );
    let original = package_with_opaque_extension(&project);
    let expected_extensions = decode_package(&original).unwrap().extensions;
    let directory = tempfile::tempdir().unwrap();
    let service = open(&directory, AccessMode::ReadWrite);
    fs::write(service.project_path(), original).unwrap();
    let edited = service.rename_project(1, "MEP preservado").unwrap();
    let current = decode_package(&fs::read(service.project_path()).unwrap()).unwrap();
    assert_eq!(current.extensions, expected_extensions);
    assert_eq!(edited.extensions, project.extensions);
    fs::write(service.project_path(), b"corrupt").unwrap();
    let status = service.storage_status().unwrap();
    let candidate = status
        .candidates
        .iter()
        .find(|candidate| {
            candidate.kind == CandidateKind::WorkingDatabase && candidate.revision == 2
        })
        .unwrap();
    service
        .execute_storage(StorageAction::Recover {
            candidate_id: candidate.id.clone(),
            inspection_sha256: status.inspection_sha256,
            confirm: true,
        })
        .unwrap();
    let recovered = decode_package(&fs::read(service.project_path()).unwrap()).unwrap();
    assert_eq!(recovered.extensions, expected_extensions);
    assert_eq!(recovered.project, edited);
    assert!(recovered.project.dependencies.results.is_empty());
}

#[test]
fn same_model_with_different_storage_authority_requires_a_recovery_choice() {
    let directory = tempfile::tempdir().unwrap();
    let service = open(&directory, AccessMode::ReadWrite);
    let project = service
        .create("Identidade compartilhada", LengthUnit::Metre)
        .unwrap();
    let replacement = package_with_opaque_extension(&project);
    let expected_extensions = decode_package(&replacement).unwrap().extensions;
    fs::write(service.project_path(), &replacement).unwrap();
    assert!(matches!(
        service.rename_project(1, "Sem escolha"),
        Err(AppError::RecoveryRequired)
    ));
    assert_eq!(fs::read(service.project_path()).unwrap(), replacement);
    let status = service.storage_status().unwrap();
    assert_eq!(status.state, StorageState::RecoveryAvailable);
    let candidate = status
        .candidates
        .iter()
        .find(|candidate| candidate.label == "Revisão publicada")
        .unwrap();
    service
        .execute_storage(StorageAction::Recover {
            candidate_id: candidate.id.clone(),
            inspection_sha256: status.inspection_sha256,
            confirm: true,
        })
        .unwrap();
    service.rename_project(1, "Escolha aplicada").unwrap();
    assert_eq!(
        decode_package(&fs::read(service.project_path()).unwrap())
            .unwrap()
            .extensions,
        expected_extensions
    );
}

#[test]
fn displayed_write_basis_rejects_file_replacement_even_without_a_working_database() {
    let source = tempfile::tempdir().unwrap();
    let producer = open(&source, AccessMode::ReadWrite);
    let project = producer.create("Base exibida", LengthUnit::Metre).unwrap();
    let initial = fs::read(producer.project_path()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let service = open(&directory, AccessMode::ReadWrite);
    fs::write(service.project_path(), &initial).unwrap();
    let (displayed, displayed_hash) = service.read_with_basis().unwrap();
    assert_eq!(displayed, project);
    assert_eq!(displayed_hash, hash_bytes(&initial));
    let guarded = service
        .with_write_basis(displayed.id, displayed_hash)
        .unwrap();
    for same_id in [false, true] {
        let mut changed = project.clone();
        if !same_id {
            changed.id = Uuid::new_v4();
        }
        changed.name = "Substituição externa na mesma revisão".into();
        let replacement = serde_json::to_vec(&changed).unwrap();
        fs::write(service.project_path(), &replacement).unwrap();
        assert!(matches!(
            guarded.rename_project(1, "Edição preparada para a base antiga"),
            Err(AppError::StorageConflict)
        ));
        assert_eq!(fs::read(service.project_path()).unwrap(), replacement);
        assert!(
            !directory
                .path()
                .join(format!(".prumo-{}", hash_bytes(b"project.prumo")))
                .exists()
        );
    }
    fs::write(service.project_path(), &initial).unwrap();
    assert_eq!(
        guarded.rename_project(1, "Base conferida").unwrap().name,
        "Base conferida"
    );
}

#[cfg(unix)]
#[test]
fn linked_working_directory_cannot_read_or_overwrite_another_project() {
    let directory = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let service = open(&directory, AccessMode::ReadWrite);
    let root = directory
        .path()
        .join(format!(".prumo-{}", hash_bytes(b"project.prumo")));
    std::os::unix::fs::symlink(other.path(), &root).unwrap();
    assert!(service.create("Inválido", LengthUnit::Metre).is_err());
    assert!(!service.project_path().exists());
    assert_eq!(fs::read_dir(other.path()).unwrap().count(), 0);
    assert!(
        service
            .storage_status()
            .unwrap()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "unsafe_working_directory")
    );
}
