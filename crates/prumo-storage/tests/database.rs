mod common;

use common::{origin, project, record, rename};
use prumo_domain::canonical::model_snapshot;
use prumo_storage::{
    CommitPhase, OpaqueEntry, SQLiteWorkingStore, StorageError, decode_package, encode_package,
    restore_database,
};
use rusqlite::Connection;
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    time::{Duration, Instant},
};
use tempfile::tempdir;

fn counts(path: &std::path::Path) -> (u32, u32, u32) {
    let db = Connection::open(path).unwrap();
    db.query_row("SELECT (SELECT count(*) FROM content_objects), (SELECT count(*) FROM revisions), (SELECT count(*) FROM head)", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap()
}

#[test]
fn database_package_and_restoration_preserve_exact_model_hashes_and_unknown_extensions() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("work.sqlite");
    let mut store = SQLiteWorkingStore::open(&path).unwrap();
    assert!(store.load().unwrap().is_none());
    let initial = project();
    let extensions = BTreeMap::from([(
        "extensions/future-mep/thermal-v18.bin".into(),
        OpaqueEntry {
            media_type: "application/vnd.test.mep".into(),
            version: 18,
            bytes: vec![0, 1, 255, 2, 0, 5],
        },
    )]);
    store
        .commit_with_extensions(None, &initial, record(&initial), &extensions)
        .unwrap();
    let next = rename(&initial, "Revisão preservada offline");
    store.commit(Some(1), &next, record(&next)).unwrap();
    let before = model_snapshot(&next).unwrap();
    let encoded = encode_package(&store, &BTreeMap::new(), origin()).unwrap();
    let decoded = decode_package(&encoded).unwrap();
    assert_eq!(decoded.project, next);
    assert_eq!(decoded.extensions, extensions);
    assert_eq!(
        model_snapshot(&decoded.project).unwrap().sha256,
        before.sha256
    );
    let restored_path = dir.path().join("restored.sqlite");
    restore_database(&restored_path, &decoded.database_bytes).unwrap();
    let restored = SQLiteWorkingStore::open_read_only(&restored_path).unwrap();
    assert_eq!(restored.load().unwrap(), Some(next));
    assert_eq!(restored.extensions().unwrap(), extensions);
    let again =
        decode_package(&encode_package(&restored, &BTreeMap::new(), origin()).unwrap()).unwrap();
    assert_eq!(again.manifest.head_sha256, decoded.manifest.head_sha256);
    assert_eq!(
        again.manifest.project_sha256,
        decoded.manifest.project_sha256
    );
    assert_eq!(again.project, decoded.project);
    assert_eq!(again.extensions, extensions);
}

#[test]
fn every_precommit_phase_rolls_back_objects_revision_and_head_together() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("atomic.sqlite");
    let mut store = SQLiteWorkingStore::open(&path).unwrap();
    let initial = project();
    store.commit(None, &initial, record(&initial)).unwrap();
    let before_counts = counts(&path);
    for phase in [
        CommitPhase::ObjectsInserted,
        CommitPhase::RevisionInserted,
        CommitPhase::HeadUpdated,
        CommitPhase::BeforeCommit,
    ] {
        let next = rename(&initial, "Esta revisão deve desaparecer por rollback");
        let result = store.commit_with_hook(Some(1), &next, record(&next), |reached| {
            if reached == phase {
                Err(StorageError::Interrupted(format!("fixture {phase:?}")))
            } else {
                Ok(())
            }
        });
        assert!(matches!(result, Err(StorageError::Interrupted(_))));
        assert_eq!(store.load().unwrap(), Some(initial.clone()));
        assert_eq!(counts(&path), before_counts);
    }
    drop(store);
    assert_eq!(
        SQLiteWorkingStore::open_read_only(&path)
            .unwrap()
            .load()
            .unwrap(),
        Some(initial)
    );
}

