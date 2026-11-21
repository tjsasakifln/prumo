use super::*;
use prumo_domain::Project;
use prumo_domain::dependencies::ResultState;

async fn edit(client: &RunningService<RoleClient, SdkClient>, project: &mut Value, edit: Value) {
    let result = call(
        client,
        "prumo.model.edit",
        json!({"expected_revision":project["revision"],"edit":edit}),
    )
    .await;
    *project = result["project"].clone();
}

fn point(values: [f64; 3], unit: &str) -> Value {
    json!({"values":values,"unit":unit})
}
fn material(name: &str, modulus: f64) -> Value {
    json!({"name":name,"family":"generic","elastic_modulus":{"value":modulus,"unit":"gigapascal"},"density":{"value":2500,"unit":"kilogram_per_cubic_metre"},"poisson_ratio":0.2})
}

fn closed_objects(value: &Value) {
    match value {
        Value::Object(object) => {
            if object.get("type") == Some(&json!("object")) {
                if let Some(Value::Object(value_schema)) = object.get("additionalProperties") {
                    // Catalogue dictionaries allow named physical properties, but
                    // every value must remain a bounded, closed PhysicalValue.
                    assert_eq!(
                        object
                            .keys()
                            .map(String::as_str)
                            .collect::<std::collections::BTreeSet<_>>(),
                        std::collections::BTreeSet::from([
                            "type",
                            "maxProperties",
                            "additionalProperties"
                        ])
                    );
                    assert_eq!(object["maxProperties"], 256);
                    assert_eq!(value_schema["type"], "object");
                    assert_eq!(value_schema["additionalProperties"], false);
                    let properties = value_schema["properties"].as_object().unwrap();
                    assert_eq!(
                        properties
                            .keys()
                            .map(String::as_str)
                            .collect::<std::collections::BTreeSet<_>>(),
                        std::collections::BTreeSet::from([
                            "quantity",
                            "decimal",
                            "unit",
                            "basis",
                            "design_basis"
                        ])
                    );
                    assert_eq!(
                        value_schema["required"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|name| name.as_str().unwrap())
                            .collect::<std::collections::BTreeSet<_>>(),
                        std::collections::BTreeSet::from(["quantity", "decimal", "unit", "basis"])
                    );
                    for field in ["quantity", "unit", "basis"] {
                        assert_eq!(properties[field]["type"], "string");
                        let variants = properties[field]["enum"].as_array().unwrap();
                        assert!(!variants.is_empty());
                        assert!(variants.iter().all(Value::is_string));
                    }
                    assert_eq!(
                        properties["decimal"],
                        json!({"type":"string","minLength":1,"maxLength":128})
                    );
                    assert_eq!(
                        properties["design_basis"],
                        json!({"type":["string","null"]})
                    );
                } else {
                    assert_eq!(object.get("additionalProperties"), Some(&json!(false)));
                }
            }
            for value in object.values() {
                closed_objects(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                closed_objects(value);
            }
        }
        _ => {}
    }
}

async fn omission_is_refused(
    client: &RunningService<RoleClient, SdkClient>,
    path: &Path,
    project: &Value,
    edit: Value,
) {
    let bytes = std::fs::read(path).unwrap();
    let response = timeout(
        LIMIT,
        client.call_tool(
            CallToolRequestParams::new("prumo.model.edit").with_arguments(
                json!({"expected_revision":project["revision"],"edit":edit})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        ),
    )
    .await
    .expect("omitted-field response timeout");
    match response
        .expect_err("Omission must be rejected; explicit null is the only clearing request")
    {
        rmcp::service::ServiceError::McpError(error) => {
            assert_eq!(serde_json::to_value(error).unwrap()["code"], -32602)
        }
        other => panic!("Expected invalid-params response, received {other}"),
    }
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    assert_eq!(
        call(client, "prumo.project.read", json!({})).await["project"],
        *project
    );
}

#[tokio::test]
async fn ui_visual_object_then_real_mcp_typed_model_links_actions_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("semantic-cross.prumo");
    let mut desktop = Desktop::new(path.clone());
    {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1500.0, 2400.0))
            .build_ui(|ui| desktop.render(ui));
        for label in [
            "Criar projeto",
            "Modelo físico e analítico",
            "Criar objeto físico",
        ] {
            harness.get_by_label(label).click();
            harness.run();
        }
    }
    assert!(!desktop.view.is_error, "{}", desktop.view.notice);
    let before = desktop.project.unwrap();
    assert!(before.model.analytical_members.is_empty());
    let physical = before.objects[0].id.to_string();
    let guid = before.model.identities[&before.objects[0].id]
        .ifc_guid
        .clone();
    let mut project = serde_json::to_value(&before).unwrap();
    let client = sdk(&path, true).await;
    let tools = client.list_tools(None).await.unwrap();
    let tool = tools
        .tools
        .iter()
        .find(|tool| tool.name == "prumo.model.edit")
        .unwrap();
    closed_objects(&Value::Object((*tool.input_schema).clone()));
    let variants = tool.input_schema["properties"]["edit"]["oneOf"]
        .as_array()
        .unwrap();
    let names: std::collections::BTreeSet<_> = variants
        .iter()
        .map(|variant| variant["properties"]["edit"]["const"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), variants.len(), "edit tags must be unique");
    assert_eq!(
        names,
        std::collections::BTreeSet::from([
            "add_analytical_member",
            "add_analytical_node",
            "add_material",
            "add_network",
            "add_nodal_action",
            "add_physical",
            "add_port",
            "apply_geometry_repair",
            "assign_catalog_entry",
            "assign_port",
            "change_engineering_frame",
            "clear_catalog_assignment",
            "connect_ports",
            "copy_physical",
            "disconnect_ports",
            "edit_assemblies",
            "edit_framing",
            "edit_layout",
            "edit_precalculation",
            "edit_surfaces",
            "link_physical_analytical",
            "merge_physical",
            "put_catalog_entry",
            "rigid_move",
            "set_display_unit",
            "set_geometry_tolerances",
            "set_georeference",
            "set_material",
            "set_nodal_action",
            "set_physical_geometry",
            "set_physical_material",
            "set_physical_role",
            "set_source_origin",
            "split_physical",
            "unlink_physical_analytical",
        ])
    );
    for (variant, field) in [
        ("set_physical_material", "material_id"),
        ("set_georeference", "georeference"),
    ] {
        let schema = variants
            .iter()
            .find(|schema| schema["properties"]["edit"]["const"] == variant)
            .unwrap();
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!(field))
        );
    }

    edit(
        &client,
        &mut project,
        json!({"edit":"set_physical_role","object_id":physical,"role":"analytical_source"}),
    )
    .await;
    edit(
        &client,
        &mut project,
        json!({"edit":"add_material","material":material("Material MCP",30.0)}),
    )
    .await;
    let material_id = project["model"]["materials"][0]["id"].clone();
    edit(
        &client,
        &mut project,
        json!({"edit":"set_physical_material","object_id":physical,"material_id":material_id}),
    )
    .await;
    edit(&client,&mut project,json!({"edit":"set_material","material_id":material_id,"material":material("Material MCP revisado",32.0)})).await;
    for (name, position) in [
        ("A", point([0.0; 3], "metre")),
        ("B", point([400.0, 0.0, 0.0], "centimetre")),
    ] {
        edit(
            &client,
            &mut project,
            json!({"edit":"add_analytical_node","name":name,"position":position}),
        )
        .await;
    }
    let node_a = project["model"]["analytical_nodes"][0]["id"].clone();
    let node_b = project["model"]["analytical_nodes"][1]["id"].clone();
    assert_eq!(
        project["model"]["analytical_nodes"][1]["position_m"],
        json!([4.0, 0.0, 0.0])
    );
    for name in ["Barra 1", "Barra 2"] {
        edit(&client,&mut project,json!({"edit":"add_analytical_member","name":name,"start_node_id":node_a,"end_node_id":node_b,"local_y":[0,1,0]})).await;
    }
    let member_a = project["model"]["analytical_members"][0]["id"].clone();
    let member_b = project["model"]["analytical_members"][1]["id"].clone();
    for analytical in [&member_a, &member_b] {
        edit(&client,&mut project,json!({"edit":"link_physical_analytical","physical_id":physical,"analytical_id":analytical})).await;
    }
    assert_eq!(project["model"]["links"].as_array().unwrap().len(), 2);
    edit(&client,&mut project,json!({"edit":"copy_physical","source_id":physical,"name":"Segunda origem","translation":point([0.0,1.0,0.0],"metre")})).await;
    let copy = project["objects"][1]["id"].clone();
    edit(
        &client,
        &mut project,
        json!({"edit":"link_physical_analytical","physical_id":copy,"analytical_id":member_a}),
    )
    .await;
    assert_eq!(project["model"]["links"].as_array().unwrap().len(), 3);
    let last_link = project["model"]["links"][2]["id"].clone();
    edit(
        &client,
        &mut project,
        json!({"edit":"unlink_physical_analytical","link_id":last_link}),
    )
    .await;

    let action = json!({"name":"Ação MCP","application_node_id":node_b,"force":{"values":[0,0,-2],"unit":"kilonewton"},"couple":{"values":[0,3,0],"unit":"kilonewton_metre"},"frame":{"kind":"member_local","member_id":member_a}});
    edit(
        &client,
        &mut project,
        json!({"edit":"add_nodal_action","action":action}),
    )
    .await;
    let action_id = project["model"]["actions"][0]["id"].clone();
    assert_eq!(
        project["model"]["actions"][0]["force"],
        json!({"values":[0.0,0.0,-2000.0],"unit":"newton"})
    );
    let mut action = action;
    action["force"] = json!({"values":[0,0,-2500],"unit":"newton"});
    edit(
        &client,
        &mut project,
        json!({"edit":"set_nodal_action","action_id":action_id,"action":action}),
    )
    .await;
    edit(&client,&mut project,json!({"edit":"set_source_origin","entity_id":physical,"origin":{"kind":"imported","system":"Fixture pública","reference":"caso-literal-4m","external_id":"P-100"}})).await;
    let current: Project = serde_json::from_value(project.clone()).unwrap();
    assert_eq!(
        current.model.identities[&before.objects[0].id].ifc_guid,
        guid
    );
    assert_eq!(current.model.links.len(), 2);
    let inspected = call(
        &client,
        "prumo.model.inspect",
        json!({"expected_revision":project["revision"]}),
    )
    .await;
    project = inspected["project"].clone();
    let trace = call(&client, "prumo.results.trace", json!({})).await;
    assert!(
        !trace["dependencies"]["results"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(trace["calculation_available"], false);
    let snapshot = call(&client, "prumo.model.snapshot", json!({})).await;
    assert_eq!(snapshot["snapshot"]["source_revision"], project["revision"]);
    assert_eq!(snapshot["snapshot"]["sha256"].as_str().unwrap().len(), 64);
    edit(&client,&mut project,json!({"edit":"set_physical_geometry","object_id":physical,"start":point([0.0;3],"metre"),"end":point([5000.0,0.0,0.0],"millimetre")})).await;
    client.cancel().await.unwrap();
    let desktop = Desktop::new(path.clone());
    let final_project = desktop.project.unwrap();
    assert_eq!(serde_json::to_value(&final_project).unwrap(), project);
    assert!(
        final_project
            .dependencies
            .results
            .iter()
            .any(|result| result.state == ResultState::Stale)
    );
    assert!(!final_project.dependencies.invalidations.is_empty());
    assert_eq!(final_project.objects[0].length_m(), 5.0);
    let readonly = sdk(&path, false).await;
    assert_eq!(
        call(&readonly, "prumo.project.read", json!({})).await["project"],
        project
    );
    readonly.cancel().await.unwrap();
}

#[tokio::test]
async fn real_mcp_coordinates_copy_split_merge_and_networks_use_explicit_commands() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("operations-mcp.prumo");
    let client = sdk(&path, true).await;
    let mut project = call(
        &client,
        "prumo.project.create",
        json!({"name":"Operações tipadas","units":"metre"}),
    )
    .await["project"]
        .clone();
    edit(&client,&mut project,json!({"edit":"add_physical","name":"Objeto","discipline":"water_sanitation","role":"visual_only","start":point([0.0;3],"metre"),"end":point([4.0,0.0,0.0],"metre"),"material_id":null,"environment_id":null})).await;
    let source = project["objects"][0]["id"].clone();
    edit(&client,&mut project,json!({"edit":"add_material","material":material("Material a remover explicitamente",30.0)})).await;
    let material_id = project["model"]["materials"][0]["id"].clone();
    edit(
        &client,
        &mut project,
        json!({"edit":"set_physical_material","object_id":source,"material_id":material_id}),
    )
    .await;
    omission_is_refused(
        &client,
        &path,
        &project,
        json!({"edit":"set_physical_material","object_id":source}),
    )
    .await;
    assert_eq!(project["objects"][0]["material_id"], material_id);
    edit(
        &client,
        &mut project,
        json!({"edit":"set_physical_material","object_id":source,"material_id":null}),
    )
    .await;
    assert!(project["objects"][0]["material_id"].is_null());
    edit(&client,&mut project,json!({"edit":"copy_physical","source_id":source,"name":"Cópia","translation":point([0.0,100.0,0.0],"centimetre")})).await;
    let copy = project["objects"][1]["id"].clone();
    edit(&client,&mut project,json!({"edit":"split_physical","object_id":source,"fraction":0.25,"names":["Primeiro","Segundo"]})).await;
    let split: Vec<Value> = project["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|object| object["id"] != copy)
        .map(|object| object["id"].clone())
        .collect();
    edit(
        &client,
        &mut project,
        json!({"edit":"merge_physical","source_ids":split,"name":"Reunido"}),
    )
    .await;
    let merged = project["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|object| object["name"] == "Reunido")
        .unwrap()["id"]
        .clone();
    edit(&client,&mut project,json!({"edit":"rigid_move","object_ids":[merged],"rotation":[[0,-1,0],[1,0,0],[0,0,1]],"translation":point([10.0,0.0,0.0],"metre")})).await;
    edit(&client,&mut project,json!({"edit":"change_engineering_frame","frame":{"origin_world_m":[100,0,0],"rotation_to_world":[[0,-1,0],[1,0,0],[0,0,1]]}})).await;
    edit(
        &client,
        &mut project,
        json!({"edit":"set_display_unit","unit":"millimetre"}),
    )
    .await;
    edit(&client,&mut project,json!({"edit":"set_georeference","georeference":{"crs_identifier":"EPSG:31983","map_origin_m":[500000,7000000,0],"rotation_radians":0.0,"scale":1.0}})).await;
    omission_is_refused(&client, &path, &project, json!({"edit":"set_georeference"})).await;
    let typed: Project = serde_json::from_value(project.clone()).unwrap();
    let object = typed
        .objects
        .iter()
        .find(|object| object.name == "Reunido")
        .unwrap();
    assert!((object.length_m() - 4.0).abs() < 1e-10);
    assert_eq!(typed.model.lineage.len(), 3);
    edit(
        &client,
        &mut project,
        json!({"edit":"set_georeference","georeference":null}),
    )
    .await;
    assert!(project["model"]["georeference"].is_null());
    edit(&client,&mut project,json!({"edit":"add_network","name":"Rede explícita","discipline":"water_sanitation","kind":"network","environment_id":null})).await;
    let network = project["networks"][0]["id"].clone();
    for object in [&copy, &merged] {
        edit(&client,&mut project,json!({"edit":"add_port","object_id":object,"discipline":"water_sanitation","kind":"hydraulic","position":point([0.0;3],"metre")})).await;
    }
    assert!(project["connections"].as_array().unwrap().is_empty());
    let ports: Vec<Value> = project["ports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|port| port["id"].clone())
        .collect();
    for port in &ports {
        edit(
            &client,
            &mut project,
            json!({"edit":"assign_port","network_id":network,"port_id":port}),
        )
        .await;
    }
    edit(&client,&mut project,json!({"edit":"connect_ports","network_id":network,"from_port":ports[0],"to_port":ports[1]})).await;
    assert_eq!(project["connections"].as_array().unwrap().len(), 1);
    let connection = project["connections"][0]["id"].clone();
    edit(
        &client,
        &mut project,
        json!({"edit":"disconnect_ports","connection_id":connection}),
    )
    .await;
    assert!(project["connections"].as_array().unwrap().is_empty());
    let before = std::fs::read(&path).unwrap();
    let snapshot = call(&client, "prumo.model.snapshot", json!({})).await;
    assert_eq!(snapshot["snapshot"]["source_revision"], project["revision"]);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    client.cancel().await.unwrap();
    assert_eq!(
        serde_json::to_value(Desktop::new(path).project.unwrap()).unwrap(),
        project
    );
}

#[tokio::test]
async fn real_mcp_semantic_errors_permissions_and_schema_cannot_bypass_domain() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rejections-mcp.prumo");
    let host = ProjectService::open(&path, AccessMode::ReadWrite, "fixture").unwrap();
    host.create("Rejeições", LengthUnit::Metre).unwrap();
    let before = host
        .add_member(1, "Barra", [0.0; 3], [4.0, 0.0, 0.0])
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let readonly = sdk(&path, false).await;
    for (name, args) in [
        (
            "prumo.model.edit",
            json!({"expected_revision":before.revision,"edit":{"edit":"set_display_unit","unit":"millimetre"}}),
        ),
        (
            "prumo.model.inspect",
            json!({"expected_revision":before.revision}),
        ),
    ] {
        let response = readonly
            .call_tool(
                CallToolRequestParams::new(name).with_arguments(args.as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        assert_eq!(response.is_error, Some(true));
        assert_eq!(
            response.structured_content.unwrap()["code"],
            "permission_denied"
        );
    }
    readonly.cancel().await.unwrap();
    let mut client = WireClient::new(&path, true);
    for edit in [
        json!({"edit":"add_nodal_action","action":{"name":"Unidade errada","application_node_id":before.model.analytical_nodes[0].id,"force":{"values":[1,0,0],"unit":"newton"},"couple":{"values":[2,0,0],"unit":"newton"},"frame":{"kind":"engineering"}}}),
        json!({"edit":"set_display_unit","unit":"metre","path":"/tmp/outside.prumo"}),
        json!({"edit":"set_source_origin","entity_id":before.objects[0].id,"origin":{"kind":"native","ifc_guid":"forged"}}),
        json!({"edit":"add_nodal_action","action":{"name":"Referência adulterada","application_node_id":before.model.analytical_nodes[0].id,"force":{"values":[1,0,0],"unit":"newton"},"couple":{"values":[2,0,0],"unit":"newton_metre"},"frame":{"kind":"engineering","member_id":before.model.analytical_members[0].id}}}),
    ] {
        let response=client.request("tools/call",json!({"name":"prumo.model.edit","arguments":{"expected_revision":before.revision,"edit":edit}})).await;
        assert_eq!(response["error"]["code"], -32602, "{response}");
    }
    for edit in [
        json!({"edit":"link_physical_analytical","physical_id":before.objects[0].id,"analytical_id":"aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa"}),
        json!({"edit":"set_physical_geometry","object_id":before.objects[0].id,"start":point([0.0;3],"metre"),"end":point([0.0;3],"metre")}),
        json!({"edit":"rigid_move","object_ids":[before.objects[0].id],"rotation":[[2,0,0],[0,1,0],[0,0,1]],"translation":point([0.0;3],"metre")}),
    ] {
        let response=client.request("tools/call",json!({"name":"prumo.model.edit","arguments":{"expected_revision":before.revision,"edit":edit}})).await;
        assert_eq!(response["result"]["isError"], true, "{response}");
        assert_eq!(
            response["result"]["structuredContent"]["code"],
            "invalid_input"
        );
    }
    let stale = client
        .request(
            "tools/call",
            json!({"name":"prumo.model.inspect","arguments":{"expected_revision":1}}),
        )
        .await;
    assert_eq!(
        stale["result"]["structuredContent"]["code"],
        "revision_conflict"
    );
    for name in ["prumo.model.snapshot", "prumo.results.trace"] {
        let response = client
            .request(
                "tools/call",
                json!({"name":name,"arguments":{"path":"outside.prumo"}}),
            )
            .await;
        assert_eq!(response["error"]["code"], -32602);
    }
    assert_eq!(host.read().unwrap(), before);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    client.stop().await;
}
