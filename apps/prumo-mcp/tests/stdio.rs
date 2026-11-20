//! Actual child-process interoperability, with SDK and independent JSON wire clients.
use std::{path::Path, process::Stdio, time::Duration};

use egui_kittest::{Harness, kittest::Queryable};
use prumo_app::{AccessMode, ProjectService};
use prumo_desktop::Desktop;
use prumo_domain::LengthUnit;
use rmcp::{
    ClientHandler, ClientLifecycleMode, ClientServiceExt, RoleClient,
    model::{CallToolRequestParams, ClientRequest, ProtocolVersion, Request},
    service::{PeerRequestOptions, RunningService},
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

#[path = "stdio/access.rs"]
mod access_tests;
#[path = "stdio/model.rs"]
mod model_tests;
#[path = "stdio/modeling.rs"]
mod modeling_tests;
#[path = "stdio/storage.rs"]
mod storage_tests;
#[path = "stdio/transactions.rs"]
mod transaction_tests;

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
    call_with_budget(client, name, args, LIMIT).await
}

async fn call_with_budget(
    client: &RunningService<RoleClient, SdkClient>,
    name: &str,
    args: Value,
    budget: Duration,
) -> Value {
    let started = std::time::Instant::now();
    let result = timeout(
        budget,
        client.call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(args.as_object().expect("argument object").clone()),
        ),
    )
    .await
    .unwrap_or_else(|error| panic!("{name} exceeded {budget:?}: {error}"))
    .unwrap_or_else(|error| panic!("{name} failed: {error}"));
    if started.elapsed() > Duration::from_secs(2) {
        eprintln!("{name}: {:?}", started.elapsed());
    }
    assert_eq!(result.is_error, Some(false), "{result:?}");
    result.structured_content.expect("structured content")
}

fn assert_default_tool_catalog(value: &Value, writable: bool) {
    use std::collections::BTreeSet;

    let tools = value.as_array().expect("tools/list must return an array");
    let names: BTreeSet<_> = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name must be a string"))
        .collect();
    assert_eq!(names.len(), tools.len(), "tool names must be unique");
    let mut expected = BTreeSet::from([
        "prumo.access.read",
        "prumo.catalogue.material",
        "prumo.catalogue.read",
        "prumo.components.list",
        "prumo.components.qualify",
        "prumo.geometry.audit",
        "prumo.geometry.face",
        "prumo.model.history",
        "prumo.model.query",
        "prumo.model.snapshot",
        "prumo.norms.package",
        "prumo.project.read",
        "prumo.registry.audit",
        "prumo.registry.read",
        "prumo.results.gate",
        "prumo.results.trace",
        "prumo.storage.status",
        "prumo.verification.protocols",
    ]);
    if writable {
        expected.extend([
            "prumo.geometry.preview",
            "prumo.model.add_environment",
            "prumo.model.add_member",
            "prumo.model.apply_change",
            "prumo.model.edit",
            "prumo.model.inspect",
            "prumo.model.preview_change",
            "prumo.model.redo",
            "prumo.model.undo",
            "prumo.norms.migrate",
            "prumo.norms.pin",
            "prumo.norms.register",
            "prumo.project.create",
            "prumo.project.rename",
            "prumo.storage.backup",
            "prumo.storage.migrate",
            "prumo.storage.recover",
        ]);
    }
    // Assert the named Read/Model contract independently of list order or size.
    // Calculation and Documents operations must not appear in either grant.
    assert_eq!(names, expected, "catalogue must match the launch grant");
    for tool in tools {
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        if !writable {
            assert_eq!(tool["annotations"]["readOnlyHint"], true);
        }
    }
}