#[test]
fn concurrent_revision_retry_identity_history_and_read_only_are_enforced() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cas.sqlite");
    let mut first = SQLiteWorkingStore::open(&path).unwrap();
    let initial = project();
    let initial_record = record(&initial);
    first
        .commit(None, &initial, initial_record.clone())
        .unwrap();
    let mut second = SQLiteWorkingStore::open(&path).unwrap();
    let next = rename(&initial, "Confirmada pelo primeiro escritor");
    let second_record = record(&next);
    first.commit(Some(1), &next, second_record.clone()).unwrap();
    assert!(matches!(
        second.commit(Some(1), &next, second_record.clone()),
        Err(StorageError::RevisionConflict {
            expected: Some(1),
            actual: Some(2)
        })
    ));
    let mut third = rename(&next, "Terceira");
    assert!(second.commit(Some(2), &third, second_record).is_err());
    third.history[0].actor = "reescrita indevida".into();
    assert!(second.commit(Some(2), &third, record(&third)).is_err());
    let mut wrong_id = rename(&next, "Outro projeto");
    wrong_id.id = uuid::Uuid::new_v4();
    assert!(
        second
            .commit(Some(2), &wrong_id, record(&wrong_id))
            .is_err()
    );
    assert_eq!(second.load().unwrap(), Some(next.clone()));
    let mut reader = SQLiteWorkingStore::open_read_only(&path).unwrap();
    let third = rename(&next, "Não autorizado");
    assert!(matches!(
        reader.commit(Some(2), &third, record(&third)),
        Err(StorageError::ReadOnly)
    ));
    assert_eq!(reader.load().unwrap(), Some(next));
}

#[test]
fn administrative_recovery_records_a_command_without_fabricating_a_model_revision() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("recovery.sqlite");
    let initial = project();
    let mut store = SQLiteWorkingStore::open(&path).unwrap();
    store.commit(None, &initial, record(&initial)).unwrap();
    let before =
        decode_package(&encode_package(&store, &BTreeMap::new(), origin()).unwrap()).unwrap();
    let mut recovery = record(&initial);
    recovery.operation = "storage.recover".into();
    recovery.actor = "host autenticado".into();
    recovery.timestamp_unix_ms = 9000;
    recovery.command = Some(json!({"operation":"storage.recover","candidate_id":"opaque-test-id"}));
    store.commit(Some(1), &initial, recovery.clone()).unwrap();
    assert_eq!(store.load().unwrap(), Some(initial.clone()));
    assert_eq!(counts(&path).1, 2);
    let after =
        decode_package(&encode_package(&store, &BTreeMap::new(), origin()).unwrap()).unwrap();
    assert_eq!(after.manifest.head_sha256, before.manifest.head_sha256);
    assert_eq!(
        after.manifest.project_sha256,
        before.manifest.project_sha256
    );
    assert_eq!(after.manifest.created_unix_ms, 9000);
    assert!(store.commit(Some(1), &initial, recovery).is_err());
    let mut different = initial.clone();
    different.name = "Mudança oculta".into();
    let mut recovery = record(&different);
    recovery.operation = "storage.recover".into();
    assert!(store.commit(Some(1), &different, recovery).is_err());
    assert_eq!(store.load().unwrap(), Some(initial));
}

#[test]
fn initial_import_retains_the_real_legacy_history_and_never_invents_prior_commands() {
    let dir = tempdir().unwrap();
    let mut legacy = rename(&project(), "Baseline importada na revisão 2");
    legacy
        .extensions
        .insert("legacy.unknown".into(), json!({"raw":[1,2,3]}));
    let mut import = record(&legacy);
    import.operation = "legacy.import".into();
    import.command = None;
    import.timestamp_unix_ms = 9000;
    let mut store = SQLiteWorkingStore::open(&dir.path().join("import.sqlite")).unwrap();
    store.commit(None, &legacy, import).unwrap();
    let next = rename(&legacy, "Revisão seguinte à importação");
    store.commit(Some(2), &next, record(&next)).unwrap();
    assert_eq!(store.load().unwrap(), Some(next.clone()));
    let invalid = rename(&next, "Payload ausente");
    let mut record = record(&invalid);
    record.command = None;
    assert!(store.commit(Some(3), &invalid, record).is_err());
}

