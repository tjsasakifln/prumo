//! Actual child-process interoperability, with SDK and independent JSON wire clients.
use std::{path::Path, process::Stdio, time::Duration};

use egui_kittest::{Harness, kittest::Queryable};
use prumo_app::{AccessMode, ProjectService};
use prumo_desktop::Desktop;
use prumo_domain::LengthUnit;
use rmcp::{
    ClientHandler, ClientLifecycleMode, ClientServiceExt, RoleClient,
    model::{CallToolRequestParams, ProtocolVersion},
    service::RunningService,
    transport::TokioChildProcess,
};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};

const LIMIT: Duration = Duration::from_secs(10);
const BINARY: &str = env!("CARGO_BIN_EXE_prumo-mcp");

#[derive(Clone)]
struct SdkClient;
impl ClientHandler for SdkClient {}

async fn sdk(path: &Path, writable: bool) -> RunningService<RoleClient, SdkClient> {
    let mut command = Command::new(BINARY);
    command.arg("--project").arg(path);
    if writable {
        command.arg("--allow-write");
    }
    timeout(
        LIMIT,
        SdkClient.serve_with_lifecycle(
            TokioChildProcess::new(command).expect("spawn server"),
            ClientLifecycleMode::Discover {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
            },
        ),
    )
    .await
    .expect("discovery timeout")
    .expect("discover server")
}

async fn call(client: &RunningService<RoleClient, SdkClient>, name: &str, args: Value) -> Value {
    let result = timeout(
        LIMIT,
        client.call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(args.as_object().expect("argument object").clone()),
        ),
    )
    .await
    .expect("request timeout")
    .expect("tool response");
    assert_eq!(result.is_error, Some(false), "{result:?}");
    result.structured_content.expect("structured content")
}

#[tokio::test]
async fn sdk_creates_modifies_and_reopens_real_project() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sdk.prumo");
    let client = sdk(&path, true).await;
    let info = client.peer_info().unwrap();
    assert_eq!(info.protocol_version, ProtocolVersion::V_2026_07_28);
    let tools = client.list_tools(None).await.unwrap();
    assert_eq!(tools.tools.len(), 12);
    assert!(
        tools
            .tools
            .iter()
            .all(|tool| tool.input_schema["additionalProperties"] == false)
    );
    let created = call(
        &client,
        "prumo.project.create",
        json!({"name":"Edifício teste","units":"centimetre"}),
    )
    .await;
    let revision = created["project"]["revision"].as_u64().unwrap();
    let added = call(
        &client,
        "prumo.model.add_member",
        json!({"expected_revision":revision,"name":"V1","start":[0,0,0],"end":[400,0,0]}),
    )
    .await;
    let object = &added["project"]["objects"][0];
    assert_eq!(object["end_m"], json!([4.0, 0.0, 0.0]));
    assert_eq!(object["state"], "modelled");
    assert_eq!(added["calculation_available"], false);
    let physical_id = object["id"].clone();
    let analytical_id = object["analytical_id"].clone();
    assert_ne!(physical_id, analytical_id);
    client.cancel().await.unwrap();

    let reopened = sdk(&path, false).await;
    let read = call(&reopened, "prumo.project.read", json!({})).await;
    assert_eq!(read, added);
    assert_eq!(read["project"]["id"], created["project"]["id"]);
    assert_eq!(read["project"]["objects"][0]["id"], physical_id);
    assert_eq!(
        read["project"]["objects"][0]["analytical_id"],
        analytical_id
    );
    reopened.cancel().await.unwrap();
}