#[tokio::test]
async fn sdk_creates_modifies_and_reopens_real_project() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sdk.prumo");
    let client = sdk(&path, true).await;
    let info = client.peer_info().unwrap();
    assert_eq!(info.protocol_version, ProtocolVersion::V_2026_07_28);
    let tools = client.list_tools(None).await.unwrap();
    assert!(
        tools
            .tools
            .iter()
            .any(|tool| tool.name == "prumo.model.preview_change")
    );
    assert!(
        tools
            .tools
            .iter()
            .any(|tool| tool.name == "prumo.project.create")
    );
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
    assert!(object.get("analytical_id").is_none());
    let analytical_id = added["project"]["model"]["links"][0]["analytical_id"].clone();
    assert_ne!(physical_id, analytical_id);
    client.cancel().await.unwrap();

    let reopened = sdk(&path, false).await;
    let read = call(&reopened, "prumo.project.read", json!({})).await;
    assert_eq!(read, added);
    assert_eq!(read["project"]["id"], created["project"]["id"]);
    assert_eq!(read["project"]["objects"][0]["id"], physical_id);
    assert_eq!(
        read["project"]["model"]["links"][0]["analytical_id"],
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
    assert_eq!(reopened_ui.model.links, initial.model.links);
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
    assert_default_tool_catalog(&serde_json::to_value(&tools.tools).unwrap(), false);
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
    let initial = client.request("tools/list", json!({})).await;
    assert_default_tool_catalog(&initial["result"]["tools"], false);
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
    assert_eq!(recovered["result"]["tools"], initial["result"]["tools"]);
    assert!(!path.exists());
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
async fn reserved_json_number_objects_are_rejected_before_sdk_conversion_and_next_line_survives() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("representation.prumo");
    let host = ProjectService::open(&path, AccessMode::ReadWrite, "fixture").unwrap();
    let before = host
        .create("Representação preservada", LengthUnit::Metre)
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let mut client = WireClient::new(&path, true);
    client.request("server/discover", json!({})).await;
    let initial = client.request("tools/list", json!({})).await;
    assert_default_tool_catalog(&initial["result"]["tools"], true);

    for key in [
        "$serde_json::private::Number",
        r"\u0024serde_json::private::Number",
        r"$serde_json::private::\u004eumber",
    ] {
        // Raw bytes are essential: constructing a serde_json::Value first could
        // already coerce the hostile object into a numeric token in the client.
        let mut line = format!(r#"{{"jsonrpc":"2.0","id":100,"method":"tools/call","params":{{"_meta":{{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{{"name":"independent-wire","version":"1"}},"io.modelcontextprotocol/clientCapabilities":{{}}}},"name":"prumo.model.add_member","arguments":{{"expected_revision":1,"name":"Objeto não numérico","start":[0,0,0],"end":[{{"{key}":"4"}},0,0]}}}}}}"#).into_bytes();
        line.push(b'\n');
        let middle = line.len() / 2;
        let input = client.input.as_mut().unwrap();
        input.write_all(&line[..middle]).await.unwrap();
        input.flush().await.unwrap();
        input.write_all(&line[middle..]).await.unwrap();
        input.flush().await.unwrap();
        let rejected = client.receive().await;
        assert_eq!(rejected["jsonrpc"], "2.0");
        assert_eq!(rejected["error"]["code"], -32600);
        assert!(rejected["id"].is_null());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(host.read().unwrap(), before);
        let resumed = client.request("tools/list", json!({})).await;
        assert_eq!(resumed["result"]["tools"], initial["result"]["tools"]);
    }

    for malformed in [
        br#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"unterminated"#.as_slice(),
        br#"{"bad\uZZZZ":0}"#.as_slice(),
    ] {
        let input = client.input.as_mut().unwrap();
        input.write_all(malformed).await.unwrap();
        input.write_all(b"\n").await.unwrap();
        input.flush().await.unwrap();
        // rmcp 3.10 ignores JSON syntax errors and resumes with the next line.
        // The representation guard must not turn them into Invalid Request.
        let resumed = client.request("tools/list", json!({})).await;
        assert_eq!(resumed["result"]["tools"], initial["result"]["tools"]);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }

    let marker_as_text = r#"{"$serde_json::private::Number":"4"}"#;
    let renamed = client
        .request(
            "tools/call",
            json!({
                "name":"prumo.project.rename",
                "arguments":{"expected_revision":1,"name":marker_as_text}
            }),
        )
        .await;
    assert_eq!(renamed["result"]["isError"], false);
    assert_eq!(
        renamed["result"]["structuredContent"]["project"]["name"],
        marker_as_text
    );
    assert!(host.read().unwrap().objects.is_empty());
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
    // The validated four-revision service probe measured migrate at 10.867 s
    // in debug and produced a 12,660,812-byte package. This functional test's
    // persistence budget allows protocol/serialization and CI scheduling;
    // it is neither a production timeout nor a performance qualification.
    const PERSISTENCE_BUDGET: Duration = Duration::from_secs(30);
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
    let pinned = call_with_budget(
        &client,
        "prumo.norms.pin",
        json!({"expected_revision":created["project"]["revision"],"package_hash":old_hash}),
        PERSISTENCE_BUDGET,
    )
    .await;
    assert_eq!(pinned["project"]["normative"]["active"]["hash"], old_hash);
    let before_active = pinned["project"]["normative"]["active"].clone();
    let updated = call_with_budget(
        &client,
        "prumo.norms.register",
        json!({"expected_revision":pinned["project"]["revision"],"reference":reference_fixture()}),
        PERSISTENCE_BUDGET,
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
    let denied = timeout(LIMIT, client.call_tool(CallToolRequestParams::new("prumo.norms.migrate").with_arguments(json!({"expected_revision":revision,"from_package_hash":old_hash,"to_package_hash":new_hash,"confirm":false}).as_object().unwrap().clone())))
        .await.expect("unconfirmed normative migration exceeded the ordinary 10 s budget").unwrap();
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
    let migrated = call_with_budget(&client,"prumo.norms.migrate",json!({"expected_revision":revision,"from_package_hash":old_hash,"to_package_hash":new_hash,"confirm":true}),PERSISTENCE_BUDGET).await;
    eprintln!(
        "normative fixture: revision {}, {} package bytes",
        migrated["project"]["revision"],
        std::fs::metadata(&path).unwrap().len()
    );
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

#[tokio::test]
async fn component_qualification_is_readonly_and_restarts_after_cancellation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("components.prumo");
    let host = ProjectService::open(&path, AccessMode::ReadWrite, "host").unwrap();
    let before = host
        .create("Qualificação de componentes", LengthUnit::Metre)
        .unwrap();
    let bytes_before = std::fs::read(&path).unwrap();
    let client = sdk(&path, false).await;
    let inventory = call(&client, "prumo.components.list", json!({})).await;
    let components = inventory["components"].as_array().unwrap();
    assert!(
        components
            .iter()
            .any(|component| component["id"] == "bim-ruststep")
    );
    assert!(
        components
            .iter()
            .any(|component| component["id"] == "documents-svg")
    );
    assert!(components.iter().all(|component| {
        component["version"]
            .as_str()
            .is_some_and(|version| !version.is_empty())
            && component["license"]
                .as_str()
                .is_some_and(|license| !license.is_empty())
    }));
    let cancelled = client
        .send_cancellable_request(
            ClientRequest::CallToolRequest(Request::new(CallToolRequestParams::new(
                "prumo.components.qualify",
            ))),
            PeerRequestOptions::no_options(),
        )
        .await
        .unwrap();
    cancelled
        .cancel(Some("Cancelamento solicitado pelo cliente de teste".into()))
        .await
        .unwrap();
    let result = call(&client, "prumo.components.qualify", json!({})).await;
    let report: prumo_domain::components::QualificationReport =
        serde_json::from_value(result["qualification"].clone()).unwrap();
    assert!(report.passed(), "{report:?}");
    assert!(!report.engineering_release);
    assert_eq!(result["calculation_available"], false);
    assert_eq!(report.experiments.len(), components.len());
    let basis = report.basis.unwrap();
    assert_eq!(basis.project_id, before.id);
    assert_eq!(basis.revision, before.revision);
    assert_eq!(report.executable_sha256.len(), 64);
    assert!(report.executable_bytes > 0);
    assert!(
        report.experiments.iter().any(|experiment| experiment
            .artifact
            .as_ref()
            .is_some_and(|artifact| artifact.media_type == "image/svg+xml"
                && artifact.content.contains("<svg")))
    );
    let invalid = client
        .call_tool(
            CallToolRequestParams::new("prumo.components.qualify").with_arguments(
                json!({"path":"/other/project","shell":"unavailable"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await;
    assert!(invalid.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes_before);
    assert_eq!(host.read().unwrap(), before);
    client.cancel().await.unwrap();
}