#[test]
fn corrupt_foreign_and_future_databases_are_rejected_without_reinitializing() {
    let dir = tempdir().unwrap();
    for case in ["future", "foreign", "trigger", "object", "orphan"] {
        let path = dir.path().join(format!("{case}.sqlite"));
        let initial = project();
        let mut store = SQLiteWorkingStore::open(&path).unwrap();
        store.commit(None, &initial, record(&initial)).unwrap();
        drop(store);
        let attacker = Connection::open(&path).unwrap();
        match case {
            "future" => attacker.pragma_update(None, "user_version", 2).unwrap(),
            "foreign" => attacker.pragma_update(None, "application_id", 1).unwrap(),
            "trigger" => attacker.execute_batch("CREATE TRIGGER injected AFTER INSERT ON content_objects BEGIN DELETE FROM head; END").unwrap(),
            "object" => { attacker.execute("UPDATE content_objects SET bytes = x'7b7d', size = 2 WHERE sha256 = (SELECT sha256 FROM content_objects LIMIT 1)", []).unwrap(); }
            "orphan" => { attacker.execute_batch("PRAGMA foreign_keys=OFF; DELETE FROM content_objects WHERE sha256=(SELECT root_sha256 FROM head)").unwrap(); }
            _ => unreachable!(),
        }
        drop(attacker);
        let original = fs::read(&path).unwrap();
        let result = SQLiteWorkingStore::open(&path);
        if case == "future" {
            assert!(matches!(
                result,
                Err(StorageError::UnsupportedVersion { found: 2, .. })
            ));
        } else {
            assert!(result.is_err(), "caso {case}");
        }
        assert_eq!(fs::read(&path).unwrap(), original, "caso {case}");
    }
    let empty = dir.path().join("empty.sqlite");
    fs::write(&empty, []).unwrap();
    assert!(SQLiteWorkingStore::open(&empty).is_err());
    assert!(fs::read(empty).unwrap().is_empty());
}

#[test]
fn unchanged_content_is_shared_and_never_overwritten_by_later_commands() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("immutable.sqlite");
    let mut initial = project();
    initial.extensions.insert(
        "large.fixture".into(),
        json!("catálogo independente ".repeat(20_000)),
    );
    let mut store = SQLiteWorkingStore::open(&path).unwrap();
    store.commit(None, &initial, record(&initial)).unwrap();
    let connection = Connection::open(&path).unwrap();
    let before: BTreeMap<String, Vec<u8>> = connection
        .prepare("SELECT sha256, bytes FROM content_objects")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    let next = rename(&initial, "Mudou só o nome");
    store.commit(Some(1), &next, record(&next)).unwrap();
    let after: BTreeMap<String, Vec<u8>> = connection
        .prepare("SELECT sha256, bytes FROM content_objects")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert!(
        before
            .iter()
            .all(|(hash, bytes)| after.get(hash) == Some(bytes))
    );
    let large_objects = after.values().filter(|bytes| bytes.len() > 100_000).count();
    assert_eq!(
        large_objects, 1,
        "catálogo estável deve ser deduplicado por conteúdo"
    );
}

#[test]
fn read_only_queries_create_no_sidecars_and_restore_refuses_active_journals() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("readonly.sqlite");
    let initial = project();
    let mut store = SQLiteWorkingStore::open(&path).unwrap();
    store.commit(None, &initial, record(&initial)).unwrap();
    let image = store.backup_bytes().unwrap();
    drop(store);
    let before = fs::read(&path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
    }
    let reader = SQLiteWorkingStore::open_read_only(&path).unwrap();
    assert_eq!(reader.load().unwrap(), Some(initial));
    let _image = reader.backup_bytes().unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    drop(reader);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let journal = dir.path().join("readonly.sqlite-journal");
    fs::write(&journal, b"host must quarantine this sidecar").unwrap();
    assert!(restore_database(&path, &image).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::remove_file(journal).unwrap();
    let mut invalid = image.clone();
    invalid[0] ^= 1;
    assert!(restore_database(&path, &invalid).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    restore_database(&path, &image).unwrap();
}

#[test]
fn backup_busy_does_not_retry_indefinitely() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("busy.sqlite");
    let initial = project();
    let mut store = SQLiteWorkingStore::open(&path).unwrap();
    store.commit(None, &initial, record(&initial)).unwrap();
    let blocker = Connection::open(&path).unwrap();
    blocker.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let start = Instant::now();
    assert!(store.backup_bytes().is_err());
    assert!(start.elapsed() < Duration::from_secs(5));
    blocker.execute_batch("ROLLBACK").unwrap();
    assert!(store.backup_bytes().is_ok());
}