#[tokio::test]
async fn actual_ui_save_restart_mcp_modify_and_ui_reopen_preserve_identity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("shared.prumo");
    let mut desktop = Desktop::new(path.clone());
    desktop.view.project_name = "Criado pela interface".into();
    desktop.view.member_name = "P1".into();
    desktop.view.start = [0.0; 3];
    desktop.view.end = [0.0, 0.0, 3.0];
    {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1120.0, 780.0))
            .build_ui(|ui| desktop.render(ui));
        harness.get_by_label("Criar projeto").click();
        harness.run();
        harness.get_by_label("Adicionar peça e salvar").click();
        harness.run();
    }
    let initial = desktop
        .project
        .clone()
        .expect("project created by UI clicks");
    assert_eq!(initial.objects.len(), 1);
    drop(desktop);
    let client = sdk(&path, true).await;
    let read = call(&client, "prumo.project.read", json!({})).await;
    assert_eq!(read["project"], serde_json::to_value(&initial).unwrap());
    let changed = call(
        &client,
        "prumo.project.rename",
        json!({"expected_revision":initial.revision,"name":"Alterado pelo MCP"}),
    )
    .await;
    let environment = call(
        &client,
        "prumo.model.add_environment",
        json!({"expected_revision":changed["project"]["revision"],"name":"Térreo"}),
    )
    .await;
    client.cancel().await.unwrap();
    let mut desktop = Desktop::new(path);
    let reopened_ui = desktop.project.clone().expect("UI reopens same project");
    assert_eq!(
        serde_json::to_value(&reopened_ui).unwrap(),
        environment["project"]
    );
    assert_eq!(reopened_ui.id, initial.id);
    assert_eq!(
        reopened_ui.objects[0].analytical_id,
        initial.objects[0].analytical_id
    );
    desktop.view.end = desktop.view.start;
    {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1120.0, 780.0))
            .build_ui(|ui| desktop.render(ui));
        harness.get_by_label("Adicionar peça e salvar").click();
        harness.run();
    }
    assert!(desktop.view.is_error);
    assert_eq!(desktop.project.unwrap(), reopened_ui);
}

