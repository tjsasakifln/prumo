use prumo_app::{AccessMode, ProjectService};
use prumo_domain::{LengthUnit, canonical::model_snapshot, transactions::TransactionRequest};

#[test]
fn scientific_member_coordinates_survive_preview_restart_apply_and_replay() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("numbers.prumo");
    let service = ProjectService::open(&path, AccessMode::ReadWrite, "operador").unwrap();
    let project = service
        .create("Coordenadas pequenas", LengthUnit::Metre)
        .unwrap();
    let key = uuid::Uuid::new_v4();
    let raw = format!(
        r#"{{"project_id":"{}","expected_revision":1,"idempotency_key":"{key}","operations":[{{"operation":"add_member","expected_revision":1,"name":"Viga micrométrica","start":[1.00e-7,0,0],"end":[4,2.500e-7,0]}}]}}"#,
        project.id
    );
    let request: TransactionRequest = serde_json::from_str(&raw).unwrap();
    let from_value: TransactionRequest =
        serde_json::from_value(serde_json::from_str(&raw).unwrap()).unwrap();
    let initial = std::fs::read(&path).unwrap();
    let preview = service.preview_transaction(request).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), initial);
    drop(service);
    let reopened = ProjectService::open(&path, AccessMode::ReadWrite, "operador").unwrap();
    let applied = reopened.apply_transaction(&preview.plan_sha256).unwrap();
    assert_eq!(
        applied.project.objects[0].start_m.as_array(),
        [1e-7, 0.0, 0.0]
    );
    assert_eq!(
        applied.project.objects[0].end_m.as_array(),
        [4.0, 2.5e-7, 0.0]
    );
    assert_eq!(
        model_snapshot(&applied.project).unwrap().sha256,
        preview.after_snapshot_sha256
    );
    assert_eq!(reopened.read().unwrap(), applied.project);
    let repeated = reopened.execute_transaction(from_value).unwrap();
    assert!(repeated.replayed);
    assert_eq!(repeated.project, applied.project);
}