#[test]
fn extensions_are_atomic_immutable_and_compared_with_the_database() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("extensions.sqlite");
    let initial = project();
    let mut store = SQLiteWorkingStore::open(&path).unwrap();
    let mut entries = BTreeMap::from([(
        "extensions/../outside.bin".into(),
        OpaqueEntry {
            media_type: "application/octet-stream".into(),
            version: 1,
            bytes: vec![7],
        },
    )]);
    assert!(
        store
            .commit_with_extensions(None, &initial, record(&initial), &entries)
            .is_err()
    );
    assert_eq!(counts(&path), (0, 0, 0));
    let entry = entries.pop_first().unwrap().1;
    entries.insert("extensions/future.bin".into(), entry);
    store
        .commit_with_extensions(None, &initial, record(&initial), &entries)
        .unwrap();
    let next = rename(&initial, "Preservar extensão");
    let mut changed = entries.clone();
    changed.values_mut().next().unwrap().bytes.push(8);
    assert!(
        store
            .commit_with_extensions(Some(1), &next, record(&next), &changed)
            .is_err()
    );
    assert!(encode_package(&store, &changed, origin()).is_err());
    store.commit(Some(1), &next, record(&next)).unwrap();
    assert_eq!(store.extensions().unwrap(), entries);
}

#[test]
fn future_root_envelopes_and_model_versions_are_detected_before_unknown_fields() {
    for (field, version, historical) in [
        ("version", 2_u64, false),
        ("version", u64::MAX, false),
        ("model_version", 3, false),
        ("version", 2, true),
    ] {
        let dir = tempdir().unwrap();
        let path = dir.path().join("future-root.sqlite");
        let mut store = SQLiteWorkingStore::open(&path).unwrap();
        let initial = project();
        store.commit(None, &initial, record(&initial)).unwrap();
        if historical {
            let next = rename(&initial, "Cabeçalho atual, raiz histórica futura");
            store.commit(Some(1), &next, record(&next)).unwrap();
        }
        drop(store);
        let raw = Connection::open(&path).unwrap();
        let old_root: String = raw
            .query_row(
                "SELECT root_sha256 FROM revisions WHERE ordinal=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let bytes: Vec<u8> = raw
            .query_row(
                "SELECT bytes FROM content_objects WHERE sha256=?1",
                [&old_root],
                |row| row.get(0),
            )
            .unwrap();
        let mut envelope: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        envelope[field] = json!(version);
        envelope["future_contract"] = json!({"preserve":true});
        let bytes = serde_json::to_vec(&envelope).unwrap();
        let hash = prumo_domain::canonical::hash_bytes(&bytes);
        raw.execute(
            "INSERT INTO content_objects (sha256,size,bytes) VALUES (?1,?2,?3)",
            rusqlite::params![hash, bytes.len() as u32, bytes],
        )
        .unwrap();
        if historical {
            raw.execute(
                "UPDATE revisions SET root_sha256=?1 WHERE ordinal=1",
                [&hash],
            )
            .unwrap();
        } else {
            raw.execute("UPDATE head SET root_sha256=?1", [&hash])
                .unwrap();
        }
        drop(raw);
        let unchanged = fs::read(&path).unwrap();
        assert!(
            matches!(SQLiteWorkingStore::open_read_only(&path),Err(StorageError::UnsupportedVersion {found,..}) if found==version)
        );
        assert_eq!(fs::read(&path).unwrap(), unchanged);
    }
}

#[test]
fn aliased_extension_bytes_are_limited_before_per_path_allocation() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("aliases.sqlite");
    let initial = project();
    let mut store = SQLiteWorkingStore::open(&path).unwrap();
    let bytes = vec![37; 1024 * 1024];
    let hash = prumo_domain::canonical::hash_bytes(&bytes);
    let entries = BTreeMap::from([(
        "extensions/original.bin".into(),
        OpaqueEntry {
            media_type: "application/octet-stream".into(),
            version: 1,
            bytes,
        },
    )]);
    store
        .commit_with_extensions(None, &initial, record(&initial), &entries)
        .unwrap();
    drop(store);
    let mut attacker = Connection::open(&path).unwrap();
    let transaction = attacker.transaction().unwrap();
    for alias in 0..1000 {
        transaction.execute("INSERT INTO extensions (path,media_type,version,sha256) VALUES (?1,'application/octet-stream',1,?2)",rusqlite::params![format!("extensions/alias-{alias:04}.bin"),hash]).unwrap();
    }
    transaction.commit().unwrap();
    drop(attacker);
    assert!(
        matches!(SQLiteWorkingStore::open_read_only(&path),Err(StorageError::Limit(detail)) if detail=="extensões expandidas")
    );
}

