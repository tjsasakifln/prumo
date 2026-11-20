//! Real MCP boundaries for geometry, verification evidence and catalogue data.
use super::access_tests::{sdk_options, wire};
use super::*;
use prumo_domain::canonical::{hash_bytes, model_snapshot};
use uuid::Uuid;

async fn wire_call(client: &mut WireClient, name: &str, arguments: Value) -> Value {
    let response = client
        .request("tools/call", json!({"name":name,"arguments":arguments}))
        .await;
    assert_eq!(response["result"]["isError"], false, "{response}");
    response["result"]["structuredContent"].clone()
}

async fn denied(client: &mut WireClient, name: &str, arguments: Value, code: &str) {
    let response = client
        .request("tools/call", json!({"name":name,"arguments":arguments}))
        .await;
    assert_eq!(response["result"]["isError"], true, "{response}");
    assert_eq!(
        response["result"]["structuredContent"]["code"], code,
        "{response}"
    );
}

fn close(actual: &Value, expected: f64, absolute_tolerance: f64) {
    let actual = actual.as_f64().unwrap();
    assert!(
        (actual - expected).abs() <= absolute_tolerance,
        "actual={actual:.16e}; expected={expected:.16e}; tolerance={absolute_tolerance:.3e}"
    );
}

fn hash(value: &Value) -> &str {
    let text = value.as_str().unwrap();
    assert_eq!(text.len(), 64);
    assert!(text.bytes().all(|byte| byte.is_ascii_hexdigit()));
    text
}

