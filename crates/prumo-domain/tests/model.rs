use prumo_domain::identity::{IfcGuid, SourceOrigin};
use prumo_domain::model::*;
use prumo_domain::units::*;
use prumo_domain::{
    ChangeRecord, Discipline, DomainError, LengthUnit, ModelState, PROJECT_SCHEMA_VERSION,
    PortKind, Project,
};
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

fn empty() -> Project {
    Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name: "Fixture sem cálculo".into(),
        revision: 1,
        units: LengthUnit::Metre,
        objects: vec![],
        environments: vec![],
        networks: vec![],
        ports: vec![],
        connections: vec![],
        history: vec![ChangeRecord {
            revision: 1,
            actor: "fixture".into(),
            operation: "create".into(),
            timestamp_unix_ms: 0,
        }],
        normative: Default::default(),
        model: Default::default(),
        dependencies: Default::default(),
        extensions: BTreeMap::new(),
    }
}
fn stamp(project: &mut Project) {
    project.revision += 1;
    project.history.push(ChangeRecord {
        revision: project.revision,
        actor: "fixture".into(),
        operation: "edit_model".into(),
        timestamp_unix_ms: project.revision,
    });
    project.validate().unwrap();
}
fn apply(project: &mut Project, edit: ModelEdit) -> ModelChange {
    let result = apply_model_edit(project, edit, "fixture").unwrap();
    stamp(project);
    result
}
fn point(values: [f64; 3]) -> PointInput {
    PointInput {
        values,
        unit: LengthUnit::Metre,
    }
}
fn add_physical(
    project: &mut Project,
    name: &str,
    start: [f64; 3],
    end: [f64; 3],
    role: PhysicalRole,
    discipline: Discipline,
) -> Uuid {
    apply(
        project,
        ModelEdit::AddPhysical {
            name: name.into(),
            discipline,
            start: point(start),
            end: point(end),
            role,
            material_id: None,
            environment_id: None,
        },
    )
    .created_ids[0]
}
fn bar(project: &mut Project, name: &str, start: [f64; 3], end: [f64; 3]) -> Uuid {
    let id = add_physical(
        project,
        name,
        start,
        end,
        PhysicalRole::AnalyticalSource,
        Discipline::Structure,
    );
    let revision = project.revision + 1;
    initialize_legacy_member(project, id, None, "fixture", revision).unwrap();
    stamp(project);
    id
}
fn material_input() -> MaterialInput {
    MaterialInput {
        name: "Dados elásticos fornecidos".into(),
        family: MaterialFamily::Generic,
        elastic_modulus: PressureInput {
            value: 200.0,
            unit: PressureUnit::Gigapascal,
        },
        density: DensityInput {
            value: 7850.0,
            unit: DensityUnit::KilogramPerCubicMetre,
        },
        poisson_ratio: 0.3,
    }
}
fn add_action(project: &mut Project, node: Uuid, frame: ActionFrame) -> Uuid {
    apply(
        project,
        ModelEdit::AddNodalAction {
            action: NodalActionInput {
                name: "Força e casal de ensaio".into(),
                application_node_id: node,
                force: ForceInput {
                    values: [0.0, 10.0, 0.0],
                    unit: ForceUnit::Newton,
                },
                couple: MomentInput {
                    values: [0.0, 0.0, 3.0],
                    unit: MomentUnit::NewtonMetre,
                },
                frame,
            },
        },
    )
    .created_ids[0]
}