#[test]
fn offline_catalog_snapshot_and_linked_measurements_survive_package_roundtrip() {
    use prumo_domain::registry::{NormativePackage, RegistryCatalog};
    let dir = tempdir().unwrap();
    let mut initial = project();
    // The committed seed contains public metadata only, with all capability
    // gates still planned. It is read from the repository, never fetched online.
    let catalog: RegistryCatalog =
        serde_json::from_str(include_str!("../../prumo-registry/data/catalogue.json")).unwrap();
    initial.normative.active = Some(NormativePackage::from_catalog(&catalog).unwrap());
    initial.normative.catalog = Some(catalog);
    initial.validate().unwrap();
    let mut store = SQLiteWorkingStore::open(&dir.path().join("offline.sqlite")).unwrap();
    store.commit(None, &initial, record(&initial)).unwrap();
    let mut measured = initial.clone();
    prumo_domain::dependencies::inspect_model(&mut measured).unwrap();
    measured.revision += 1;
    measured.history.push(prumo_domain::ChangeRecord {
        revision: 2,
        actor: "fixture".into(),
        operation: "inspect_model".into(),
        timestamp_unix_ms: 1002,
    });
    measured.validate().unwrap();
    store.commit(Some(1), &measured, record(&measured)).unwrap();
    let packaged = encode_package(&store, &BTreeMap::new(), origin()).unwrap();
    drop(store);
    let reopened = decode_package(&packaged).unwrap();
    assert_eq!(reopened.project.normative, measured.normative);
    assert_eq!(reopened.project.dependencies, measured.dependencies);
    assert!(!reopened.project.dependencies.snapshots.is_empty());
    assert!(
        reopened
            .project
            .dependencies
            .results
            .iter()
            .all(|result| !result.engineering_release)
    );
    assert_eq!(
        model_snapshot(&reopened.project).unwrap().sha256,
        model_snapshot(&measured).unwrap().sha256
    );
}

#[test]
fn combined_package_size_is_refused_before_a_commit_that_could_not_be_recovered() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("combined-limit.sqlite");
    let initial = project();
    let mut store = SQLiteWorkingStore::open(&path).unwrap();
    let bytes_before = fs::read(&path).unwrap();
    // 44 MiB fits every single-object/database limit. The database, immutable
    // object projection and extension projection together exceed 128 MiB.
    let entries = BTreeMap::from([(
        "extensions/large-but-individually-valid.bin".into(),
        OpaqueEntry {
            media_type: "application/octet-stream".into(),
            version: 1,
            bytes: vec![83; 44 * 1024 * 1024],
        },
    )]);
    assert!(
        matches!(store.commit_with_extensions(None,&initial,record(&initial),&entries),Err(StorageError::Limit(detail)) if detail=="pacote produzido")
    );
    assert_eq!(store.load().unwrap(), None);
    assert_eq!(counts(&path), (0, 0, 0));
    assert_eq!(fs::read(&path).unwrap(), bytes_before);
    store.commit(None, &initial, record(&initial)).unwrap();
    assert!(decode_package(&encode_package(&store, &BTreeMap::new(), origin()).unwrap()).is_ok());
}

