use super::*;
use prumo_domain::Project;
use prumo_domain::storage::{CandidateKind, StorageStatus};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn legacy(path: &Path) -> Project {
    let seed = tempfile::tempdir().unwrap();
    let host = ProjectService::open(
        seed.path().join("seed.prumo"),
        AccessMode::ReadWrite,
        "legacy-mcp-fixture",
    )
    .unwrap();
    host.create("Legado preservado", LengthUnit::Millimetre)
        .unwrap();
    let mut project = host
        .add_member(1, "Objeto legado", [0.0; 3], [4000.0, 0.0, 0.0])
        .unwrap();
    project.extensions.insert(
        "future-mep-module".into(),
        json!({"version":31,"payload":{"circuit":"uninterpreted","values":[1,2,3]}}),
    );
    project.validate().unwrap();
    std::fs::write(path, serde_json::to_vec(&project).unwrap()).unwrap();
    project
}

fn files(directory: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(base: &Path, current: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(base, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(base).unwrap().to_path_buf(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(directory, directory, &mut result);
    result
}

async fn status(client: &RunningService<RoleClient, SdkClient>) -> StorageStatus {
    serde_json::from_value(call(client, "prumo.storage.status", json!({})).await["storage"].clone())
        .unwrap()
}

async fn refused(client: &RunningService<RoleClient, SdkClient>, name: &str, args: Value) -> Value {
    let response = timeout(
        LIMIT,
        client.call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(args.as_object().unwrap().clone()),
        ),
    )
    .await
    .expect("storage refusal timeout")
    .unwrap();
    assert_eq!(response.is_error, Some(true), "{response:?}");
    response.structured_content.unwrap()
}

#[tokio::test]
async fn real_mcp_migrates_legacy_preserves_original_and_refuses_readonly_mutations() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.prumo");
    let expected = legacy(&path);
    let bytes = std::fs::read(&path).unwrap();
    let initial_files = files(directory.path());
    let readonly = sdk(&path, false).await;
    let diagnosis = status(&readonly).await;
    assert_eq!(serde_json::to_value(&diagnosis).unwrap()["state"], "legacy");
    assert!(!diagnosis.can_migrate);
    assert_eq!(files(directory.path()), initial_files);
    let project = call(&readonly, "prumo.project.read", json!({})).await;
    assert_eq!(project["project"], serde_json::to_value(&expected).unwrap());
    for (name, args) in [
        (
            "prumo.storage.migrate",
            json!({"expected_revision":expected.revision,"inspection_sha256":diagnosis.inspection_sha256,"confirm":true}),
        ),
        (
            "prumo.storage.backup",
            json!({"expected_revision":expected.revision,"inspection_sha256":diagnosis.inspection_sha256}),
        ),
        (
            "prumo.storage.recover",
            json!({"candidate_id":"untrusted","inspection_sha256":diagnosis.inspection_sha256,"confirm":true}),
        ),
    ] {
        assert_eq!(
            refused(&readonly, name, args).await["code"],
            "permission_denied"
        );
    }
    assert_eq!(files(directory.path()), initial_files);
    readonly.cancel().await.unwrap();
    let client = sdk(&path, true).await;
    let diagnosis = status(&client).await;
    refused(&client,"prumo.storage.migrate",json!({"expected_revision":expected.revision,"inspection_sha256":diagnosis.inspection_sha256,"confirm":false})).await;
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let diagnosis = status(&client).await;
    let migrated=call(&client,"prumo.storage.migrate",json!({"expected_revision":expected.revision,"inspection_sha256":diagnosis.inspection_sha256,"confirm":true})).await;
    assert_eq!(migrated["storage"]["format"], "native_package");
    assert_eq!(migrated["calculation_available"], false);
    assert_eq!(migrated["project"]["id"], expected.id.to_string());
    assert_eq!(
        migrated["project"]["objects"],
        serde_json::to_value(&expected.objects).unwrap()
    );
    assert_eq!(
        migrated["project"]["extensions"],
        serde_json::to_value(&expected.extensions).unwrap()
    );
    let after: StorageStatus = serde_json::from_value(migrated["storage"].clone()).unwrap();
    assert!(
        after
            .candidates
            .iter()
            .any(|candidate| candidate.kind == CandidateKind::Original
                && candidate.sha256 == prumo_domain::canonical::hash_bytes(&bytes))
    );
    assert!(std::fs::read(&path).unwrap().starts_with(b"PK"));
    client.cancel().await.unwrap();
    let desktop = Desktop::new(path.clone());
    assert_eq!(
        serde_json::to_value(desktop.project.unwrap()).unwrap(),
        migrated["project"]
    );
    let readonly = sdk(&path, false).await;
    assert_eq!(
        call(&readonly, "prumo.project.read", json!({})).await["project"],
        migrated["project"]
    );
    readonly.cancel().await.unwrap();
}

#[tokio::test]
async fn actual_ui_saved_package_is_backed_up_and_recovered_by_real_mcp_after_corruption() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("recover.prumo");
    let mut desktop = Desktop::new(path.clone());
    {
        let mut ui = Harness::builder()
            .with_size(egui::vec2(1500.0, 1200.0))
            .build_ui(|ui| desktop.render(ui));
        for label in ["Criar projeto", "Adicionar peça e salvar"] {
            ui.get_by_label(label).click();
            ui.run();
        }
    }
    assert!(!desktop.view.is_error, "{}", desktop.view.notice);
    let expected = desktop.project.unwrap();
    let client = sdk(&path, true).await;
    let diagnosis = status(&client).await;
    let backup=call(&client,"prumo.storage.backup",json!({"expected_revision":expected.revision,"inspection_sha256":diagnosis.inspection_sha256})).await;
    assert_eq!(backup["project"], serde_json::to_value(&expected).unwrap());
    client.cancel().await.unwrap();
    let corrupt = b"PK altered after completed backup";
    std::fs::write(&path, corrupt).unwrap();
    let readonly = sdk(&path, false).await;
    let diagnosis = status(&readonly).await;
    assert!(!diagnosis.can_recover);
    let candidate = diagnosis
        .candidates
        .iter()
        .find(|candidate| candidate.kind == CandidateKind::Backup)
        .unwrap()
        .clone();
    let before = files(directory.path());
    let denied=refused(&readonly,"prumo.storage.recover",json!({"candidate_id":candidate.id,"inspection_sha256":diagnosis.inspection_sha256,"confirm":true})).await;
    assert_eq!(denied["code"], "permission_denied");
    assert_eq!(files(directory.path()), before);
    readonly.cancel().await.unwrap();
    let client = sdk(&path, true).await;
    let diagnosis = status(&client).await;
    assert!(diagnosis.can_recover);
    refused(&client,"prumo.storage.recover",json!({"candidate_id":candidate.id,"inspection_sha256":diagnosis.inspection_sha256,"confirm":false})).await;
    refused(
        &client,
        "prumo.storage.recover",
        json!({"candidate_id":candidate.id,"inspection_sha256":"0".repeat(64),"confirm":true}),
    )
    .await;
    assert_eq!(std::fs::read(&path).unwrap(), corrupt);
    let diagnosis = status(&client).await;
    let restored=call(&client,"prumo.storage.recover",json!({"candidate_id":candidate.id,"inspection_sha256":diagnosis.inspection_sha256,"confirm":true})).await;
    assert_eq!(restored["project"]["id"], expected.id.to_string());
    assert_eq!(
        restored["project"]["objects"],
        serde_json::to_value(&expected.objects).unwrap()
    );
    assert_eq!(
        restored["project"]["model"],
        serde_json::to_value(&expected.model).unwrap()
    );
    assert_eq!(restored["storage"]["format"], "native_package");
    client.cancel().await.unwrap();
    let desktop = Desktop::new(path);
    assert_eq!(
        serde_json::to_value(desktop.project.unwrap()).unwrap(),
        restored["project"]
    );
}