#[test]
fn quantities_preserve_dimensions_signs_and_exact_si_references() {
    assert_eq!(
        PointInput {
            values: [2500.0, -1000.0, -0.0],
            unit: LengthUnit::Millimetre
        }
        .to_si()
        .unwrap()
        .as_array(),
        [2.5, -1.0, 0.0]
    );
    let force = Force3::try_from(ForceInput {
        values: [1.0, -2.0, -0.0],
        unit: ForceUnit::Kilonewton,
    })
    .unwrap();
    assert_eq!(force.si(), [1000.0, -2000.0, 0.0]);
    assert!(!force.si()[2].is_sign_negative());
    let moment = Moment3::try_from(MomentInput {
        values: [2.0, 0.0, 0.0],
        unit: MomentUnit::KilonewtonMetre,
    })
    .unwrap();
    assert_eq!(moment.si(), [2000.0, 0.0, 0.0]);
    assert!(
        serde_json::from_value::<MomentInput>(json!({"values":[0,0,1],"unit":"kilonewton"}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<ForceInput>(json!({"values":[0,0,1],"unit":"kilonewton_metre"}))
            .is_err()
    );
    assert_eq!(
        Length::new(25.0, LengthUnit::Centimetre).unwrap().si(),
        0.25
    );
    assert_eq!(Mass::new(2.0, MassUnit::Tonne).unwrap().si(), 2000.0);
    assert_eq!(Time::new(2.0, TimeUnit::Hour).unwrap().si(), 7200.0);
    assert!(
        (Angle::new(90.0, AngleUnit::Degree).unwrap().si() - std::f64::consts::FRAC_PI_2).abs()
            < 1.0e-15
    );
    assert_eq!(
        Stiffness::new(20.0, StiffnessUnit::KilonewtonPerMetre)
            .unwrap()
            .si(),
        20000.0
    );
    assert_eq!(
        Stress::new(-2.0, PressureUnit::Megapascal).unwrap().si(),
        -2.0e6
    );
    assert!(
        (AbsoluteTemperature::new(20.0, TemperatureUnit::DegreeCelsius)
            .unwrap()
            .si()
            - 293.15)
            .abs()
            < 1.0e-12
    );
    assert_eq!(
        TemperatureInterval::new(20.0, TemperatureUnit::DegreeCelsius)
            .unwrap()
            .si(),
        20.0
    );
    assert!(AbsoluteTemperature::new(-274.0, TemperatureUnit::DegreeCelsius).is_err());
    assert!(
        Pressure::try_from(PressureInput {
            value: 0.0,
            unit: PressureUnit::Pascal
        })
        .is_err()
    );
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Point3::try_from([bad, 0.0, 0.0]).is_err());
        assert!(Force3::from_si([bad, 0.0, 0.0]).is_err());
        assert!(Moment3::from_si([bad, 0.0, 0.0]).is_err());
        assert!(Angle::try_from(bad).is_err());
    }
}

#[test]
fn ifc_guid_matches_the_independent_buildingsmart_vector() {
    let id = Uuid::parse_str("f70dd363-bfe3-495d-84a0-2c02dcb7d4d2").unwrap();
    let guid = IfcGuid::from_uuid(id);
    assert_eq!(guid.as_str(), "3t3TDZl_D9NOIWB0BSjzJI");
    assert_eq!(guid.to_uuid(), id);
    assert_eq!(
        IfcGuid::try_from("3t3TDZl_D9NOIWB0BSjzJI".to_owned())
            .unwrap()
            .to_uuid(),
        id
    );
    for bad in [
        "",
        "4t3TDZl_D9NOIWB0BSjzJI",
        "3t3TDZl/D9NOIWB0BSjzJI",
        "3t3TDZl_D9NOIWB0BSjzJ",
    ] {
        assert!(IfcGuid::try_from(bad.to_owned()).is_err());
    }
}

