use prumo_app::{AccessMode, ProjectService};
use prumo_domain::{
    canonical::{hash_bytes, model_snapshot},
    storage::{StorageAction, StorageState},
};
use serde_json::json;
use std::fs;
use uuid::Uuid;

fn legacy_with_payload(payload: &str) -> Vec<u8> {
    let value = json!({
        "schema_version":1,"id":Uuid::new_v4(),"name":"Extensões futuras em JSON",
        "revision":1,"units":"metre","objects":[],"environments":[],
        "ports":[],"networks":[],"connections":[],
        "history":[{"revision":1,"actor":"fixture:legacy","operation":"create","timestamp_unix_ms":100}],
        "extensions":{"future.mep":"OPAQUE_PAYLOAD_MARKER"}
    });
    serde_json::to_string(&value)
        .unwrap()
        .replace("\"OPAQUE_PAYLOAD_MARKER\"", payload)
        .into_bytes()
}

#[test]
fn legacy_numbers_remain_exact_through_migration_edit_inspection_and_reopening() {
    let payload = r#"{"identity":18446744073709551617,"precise":0.12345678901234567890123456789,"underflow":1e-9999,"large":1e9999,"label":"$serde_json::private::Number"}"#;
    let raw = legacy_with_payload(payload);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("precision.prumo");
    fs::write(&path, &raw).unwrap();
    let service = ProjectService::open(&path, AccessMode::ReadWrite, "fixture:precision").unwrap();
    let original = service.read().unwrap();
    let values = &original.extensions["future.mep"];
    for (field, expected) in [
        ("identity", "18446744073709551617"),
        ("precise", "0.12345678901234567890123456789"),
        ("underflow", "1e-9999"),
        // serde_json may make a positive exponent sign explicit; precision
        // remains unchanged and the original file is preserved byte for byte.
        ("large", "1e+9999"),
    ] {
        assert_eq!(values[field].to_string(), expected, "{field}");
    }
    let snapshot = model_snapshot(&original).unwrap();
    assert!(snapshot.canonical_json.contains("\"underflow\":1e-9999"));
    let status = service.storage_status().unwrap();
    assert_eq!(status.state, StorageState::Legacy);
    let migrated = service
        .execute_storage(StorageAction::Migrate {
            expected_revision: 1,
            inspection_sha256: status.inspection_sha256,
            confirm: true,
        })
        .unwrap();
    assert_eq!(migrated.project.extensions, original.extensions);
    assert_eq!(model_snapshot(&migrated.project).unwrap(), snapshot);
    assert_eq!(
        fs::read(
            directory
                .path()
                .join(format!("prumo-v1-{}.backup", hash_bytes(&raw)))
        )
        .unwrap(),
        raw
    );
    let coordinate = 1.910_905_980_455_653_9_f64;
    service
        .add_member(1, "Precisão conhecida", [0.0; 3], [coordinate, 0.0, 0.0])
        .unwrap();
    let measured = service.inspect_model(2).unwrap();
    let reader = ProjectService::open(&path, AccessMode::ReadOnly, "fixture:reader").unwrap();
    let reopened = reader.read().unwrap();
    assert_eq!(reopened, measured);
    assert_eq!(reopened.extensions, original.extensions);
    assert_eq!(reopened.objects[0].end_m[0].to_bits(), coordinate.to_bits());
    assert_eq!(
        model_snapshot(&reopened).unwrap(),
        model_snapshot(&measured).unwrap()
    );
    reopened.validate().unwrap();
}

#[test]
fn reserved_json_objects_are_rejected_before_migration_without_altering_the_source() {
    for payload in [
        r#"{"$serde_json::private::Number":"1"}"#,
        r#"{"nested":{"\u0024serde_json::private::Number":"1"}}"#,
    ] {
        let raw = legacy_with_payload(payload);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("reserved.prumo");
        fs::write(&path, &raw).unwrap();
        let service =
            ProjectService::open(&path, AccessMode::ReadWrite, "fixture:reserved").unwrap();
        assert!(
            service
                .read()
                .unwrap_err()
                .to_string()
                .contains("namespace")
        );
        let status = service.storage_status().unwrap();
        assert_eq!(status.state, StorageState::Unsupported);
        assert!(!status.can_migrate && !status.can_backup && !status.can_recover);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        assert!(
            service
                .execute_storage(StorageAction::Migrate {
                    expected_revision: 1,
                    inspection_sha256: status.inspection_sha256,
                    confirm: true,
                })
                .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), raw);
    }
}

#[test]
fn arbitrarily_large_schema_numbers_remain_unsupported_and_preserve_the_source() {
    for version in ["1e9999", "18446744073709551617"] {
        let raw = String::from_utf8(legacy_with_payload("null"))
            .unwrap()
            .replace(
                "\"schema_version\":1",
                &format!("\"schema_version\":{version}"),
            )
            .into_bytes();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("future.prumo");
        fs::write(&path, &raw).unwrap();
        let service = ProjectService::open(&path, AccessMode::ReadWrite, "fixture:future").unwrap();
        let error = service.read().unwrap_err();
        assert_eq!(error.code(), "unsupported_schema");
        let diagnostic_number = if version == "1e9999" {
            "1e+9999"
        } else {
            version
        };
        assert!(error.to_string().contains(diagnostic_number));
        let status = service.storage_status().unwrap();
        assert_eq!(status.state, StorageState::Unsupported);
        assert!(!status.can_migrate && !status.can_recover && !status.can_backup);
        assert!(
            service
                .execute_storage(StorageAction::Migrate {
                    expected_revision: 1,
                    inspection_sha256: status.inspection_sha256,
                    confirm: true,
                })
                .is_err()
        );
        assert_eq!(fs::read(path).unwrap(), raw);
    }
}