#[tokio::test]
async fn real_mcp_diagnoses_missing_corrupt_and_future_files_without_writing_and_blocks_stale_tokens()
 {
    let directory = tempfile::tempdir().unwrap();
    for (name, bytes, expected_state) in [
        ("missing", None, "missing"),
        ("corrupt", Some(b"PK invalid archive".as_slice()), "invalid"),
        (
            "future",
            Some(
                include_bytes!(
                    "../../../../crates/prumo-storage/tests/fixtures/future-format.prumo"
                )
                .as_slice(),
            ),
            "unsupported",
        ),
    ] {
        let folder = directory.path().join(name);
        std::fs::create_dir(&folder).unwrap();
        let path = folder.join("selected.prumo");
        if let Some(bytes) = bytes {
            std::fs::write(&path, bytes).unwrap();
        }
        let before = files(&folder);
        let client = sdk(&path, false).await;
        let diagnosis = status(&client).await;
        assert_eq!(
            serde_json::to_value(&diagnosis).unwrap()["state"],
            expected_state
        );
        assert_eq!(diagnosis.inspection_sha256.len(), 64);
        assert!(!diagnosis.can_migrate && !diagnosis.can_backup && !diagnosis.can_recover);
        if expected_state != "missing" {
            assert!(!diagnosis.diagnostics.is_empty());
        }
        assert_eq!(files(&folder), before);
        client.cancel().await.unwrap();
        let mut wire = WireClient::new(&path, false);
        let extra = wire
            .request(
                "tools/call",
                json!({"name":"prumo.storage.status","arguments":{"path":"outside.prumo"}}),
            )
            .await;
        assert_eq!(extra["error"]["code"], -32602);
        wire.stop().await;
        assert_eq!(files(&folder), before);
    }
    let path = directory.path().join("stale.prumo");
    let client = sdk(&path, true).await;
    let created = call(
        &client,
        "prumo.project.create",
        json!({"name":"Token de inspeção","units":"metre"}),
    )
    .await;
    let old = status(&client).await;
    let changed = call(
        &client,
        "prumo.project.rename",
        json!({"expected_revision":created["project"]["revision"],"name":"Revisão nova"}),
    )
    .await;
    let bytes = std::fs::read(&path).unwrap();
    let denied=refused(&client,"prumo.storage.backup",json!({"expected_revision":changed["project"]["revision"],"inspection_sha256":old.inspection_sha256})).await;
    assert_eq!(denied["code"], "storage_conflict");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let fresh = status(&client).await;
    let backed=call(&client,"prumo.storage.backup",json!({"expected_revision":changed["project"]["revision"],"inspection_sha256":fresh.inspection_sha256})).await;
    assert_eq!(backed["project"], changed["project"]);
    client.cancel().await.unwrap();
}