#[tokio::test]
async fn readonly_is_enforced_for_unadvertised_mutations_and_creation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("protected.prumo");
    let client = sdk(&path, false).await;
    let tools = client.list_tools(None).await.unwrap();
    assert_eq!(tools.tools.len(), 5);
    assert_eq!(tools.tools[0].name, "prumo.project.read");
    let denied = client
        .call_tool(
            CallToolRequestParams::new("prumo.project.create").with_arguments(
                json!({"name":"Não autorizado","units":"metre"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(denied.is_error, Some(true));
    assert_eq!(
        denied.structured_content.unwrap()["code"],
        "permission_denied"
    );
    assert!(!path.exists());
    client.cancel().await.unwrap();

    let host = ProjectService::open(&path, AccessMode::ReadWrite, "host").unwrap();
    let before = host.create("Protegido", LengthUnit::Metre).unwrap();
    let client = sdk(&path, false).await;
    for (name, args) in [
        (
            "prumo.project.rename",
            json!({"expected_revision":before.revision,"name":"Recusado"}),
        ),
        (
            "prumo.model.add_member",
            json!({"expected_revision":before.revision,"name":"Recusado","start":[0,0,0],"end":[1,0,0]}),
        ),
        (
            "prumo.model.add_environment",
            json!({"expected_revision":before.revision,"name":"Recusado"}),
        ),
    ] {
        let denied = client
            .call_tool(
                CallToolRequestParams::new(name).with_arguments(args.as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        assert_eq!(denied.is_error, Some(true));
        assert_eq!(
            denied.structured_content.unwrap()["code"],
            "permission_denied"
        );
    }
    assert_eq!(host.read().unwrap(), before);
    client.cancel().await.unwrap();
}

/// Uses only JSON and stdio, independently of rmcp's wire types and transport.
struct WireClient {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    id: u64,
}

impl WireClient {
    fn new(path: &Path, writable: bool) -> Self {
        let mut command = Command::new(BINARY);
        command
            .arg("--project")
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if writable {
            command.arg("--allow-write");
        }
        let mut child = command.spawn().unwrap();
        Self {
            input: child.stdin.take(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
            id: 0,
        }
    }

    async fn request_version(&mut self, method: &str, mut params: Value, version: &str) -> Value {
        params["_meta"] = json!({
            "io.modelcontextprotocol/protocolVersion":version,
            "io.modelcontextprotocol/clientInfo":{"name":"prumo-independent-wire-client","version":"1.0.0"},
            "io.modelcontextprotocol/clientCapabilities":{}
        });
        self.id += 1;
        self.send(json!({"jsonrpc":"2.0","id":self.id,"method":method,"params":params}))
            .await;
        let value = self.receive().await;
        assert_eq!(value["id"], self.id);
        assert_eq!(value["jsonrpc"], "2.0");
        value
    }

    async fn request(&mut self, method: &str, params: Value) -> Value {
        self.request_version(method, params, "2026-07-28").await
    }

    async fn send(&mut self, value: Value) {
        let mut line = serde_json::to_vec(&value).unwrap();
        line.push(b'\n');
        self.input.as_mut().unwrap().write_all(&line).await.unwrap();
        self.input.as_mut().unwrap().flush().await.unwrap();
    }

    async fn receive(&mut self) -> Value {
        let mut line = String::new();
        let bytes = timeout(LIMIT, self.output.read_line(&mut line))
            .await
            .expect("wire response timeout")
            .unwrap();
        assert!(bytes > 0, "server closed stdout before responding");
        serde_json::from_str(&line).expect("stdout contains protocol JSON only")
    }

    async fn stop(mut self) {
        drop(self.input.take());
        let status = timeout(LIMIT, self.child.wait())
            .await
            .expect("server exits on EOF")
            .unwrap();
        assert!(status.success(), "server exit {status}");
    }
}

#[tokio::test]
async fn independent_wire_client_checks_contract_failures_without_modification() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wire.prumo");
    let mut client = WireClient::new(&path, true);
    let discover = client.request("server/discover", json!({})).await;
    assert_eq!(
        discover["result"]["supportedVersions"],
        json!(["2026-07-28"])
    );
    assert_eq!(discover["result"]["resultType"], "complete");
    let create = client
        .request(
            "tools/call",
            json!({"name":"prumo.project.create","arguments":{"name":"Wire","units":"metre"}}),
        )
        .await;
    let project = create["result"]["structuredContent"]["project"].clone();
    let revision = project["revision"].clone();
    for args in [
        json!({"expected_revision":revision,"name":"X","start":[0,0,0],"end":[1,0,0],"path":"../out.prumo"}),
        json!({"expected_revision":revision,"name":"X","start":[0,0],"end":[1,0,0]}),
        json!({"expected_revision":-1,"name":"X","start":[0,0,0],"end":[1,0,0]}),
    ] {
        let response = client
            .request(
                "tools/call",
                json!({"name":"prumo.model.add_member","arguments":args}),
            )
            .await;
        assert_eq!(response["error"]["code"], -32602);
    }
    let invalid_geometry = client.request("tools/call",json!({"name":"prumo.model.add_member","arguments":{"expected_revision":revision,"name":"X","start":[0,0,0],"end":[0,0,0]}})).await;
    assert_eq!(invalid_geometry["result"]["isError"], true);
    let stale = client.request("tools/call",json!({"name":"prumo.project.rename","arguments":{"expected_revision":0,"name":"Stale"}})).await;
    assert_eq!(
        stale["result"]["structuredContent"]["code"],
        "revision_conflict"
    );
    let unexpected = client
        .request(
            "tools/call",
            json!({"name":"shell","arguments":{"command":"ignored"}}),
        )
        .await;
    assert_eq!(unexpected["error"]["code"], -32602);
    let read = client
        .request(
            "tools/call",
            json!({"name":"prumo.project.read","arguments":{}}),
        )
        .await;
    assert_eq!(read["result"]["structuredContent"]["project"], project);
    client.stop().await;
}

#[tokio::test]
async fn unknown_version_and_missing_request_metadata_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("version.prumo");
    let mut client = WireClient::new(&path, false);
    client.request("server/discover", json!({})).await;
    let unknown = client
        .request_version("tools/list", json!({}), "2099-01-01")
        .await;
    assert_eq!(unknown["error"]["code"], -32022);
    assert_eq!(unknown["error"]["data"]["supported"], json!(["2026-07-28"]));
    client
        .send(json!({"jsonrpc":"2.0","id":999,"method":"tools/list","params":{}}))
        .await;
    let missing = client.receive().await;
    assert_eq!(missing["error"]["code"], -32602);
    let recovered = client.request("tools/list", json!({})).await;
    assert_eq!(recovered["result"]["tools"].as_array().unwrap().len(), 5);
    client.stop().await;
}

#[tokio::test]
async fn oversized_message_cannot_mutate_project_and_transport_recovers() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bounded.prumo");
    let mut client = WireClient::new(&path, true);
    client.request("server/discover", json!({})).await;
    client.send(json!({"jsonrpc":"2.0","id":500,"method":"tools/call","params":{"name":"prumo.project.create","arguments":{"name":"x".repeat(70 * 1024),"units":"metre"}}})).await;
    let recovered = client.request("tools/list", json!({})).await;
    assert!(recovered["result"]["tools"].is_array());
    assert!(!path.exists());
    client.stop().await;
}

#[tokio::test]
async fn cli_requires_selected_project_and_does_not_print_human_text_to_stdout() {
    for args in [vec![], vec!["--help"], vec!["--project"], vec!["--shell"]] {
        let output = Command::new(BINARY).args(&args).output().await.unwrap();
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert_eq!(output.status.success(), args == ["--help"]);
    }
}

#[tokio::test]
async fn registry_inventory_exposes_all_families_without_normative_approval() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("registry.prumo");
    let client = sdk(&path, true).await;
    call(
        &client,
        "prumo.project.create",
        json!({"name":"Registro normativo","units":"metre"}),
    )
    .await;
    let registry = call(&client, "prumo.registry.read", json!({})).await;
    let capabilities = registry["catalog"]["capabilities"].as_array().unwrap();
    let families: std::collections::BTreeSet<_> = capabilities
        .iter()
        .map(|capability| capability["family"].as_str().unwrap())
        .collect();
    assert_eq!(families.len(), 72);
    assert!(capabilities.len() > families.len());
    assert!(capabilities.iter().all(
        |capability| capability["state"] == "planned" && capability["qualification"].is_null()
    ));
    assert!(
        registry["catalog"]["norms"]
            .as_array()
            .unwrap()
            .iter()
            .all(|norm| norm["access"] == "public_metadata_only"
                && norm["licensed_evidence"].is_null())
    );
    let audit = call(&client, "prumo.registry.audit", json!({})).await;
    assert_eq!(audit["audit"]["present_families"], 72);
    assert!(
        audit["audit"]["errors"].as_array().unwrap().is_empty(),
        "{audit}"
    );
    assert!(!audit["audit"]["pending"].as_array().unwrap().is_empty());
    let gate = call(
        &client,
        "prumo.results.gate",
        json!({"capability_id":"PAR-06-calculate"}),
    )
    .await;
    assert_eq!(gate["gate"]["allowed"], false);
    client.cancel().await.unwrap();
}

fn reference_fixture() -> Value {
    json!({
        "id":"nbr-6118","designation":"ABNT NBR 6118","part":null,
        "edition":"fixture-next","amendments":[],"errata":[],
        "title":"Metadado sintético para ensaio de migração; não é edição publicada",
        "publisher":"Fonte de teste","source_url":"https://example.invalid/fixture",
        "domain":"structure","interpretation_owner_role":"Papel de teste sem aprovação profissional"
    })
}

#[tokio::test]
async fn normative_change_preserves_active_snapshot_until_explicit_migration() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("migration.prumo");
    let client = sdk(&path, true).await;
    let created = call(
        &client,
        "prumo.project.create",
        json!({"name":"Migração de teste","units":"metre"}),
    )
    .await;
    let candidate = call(&client, "prumo.norms.package", json!({})).await;
    assert_eq!(
        candidate["project_revision"],
        created["project"]["revision"]
    );
    assert!(candidate["active_hash"].is_null());
    assert!(candidate["impact"].is_null());
    let old_hash = candidate["package"]["hash"].clone();
    let pinned = call(
        &client,
        "prumo.norms.pin",
        json!({"expected_revision":created["project"]["revision"],"package_hash":old_hash}),
    )
    .await;
    assert_eq!(pinned["project"]["normative"]["active"]["hash"], old_hash);
    let before_active = pinned["project"]["normative"]["active"].clone();
    let updated = call(
        &client,
        "prumo.norms.register",
        json!({"expected_revision":pinned["project"]["revision"],"reference":reference_fixture()}),
    )
    .await;
    assert_eq!(updated["project"]["normative"]["active"], before_active);
    let candidate = call(&client, "prumo.norms.package", json!({})).await;
    let new_hash = candidate["package"]["hash"].clone();
    assert_ne!(new_hash, old_hash);
    let revision = updated["project"]["revision"].clone();
    assert_eq!(candidate["project_revision"], revision);
    assert_eq!(candidate["active_hash"], old_hash);
    assert_eq!(candidate["impact"]["from_hash"], old_hash);
    assert_eq!(candidate["impact"]["to_hash"], new_hash);
    assert_eq!(
        candidate["impact"]["revision"].as_u64(),
        revision.as_u64().map(|revision| revision + 1)
    );
    assert!(
        candidate["impact"]["changed_norm_ids"]
            .as_array()
            .unwrap()
            .contains(&json!("nbr-6118"))
    );
    assert!(
        candidate["impact"]["affected_capability_ids"]
            .as_array()
            .unwrap()
            .contains(&json!("PAR-06-calculate"))
    );
    let after_preview = call(&client, "prumo.project.read", json!({})).await;
    assert_eq!(
        after_preview, updated,
        "preview must not mutate the project"
    );
    let denied = client.call_tool(CallToolRequestParams::new("prumo.norms.migrate").with_arguments(json!({"expected_revision":revision,"from_package_hash":old_hash,"to_package_hash":new_hash,"confirm":false}).as_object().unwrap().clone())).await.unwrap();
    assert_eq!(denied.is_error, Some(true));
    let after_refusal = call(&client, "prumo.project.read", json!({})).await;
    assert_eq!(after_refusal, updated);
    client.cancel().await.unwrap();
    let old_desktop = Desktop::new(path.clone());
    let old_project = old_desktop
        .project
        .as_ref()
        .expect("UI opens the old package before migration");
    assert_eq!(
        serde_json::to_value(old_project).unwrap(),
        updated["project"]
    );
    assert_eq!(
        serde_json::to_value(old_project.normative.active.as_ref().unwrap()).unwrap(),
        before_active
    );
    drop(old_desktop);
    let client = sdk(&path, true).await;
    let after_restart = call(&client, "prumo.project.read", json!({})).await;
    assert_eq!(after_restart, updated);
    let migrated = call(&client,"prumo.norms.migrate",json!({"expected_revision":revision,"from_package_hash":old_hash,"to_package_hash":new_hash,"confirm":true})).await;
    let state = &migrated["project"]["normative"];
    assert_eq!(state["migrations"][0], candidate["impact"]);
    assert_eq!(state["active"]["hash"], new_hash);
    assert_eq!(state["previous"][0], before_active);
    assert_eq!(state["recalculation_required"], true);
    assert!(
        state["migrations"][0]["changed_norm_ids"]
            .as_array()
            .unwrap()
            .contains(&json!("nbr-6118"))
    );
    assert!(
        state["migrations"][0]["affected_capability_ids"]
            .as_array()
            .unwrap()
            .contains(&json!("PAR-06-calculate"))
    );
    let gate = call(
        &client,
        "prumo.results.gate",
        json!({"capability_id":"PAR-06-calculate"}),
    )
    .await;
    assert_eq!(gate["gate"]["allowed"], false);
    client.cancel().await.unwrap();

    let desktop = Desktop::new(path);
    let project = desktop.project.expect("UI reopens normative history");
    assert_eq!(serde_json::to_value(&project).unwrap(), migrated["project"]);
    assert_eq!(project.normative.previous.len(), 1);
    assert_eq!(
        project.normative.active.unwrap().hash,
        new_hash.as_str().unwrap()
    );
}

#[tokio::test]
async fn registry_mutations_require_write_and_cannot_forge_license_or_approval() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("registry-security.prumo");
    let host = ProjectService::open(&path, AccessMode::ReadWrite, "host").unwrap();
    let before = host.create("Protegido", LengthUnit::Metre).unwrap();
    let client = sdk(&path, false).await;
    let package = call(&client, "prumo.norms.package", json!({})).await;
    let hash = package["package"]["hash"].clone();
    for (tool, args) in [
        (
            "prumo.norms.register",
            json!({"expected_revision":before.revision,"reference":reference_fixture()}),
        ),
        (
            "prumo.norms.pin",
            json!({"expected_revision":before.revision,"package_hash":hash}),
        ),
        (
            "prumo.norms.migrate",
            json!({"expected_revision":before.revision,"from_package_hash":hash,"to_package_hash":hash,"confirm":true}),
        ),
    ] {
        let denied = client
            .call_tool(
                CallToolRequestParams::new(tool).with_arguments(args.as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        assert_eq!(denied.is_error, Some(true));
        assert_eq!(
            denied.structured_content.unwrap()["code"],
            "permission_denied"
        );
    }
    client.cancel().await.unwrap();
    let client = sdk(&path, true).await;
    let mut forged = reference_fixture();
    forged["access"] = json!("licensed_text_consulted");
    forged["qualification"] = json!({"approved":true});
    let result = client
        .call_tool(
            CallToolRequestParams::new("prumo.norms.register").with_arguments(
                json!({"expected_revision":before.revision,"reference":forged})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await;
    assert!(result.is_err());
    assert_eq!(host.read().unwrap(), before);
    client.cancel().await.unwrap();
}