#[test]
fn tagged_unit_like_variants_reject_unknown_fields_instead_of_ignoring_them() {
    assert!(
        serde_json::from_value::<SourceOrigin>(json!({"kind":"native","ifc_guid":"forged"}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<ActionFrame>(
            json!({"kind":"engineering","member_id":Uuid::new_v4()})
        )
        .is_err()
    );
    assert!(serde_json::from_value::<SourceOrigin>(json!({"kind":"native"})).is_ok());
    assert!(serde_json::from_value::<ActionFrame>(json!({"kind":"engineering"})).is_ok());
}

#[test]
fn optional_updates_require_presence_and_only_explicit_null_removes_values() {
    let mut project = empty();
    let object_id = bar(&mut project, "Peça com material", [0.0; 3], [2.0, 0.0, 0.0]);
    let material_id = apply(
        &mut project,
        ModelEdit::AddMaterial {
            material: material_input(),
        },
    )
    .created_ids[0];
    apply(
        &mut project,
        ModelEdit::SetPhysicalMaterial {
            object_id,
            material_id: Some(material_id),
        },
    );
    apply(
        &mut project,
        ModelEdit::SetGeoreference {
            georeference: Some(Georeference {
                crs_identifier: "CRS declarado".into(),
                map_origin_m: Point3::ORIGIN,
                rotation_radians: Angle::try_from(0.0).unwrap(),
                scale: 1.0,
            }),
        },
    );
    assert!(serde_json::from_value::<ModelEdit>(json!({"edit":"set_georeference"})).is_err());
    assert!(
        serde_json::from_value::<ModelEdit>(
            json!({"edit":"set_physical_material","object_id":object_id})
        )
        .is_err()
    );
    assert_eq!(project.objects[0].material_id, Some(material_id));
    assert!(project.model.georeference.is_some());
    let clear_material: ModelEdit = serde_json::from_value(
        json!({"edit":"set_physical_material","object_id":object_id,"material_id":null}),
    )
    .unwrap();
    apply(&mut project, clear_material);
    assert!(project.objects[0].material_id.is_none());
    let clear_georeference: ModelEdit =
        serde_json::from_value(json!({"edit":"set_georeference","georeference":null})).unwrap();
    apply(&mut project, clear_georeference);
    assert!(project.model.georeference.is_none());
    // Creation still permits unspecified optional associations.
    assert!(serde_json::from_value::<ModelEdit>(json!({"edit":"add_physical","name":"Visual","discipline":"structure","role":"visual_only","start":{"values":[0,0,0],"unit":"metre"},"end":{"values":[1,0,0],"unit":"metre"}})).is_ok());
    assert!(serde_json::from_value::<ModelEdit>(json!({"edit":"add_network","name":"Rede","discipline":"water_sanitation","kind":"network"})).is_ok());
}

#[test]
fn native_roundtrip_preserves_material_actions_guids_and_many_to_many_mapping() {
    let mut p = empty();
    let a = bar(&mut p, "A", [0.0; 3], [2.0, 0.0, 0.0]);
    let b = bar(&mut p, "B", [2.0, 0.0, 0.0], [4.0, 0.0, 0.0]);
    let first = p.model.analytical_members[0].id;
    let second = p.model.analytical_members[1].id;
    apply(
        &mut p,
        ModelEdit::LinkPhysicalAnalytical {
            physical_id: a,
            analytical_id: second,
        },
    );
    apply(
        &mut p,
        ModelEdit::LinkPhysicalAnalytical {
            physical_id: b,
            analytical_id: first,
        },
    );
    let material_id = apply(
        &mut p,
        ModelEdit::AddMaterial {
            material: material_input(),
        },
    )
    .created_ids[0];
    apply(
        &mut p,
        ModelEdit::SetPhysicalMaterial {
            object_id: a,
            material_id: Some(material_id),
        },
    );
    let node = p.model.analytical_nodes[0].id;
    add_action(&mut p, node, ActionFrame::Engineering {});
    let encoded = serde_json::to_value(&p).unwrap();
    let reopened = decode_project(encoded).unwrap();
    assert_eq!(p, reopened);
    assert_eq!(
        reopened
            .model
            .links
            .iter()
            .filter(|x| x.physical_id == a)
            .count(),
        2
    );
    assert_eq!(
        reopened
            .model
            .links
            .iter()
            .filter(|x| x.analytical_id == first)
            .count(),
        2
    );
    assert!(
        reopened
            .objects
            .iter()
            .all(|x| x.state == ModelState::Modelled)
    );
}

#[test]
fn textual_native_roundtrip_preserves_coordinate_force_and_moment_bits() {
    let value = 1.910_905_980_455_653_9_f64;
    let mut p = empty();
    bar(
        &mut p,
        "Precisão textual",
        [value, 0.0, 0.0],
        [value + 2.0, 0.0, 0.0],
    );
    let node = p.model.analytical_nodes[0].id;
    apply(
        &mut p,
        ModelEdit::AddNodalAction {
            action: NodalActionInput {
                name: "Valores declarados".into(),
                application_node_id: node,
                force: ForceInput {
                    values: [value, -value, 0.0],
                    unit: ForceUnit::Newton,
                },
                couple: MomentInput {
                    values: [value, 0.0, -value],
                    unit: MomentUnit::NewtonMetre,
                },
                frame: ActionFrame::Engineering {},
            },
        },
    );
    let bytes = serde_json::to_vec(&p).unwrap();
    let decoded: Project = serde_json::from_slice(&bytes).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded.objects[0].start_m[0].to_bits(), value.to_bits());
    assert_eq!(
        decoded.model.actions[0].force.si()[0].to_bits(),
        value.to_bits()
    );
    assert_eq!(
        decoded.model.actions[0].couple.si()[0].to_bits(),
        value.to_bits()
    );
    let value_form: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decode_project(value_form).unwrap(), p);
    assert_eq!(decoded, p);
}

