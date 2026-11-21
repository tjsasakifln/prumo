use prumo_domain::identity::IfcGuid;
use prumo_domain::model::{ModelEdit, PhysicalRole, PointInput};
use prumo_domain::precalculation::{
    self, Calibration, DrawingReference, Freedom, IfcCandidate, IfcReference, IfcSchema,
    NodeSupport, PrecalculationEdit, ReferenceFormat, ReferenceTrace,
};
use prumo_domain::{
    ChangeRecord, Command, Discipline, LengthUnit, Project, PROJECT_SCHEMA_VERSION,
};
use uuid::Uuid;

fn project() -> Project {
    Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name: "Modelo de prova".into(),
        revision: 1,
        units: LengthUnit::Metre,
        objects: Vec::new(),
        environments: Vec::new(),
        networks: Vec::new(),
        ports: Vec::new(),
        connections: Vec::new(),
        history: vec![ChangeRecord {
            revision: 1,
            actor: "revisor".into(),
            operation: "create".into(),
            timestamp_unix_ms: 0,
        }],
        normative: Default::default(),
        model: Default::default(),
        dependencies: Default::default(),
        extensions: Default::default(),
    }
}

fn execute(project: &mut Project, edit: ModelEdit) -> Result<(), prumo_domain::DomainError> {
    prumo_domain::model::apply_model_edit(project, edit, "revisor")?;
    project.revision += 1;
    project.history.push(ChangeRecord {
        revision: project.revision,
        actor: "revisor".into(),
        operation: "edit".into(),
        timestamp_unix_ms: 0,
    });
    project.validate()?;
    Ok(())
}

fn stage(project: &mut Project, edit: PrecalculationEdit) -> Result<(), prumo_domain::DomainError> {
    execute(project, ModelEdit::EditPrecalculation { command: edit })
}

fn at(values: [f64; 3]) -> PointInput {
    PointInput { values, unit: LengthUnit::Metre }
}

fn support(id: Uuid, node_id: Uuid, stiffness: f64) -> NodeSupport {
    NodeSupport {
        id, node_id,
        rotation_to_engineering: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        freedoms: [
            Freedom::Spring { stiffness_si: stiffness },
            Freedom::Fixed {}, Freedom::Fixed {},
            Freedom::Free {}, Freedom::Free {}, Freedom::Free {},
        ],
        justification: "Apoio girado noventa graus".into(),
    }
}

fn drawing(id: Uuid, discrepancy: bool) -> DrawingReference {
    DrawingReference {
        id,
        label: "Planta do pavimento".into(),
        format: ReferenceFormat::Dxf,
        sha256: "c".repeat(64),
        calibration: Calibration {
            measured_a: 100.0, actual_a_m: 4.0,
            measured_b: 50.0, actual_b_m: if discrepancy { 2.4 } else { 2.0 },
        },
        origin_m: [10.0, 5.0, 0.0],
        traces: vec![ReferenceTrace {
            id: Uuid::new_v4(), layer: "Estrutura".into(),
            a: [0.0, 0.0, 0.0], b: [100.0, 0.0, 0.0], visible: true,
        }],
    }
}

#[test]
fn rotated_support_retains_six_explicit_freedoms_and_rejects_conflicting_support() {
    let mut p = project();
    stage(&mut p, PrecalculationEdit::PutSupport {
        support: support(Uuid::new_v4(), Uuid::new_v4(), 1000.0),
    }).expect_err("sem nó analítico");
    execute(&mut p, ModelEdit::AddAnalyticalNode {
        name: "N01".into(), position: at([0.0, 0.0, 0.0]),
    }).unwrap();
    let node = p.model.analytical_nodes[0].id;
    let input = support(Uuid::new_v4(), node, 1e6);
    stage(&mut p, PrecalculationEdit::PutSupport { support: input.clone() }).unwrap();
    let s = precalculation::state(&p).unwrap();
    assert_eq!(s.supports[0], input);
    assert_eq!(s.supports[0].rotation_to_engineering[1][0], 1.0);
    assert!(stage(&mut p, PrecalculationEdit::PutSupport {
        support: support(Uuid::new_v4(), node, 1e3),
    }).is_err());
    assert!(stage(&mut p, PrecalculationEdit::PutSupport {
        support: support(input.id, node, f64::NAN),
    }).is_err());
}

#[test]
fn distorted_reference_blocks_promotion_and_fixed_scale_preserves_distance() {
    let mut p = project();
    let id = Uuid::new_v4();
    let distorted = drawing(id, true);
    let trace_id = distorted.traces[0].id;
    stage(&mut p, PrecalculationEdit::PutDrawing {
        reference: distorted.clone(), expected_sha256: None,
    }).unwrap();
    assert!(precalculation::review(&p).unwrap().findings.iter().any(|f|
        f.code == "distorted_reference"));
    assert!(stage(&mut p, PrecalculationEdit::PromoteDrawing {
        reference_id: id, trace_id, name: "Viga".into(),
        material_id: None, confirm_dimensions: true,
    }).is_err());
    let mut fixed = distorted;
    fixed.calibration.actual_b_m = 2.0;
    stage(&mut p, PrecalculationEdit::PutDrawing {
        reference: fixed.clone(), expected_sha256: Some("c".repeat(64)),
    }).unwrap();
    stage(&mut p, PrecalculationEdit::PromoteDrawing {
        reference_id: id, trace_id, name: "Viga".into(),
        material_id: None, confirm_dimensions: true,
    }).unwrap();
    let beam = &p.objects[0];
    assert_eq!(beam.start_m.as_array(), [10.0, 5.0, 0.0]);
    assert_eq!(beam.end_m.as_array(), [14.0, 5.0, 0.0]);
    assert!(matches!(&p.model.identities[&beam.id].origin,
        prumo_domain::identity::SourceOrigin::Imported {..}));
}

