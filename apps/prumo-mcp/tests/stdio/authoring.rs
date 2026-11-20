//! Model authoring through actual child-process MCP clients and native widgets.
use super::access_tests::wire;
use super::*;
use prumo_domain::{Project, canonical::model_snapshot};
use uuid::Uuid;

async fn edit(client: &RunningService<RoleClient, SdkClient>, project: &mut Value, edit: Value) {
    *project = call(
        client,
        "prumo.model.edit",
        json!({"expected_revision":project["revision"],"edit":edit}),
    )
    .await["project"]
        .clone();
}
fn coordinate(values: [f64; 3]) -> Value {
    json!({"point":"coordinates","position":{"values":values,"unit":"metre"}})
}
fn outline(x: f64, y: f64, w: f64, h: f64) -> Value {
    json!([{"x_m":x,"y_m":y},{"x_m":x+w,"y_m":y},{"x_m":x+w,"y_m":y+h},{"x_m":x,"y_m":y+h}])
}
fn frame(origin: [f64; 3]) -> Value {
    json!({"origin_m":origin,"u":[1,0,0],"v":[0,1,0]})
}
fn seed(path: &Path) -> (Project, Value) {
    let host = ProjectService::open(path, AccessMode::ReadWrite, "fixture").unwrap();
    let mut project = host
        .create("Autoria pela interface e MCP", LengthUnit::Metre)
        .unwrap();
    let material = json!({"edit":"add_material","material":{"name":"Aço informado","family":"steel",
        "elastic_modulus":{"value":200,"unit":"gigapascal"},"density":{"value":7850,"unit":"kilogram_per_cubic_metre"},"poisson_ratio":0.3}});
    project = host
        .edit_model(project.revision, serde_json::from_value(material).unwrap())
        .unwrap();
    let section = Uuid::new_v4();
    let entry = json!({"edit":"put_catalog_entry","entry":{"id":section,"revision":1,"name":"Seção retangular de ensaio",
        "source":{"publisher":"Prumo test","reference":"Geometria declarada","edition":"1","revision":"1","data_license":"Fixture própria","original_record":null},
        "applicability":[],"customization":null,"payload":{"kind":"section","shape":{"kind":"rectangle","width_m":0.2,"height_m":0.4}}}});
    project = host
        .edit_model(project.revision, serde_json::from_value(entry).unwrap())
        .unwrap();
    let input = json!({"name":"Viga 1","kind":"beam","path":{"kind":"straight","start":coordinate([0.,0.,0.]),"end":coordinate([4.,0.,0.])},
        "material_id":project.model.materials[0].id,"sections":[{"fraction":0,"section_id":section}],
        "variation":"constant","alignment":"centroid","eccentricity":{"values":[0,0,0],"unit":"metre"},"roll_degrees":0,"connections":[null,null]});
    project = host.edit_model(project.revision, serde_json::from_value(json!({"edit":"edit_framing","command":{"operation":"put_member","id":Uuid::new_v4(),"input":input}})).unwrap()).unwrap();
    (project, input)
}