#[tokio::test]
async fn geometry_queries_and_reviewed_repairs_preserve_units_and_explicit_topology() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("geometry-tools.prumo");
    let host = ProjectService::open(&path, AccessMode::ReadWrite, "host").unwrap();
    let project = host
        .create("Geometria em centímetros", LengthUnit::Centimetre)
        .unwrap();
    let project = host
        .add_member(project.revision, "A", [0.0; 3], [100.0, 0.0, 0.0])
        .unwrap();
    let a = project.objects[0].id;
    let project = host
        .add_member(
            project.revision,
            "B",
            [100.00005, 0.0, 0.0],
            [200.0, 0.0, 0.0],
        )
        .unwrap();
    let mut client = wire(&path, &["--allow-write"]);

    // Preserve real exponent tokens across JSON, the tagged command and storage.
    let mut arguments: Value = serde_json::from_str(r#"{"expected_revision":1,"edit":{"edit":"set_geometry_tolerances","tolerances":{"selection_m":1e-2,"topology_m":1e-6,"mesh_m":1e-4,"analysis_m":1e-8}}}"#).unwrap();
    arguments["expected_revision"] = json!(project.revision);
    assert!(arguments.to_string().contains("1e-6"));
    let changed = wire_call(&mut client, "prumo.model.edit", arguments).await;
    let revision = changed["project"]["revision"].as_u64().unwrap();
    assert_eq!(changed["project"]["units"], "centimetre");
    let basis = host.model_snapshot().unwrap();
    let before_bytes = std::fs::read(&path).unwrap();
    let audit = wire_call(
        &mut client,
        "prumo.geometry.audit",
        json!({"filter":{"discipline":"structure"}}),
    )
    .await;
    assert_eq!(audit["project_id"], project.id.to_string());
    assert_eq!(audit["revision"], revision);
    assert_eq!(audit["snapshot_sha256"], basis.sha256);
    close(&audit["tolerances"]["topology_m"], 1e-6, 1e-18);
    assert_eq!(audit["object_count"], 2);
    assert_eq!(audit["topology_is_explicit"], true);
    assert_eq!(audit["filter"]["discipline"], "structure");
    assert_eq!(audit["engineering_calculation_available"], false);
    assert!(
        audit["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["kind"] == "near_contact"
                && finding["explicitly_connected"] == false)
    );

    let face = json!({"vertices_m":[[0,0,0],[2,0,0],[2,3,0],[0,3,0]],"expected_normal":[0,0,1]});
    let inspected = wire_call(&mut client, "prumo.geometry.face", json!({"face":face})).await;
    assert_eq!(inspected["project_id"], project.id.to_string());
    assert_eq!(inspected["revision"], revision);
    assert_eq!(inspected["snapshot_sha256"], basis.sha256);
    assert_eq!(inspected["tolerances"], audit["tolerances"]);
    let canonical_face = serde_json::to_value(
        serde_json::from_value::<prumo_domain::geometry::FaceInput>(face.clone()).unwrap(),
    )
    .unwrap();
    assert_eq!(inspected["face"], canonical_face);
    assert_eq!(inspected["valid"], true);
    assert_eq!(inspected["inverted"], false);
    close(&inspected["area_m2"], 6.0, 1e-12);
    let mut inverted = face;
    inverted["expected_normal"] = json!([0, 0, -1]);
    let inspected = wire_call(&mut client, "prumo.geometry.face", json!({"face":inverted})).await;
    assert_eq!(inspected["inverted"], true);
    assert_eq!(inspected["valid"], false);

    let repairs = json!([{"repair":"snap_endpoint","object_id":a,"endpoint":"end",
        "target_m":[1.0000005,0,0],"reason":"Contato conferido no ensaio"}]);
    let preview = wire_call(
        &mut client,
        "prumo.geometry.preview",
        json!({"repairs":repairs}),
    )
    .await;
    assert_eq!(preview["revision"], revision);
    assert_eq!(preview["snapshot_sha256"], basis.sha256);
    hash(&preview["plan_hash"]);
    assert_eq!(preview["explicit_connections_added"], 0);
    assert_eq!(preview["dimensions_changed"].as_array().unwrap().len(), 1);
    close(&preview["dimensions_changed"][0]["before_m"], 1.0, 1e-12);
    close(
        &preview["dimensions_changed"][0]["after_m"],
        1.0000005,
        1e-12,
    );
    assert_eq!(std::fs::read(&path).unwrap(), before_bytes);
    let applied = wire_call(&mut client, "prumo.model.edit", json!({"expected_revision":revision,
        "edit":{"edit":"apply_geometry_repair","plan_hash":preview["plan_hash"],"repairs":repairs}})).await;
    assert_eq!(applied["project"]["revision"], revision + 1);
    assert_eq!(applied["project"]["objects"][0]["id"], a.to_string());
    close(
        &applied["project"]["objects"][0]["end_m"][0],
        1.0000005,
        1e-12,
    );
    assert!(
        applied["project"]["connections"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        applied["project"]["history"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["actor"],
        "mcp-local"
    );
    let saved = std::fs::read(&path).unwrap();
    denied(&mut client, "prumo.model.edit", json!({"expected_revision":revision+1,
        "edit":{"edit":"apply_geometry_repair","plan_hash":preview["plan_hash"],"repairs":repairs}}), "invalid_input").await;
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    client.stop().await;
    assert_eq!(
        serde_json::to_value(Desktop::new(path).project.unwrap()).unwrap(),
        applied["project"]
    );
}

#[tokio::test]
async fn verification_checks_calculation_scope_and_reports_the_real_binary_and_basis() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("verification-tools.prumo");
    let host = ProjectService::open(&path, AccessMode::ReadWrite, "host").unwrap();
    let project = host.create("Base da evidência", LengthUnit::Metre).unwrap();
    let before = std::fs::read(&path).unwrap();
    let basis = model_snapshot(&project).unwrap();
    let mut readonly = wire(&path, &[]);
    let protocols = wire_call(&mut readonly, "prumo.verification.protocols", json!({})).await;
    assert_eq!(protocols.as_array().unwrap().len(), 4);
    for protocol in protocols.as_array().unwrap() {
        hash(&protocol["protocol_sha256"]);
        assert_eq!(protocol["review"]["kind"], "software");
    }
    denied(
        &mut readonly,
        "prumo.verification.run",
        json!({}),
        "permission_denied",
    )
    .await;
    denied(
        &mut readonly,
        "prumo.geometry.preview",
        json!({"repairs":[]}),
        "permission_denied",
    )
    .await;
    denied(
        &mut readonly,
        "prumo.catalogue.section",
        json!({"entry_id":Uuid::new_v4()}),
        "permission_denied",
    )
    .await;
    readonly.stop().await;

    let mut partial = wire(
        &path,
        &[
            "--scopes",
            "read,model,calculation",
            "--disciplines",
            "structure",
        ],
    );
    for (name, arguments) in [
        (
            "prumo.geometry.audit",
            json!({"filter":{"discipline":"structure"}}),
        ),
        ("prumo.verification.run", json!({})),
        ("prumo.catalogue.read", json!({})),
    ] {
        denied(&mut partial, name, arguments, "permission_denied").await;
    }
    partial.stop().await;
    let client = sdk_options(&path, &["--scopes", "read,calculation"], false).await;
    let report = call(&client, "prumo.verification.run", json!({})).await;
    hash(&report["evidence_sha256"]);
    let evidence = &report["evidence"];
    assert_eq!(evidence["verification_passed"], true);
    assert_eq!(evidence["engineering_release"], false);
    assert_eq!(
        evidence["context"]["basis"]["project_id"],
        project.id.to_string()
    );
    assert_eq!(evidence["context"]["basis"]["revision"], project.revision);
    assert_eq!(evidence["context"]["basis"]["content_sha256"], basis.sha256);
    let binary = std::fs::read(BINARY).unwrap();
    assert_eq!(
        evidence["context"]["build"]["prumo_artifact_sha256"],
        hash_bytes(&binary)
    );
    assert_eq!(evidence["context"]["build"]["artifact_bytes"], binary.len());
    hash(&evidence["context"]["build"]["dependencies_sha256"]);
    assert_eq!(evidence["cases"].as_array().unwrap().len(), 4);
    for case in evidence["cases"].as_array().unwrap() {
        assert_eq!(case["decision"], "passed");
        assert!(
            protocols
                .as_array()
                .unwrap()
                .iter()
                .any(|protocol| protocol["protocol_sha256"] == case["prepared"]["protocol_sha256"])
        );
        for metric in case["metrics"].as_array().unwrap() {
            assert_eq!(metric["passed"], true);
            assert_eq!(metric["actual"]["unit"], metric["expected"]["unit"]);
            assert!(
                metric["absolute_error"].as_f64().unwrap()
                    <= metric["accepted_error"].as_f64().unwrap()
            );
        }
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
    client.cancel().await.unwrap();
}

fn source() -> Value {
    json!({"publisher":"Prumo synthetic fixture","reference":"Independent JSON input",
        "edition":"1","revision":"1","data_license":"Original test fixture",
        "original_record":"Decimal input supplied by this integration test"})
}

fn physical_value(quantity: &str, decimal: &str, unit: &str, basis: &str) -> Value {
    json!({"quantity":quantity,"decimal":decimal,"unit":unit,"basis":basis,"design_basis":null})
}

fn temperature(decimal: &str, unit: &str) -> Value {
    physical_value("absolute_temperature", decimal, unit, "nominal")
}

fn material(id: Uuid) -> Value {
    let range = json!({"quantity":"absolute_temperature",
        "minimum":temperature("273.15","kelvin"),"maximum":temperature("323.15","kelvin")});
    let mut design = physical_value(
        "strength",
        "20.083333333333333333333",
        "megapascal",
        "design",
    );
    design["design_basis"] = json!("Synthetic documented ratio, not a normative design value");
    json!({"id":id,"revision":1,"name":"Material de ensaio","source":source(),
        "applicability":[range],"customization":null,"payload":{"kind":"material","material":{
            "class":"Elastic fixture","states":[{"name":"Estado de referência","conditions":[range],
                "elastic":{"kind":"isotropic","young_modulus":physical_value("elastic_modulus","200.000000000000000001","gigapascal","nominal"),"poisson_ratio":0.3},
                "density":physical_value("density","7850","kilogram_per_cubic_metre","nominal"),
                "properties":{"characteristic_strength":physical_value("strength","30.125","megapascal","characteristic"),"design_strength":design}}]}}})
}

#[tokio::test]
async fn catalogue_roundtrip_preserves_decimal_sources_versions_assignments_and_section_units() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalogue-tools.prumo");
    let host = ProjectService::open(&path, AccessMode::ReadWrite, "host").unwrap();
    let project = host.create("Catálogo MCP", LengthUnit::Metre).unwrap();
    let project = host
        .add_member(project.revision, "Elemento", [0.0; 3], [1.0, 0.0, 0.0])
        .unwrap();
    let object = project.objects[0].id;
    let mut revision = project.revision;
    let section_id = Uuid::new_v4();
    let material_id = Uuid::new_v4();
    let mut section = json!({"id":section_id,"revision":1,"name":"Seção sintética","source":source(),
        "applicability":[],"customization":null,"payload":{"kind":"section","shape":
            serde_json::from_str::<Value>(r#"{"kind":"rectangle","width_m":2e-1,"height_m":4e-1}"#).unwrap()}});
    assert!(section.to_string().contains("2e-1"));
    let material = material(material_id);
    let mut client = wire(&path, &["--scopes", "read,model,calculation"]);
    for entry in [&section, &material] {
        let saved = wire_call(
            &mut client,
            "prumo.model.edit",
            json!({"expected_revision":revision,
            "edit":{"edit":"put_catalog_entry","entry":entry}}),
        )
        .await;
        revision = saved["project"]["revision"].as_u64().unwrap();
    }
    let view = wire_call(&mut client, "prumo.catalogue.read", json!({})).await;
    assert_eq!(view["revision"], revision);
    assert_eq!(
        view["snapshot_sha256"],
        host.model_snapshot().unwrap().sha256
    );
    let section_key = section_id.to_string();
    let material_key = material_id.to_string();
    assert_eq!(
        view["catalogue"]["entries"][&material_key][0]["entry"],
        material
    );
    assert_eq!(
        view["catalogue"]["entries"][&section_key][0]["actor"],
        "mcp-local"
    );
    let before_queries = std::fs::read(&path).unwrap();
    let report = wire_call(
        &mut client,
        "prumo.catalogue.section",
        json!({"entry_id":section_id}),
    )
    .await;
    assert_eq!(report["snapshot_sha256"], view["snapshot_sha256"]);
    assert_eq!(
        report["entry_sha256"],
        view["catalogue"]["entries"][&section_key][0]["entry_sha256"]
    );
    close(&report["analysis"]["geometry"]["area_m2"], 0.08, 1e-14);
    close(&report["analysis"]["geometry"]["centroid_m"][0], 0.1, 1e-14);
    close(&report["analysis"]["geometry"]["centroid_m"][1], 0.2, 1e-14);
    close(
        &report["analysis"]["geometry"]["ixx_m4"],
        0.2 * 0.4_f64.powi(3) / 12.0,
        1e-14,
    );
    close(
        &report["analysis"]["geometry"]["iyy_m4"],
        0.4 * 0.2_f64.powi(3) / 12.0,
        1e-14,
    );
    assert_eq!(report["analysis"]["torsion"]["state"], "analytical");
    assert!(report["analysis"]["torsion"]["j_m4"].as_f64().unwrap() > 0.0);
    assert_eq!(report["analysis"]["engineering_release"], false);
    let selected = wire_call(&mut client,"prumo.catalogue.material",json!({"entry_id":material_id,
        "conditions":{"absolute_temperature":temperature("20.00000000000000000001","degree_celsius")}})).await;
    assert_eq!(selected["revision"], revision);
    assert_eq!(selected["snapshot_sha256"], view["snapshot_sha256"]);
    hash(&selected["entry_sha256"]);
    assert_eq!(selected["engineering_release"], false);
    assert_eq!(
        selected["state"]["elastic"]["young_modulus"]["decimal"],
        "200.000000000000000001"
    );
    assert_eq!(
        selected["state"]["elastic"]["young_modulus"]["unit"],
        "gigapascal"
    );
    assert_eq!(
        selected["state"]["properties"]["characteristic_strength"]["basis"],
        "characteristic"
    );
    assert_eq!(
        selected["state"]["properties"]["design_strength"]["basis"],
        "design"
    );
    assert_eq!(
        selected["conditions"]["absolute_temperature"]["decimal"],
        "20.00000000000000000001"
    );
    for conditions in [
        json!({}),
        json!({"absolute_temperature":temperature("100","degree_celsius")}),
        json!({"absolute_temperature":temperature("20","watt")}),
    ] {
        denied(
            &mut client,
            "prumo.catalogue.material",
            json!({"entry_id":material_id,"conditions":conditions}),
            "invalid_input",
        )
        .await;
    }
    assert_eq!(std::fs::read(&path).unwrap(), before_queries);
    for entry_id in [section_id, material_id] {
        let saved = wire_call(
            &mut client,
            "prumo.model.edit",
            json!({"expected_revision":revision,
            "edit":{"edit":"assign_catalog_entry","object_id":object,"entry_id":entry_id}}),
        )
        .await;
        revision = saved["project"]["revision"].as_u64().unwrap();
    }
    let bound = host.read().unwrap();
    let assignments = prumo_domain::catalogue::assignments(&bound.objects[0]).unwrap();
    assert_eq!(
        assignments[&prumo_domain::catalogue::CatalogKind::Section],
        section_id
    );
    assert_eq!(
        assignments[&prumo_domain::catalogue::CatalogKind::Material],
        material_id
    );
    section["revision"] = json!(2);
    section["source"]["revision"] = json!("2");
    section["payload"]["shape"]["width_m"] = json!(0.25);
    let saved = wire_call(
        &mut client,
        "prumo.model.edit",
        json!({"expected_revision":revision,
        "edit":{"edit":"put_catalog_entry","entry":section}}),
    )
    .await;
    revision = saved["project"]["revision"].as_u64().unwrap();
    let updated = wire_call(&mut client, "prumo.catalogue.read", json!({})).await;
    assert_ne!(updated["snapshot_sha256"], view["snapshot_sha256"]);
    let versions = updated["catalogue"]["entries"][&section_key]
        .as_array()
        .unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0], view["catalogue"]["entries"][&section_key][0]);
    assert_eq!(versions[1]["entry"]["revision"], 2);
    let mut final_project = Value::Null;
    for kind in ["section", "material"] {
        let saved = wire_call(
            &mut client,
            "prumo.model.edit",
            json!({"expected_revision":revision,
            "edit":{"edit":"clear_catalog_assignment","object_id":object,"kind":kind}}),
        )
        .await;
        revision = saved["project"]["revision"].as_u64().unwrap();
        final_project = saved["project"].clone();
    }
    assert!(
        prumo_domain::catalogue::assignments(&host.read().unwrap().objects[0])
            .unwrap()
            .is_empty()
    );
    client.stop().await;
    let reopened = sdk(&path, false).await;
    assert_eq!(
        call(&reopened, "prumo.project.read", json!({})).await["project"],
        final_project
    );
    assert_eq!(
        call(&reopened, "prumo.catalogue.read", json!({})).await["catalogue"],
        updated["catalogue"]
    );
    reopened.cancel().await.unwrap();
    assert_eq!(
        serde_json::to_value(Desktop::new(path).project.unwrap()).unwrap(),
        final_project
    );
}
