use super::*;
use uuid::Uuid;

#[tokio::test]
async fn preview_apply_replay_undo_redo_survive_a_real_mcp_reconnection() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("mcp-transactions.prumo");
    let host = ProjectService::open(&path, AccessMode::ReadWrite, "host").unwrap();
    let project = host.create("Lotes", LengthUnit::Centimetre).unwrap();
    let request = json!({
        "project_id":project.id,"expected_revision":project.revision,"idempotency_key":Uuid::new_v4(),
        "operations":[
            {"operation":"add_member","expected_revision":project.revision,"name":"Viga em centímetros","start":[0,0,0],"end":[400,0,0]},
            {"operation":"add_environment","expected_revision":project.revision,"name":"Ambiente do lote"}
        ]
    });
    let client = sdk(&path, true).await;
    let preview = call(&client, "prumo.model.preview_change", request).await;
    assert_eq!(host.read().unwrap(), project);
    assert_eq!(preview["preview"]["operation_count"], 2);
    let plan = preview["preview"]["plan_sha256"].clone();
    let applied = call(
        &client,
        "prumo.model.apply_change",
        json!({"plan_sha256":plan}),
    )
    .await;
    assert_eq!(applied["replayed"], false);
    assert_eq!(applied["project"]["revision"], project.revision + 1);
    assert_eq!(applied["project"]["objects"].as_array().unwrap().len(), 1);
    assert_eq!(
        applied["project"]["environments"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        applied["project"]["objects"][0]["end_m"],
        json!([4.0, 0.0, 0.0])
    );
    let object_id = applied["project"]["objects"][0]["id"].clone();
    let analytical_id = applied["project"]["model"]["links"][0]["analytical_id"].clone();
    let command_id = applied["receipt"]["command_id"].clone();
    client.cancel().await.unwrap();

    let client = sdk(&path, true).await;
    let replay = call(
        &client,
        "prumo.model.apply_change",
        json!({"plan_sha256":plan}),
    )
    .await;
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["receipt"], applied["receipt"]);
    assert_eq!(replay["project"], applied["project"]);
    let history = call(&client, "prumo.model.history", json!({})).await;
    assert_eq!(history["history"]["undo_command_id"], command_id);
    let undone = call(&client, "prumo.model.undo", json!({"project_id":project.id,
        "expected_revision":replay["project"]["revision"],"idempotency_key":Uuid::new_v4(),"target_command_id":command_id})).await;
    assert!(undone["project"]["objects"].as_array().unwrap().is_empty());
    assert!(
        undone["project"]["environments"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let history = call(&client, "prumo.model.history", json!({})).await;
    let redone = call(
        &client,
        "prumo.model.redo",
        json!({"project_id":project.id,
        "expected_revision":undone["project"]["revision"],"idempotency_key":Uuid::new_v4(),
        "target_command_id":history["history"]["redo_command_id"]}),
    )
    .await;
    assert_eq!(redone["project"]["objects"][0]["id"], object_id);
    assert_eq!(
        redone["project"]["model"]["links"][0]["analytical_id"],
        analytical_id
    );
    assert_eq!(redone["project"]["units"], "centimetre");
    client.cancel().await.unwrap();
    let desktop = Desktop::new(path);
    assert_eq!(
        serde_json::to_value(desktop.project.unwrap()).unwrap(),
        redone["project"]
    );
}

#[tokio::test]
async fn atomic_batch_rejects_middle_failure_and_stale_plan_without_writes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("mcp-atomic.prumo");
    let host = ProjectService::open(&path, AccessMode::ReadWrite, "host").unwrap();
    let project = host.create("Atomicidade", LengthUnit::Metre).unwrap();
    let before_bytes = std::fs::read(&path).unwrap();
    let client = sdk(&path, true).await;
    let failed = client.call_tool(CallToolRequestParams::new("prumo.model.preview_change").with_arguments(json!({
        "project_id":project.id,"expected_revision":project.revision,"idempotency_key":Uuid::new_v4(),"operations":[
            {"operation":"add_environment","expected_revision":project.revision,"name":"Não persistir"},
            {"operation":"add_member","expected_revision":project.revision,"name":"Barra inválida","start":[0,0,0],"end":[0,0,0]}
        ]
    }).as_object().unwrap().clone())).await.unwrap();
    assert_eq!(failed.is_error, Some(true));
    assert_eq!(std::fs::read(&path).unwrap(), before_bytes);
    let preview = call(&client, "prumo.model.preview_change", json!({
        "project_id":project.id,"expected_revision":project.revision,"idempotency_key":Uuid::new_v4(),"operations":[
            {"operation":"add_environment","expected_revision":project.revision,"name":"Plano obsoleto"}
        ]
    })).await;
    let latest = host
        .rename_project(project.revision, "Alteração concorrente")
        .unwrap();
    let rejected = client
        .call_tool(
            CallToolRequestParams::new("prumo.model.apply_change").with_arguments(
                json!({"plan_sha256":preview["preview"]["plan_sha256"]})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(rejected.is_error, Some(true));
    assert_eq!(host.read().unwrap(), latest);
    assert!(latest.environments.is_empty());
    client.cancel().await.unwrap();
}