#[tokio::test]
async fn authoring_queries_and_edits_use_real_mcp_with_durable_previews() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("authoring.prumo");
    let (seed, _) = seed(&path);
    let beam = seed.objects[0].id;
    let mut project = serde_json::to_value(&seed).unwrap();
    let client = sdk(&path, true).await;
    let level = Uuid::new_v4();
    edit(&client, &mut project, json!({"edit":"edit_layout","command":{"operation":"put_level","id":level,"name":"Superior","elevation":"3 m + 20 cm"}})).await;
    let layout = call(&client, "prumo.layout.read", json!({})).await;
    assert_eq!(layout["layout"]["levels"][0]["elevation_m"], 3.2);
    assert_eq!(layout["revision"], project["revision"]);
    let snap = call(
        &client,
        "prumo.layout.snap",
        json!({"request":{"kind":"endpoint","object_id":beam,"end":true}}),
    )
    .await;
    assert_eq!(snap["point_m"], json!([4., 0., 0.]));
    assert_eq!(snap["creates_structural_connection"], false);
    let measure = call(
        &client,
        "prumo.layout.measure",
        json!({"expression":"(2,5 m + 50 cm) / 2"}),
    )
    .await;
    assert_eq!(measure["value_m"], 1.5);
    let member = call(&client, "prumo.framing.inspect", json!({"object_id":beam})).await;
    assert_eq!(member["object_id"], beam.to_string());
    assert_eq!(member["record"]["physical_length_m"], 4.0);
    assert_eq!(member["record"]["analytical_path_length_m"], 4.0);
    assert_eq!(member["record"]["physical_length_is_exact"], true);
    assert_eq!(member["engineering_release"], false);
    assert_eq!(member["snapshot_sha256"], layout["snapshot_sha256"]);
    let surface = json!({"name":"Laje com vão","role":"analytical_source","material_id":seed.model.materials[0].id,
        "frame":frame([0.,0.,3.2]),"kind":{"kind":"solid_slab"},"outline":outline(0.,0.,4.,3.),
        "openings":[],"thickness_m":0.2,"regions":[],"supports":[],"edge_conditions":[{"edge":0,"behavior":{"kind":"free"}}]});
    let before = std::fs::read(&path).unwrap();
    let preview = call(
        &client,
        "prumo.surfaces.preview",
        json!({"surface":surface}),
    )
    .await;
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(preview["revision"], project["revision"]);
    assert_eq!(preview["net_area_m2"], 12.0);
    edit(
        &client,
        &mut project,
        json!({"edit":"edit_surfaces","command":{"action":"create","surface":surface}}),
    )
    .await;
    let slab = project["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["name"] == "Laje com vão")
        .unwrap()["id"]
        .clone();
    let inspection = call(&client, "prumo.surfaces.inspect", json!({"object_id":slab})).await;
    assert!(inspection["region_id"].is_string());
    assert_eq!(
        inspection["preview"]["boundaries"][0]["behavior"]["kind"],
        "free"
    );
    let assembly = json!({"name":"Rampa e patamar","role":"analytical_source","material_id":seed.model.materials[0].id,
        "shape":{"kind":"ramp","flights":[{"name":"Lance","start_m":[0,5,0],"end_m":[4,5,1],"width_m":1.2,"thickness_m":0.15,"steps":0}],
        "landings":[{"name":"Patamar","frame":frame([4.,5.,1.]),"outline":outline(0.,0.,1.2,1.2),"thickness_m":0.15}]},"supports":[],"continuities":[]});
    let preview = call(
        &client,
        "prumo.assemblies.preview",
        json!({"assembly":assembly}),
    )
    .await;
    assert_eq!(preview["parts"].as_array().unwrap().len(), 2);
    edit(
        &client,
        &mut project,
        json!({"edit":"edit_assemblies","command":{"action":"create","assembly":assembly}}),
    )
    .await;
    let ramp = project["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["name"] == "Rampa e patamar")
        .unwrap()["id"]
        .clone();
    let inspection = call(
        &client,
        "prumo.assemblies.inspect",
        json!({"object_id":ramp}),
    )
    .await;
    assert_eq!(inspection["regions"].as_array().unwrap().len(), 2);
    assert_eq!(inspection["revision"], project["revision"]);
    let unchanged = std::fs::read(&path).unwrap();
    let selected = call(
        &client,
        "prumo.view.command",
        json!({"command":{"action":"select","object_id":slab,"reveal":true}}),
    )
    .await;
    assert_eq!(selected["state"]["selected_id"], slab);
    call(
        &client,
        "prumo.view.command",
        json!({"command":{"action":"hide_selected"}}),
    )
    .await;
    let located = call(
        &client,
        "prumo.view.command",
        json!({"command":{"action":"locate_selected"}}),
    )
    .await;
    assert_eq!(located["state"]["selected_id"], slab);
    assert_eq!(located["changes_project"], false);
    let plan = call(
        &client,
        "prumo.view.command",
        json!({"command":{"action":"set_view","view":"plan"}}),
    )
    .await;
    assert_eq!(plan["state"]["active_view"], "plan");
    assert_eq!(std::fs::read(&path).unwrap(), unchanged);
    let request = json!({"project_id":seed.id,"expected_revision":project["revision"],"idempotency_key":Uuid::new_v4(),
        "operations":[{"operation":"edit_model","expected_revision":project["revision"],"edit":{"edit":"edit_surfaces","command":{"action":"set_openings","object_id":slab,
        "openings":[{"name":"Abertura revisada","outline":outline(1.,1.,1.,1.)}]}}}]});
    let change = call(&client, "prumo.model.preview_change", request).await;
    assert_eq!(std::fs::read(&path).unwrap(), unchanged);
    client.cancel().await.unwrap();
    let client = sdk(&path, true).await;
    let applied = call(
        &client,
        "prumo.model.apply_change",
        json!({"plan_sha256":change["preview"]["plan_sha256"]}),
    )
    .await;
    let opened = call(&client, "prumo.surfaces.inspect", json!({"object_id":slab})).await;
    assert_eq!(opened["preview"]["net_area_m2"], 11.0);
    assert_eq!(
        opened["region_id"],
        inspection_region(&applied["project"], &slab)
    );
    let exported = call(&client, "prumo.view.export", json!({})).await;
    let export_path = directory
        .path()
        .join(exported["file_name"].as_str().unwrap());
    let copy: Project = serde_json::from_slice(&std::fs::read(export_path).unwrap()).unwrap();
    assert_eq!(
        model_snapshot(&copy).unwrap().sha256,
        exported["snapshot_sha256"]
    );
    assert_eq!(serde_json::to_value(copy).unwrap(), applied["project"]);
    client.cancel().await.unwrap();
    assert_eq!(
        serde_json::to_value(Desktop::new(path).project.unwrap()).unwrap(),
        applied["project"]
    );
}

fn inspection_region(project: &Value, object: &Value) -> Value {
    project["model"]["links"]
        .as_array()
        .unwrap()
        .iter()
        .find(|link| link["physical_id"] == *object)
        .unwrap()["analytical_id"]
        .clone()
}

#[tokio::test]
async fn navigation_and_authoring_queries_obey_the_launch_grant_and_reject_unknown_fields() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("grants.prumo");
    let (project, _) = seed(&path);
    let before = std::fs::read(&path).unwrap();
    let mut reader = wire(&path, &[]);
    let selected=reader.request("tools/call",json!({"name":"prumo.view.command","arguments":{"command":{"action":"select","object_id":project.objects[0].id,"reveal":true}}})).await;
    assert_eq!(selected["result"]["isError"], false, "{selected}");
    for (tool, arguments) in [
        ("prumo.view.export", json!({})),
        (
            "prumo.model.edit",
            json!({"expected_revision":project.revision,"edit":{"edit":"edit_layout","command":{"operation":"put_level","id":Uuid::new_v4(),"name":"Não permitido","elevation":"1 m"}}}),
        ),
    ] {
        let denied = reader
            .request("tools/call", json!({"name":tool,"arguments":arguments}))
            .await;
        assert_eq!(
            denied["result"]["structuredContent"]["code"], "permission_denied",
            "{denied}"
        );
    }
    let malformed=reader.request("tools/call",json!({"name":"prumo.view.command","arguments":{"command":{"action":"fit","unrecognized":true}}})).await;
    assert_eq!(malformed["error"]["code"], -32602, "{malformed}");
    reader.stop().await;
    let mut scoped = wire(
        &path,
        &["--scopes", "read,model", "--disciplines", "structure"],
    );
    for (tool, arguments) in [
        ("prumo.view.read", json!({})),
        ("prumo.layout.read", json!({})),
        (
            "prumo.framing.inspect",
            json!({"object_id":project.objects[0].id}),
        ),
        ("prumo.view.export", json!({})),
    ] {
        let denied = scoped
            .request("tools/call", json!({"name":tool,"arguments":arguments}))
            .await;
        assert_eq!(
            denied["result"]["structuredContent"]["code"], "permission_denied",
            "{denied}"
        );
    }
    scoped.stop().await;
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(
        !std::fs::read_dir(directory.path())
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().ends_with(".view.json"))
    );
}

