// Ensaio administrativo de uma execução por operação. Não integra o runtime.
// A edição fixture-next é sintética e não representa norma publicada.

use prumo_app::{AccessMode, ProjectService};
use prumo_storage::{PackageOrigin, SQLiteWorkingStore, decode_package, encode_package};
use serde_json::json;
use std::{collections::BTreeMap, time::Instant};
fn measure<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let out = f();
    println!(
        "{}",
        json!({"operation":label,"elapsed_ms":t.elapsed().as_secs_f64()*1000.0})
    );
    out
}
fn main() {
    let folder = std::env::args_os().nth(1).unwrap();
    let folder = std::path::PathBuf::from(folder);
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("latency.prumo");
    let service = ProjectService::open(path.clone(), AccessMode::ReadWrite, "fixture").unwrap();
    let created = measure("create", || {
        service
            .create("Migração de teste", prumo_domain::LengthUnit::Metre)
            .unwrap()
    });
    let candidate = measure("norms.package.before_pin", || {
        service.normative_package_preview().unwrap()
    });
    let pinned = measure("norms.pin", || {
        service
            .pin_normative_package(created.revision, &candidate.candidate.hash)
            .unwrap()
    });
    let _pinned_read = measure("project.read.pinned", || service.read().unwrap());
    let _pinned_status = measure("storage.status.pinned", || {
        service.storage_status().unwrap()
    });
    let reference = prumo_domain::registry::NormReferenceInput {
        id: "nbr-6118".into(),
        designation: "ABNT NBR 6118".into(),
        part: None,
        edition: Some("fixture-next".into()),
        amendments: vec![],
        errata: vec![],
        title: "Metadado sintético para ensaio de migração; não é edição publicada".into(),
        publisher: "Fonte de teste".into(),
        source_url: "https://example.invalid/fixture".into(),
        domain: "structure".into(),
        interpretation_owner_role: "Papel de teste sem aprovação profissional".into(),
    };
    let updated = measure("norms.register", || {
        service
            .register_norm_reference(pinned.revision, reference)
            .unwrap()
    });
    let migration_candidate = measure("norms.package.before_migrate", || {
        service.normative_package_preview().unwrap()
    });
    let updated = measure("norms.migrate", || {
        service
            .migrate_normative_package(
                updated.revision,
                &candidate.candidate.hash,
                &migration_candidate.candidate.hash,
                true,
            )
            .unwrap()
    });
    let _read = measure("project.read.updated", || service.read().unwrap());
    let _status = measure("storage.status.updated", || {
        service.storage_status().unwrap()
    });
    let bytes = std::fs::read(&path).unwrap();
    println!(
        "{}",
        json!({"package_bytes":bytes.len(),"project_revision":updated.revision})
    );
    let decoded = measure("decode_package.updated", || decode_package(&bytes).unwrap());
    let database = std::fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().path().join("state.sqlite"))
        .find(|entry| entry.is_file())
        .unwrap();
    let store = measure("open_read_only.updated", || {
        SQLiteWorkingStore::open_read_only(&database).unwrap()
    });
    let _project = measure("load.after_open.updated", || store.load().unwrap());
    let _opened = measure("open_with_project.updated", || {
        SQLiteWorkingStore::open_with_project(&database).unwrap()
    });
    let _snapshot = measure("snapshot_read_only.updated", || {
        SQLiteWorkingStore::snapshot_read_only(&database).unwrap()
    });
    let _image = measure("backup_bytes.updated", || store.backup_bytes().unwrap());
    let _encoded = measure("encode_package.updated", || {
        encode_package(
            &store,
            &BTreeMap::new(),
            PackageOrigin {
                application: "Prumo".into(),
                version: "0.1.0".into(),
                platform: "linux".into(),
            },
        )
        .unwrap()
    });
    assert_eq!(decoded.project.revision, updated.revision);
}