#[test]
fn staged_ifc_never_implicitly_creates_calculated_members_and_preserves_guid() {
    let mut p = project();
    let id = Uuid::new_v4();
    let guid = IfcGuid::from_uuid(Uuid::new_v4()).as_str().to_owned();
    stage(&mut p, PrecalculationEdit::PutIfc {
        reference: IfcReference {
            id, label: "Modelo arquitetônico".into(),
            sha256: "a".repeat(64), schema: IfcSchema::Ifc4x3,
            unknown_entity_count: 7,
            candidates: vec![IfcCandidate {
                guid: guid.clone(), name: "Pilar P01".into(),
                entity_type: "IfcColumn".into(),
                start_m: [0.0, 0.0, 0.0], end_m: [0.0, 0.0, 3.0],
            }],
        },
        expected_sha256: None,
    }).unwrap();
    assert!(p.objects.is_empty());
    assert!(!precalculation::review(&p).unwrap().calculation_approved);
    assert!(stage(&mut p, PrecalculationEdit::PromoteIfc {
        reference_id: id, guid: guid.clone(), material_id: None,
        confirm_hypotheses: false,
    }).is_err());
    stage(&mut p, PrecalculationEdit::PromoteIfc {
        reference_id: id, guid: guid.clone(), material_id: None,
        confirm_hypotheses: true,
    }).unwrap();
    let obj = &p.objects[0];
    assert_eq!(p.model.identities[&obj.id].ifc_guid.as_ref().unwrap().as_str(), guid);
    let restored: Project = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.model.identities[&obj.id].ifc_guid.as_ref().unwrap().as_str(), guid);
    assert!(stage(&mut p, PrecalculationEdit::PromoteIfc {
        reference_id: id, guid, material_id: None, confirm_hypotheses: true,
    }).is_err());
}

#[test]
fn batch_copies_have_distinct_identities_and_explicit_source_override() {
    let mut p = project();
    execute(&mut p, ModelEdit::AddPhysical {
        name: "Viga origem".into(), discipline: Discipline::Structure,
        start: at([0.0,0.0,0.0]), end: at([4.0,0.0,0.0]),
        role: PhysicalRole::VisualOnly, material_id: None, environment_id: None,
    }).unwrap();
    let original = p.objects[0].id;
    stage(&mut p, PrecalculationEdit::CopyBatch {
        object_ids: vec![original],
        translation: at([0.0,0.0,3.0]),
        linked: true,
    }).unwrap();
    let replica = p.objects[1].id;
    assert_ne!(original, replica);
    assert_eq!(p.objects[1].start_m.as_array(), [0.0,0.0,3.0]);
    let instance = precalculation::state(&p).unwrap().instances[0].clone();
    assert_eq!(instance.source_id, original);
    assert!(instance.linked);
    assert!(!instance.overridden);
    stage(&mut p, PrecalculationEdit::SetMaterialBatch {
        object_ids: vec![replica], material_id: None,
    }).unwrap();
    assert!(precalculation::state(&p).unwrap().instances[0].overridden);
    assert_eq!(p.objects[0].start_m.as_array(), [0.0,0.0,0.0]);
}

#[test]
fn staging_rejects_invalid_hash_and_revision_conflict() {
    let mut p = project();
    let id = Uuid::new_v4();
    let mut source = drawing(id, false);
    source.sha256 = "invalid".into();
    assert!(stage(&mut p, PrecalculationEdit::PutDrawing {
        reference: source, expected_sha256: None,
    }).is_err());
    let first = drawing(id, false);
    stage(&mut p, PrecalculationEdit::PutDrawing {
        reference: first.clone(), expected_sha256: None,
    }).unwrap();
    assert!(stage(&mut p, PrecalculationEdit::PutDrawing {
        reference: first, expected_sha256: None,
    }).is_err());
}

#[test]
fn model_edit_command_json_uses_shared_typed_contract() {
    let command = Command::EditModel {
        expected_revision: 1,
        edit: ModelEdit::EditPrecalculation {
            command: PrecalculationEdit::RemoveSupport { id: Uuid::new_v4() },
        },
    };
    let json = serde_json::to_string(&command).unwrap();
    assert!(json.contains("edit_precalculation"));
    assert!(json.contains("remove_support"));
    let restored: Command = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), serde_json::to_value(command).unwrap());
}