fn click_widget(ui: &mut Harness<'_, Desktop>, label: &str) {
    ui.get_by_label(label).scroll_to_me();
    ui.run();
    ui.get_by_label(label).click();
    ui.run();
    assert!(!ui.state().view.is_error, "{}", ui.state().view.notice);
}
fn enter_widget(ui: &mut Harness<'_, Desktop>, label: &str, value: &str) {
    ui.get_by_label(label).scroll_to_me();
    ui.run();
    ui.get_by_label(label).click();
    ui.run();
    ui.key_press_modifiers(
        egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
        egui::Key::A,
    );
    ui.run();
    ui.get_by_label(label).type_text(value);
    ui.run();
}

#[tokio::test]
async fn framing_widget_and_real_mcp_produce_identical_canonical_snapshots() {
    let directory = tempfile::tempdir().unwrap();
    let ui_path = directory.path().join("widget.prumo");
    let mcp_path = directory.path().join("mcp.prumo");
    let (base, mut input) = seed(&ui_path);
    let material_id = base.model.materials[0].id;
    let section_id = input["sections"][0]["section_id"].clone();
    std::fs::copy(&ui_path, &mcp_path).unwrap();
    let mut ui = Harness::builder()
        .with_size(egui::vec2(1600., 4400.))
        .build_ui_state(
            |ui, desktop: &mut Desktop| desktop.render(ui),
            Desktop::new(ui_path),
        );
    click_widget(&mut ui, "Pilares, vigas, barras e treliças");
    click_widget(&mut ui, "Editar parâmetros de Viga 1");
    enter_widget(&mut ui, "Nome da peça estrutural", "Viga revisada");
    click_widget(&mut ui, "Alinhamento, excentricidade e conexões explícitas");
    enter_widget(&mut ui, "Rotação da seção (graus)", "30");
    click_widget(&mut ui, "Conferir peça física e analítica");
    click_widget(&mut ui, "Preparar alteração da peça");
    assert_eq!(ui.state().project.as_ref().unwrap(), &base);
    click_widget(&mut ui, "Alterações em lote e histórico");
    click_widget(&mut ui, "Aplicar lote conferido");
    let manual = ui.state().project.as_ref().unwrap().clone();
    input["name"] = json!("Viga revisada");
    input["roll_degrees"] = json!(30);
    let client = sdk(&mcp_path, true).await;
    let mut remote = serde_json::to_value(base.clone()).unwrap();
    edit(&client,&mut remote,json!({"edit":"edit_framing","command":{"operation":"put_member","id":base.objects[0].id,"input":input}})).await;
    let remote: Project = serde_json::from_value(remote).unwrap();
    assert_eq!(manual.objects[0].id, base.objects[0].id);
    assert_eq!(
        model_snapshot(&manual).unwrap(),
        model_snapshot(&remote).unwrap()
    );
    assert_ne!(
        manual.history.last().unwrap().actor,
        remote.history.last().unwrap().actor
    );

    // Create another member through the actual form. Unit conversions and the
    // sloping path are specified independently in the MCP request below.
    click_widget(&mut ui, "Preparar nova peça");
    enter_widget(
        &mut ui,
        "Nome da peça estrutural",
        "Pilar inclinado independente",
    );
    click_widget(&mut ui, "Tipo de peça estrutural");
    click_widget(&mut ui, "Pilar");
    for (label, value) in [
        ("Início da peça: X (com unidade)", "100 cm"),
        ("Início da peça: Y (com unidade)", "2000 mm"),
        ("Início da peça: Z (com unidade)", "0,5 m"),
        ("Fim da peça: X (com unidade)", "4 m"),
        ("Fim da peça: Y (com unidade)", "600 cm"),
        ("Fim da peça: Z (com unidade)", "5500 mm"),
    ] {
        enter_widget(&mut ui, label, value);
    }
    enter_widget(&mut ui, "Rotação da seção (graus)", "15");
    click_widget(&mut ui, "Conferir peça física e analítica");
    assert!(ui.state().view.framing.preview.is_some());
    assert_eq!(ui.state().project.as_ref().unwrap(), &manual);
    click_widget(&mut ui, "Criar peça conferida");
    let created = ui.state().project.as_ref().unwrap().clone();
    assert_eq!(created.objects.len(), base.objects.len() + 1);
    let created_id = created
        .objects
        .iter()
        .find(|object| !base.objects.iter().any(|previous| previous.id == object.id))
        .unwrap()
        .id;

    // Only the new physical UUID is taken from the UI result. Material and
    // section come from the common seed; every construction parameter is a
    // separate literal, not a serialization of the form or its metadata.
    let creation_input = json!({
        "name":"Pilar inclinado independente",
        "kind":"column",
        "path":{
            "kind":"straight",
            "start":coordinate([1.0,2.0,0.5]),
            "end":coordinate([4.0,6.0,5.5])
        },
        "material_id":material_id,
        "sections":[{"fraction":0,"section_id":section_id}],
        "variation":"constant",
        "alignment":"centroid",
        "eccentricity":{"values":[0,0,0],"unit":"metre"},
        "roll_degrees":15,
        "connections":[null,null]
    });
    let expected_input: prumo_domain::framing::FramingInput =
        serde_json::from_value(creation_input.clone()).unwrap();
    let mut remote_created = serde_json::to_value(&remote).unwrap();
    edit(
        &client,
        &mut remote_created,
        json!({"edit":"edit_framing","command":{"operation":"put_member","id":created_id,"input":creation_input}}),
    )
    .await;
    let remote_created: Project = serde_json::from_value(remote_created).unwrap();
    let ui_inspection = prumo_domain::framing::inspect(&created, created_id).unwrap();
    let mcp_inspection = prumo_domain::framing::inspect(&remote_created, created_id).unwrap();
    assert_eq!(ui_inspection.record.input, expected_input);
    assert_eq!(
        serde_json::to_value(&ui_inspection).unwrap(),
        serde_json::to_value(&mcp_inspection).unwrap()
    );
    assert_eq!(created.objects, remote_created.objects);
    assert_eq!(
        created.model.analytical_nodes,
        remote_created.model.analytical_nodes
    );
    assert_eq!(
        created.model.analytical_members,
        remote_created.model.analytical_members
    );
    assert_eq!(created.model.links, remote_created.model.links);
    assert_eq!(
        created.model.analytical_nodes.len(),
        base.model.analytical_nodes.len() + 2
    );
    assert_eq!(
        created.model.analytical_members.len(),
        base.model.analytical_members.len() + 1
    );
    assert_eq!(created.model.links.len(), base.model.links.len() + 1);
    assert!((ui_inspection.record.physical_length_m - 50_f64.sqrt()).abs() <= 1e-12);

    // The technical model is equal. These remain distinct creation operations:
    // audit actors and newly allocated IFC GUIDs are neither copied nor erased.
    for id in std::iter::once(created_id)
        .chain(ui_inspection.record.node_ids.iter().copied())
        .chain(ui_inspection.record.member_ids.iter().copied())
        .chain(ui_inspection.record.link_ids.iter().copied())
    {
        let ui_identity = &created.model.identities[&id];
        let mcp_identity = &remote_created.model.identities[&id];
        assert_eq!(ui_identity.created_by.as_deref(), Some("interface-local"));
        assert_eq!(mcp_identity.created_by.as_deref(), Some("mcp-local"));
        assert_eq!(ui_identity.created_revision, Some(base.revision + 2));
        assert_eq!(mcp_identity.created_revision, Some(base.revision + 2));
        if id == created_id {
            assert!(ui_identity.ifc_guid.is_some());
            assert!(mcp_identity.ifc_guid.is_some());
            assert_ne!(ui_identity.ifc_guid, mcp_identity.ifc_guid);
        } else {
            assert!(ui_identity.ifc_guid.is_none());
            assert!(mcp_identity.ifc_guid.is_none());
        }
    }
    assert_ne!(
        model_snapshot(&created).unwrap().sha256,
        model_snapshot(&remote_created).unwrap().sha256
    );
    client.cancel().await.unwrap();
}