#[test]
fn snapshot_apis_return_matching_bytes_and_preserve_arbitrary_unknown_numbers() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("snapshot.sqlite");
    let (mut store, empty) = SQLiteWorkingStore::open_with_project(&path).unwrap();
    assert!(empty.is_none());
    assert!(
        SQLiteWorkingStore::snapshot_read_only(&path)
            .unwrap()
            .project
            .is_none()
    );
    let mut initial = project();
    initial.extensions.insert("future.precision".into(),serde_json::from_str(r#"{"integer":18446744073709551617,"decimal":0.12345678901234567890123456789,"small":1e-9999,"large":1e9999}"#).unwrap());
    let opaque = BTreeMap::from([(
        "extensions/raw-reserved-looking.json".into(),
        OpaqueEntry {
            media_type: "application/json".into(),
            version: 17,
            bytes: br#"{"$serde_json::private::Number":"not-interpreted-as-json"}"#.to_vec(),
        },
    )]);
    store
        .commit_with_extensions(None, &initial, record(&initial), &opaque)
        .unwrap();
    let first = SQLiteWorkingStore::snapshot_read_only(&path).unwrap();
    assert_eq!(first.project, Some(initial.clone()));
    assert_eq!(
        SQLiteWorkingStore::backup_read_only(&path).unwrap(),
        first.database_bytes
    );
    let next = rename(&initial, "Revisão nova após snapshot congelado");
    store.commit(Some(1), &next, record(&next)).unwrap();
    let second = SQLiteWorkingStore::snapshot_read_only(&path).unwrap();
    assert_eq!(second.project, Some(next.clone()));
    assert_ne!(first.database_bytes, second.database_bytes);
    let frozen_path = dir.path().join("frozen.sqlite");
    restore_database(&frozen_path, &first.database_bytes).unwrap();
    let (frozen, project) = SQLiteWorkingStore::open_with_project(&frozen_path).unwrap();
    assert_eq!(project, Some(initial.clone()));
    let decoded =
        decode_package(&encode_package(&frozen, &BTreeMap::new(), origin()).unwrap()).unwrap();
    assert_eq!(decoded.project.extensions, initial.extensions);
    assert_eq!(decoded.extensions, opaque);
    assert_eq!(
        decoded.project.extensions["future.precision"]["integer"].to_string(),
        "18446744073709551617"
    );
    assert_eq!(
        decoded.project.extensions["future.precision"]["small"].to_string(),
        "1e-9999"
    );
    assert_eq!(
        decoded.project.extensions["future.precision"]["large"].to_string(),
        "1e+9999"
    );
    assert_eq!(
        model_snapshot(&decoded.project).unwrap().sha256,
        model_snapshot(&initial).unwrap().sha256
    );
}

#[test]
fn read_only_backup_is_an_image_not_a_claim_that_its_objects_are_valid() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("untrusted-image.sqlite");
    let initial = project();
    let mut store = SQLiteWorkingStore::open(&path).unwrap();
    store.commit(None, &initial, record(&initial)).unwrap();
    drop(store);
    let valid = SQLiteWorkingStore::snapshot_read_only(&path).unwrap();

    let database = Connection::open(&path).unwrap();
    let original_name = serde_json::to_vec(&initial.name).unwrap();
    let forged_name = serde_json::to_vec("Objeto alterado por ferramenta externa").unwrap();
    assert_eq!(
        database
            .execute(
                "UPDATE content_objects SET size = ?1, bytes = ?2 WHERE sha256 = ?3",
                rusqlite::params![
                    forged_name.len() as u32,
                    forged_name,
                    prumo_domain::canonical::hash_bytes(&original_name)
                ],
            )
            .unwrap(),
        1
    );
    drop(database);
    let before = fs::read(&path).unwrap();
    let image = SQLiteWorkingStore::backup_read_only(&path).unwrap();
    assert_ne!(image, valid.database_bytes);
    assert!(matches!(
        SQLiteWorkingStore::snapshot_read_only(&path),
        Err(StorageError::Integrity(message)) if message.contains("Objeto imutável adulterado")
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