#[test]
fn visual_only_and_orphan_links_are_rejected_atomically() {
    let mut p = empty();
    let visual = add_physical(
        &mut p,
        "Visual",
        [0.0; 3],
        [2.0, 0.0, 0.0],
        PhysicalRole::VisualOnly,
        Discipline::Structure,
    );
    bar(&mut p, "Fonte", [0.0, 1.0, 0.0], [2.0, 1.0, 0.0]);
    let member = p.model.analytical_members[0].id;
    let before = p.clone();
    assert!(
        apply_model_edit(
            &mut p,
            ModelEdit::LinkPhysicalAnalytical {
                physical_id: visual,
                analytical_id: member
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(p, before);
    let source = p.objects[1].id;
    assert!(
        apply_model_edit(
            &mut p,
            ModelEdit::LinkPhysicalAnalytical {
                physical_id: source,
                analytical_id: Uuid::new_v4()
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(p, before);
    let mut forged = p.clone();
    forged.model.links[0].analytical_id = Uuid::new_v4();
    assert!(forged.validate().is_err());
}

#[test]
fn poisson_endpoints_and_material_orphans_fail_without_changes() {
    let mut p = empty();
    let before = p.clone();
    for poisson in [-1.0, 0.5, f64::NAN] {
        let mut m = material_input();
        m.poisson_ratio = poisson;
        assert!(
            apply_model_edit(&mut p, ModelEdit::AddMaterial { material: m }, "fixture").is_err()
        );
        assert_eq!(p, before);
    }
    let id = bar(&mut p, "Peça", [0.0; 3], [1.0, 0.0, 0.0]);
    let before = p.clone();
    assert!(
        apply_model_edit(
            &mut p,
            ModelEdit::SetPhysicalMaterial {
                object_id: id,
                material_id: Some(Uuid::new_v4())
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(p, before);
}

#[test]
fn active_rigid_motion_preserves_length_and_rotates_force_and_free_couple() {
    let mut p = empty();
    let id = bar(&mut p, "Peça", [1.0, 2.0, 0.0], [3.0, 2.0, 0.0]);
    let node = p.model.analytical_nodes[0].id;
    add_action(&mut p, node, ActionFrame::Engineering {});
    apply(
        &mut p,
        ModelEdit::RigidMove {
            object_ids: vec![id],
            rotation: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            translation: point([5.0, -1.0, 0.0]),
        },
    );
    assert_eq!(p.objects[0].length_m(), 2.0);
    assert_eq!(p.objects[0].start_m.as_array(), [3.0, 0.0, 0.0]);
    assert_eq!(p.model.actions[0].force.si(), [-10.0, 0.0, 0.0]);
    assert_eq!(p.model.actions[0].couple.si(), [0.0, 0.0, 3.0]);
    let (force, moment) = resultant_about(
        &p,
        &p.model.actions[0],
        Point3::try_from([5.0, -1.0, 0.0]).unwrap(),
    )
    .unwrap();
    assert_eq!(force, [-10.0, 0.0, 0.0]);
    assert_eq!(moment, [0.0, 0.0, 13.0]);
}

#[test]
fn passive_frame_change_preserves_world_position_and_resultant_about_same_point() {
    let mut p = empty();
    bar(&mut p, "Peça", [1.0, 2.0, 0.0], [3.0, 2.0, 0.0]);
    let node = p.model.analytical_nodes[0].id;
    add_action(&mut p, node, ActionFrame::Engineering {});
    apply(
        &mut p,
        ModelEdit::ChangeEngineeringFrame {
            frame: EngineeringFrame {
                origin_world_m: Point3::try_from([5.0, -1.0, 0.0]).unwrap(),
                rotation_to_world: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            },
        },
    );
    assert_eq!(p.objects[0].start_m.as_array(), [3.0, 4.0, 0.0]);
    assert_eq!(p.model.actions[0].force.si(), [10.0, 0.0, 0.0]);
    assert_eq!(p.model.actions[0].couple.si(), [0.0, 0.0, 3.0]);
    // Old world origin has new coordinates (1,5,0). Its torque remains +13 Nm.
    assert_eq!(
        resultant_about(
            &p,
            &p.model.actions[0],
            Point3::try_from([1.0, 5.0, 0.0]).unwrap()
        )
        .unwrap()
        .1,
        [0.0, 0.0, 13.0]
    );
    // About the new origin, the independent arm (-4,3,0) x (0,10,0) gives -40+3.
    assert_eq!(
        resultant_about(&p, &p.model.actions[0], Point3::ORIGIN)
            .unwrap()
            .1,
        [0.0, 0.0, -37.0]
    );
    assert_eq!(p.objects[0].length_m(), 2.0);
}

#[test]
fn local_load_and_collection_permutation_preserve_the_same_physical_action() {
    let mut p = empty();
    bar(&mut p, "Peça", [0.0; 3], [0.0, 2.0, 0.0]);
    let node = p.model.analytical_nodes[0].id;
    let member = p.model.analytical_members[0].id;
    add_action(&mut p, node, ActionFrame::MemberLocal { member_id: member });
    let expected = force_and_couple_in_engineering(&p, &p.model.actions[0]).unwrap();
    p.model.analytical_nodes.reverse();
    p.model.analytical_members.reverse();
    p.model.links.reverse();
    p.objects.reverse();
    p.validate().unwrap();
    assert_eq!(
        force_and_couple_in_engineering(&p, &p.model.actions[0]).unwrap(),
        expected
    );
    let member = &mut p.model.analytical_members[0];
    std::mem::swap(&mut member.start_node_id, &mut member.end_node_id);
    p.validate().unwrap();
    let reversed = force_and_couple_in_engineering(&p, &p.model.actions[0]).unwrap();
    // Reversing x changes z. Adjusting local couple z restores its world vector.
    assert_eq!(reversed.0, expected.0);
    assert_eq!(reversed.1, expected.1.map(|x| -x));
    p.model.actions[0].couple = Moment3::from_si([0.0, 0.0, -3.0]).unwrap();
    assert_eq!(
        force_and_couple_in_engineering(&p, &p.model.actions[0]).unwrap(),
        expected
    );
}

#[test]
fn shared_member_and_shared_node_prevent_partial_rigid_motion() {
    let mut p = empty();
    let a = bar(&mut p, "A", [0.0; 3], [2.0, 0.0, 0.0]);
    let b = add_physical(
        &mut p,
        "B",
        [0.0; 3],
        [2.0, 0.0, 0.0],
        PhysicalRole::AnalyticalSource,
        Discipline::Structure,
    );
    let member = p.model.analytical_members[0].id;
    apply(
        &mut p,
        ModelEdit::LinkPhysicalAnalytical {
            physical_id: b,
            analytical_id: member,
        },
    );
    let before = p.clone();
    let edit = ModelEdit::RigidMove {
        object_ids: vec![a],
        rotation: EngineeringFrame::default().rotation_to_world,
        translation: point([1.0, 0.0, 0.0]),
    };
    assert!(matches!(
        apply_model_edit(&mut p, edit, "fixture"),
        Err(DomainError::SharedTopology)
    ));
    assert_eq!(p, before);
    apply(
        &mut p,
        ModelEdit::RigidMove {
            object_ids: vec![a, b],
            rotation: EngineeringFrame::default().rotation_to_world,
            translation: point([1.0, 0.0, 0.0]),
        },
    );
    assert_eq!(
        p.model.analytical_nodes[0].position_m.as_array(),
        [1.0, 0.0, 0.0]
    );
    let shared = p.model.analytical_nodes[1].id;
    let node = apply(
        &mut p,
        ModelEdit::AddAnalyticalNode {
            name: "Extensão".into(),
            position: point([5.0, 0.0, 0.0]),
        },
    )
    .created_ids[0];
    apply(
        &mut p,
        ModelEdit::AddAnalyticalMember {
            name: "Membro fora da seleção".into(),
            start_node_id: shared,
            end_node_id: node,
            local_y: [0.0, 1.0, 0.0],
        },
    );
    let before = p.clone();
    assert!(matches!(
        apply_model_edit(
            &mut p,
            ModelEdit::RigidMove {
                object_ids: vec![a, b],
                rotation: EngineeringFrame::default().rotation_to_world,
                translation: point([1.0, 0.0, 0.0])
            },
            "fixture"
        ),
        Err(DomainError::SharedTopology)
    ));
    assert_eq!(p, before);
}

#[test]
fn copy_merge_split_preserve_ancestry_guids_and_analytical_mappings() {
    let mut p = empty();
    let source = bar(&mut p, "Original", [0.0; 3], [4.0, 0.0, 0.0]);
    let original_guid = p.model.identities[&source].ifc_guid.clone();
    let member = p.model.analytical_members[0].id;
    let copied = apply(
        &mut p,
        ModelEdit::CopyPhysical {
            source_id: source,
            name: "Cópia".into(),
            translation: point([0.0, 5.0, 0.0]),
        },
    )
    .created_ids[0];
    assert_ne!(p.model.identities[&copied].ifc_guid, original_guid);
    assert!(!p.model.links.iter().any(|x| x.physical_id == copied));
    assert_eq!(p.model.identities[&source].ifc_guid, original_guid);
    apply(
        &mut p,
        ModelEdit::SplitPhysical {
            object_id: source,
            fraction: 0.25,
            names: ["Parte A".into(), "Parte B".into()],
        },
    );
    let children = p.model.lineage.last().unwrap().child_ids.clone();
    assert_eq!(
        p.model
            .links
            .iter()
            .filter(|x| x.analytical_id == member)
            .count(),
        2
    );
    assert_eq!(
        p.model.lineage.last().unwrap().retired_objects[0].id,
        source
    );
    assert_eq!(p.model.identities[&source].ifc_guid, original_guid);
    apply(
        &mut p,
        ModelEdit::MergePhysical {
            source_ids: children.clone(),
            name: "Reunida".into(),
        },
    );
    let merged = p.model.lineage.last().unwrap().child_ids[0];
    assert_eq!(
        p.objects
            .iter()
            .find(|x| x.id == merged)
            .unwrap()
            .length_m(),
        4.0
    );
    assert_eq!(
        p.model
            .links
            .iter()
            .filter(|x| x.physical_id == merged)
            .count(),
        1
    );
    assert_eq!(p.model.analytical_members[0].id, member);
    assert!(
        children
            .iter()
            .all(|id| p.model.identities[id].retired_revision.is_some())
    );
    let reopened = decode_project(serde_json::to_value(&p).unwrap()).unwrap();
    assert_eq!(reopened, p);
}

#[test]
fn forged_lineage_and_retirement_chronology_are_rejected_on_reopen() {
    let mut p = empty();
    let a = bar(&mut p, "A", [0.0; 3], [1.0, 0.0, 0.0]);
    let b = bar(&mut p, "B", [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
    apply(
        &mut p,
        ModelEdit::MergePhysical {
            source_ids: vec![a, b],
            name: "AB".into(),
        },
    );
    let mut forged = p.clone();
    forged.model.lineage[0].parent_ids[1] = a;
    assert!(forged.validate().is_err());
    let mut forged = p.clone();
    forged
        .model
        .identities
        .get_mut(&a)
        .unwrap()
        .retired_revision = Some(1);
    assert!(forged.validate().is_err());
    let child = p.model.lineage[0].child_ids[0];
    let mut forged = p.clone();
    forged
        .model
        .identities
        .get_mut(&child)
        .unwrap()
        .created_revision = Some(1);
    assert!(forged.validate().is_err());
    let mut forged = p.clone();
    forged.model.lineage[0].retired_objects.clear();
    assert!(forged.validate().is_err());
}

#[test]
fn visual_crossing_has_no_engineering_connection_until_ports_are_explicitly_linked() {
    let mut p = empty();
    let a = add_physical(
        &mut p,
        "Tubo A",
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        PhysicalRole::VisualOnly,
        Discipline::WaterSanitation,
    );
    let b = add_physical(
        &mut p,
        "Tubo B",
        [0.0, -1.0, 0.0],
        [0.0, 1.0, 0.0],
        PhysicalRole::VisualOnly,
        Discipline::WaterSanitation,
    );
    assert!(p.connections.is_empty());
    assert!(p.model.links.is_empty());
    let network = apply(
        &mut p,
        ModelEdit::AddNetwork {
            name: "Rede de água".into(),
            discipline: Discipline::WaterSanitation,
            kind: NetworkKind::Network,
            environment_id: None,
        },
    )
    .created_ids[0];
    let port_a = apply(
        &mut p,
        ModelEdit::AddPort {
            object_id: a,
            discipline: Discipline::WaterSanitation,
            kind: PortKind::Hydraulic,
            position: point([0.0; 3]),
        },
    )
    .created_ids[0];
    let port_b = apply(
        &mut p,
        ModelEdit::AddPort {
            object_id: b,
            discipline: Discipline::WaterSanitation,
            kind: PortKind::Hydraulic,
            position: point([0.0; 3]),
        },
    )
    .created_ids[0];
    assert!(p.connections.is_empty());
    apply(
        &mut p,
        ModelEdit::AssignPort {
            network_id: network,
            port_id: port_a,
        },
    );
    apply(
        &mut p,
        ModelEdit::AssignPort {
            network_id: network,
            port_id: port_b,
        },
    );
    apply(
        &mut p,
        ModelEdit::ConnectPorts {
            network_id: network,
            from_port: port_a,
            to_port: port_b,
        },
    );
    assert_eq!(p.connections.len(), 1);
    let before = p.clone();
    assert!(
        apply_model_edit(
            &mut p,
            ModelEdit::AddPort {
                object_id: a,
                discipline: Discipline::Gas,
                kind: PortKind::Gas,
                position: point([0.0; 3])
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(p, before);
    assert!(
        apply_model_edit(
            &mut p,
            ModelEdit::SplitPhysical {
                object_id: a,
                fraction: 0.5,
                names: ["A1".into(), "A2".into()]
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(p, before);
}

#[test]
fn metre_centimetre_millimetre_inputs_and_display_changes_leave_si_unchanged() {
    let mut p = empty();
    for (unit, scale) in [
        (LengthUnit::Metre, 1.0),
        (LengthUnit::Centimetre, 100.0),
        (LengthUnit::Millimetre, 1000.0),
    ] {
        apply(
            &mut p,
            ModelEdit::AddPhysical {
                name: "Peça equivalente".into(),
                discipline: Discipline::Structure,
                start: PointInput {
                    values: [0.0; 3],
                    unit,
                },
                end: PointInput {
                    values: [2.5 * scale, 0.0, 0.0],
                    unit,
                },
                role: PhysicalRole::VisualOnly,
                material_id: None,
                environment_id: None,
            },
        );
    }
    assert!(p.objects.iter().all(|x| x.length_m() == 2.5));
    let before = p.objects.clone();
    apply(
        &mut p,
        ModelEdit::SetDisplayUnit {
            unit: LengthUnit::Millimetre,
        },
    );
    assert_eq!(p.objects, before);
}

#[test]
fn migration_keeps_actual_legacy_ids_and_derives_repeatable_new_identities() {
    let physical = Uuid::new_v4();
    let analytical = Uuid::new_v4();
    let project = Uuid::new_v4();
    let legacy = json!({"schema_version":1,"id":project,"name":"Legado","revision":1,"units":"centimetre","objects":[{"id":physical,"analytical_id":analytical,"name":"Peça","discipline":"structure","start_m":[0,0,0],"end_m":[2,0,0],"state":"modelled","extensions":{"reference":"preservada"}}],"environments":[],"networks":[],"ports":[],"connections":[],"history":[{"revision":1,"actor":"autor legado","operation":"create","timestamp_unix_ms":123}],"extensions":{"legacy":"mantido"}});
    let a = decode_project(legacy.clone()).unwrap();
    let b = decode_project(legacy).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.objects[0].id, physical);
    assert_eq!(a.model.analytical_members[0].id, analytical);
    assert_eq!(a.revision, 1);
    assert_eq!(a.history[0].timestamp_unix_ms, 123);
    assert_eq!(a.objects[0].extensions["reference"], "preservada");
    assert_eq!(a.objects[0].length_m(), 2.0);
    assert!(matches!(
        a.model.identities[&physical].origin,
        SourceOrigin::Imported { .. }
    ));
    for node in &a.model.analytical_nodes {
        assert!(matches!(
            a.model.identities[&node.id].origin,
            SourceOrigin::Derived { .. }
        ));
    }
    assert!(
        a.model
            .identities
            .values()
            .all(|x| x.created_by.is_none() && x.created_revision.is_none())
    );
    assert_eq!(
        decode_project(serde_json::to_value(a.clone()).unwrap()).unwrap(),
        a
    );
}

#[test]
fn georeference_is_metadata_and_invalid_frames_do_not_mutate_coordinates() {
    let mut p = empty();
    let id = bar(&mut p, "Peça", [0.0; 3], [1.0, 0.0, 0.0]);
    let before = p.objects.clone();
    apply(
        &mut p,
        ModelEdit::SetGeoreference {
            georeference: Some(Georeference {
                crs_identifier: "Referência declarada, sem projeção".into(),
                map_origin_m: Point3::try_from([500000.0, 7000000.0, 10.0]).unwrap(),
                rotation_radians: Angle::try_from(0.0).unwrap(),
                scale: 1.0,
            }),
        },
    );
    assert_eq!(p.objects, before);
    let before = p.clone();
    for rotation in [
        [[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    ] {
        assert!(
            apply_model_edit(
                &mut p,
                ModelEdit::RigidMove {
                    object_ids: vec![id],
                    rotation,
                    translation: point([0.0; 3])
                },
                "fixture"
            )
            .is_err()
        );
        assert_eq!(p, before);
    }
}
