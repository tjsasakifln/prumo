use prumo_domain::{
    ChangeRecord, LengthUnit, PROJECT_SCHEMA_VERSION, Project,
    catalogue::{CatalogEntry, CatalogPayload, CatalogSource, SectionShape},
    framing::*,
    layout::{AxisInput, LayoutEdit, LayoutPoint},
    model::{MaterialFamily, MaterialInput, ModelEdit, PhysicalRole, apply_model_edit},
    units::{DensityInput, DensityUnit, PointInput, PressureInput, PressureUnit},
};
use std::collections::BTreeMap;
use uuid::Uuid;
fn project() -> Project {
    Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name: "Structural fixture".into(),
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
            timestamp_unix_ms: 1,
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
        operation: "edit".into(),
        timestamp_unix_ms: project.revision,
    });
    project.validate().unwrap();
}
fn apply(project: &mut Project, edit: ModelEdit) {
    let before = prumo_domain::canonical::source_index_unchecked(project).unwrap();
    apply_model_edit(project, edit, "fixture").unwrap();
    let after = prumo_domain::canonical::source_index_unchecked(project).unwrap();
    project
        .dependencies
        .invalidate_changes(
            &before,
            &after,
            project.revision + 1,
            "fixture",
            "Parametric edit fixture",
        )
        .unwrap();
    stamp(project);
}
fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected} at {tolerance}"
    );
}
fn p(values: [f64; 3]) -> PointInput {
    PointInput {
        values,
        unit: LengthUnit::Metre,
    }
}
fn at(values: [f64; 3]) -> LayoutPoint {
    LayoutPoint::Coordinates {
        position: p(values),
    }
}
fn setup() -> (Project, Uuid, Uuid) {
    let mut project = project();
    apply(
        &mut project,
        ModelEdit::AddMaterial {
            material: MaterialInput {
                name: "Steel fixture".into(),
                family: MaterialFamily::Steel,
                elastic_modulus: PressureInput {
                    value: 200.,
                    unit: PressureUnit::Gigapascal,
                },
                density: DensityInput {
                    value: 7850.,
                    unit: DensityUnit::KilogramPerCubicMetre,
                },
                poisson_ratio: 0.3,
            },
        },
    );
    let material = project.model.materials[0].id;
    let section = Uuid::new_v4();
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: entry(section, 0.2, 0.4),
        },
    );
    (project, material, section)
}
fn entry(id: Uuid, width_m: f64, height_m: f64) -> CatalogEntry {
    CatalogEntry {
        id,
        revision: 1,
        name: format!("Fixture {width_m}x{height_m}"),
        source: CatalogSource {
            publisher: "Prumo test".into(),
            reference: "Original independent dimensions".into(),
            edition: "1".into(),
            revision: "1".into(),
            data_license: "Original fixture".into(),
            original_record: None,
        },
        applicability: vec![],
        customization: None,
        payload: CatalogPayload::Section {
            shape: SectionShape::Rectangle { width_m, height_m },
        },
    }
}
fn input(material_id: Uuid, section_id: Uuid) -> FramingInput {
    FramingInput {
        name: "Beam fixture".into(),
        kind: MemberKind::Beam,
        path: MemberPath::Straight {
            start: at([0.; 3]),
            end: at([4., 0., 0.]),
        },
        material_id,
        sections: vec![SectionStation {
            fraction: 0.,
            section_id,
        }],
        variation: SectionVariation::Constant,
        alignment: SectionAlignment::Centroid,
        eccentricity: p([0.; 3]),
        roll_degrees: 0.,
        connections: [None, None],
    }
}
fn put(project: &mut Project, id: Uuid, input: FramingInput) {
    apply(
        project,
        ModelEdit::EditFraming {
            command: FramingEdit::PutMember {
                id,
                input: Box::new(input),
            },
        },
    );
}

#[test]
fn inclined_column_and_variable_sections_keep_material_topology_and_identity() {
    let (mut project, material, section) = setup();
    let end_section = Uuid::new_v4();
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: entry(end_section, 0.3, 0.6),
        },
    );
    let id = Uuid::new_v4();
    let mut value = input(material, section);
    value.kind = MemberKind::Column;
    value.path = MemberPath::Straight {
        start: at([0.; 3]),
        end: at([3., 0., 4.]),
    };
    value.sections.push(SectionStation {
        fraction: 1.,
        section_id: end_section,
    });
    value.variation = SectionVariation::Linear;
    put(&mut project, id, value.clone());
    let before = inspect(&project, id).unwrap();
    near(before.record.analytical_path_length_m, 5., 1e-12);
    assert_eq!(before.sections.len(), 2);
    assert_eq!(before.record.member_ids.len(), 1);
    assert_eq!(project.model.links.len(), 1);
    assert!(!before.engineering_release);
    value.path = MemberPath::Straight {
        start: at([0.; 3]),
        end: at([0., 0., 6.]),
    };
    value.sections[0].section_id = end_section;
    put(&mut project, id, value);
    let after = inspect(&project, id).unwrap();
    assert_eq!(before.record.node_ids, after.record.node_ids);
    assert_eq!(before.record.member_ids, after.record.member_ids);
    near(after.record.analytical_path_length_m, 6., 1e-12);
    assert_ne!(before.sections[0].sha256, after.sections[0].sha256);
}
#[test]
fn semicircular_beam_matches_independent_pi_length_and_sagitta() {
    let (mut project, material, section) = setup();
    let id = Uuid::new_v4();
    let mut value = input(material, section);
    value.path = MemberPath::CircularArc {
        start: at([0., 0., 0.]),
        through: at([1., 1., 0.]),
        end: at([2., 0., 0.]),
        segments: 16,
    };
    put(&mut project, id, value);
    let report = inspect(&project, id).unwrap();
    near(
        report.record.analytical_path_length_m,
        std::f64::consts::PI,
        1e-12,
    );
    near(project.objects[0].length_m(), std::f64::consts::PI, 1e-12);
    assert_eq!(report.record.node_ids.len(), 17);
    assert_eq!(report.record.member_ids.len(), 16);
    near(
        report.record.maximum_chord_error_m,
        1.0 - (std::f64::consts::PI / 32.).cos(),
        1e-12,
    );
    near(
        report.record.analytical_chord_length_m,
        32. * (std::f64::consts::PI / 32.).sin(),
        1e-12,
    );
    let lines = display_polylines(&project, id).unwrap();
    assert_eq!(lines.len(), 2);
    near(lines[0][8][0], 1., 1e-12);
    near(lines[0][8][1], 1., 1e-12);
}
#[test]
fn curved_beam_is_stable_after_large_coordinate_translation() {
    let (project, material, section) = setup();
    let mut value = input(material, section);
    value.path = MemberPath::CircularArc {
        start: at([900_000_000., 900_000_000., 10.]),
        through: at([900_000_001., 900_000_001., 10.]),
        end: at([900_000_002., 900_000_000., 10.]),
        segments: 16,
    };
    let geometry = resolve_geometry(&project, &value).unwrap();
    near(
        geometry.analytical_path_length_m,
        std::f64::consts::PI,
        1e-12,
    );
    near(geometry.physical_points_m[8][1], 900_000_001., 1.2e-7);
}
#[test]
fn truss_has_axial_only_contract_and_explicit_shared_nodes() {
    let (mut project, material_id, section_id) = setup();
    let id = Uuid::new_v4();
    apply(
        &mut project,
        ModelEdit::EditFraming {
            command: FramingEdit::CreateTruss {
                id,
                name: "Triangle fixture".into(),
                points: vec![at([0.; 3]), at([4., 0., 0.]), at([2., 0., 3.])],
                edges: vec![[0, 1], [0, 2], [2, 1]],
                material_id,
                section_id,
            },
        },
    );
    assert_eq!(project.objects.len(), 3);
    assert_eq!(project.model.analytical_nodes.len(), 3);
    assert_eq!(project.model.analytical_members.len(), 3);
    assert_eq!(project.model.links.len(), 3);
    for object in &project.objects {
        let report = inspect(&project, object.id).unwrap();
        assert_eq!(report.record.behavior, MemberBehavior::AxialOnly);
        assert!(!report.bending_stiffness_permitted);
        assert_eq!(report.record.truss_id, Some(id));
        for member_id in &report.record.member_ids {
            assert_eq!(
                analytical_behavior(&project, *member_id).unwrap(),
                MemberBehavior::AxialOnly
            );
        }
    }
}
#[test]
fn duplicate_and_zero_truss_edges_reject_entire_command_without_orphans() {
    for (points, edges) in [
        (vec![at([0.; 3]), at([4., 0., 0.])], vec![[0, 1], [1, 0]]),
        (vec![at([0.; 3]), at([0.; 3])], vec![[0, 1]]),
        (
            vec![at([0.; 3]), at([4., 0., 0.]), at([2., 0., 3.])],
            vec![[0, 1]],
        ),
    ] {
        let (mut project, material_id, section_id) = setup();
        let before = serde_json::to_vec(&project).unwrap();
        let result = apply_model_edit(
            &mut project,
            ModelEdit::EditFraming {
                command: FramingEdit::CreateTruss {
                    id: Uuid::new_v4(),
                    name: "Invalid fixture".into(),
                    points,
                    edges,
                    material_id,
                    section_id,
                },
            },
            "fixture",
        );
        assert!(result.is_err());
        assert_eq!(before, serde_json::to_vec(&project).unwrap());
    }
}
#[test]
fn coincidence_alone_does_not_share_nodes_and_duplicate_explicit_edge_is_refused() {
    let (mut project, material, section) = setup();
    let first = Uuid::new_v4();
    let value = input(material, section);
    put(&mut project, first, value.clone());
    let first_record = inspect(&project, first).unwrap().record;
    let second = Uuid::new_v4();
    put(&mut project, second, value.clone());
    assert_eq!(project.model.analytical_nodes.len(), 4);
    let second_record = inspect(&project, second).unwrap().record;
    assert_ne!(first_record.node_ids, second_record.node_ids);
    let mut duplicate = value;
    duplicate.connections = [
        Some(first_record.node_ids[0]),
        Some(first_record.node_ids[1]),
    ];
    let before = serde_json::to_vec(&project).unwrap();
    assert!(
        apply_model_edit(
            &mut project,
            ModelEdit::EditFraming {
                command: FramingEdit::PutMember {
                    id: Uuid::new_v4(),
                    input: Box::new(duplicate)
                }
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(before, serde_json::to_vec(&project).unwrap());
}
#[test]
fn endpoint_level_edit_rebuilds_inclined_member_and_keeps_all_links() {
    let (mut project, material, section) = setup();
    let level1 = Uuid::new_v4();
    let level2 = Uuid::new_v4();
    let axis = Uuid::new_v4();
    for (id, elevation) in [(level1, "0 m"), (level2, "3 m")] {
        apply(
            &mut project,
            ModelEdit::EditLayout {
                command: LayoutEdit::PutLevel {
                    id,
                    name: elevation.into(),
                    elevation: elevation.into(),
                },
            },
        );
    }
    apply(
        &mut project,
        ModelEdit::EditLayout {
            command: LayoutEdit::PutAxis {
                id: axis,
                input: AxisInput {
                    name: "Inclined axis".into(),
                    origin: p([0.; 3]),
                    angle_degrees: 30.,
                    offset: "0 m".into(),
                    extent: "10 m".into(),
                    frame_id: None,
                },
            },
        },
    );
    let mut value = input(material, section);
    value.kind = MemberKind::Column;
    value.path = MemberPath::Straight {
        start: LayoutPoint::AxisStation {
            axis_id: axis,
            station: "0 m".into(),
            level_id: level1,
            offset: "0 m".into(),
        },
        end: LayoutPoint::AxisStation {
            axis_id: axis,
            station: "1 m".into(),
            level_id: level2,
            offset: "20 cm".into(),
        },
    };
    let id = Uuid::new_v4();
    put(&mut project, id, value);
    let before = inspect(&project, id).unwrap().record;
    near(before.physical_points_m[1][2], 3.2, 1e-12);
    apply(
        &mut project,
        ModelEdit::EditLayout {
            command: LayoutEdit::PutLevel {
                id: level2,
                name: "Upper".into(),
                elevation: "4 m".into(),
            },
        },
    );
    let after = inspect(&project, id).unwrap().record;
    near(after.physical_points_m[1][2], 4.2, 1e-12);
    assert_eq!(before.node_ids, after.node_ids);
    assert_eq!(before.member_ids, after.member_ids);
    assert_eq!(before.link_ids, after.link_ids);
    assert!(after.analytical_path_length_m > before.analytical_path_length_m);
}
#[test]
fn low_level_edits_cannot_leave_parametric_metadata_out_of_sync() {
    let (mut project, material, section) = setup();
    let id = Uuid::new_v4();
    put(&mut project, id, input(material, section));
    for edit in [
        ModelEdit::SetPhysicalGeometry {
            object_id: id,
            start: p([0.; 3]),
            end: p([5., 0., 0.]),
        },
        ModelEdit::SetPhysicalRole {
            object_id: id,
            role: PhysicalRole::VisualOnly,
        },
        ModelEdit::SetPhysicalMaterial {
            object_id: id,
            material_id: None,
        },
    ] {
        let before = serde_json::to_vec(&project).unwrap();
        assert!(apply_model_edit(&mut project, edit, "fixture").is_err());
        assert_eq!(before, serde_json::to_vec(&project).unwrap());
    }
}
#[test]
fn alignment_and_eccentricity_are_visible_and_analytical_axis_stays_explicit() {
    let (mut project, material, section) = setup();
    let id = Uuid::new_v4();
    let mut value = input(material, section);
    value.eccentricity = p([0., 0., 0.25]);
    value.alignment = SectionAlignment::SectionOrigin;
    put(&mut project, id, value);
    let record = inspect(&project, id).unwrap().record;
    assert_eq!(record.analytical_points_m[0].as_array(), [0.; 3]);
    near(record.physical_points_m[0][1], 0.1, 1e-12);
    near(record.physical_points_m[0][2], 0.45, 1e-12);
}
#[test]
fn section_origin_follows_each_linear_section_centroid_without_changing_analytical_length() {
    let (mut project, material, section) = setup();
    let second = Uuid::new_v4();
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: entry(second, 0.4, 0.6),
        },
    );
    let mut value = input(material, section);
    value.alignment = SectionAlignment::SectionOrigin;
    value.variation = SectionVariation::Linear;
    value.sections.push(SectionStation {
        fraction: 1.0,
        section_id: second,
    });
    let id = Uuid::new_v4();
    put(&mut project, id, value);
    let record = inspect(&project, id).unwrap().record;
    for (actual, expected) in record
        .physical_points_m
        .iter()
        .zip([[0., 0.1, 0.2], [4., 0.2, 0.3]])
    {
        for (axis, value) in expected.iter().enumerate() {
            near(actual[axis], *value, 1e-12);
        }
    }
    near(record.analytical_path_length_m, 4.0, 1e-12);
    near(record.physical_length_m, 16.02_f64.sqrt(), 1e-12);
    near(project.objects[0].length_m(), 16.02_f64.sqrt(), 1e-12);
    assert!(record.physical_length_is_exact);
    assert_eq!(record.physical_length_error_estimate_m, 0.0);
    assert_eq!(record.node_ids.len(), 2);
    assert!(record.physical_breaks.is_empty());
}
#[test]
fn section_origin_on_a_semicircle_follows_the_rotating_radial_frame() {
    let (mut project, material, section) = setup();
    let mut value = input(material, section);
    value.alignment = SectionAlignment::SectionOrigin;
    value.path = MemberPath::CircularArc {
        start: at([0., 0., 0.]),
        through: at([1., 1., 0.]),
        end: at([2., 0., 0.]),
        segments: 16,
    };
    let id = Uuid::new_v4();
    put(&mut project, id, value);
    let record = inspect(&project, id).unwrap().record;
    // The path radius is 1 m. Rectangle centroid y=0.1 m points inward;
    // its z=0.2 m follows the negative normal of this oriented XY arc.
    for (index, expected) in [
        (0, [0.1, 0., -0.2]),
        (8, [1., 0.9, -0.2]),
        (16, [1.9, 0., -0.2]),
    ] {
        for (axis, value) in expected.iter().enumerate() {
            near(record.physical_points_m[index][axis], *value, 1e-12);
        }
    }
    near(record.analytical_path_length_m, std::f64::consts::PI, 1e-12);
    near(record.physical_length_m, 0.9 * std::f64::consts::PI, 1e-12);
    near(
        project.objects[0].length_m(),
        0.9 * std::f64::consts::PI,
        1e-12,
    );
    near(
        record.maximum_physical_chord_error_m,
        0.9 * (1. - (std::f64::consts::PI / 32.).cos()),
        1e-12,
    );
    assert!(record.physical_length_is_exact);
}
#[test]
fn variable_centroid_arc_length_matches_an_independent_antiderivative() {
    let (mut project, material, section) = setup();
    let second = Uuid::new_v4();
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: entry(second, 0.4, 0.6),
        },
    );
    let mut value = input(material, section);
    value.alignment = SectionAlignment::SectionOrigin;
    value.variation = SectionVariation::Linear;
    value.sections.push(SectionStation {
        fraction: 1.0,
        section_id: second,
    });
    value.path = MemberPath::CircularArc {
        start: at([0., 0., 0.]),
        through: at([1., 1., 0.]),
        end: at([2., 0., 0.]),
        segments: 16,
    };
    let id = Uuid::new_v4();
    put(&mut project, id, value);
    let record = inspect(&project, id).unwrap().record;
    // Radius varies 0.9 -> 0.8 m and normal offset varies by 0.1 m.
    // Integrate sqrt(0.1^2+0.1^2+(pi*r)^2) using its closed primitive,
    // independently of production composite Simpson integration.
    let pi = std::f64::consts::PI;
    let a = 0.02_f64.sqrt();
    let primitive = |u: f64| 0.5 * (u * u.hypot(a) + a * a * (u / a).asinh());
    let expected = (primitive(pi * 0.8) - primitive(pi * 0.9)) / (-0.1 * pi);
    near(record.physical_length_m, expected, 1e-11);
    assert!(!record.physical_length_is_exact);
    assert!(record.physical_length_error_estimate_m <= 1e-11 * record.physical_length_m);
    near(record.physical_points_m[8][1], 0.85, 1e-12);
    near(record.physical_points_m[8][2], -0.25, 1e-12);
    // An independent midpoint of the first smooth interval lies inside the
    // declared deviation bound from its chord midpoint.
    let t = 1.0 / 32.0;
    let radius = 0.9 - 0.1 * t;
    let exact = [
        1.0 - radius * (pi * t).cos(),
        radius * (pi * t).sin(),
        -0.2 - 0.1 * t,
    ];
    let chord = std::array::from_fn::<_, 3, _>(|axis| {
        (record.physical_points_m[0][axis] + record.physical_points_m[1][axis]) / 2.0
    });
    let deviation = (exact[0] - chord[0])
        .hypot(exact[1] - chord[1])
        .hypot(exact[2] - chord[2]);
    assert!(deviation <= record.maximum_physical_chord_error_m);
}
#[test]
fn stepped_centroids_have_explicit_gaps_without_transverse_length_or_extra_connections() {
    let (mut project, material, section) = setup();
    let second = Uuid::new_v4();
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: entry(second, 0.4, 0.6),
        },
    );
    let mut value = input(material, section);
    value.alignment = SectionAlignment::SectionOrigin;
    value.variation = SectionVariation::Stepped;
    value.sections.push(SectionStation {
        fraction: 0.5,
        section_id: second,
    });
    let id = Uuid::new_v4();
    put(&mut project, id, value);
    let record = inspect(&project, id).unwrap().record;
    assert_eq!(record.physical_breaks, vec![2]);
    assert_eq!(record.physical_points_m.len(), 4);
    near(record.physical_length_m, 4.0, 1e-12);
    near(record.physical_chord_length_m, 4.0, 1e-12);
    assert_eq!(record.member_ids.len(), 1);
    assert_eq!(record.node_ids.len(), 2);
    let lines = display_polylines(&project, id).unwrap();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].len(), 2);
    assert_eq!(lines[1].len(), 2);
    near(lines[0][1][1], 0.1, 1e-12);
    near(lines[1][0][1], 0.2, 1e-12);
    let mut forged = project.clone();
    forged.objects[0].extensions.get_mut(FRAMING_KEY).unwrap()["physical_breaks"] =
        serde_json::json!([]);
    assert!(forged.validate().is_err());
}
#[test]
fn folded_physical_arc_and_forged_level_binding_are_rejected() {
    let (mut project, material, section) = setup();
    let large = Uuid::new_v4();
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: entry(large, 2.0, 0.4),
        },
    );
    let mut value = input(material, large);
    value.alignment = SectionAlignment::SectionOrigin;
    value.path = MemberPath::CircularArc {
        start: at([0., 0., 0.]),
        through: at([1., 1., 0.]),
        end: at([2., 0., 0.]),
        segments: 16,
    };
    assert!(resolve_geometry(&project, &value).is_err());
    let id = Uuid::new_v4();
    put(&mut project, id, input(material, section));
    let level = Uuid::new_v4();
    apply(
        &mut project,
        ModelEdit::EditLayout {
            command: LayoutEdit::PutLevel {
                id: level,
                name: "Ground".into(),
                elevation: "0 m".into(),
            },
        },
    );
    let mut layout = prumo_domain::layout::state(&project).unwrap();
    layout.bindings.push(prumo_domain::layout::LevelBinding {
        object_id: id,
        level_id: level,
        offset_m: 0.0,
    });
    project.extensions.insert(
        prumo_domain::layout::LAYOUT_KEY.into(),
        serde_json::to_value(layout).unwrap(),
    );
    assert!(project.validate().is_err());
}
#[test]
fn forged_axial_metadata_or_curve_data_fails_project_validation() {
    let (mut project, material, section) = setup();
    let id = Uuid::new_v4();
    let mut value = input(material, section);
    value.kind = MemberKind::TrussBar;
    put(&mut project, id, value);
    let mut forged = project.clone();
    forged.objects[0].extensions.get_mut(FRAMING_KEY).unwrap()["behavior"] =
        serde_json::json!("frame");
    assert!(forged.validate().is_err());
    let mut forged = project.clone();
    forged.objects[0].extensions.get_mut(FRAMING_KEY).unwrap()["analytical_path_length_m"] =
        serde_json::json!(5.);
    assert!(forged.validate().is_err());
    let mut forged = project;
    forged.objects[0].extensions.get_mut(FRAMING_KEY).unwrap()["input"]["sections"][0]["fraction"] =
        serde_json::json!(0.5);
    assert!(forged.validate().is_err());
}
#[test]
fn invalid_section_sequences_arcs_and_singular_members_are_rejected() {
    let (project, material, section) = setup();
    let mut zero = input(material, section);
    zero.path = MemberPath::Straight {
        start: at([0.; 3]),
        end: at([0.; 3]),
    };
    assert!(resolve_geometry(&project, &zero).is_err());
    let mut curve = input(material, section);
    curve.path = MemberPath::CircularArc {
        start: at([0.; 3]),
        through: at([1., 0., 0.]),
        end: at([2., 0., 0.]),
        segments: 12,
    };
    assert!(resolve_geometry(&project, &curve).is_err());
    let mut variable = input(material, section);
    variable.variation = SectionVariation::Linear;
    assert!(resolve_geometry(&project, &variable).is_err());
    variable.sections.push(SectionStation {
        fraction: 0.,
        section_id: section,
    });
    assert!(resolve_geometry(&project, &variable).is_err());
}

#[test]
fn node_references_follow_level_changes_in_dependency_order() {
    let (mut project, material, section) = setup();
    let level0 = Uuid::new_v4();
    let level1 = Uuid::new_v4();
    let axis = Uuid::new_v4();
    for (id, elevation) in [(level0, "0 m"), (level1, "3 m")] {
        apply(
            &mut project,
            ModelEdit::EditLayout {
                command: LayoutEdit::PutLevel {
                    id,
                    name: elevation.into(),
                    elevation: elevation.into(),
                },
            },
        );
    }
    apply(
        &mut project,
        ModelEdit::EditLayout {
            command: LayoutEdit::PutAxis {
                id: axis,
                input: AxisInput {
                    name: "A".into(),
                    origin: p([0.; 3]),
                    angle_degrees: 0.,
                    offset: "0 m".into(),
                    extent: "10 m".into(),
                    frame_id: None,
                },
            },
        },
    );
    let a = Uuid::new_v4();
    let mut column = input(material, section);
    column.path = MemberPath::Straight {
        start: LayoutPoint::AxisStation {
            axis_id: axis,
            station: "0 m".into(),
            level_id: level0,
            offset: "0 m".into(),
        },
        end: LayoutPoint::AxisStation {
            axis_id: axis,
            station: "0 m".into(),
            level_id: level1,
            offset: "0 m".into(),
        },
    };
    put(&mut project, a, column);
    let top = inspect(&project, a).unwrap().record.node_ids[1];
    let b = Uuid::new_v4();
    let mut beam = input(material, section);
    beam.path = MemberPath::Straight {
        start: LayoutPoint::Node { node_id: top },
        end: at([4., 0., 3.]),
    };
    beam.connections[0] = Some(top);
    put(&mut project, b, beam);
    apply(
        &mut project,
        ModelEdit::EditLayout {
            command: LayoutEdit::PutLevel {
                id: level1,
                name: "Upper".into(),
                elevation: "4 m".into(),
            },
        },
    );
    let report = inspect(&project, b).unwrap();
    assert_eq!(report.record.analytical_points_m[0][2], 4.);
    assert_eq!(report.record.analytical_points_m[1][2], 3.);
    assert_eq!(report.record.node_ids[0], top);
}

#[test]
fn direct_member_edits_follow_geometric_node_references_and_keep_explicit_topology() {
    use prumo_domain::dependencies::{SourceKey, SourceKind};
    let (mut project, material, section) = setup();
    let column_id = Uuid::new_v4();
    let mut column = input(material, section);
    column.kind = MemberKind::Column;
    column.path = MemberPath::Straight {
        start: at([0.; 3]),
        end: at([0., 0., 3.]),
    };
    put(&mut project, column_id, column.clone());
    let top = inspect(&project, column_id).unwrap().record.node_ids[1];
    let geometric_id = Uuid::new_v4();
    let mut geometric = input(material, section);
    geometric.path = MemberPath::Straight {
        start: LayoutPoint::Node { node_id: top },
        end: at([4., 0., 3.]),
    };
    put(&mut project, geometric_id, geometric.clone());
    let geometric_before = inspect(&project, geometric_id).unwrap().record;
    assert_ne!(geometric_before.node_ids[0], top);
    assert!(
        object_sources(&project, geometric_id)
            .unwrap()
            .contains(&SourceKey {
                kind: SourceKind::AnalyticalNode,
                id: top
            })
    );
    let connected_id = Uuid::new_v4();
    let mut connected = geometric;
    connected.path = MemberPath::Straight {
        start: LayoutPoint::Node { node_id: top },
        end: at([0., 4., 3.]),
    };
    connected.connections[0] = Some(top);
    put(&mut project, connected_id, connected);
    let connected_before = inspect(&project, connected_id).unwrap().record;
    let counts = (
        project.model.analytical_nodes.len(),
        project.model.analytical_members.len(),
        project.model.links.len(),
    );
    column.path = MemberPath::Straight {
        start: at([0.; 3]),
        end: at([0., 0., 4.]),
    };
    put(&mut project, column_id, column);
    let geometric_after = inspect(&project, geometric_id).unwrap().record;
    let connected_after = inspect(&project, connected_id).unwrap().record;
    assert_eq!(
        geometric_after.analytical_points_m[0].as_array(),
        [0., 0., 4.]
    );
    assert_eq!(
        connected_after.analytical_points_m[0].as_array(),
        [0., 0., 4.]
    );
    assert_eq!(geometric_before.node_ids, geometric_after.node_ids);
    assert_eq!(connected_before.node_ids, connected_after.node_ids);
    assert_eq!(connected_after.node_ids[0], top);
    assert_eq!(
        counts,
        (
            project.model.analytical_nodes.len(),
            project.model.analytical_members.len(),
            project.model.links.len()
        )
    );
    near(geometric_after.physical_length_m, 17_f64.sqrt(), 1e-12);
    near(connected_after.physical_length_m, 17_f64.sqrt(), 1e-12);
    // Coincident coordinates do not make a cyclic construction dependency safe.
    let mut cyclic = inspect(&project, column_id).unwrap().record.input;
    cyclic.path = MemberPath::Straight {
        start: at([0.; 3]),
        end: LayoutPoint::Node {
            node_id: geometric_after.node_ids[0],
        },
    };
    let before = serde_json::to_vec(&project).unwrap();
    assert!(
        apply_model_edit(
            &mut project,
            ModelEdit::EditFraming {
                command: FramingEdit::PutMember {
                    id: column_id,
                    input: Box::new(cyclic)
                }
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(before, serde_json::to_vec(&project).unwrap());
}

#[test]
fn direct_member_edits_cannot_drag_conflicting_or_nonparametric_shared_nodes() {
    for parametric in [true, false] {
        let (mut project, material, section) = setup();
        let column_id = Uuid::new_v4();
        let mut column = input(material, section);
        column.kind = MemberKind::Column;
        column.path = MemberPath::Straight {
            start: at([0.; 3]),
            end: at([0., 0., 3.]),
        };
        put(&mut project, column_id, column.clone());
        let top = inspect(&project, column_id).unwrap().record.node_ids[1];
        if parametric {
            let mut fixed = input(material, section);
            fixed.path = MemberPath::Straight {
                start: at([0., 0., 3.]),
                end: at([4., 0., 3.]),
            };
            fixed.connections[0] = Some(top);
            put(&mut project, Uuid::new_v4(), fixed);
        } else {
            apply(
                &mut project,
                ModelEdit::AddAnalyticalNode {
                    name: "Independent end".into(),
                    position: p([4., 0., 3.]),
                },
            );
            let end = project.model.analytical_nodes.last().unwrap().id;
            apply(
                &mut project,
                ModelEdit::AddAnalyticalMember {
                    name: "Nonparametric member".into(),
                    start_node_id: top,
                    end_node_id: end,
                    local_y: [0., 1., 0.],
                },
            );
        }
        column.path = MemberPath::Straight {
            start: at([0.; 3]),
            end: at([0., 0., 4.]),
        };
        let before = serde_json::to_vec(&project).unwrap();
        assert!(
            apply_model_edit(
                &mut project,
                ModelEdit::EditFraming {
                    command: FramingEdit::PutMember {
                        id: column_id,
                        input: Box::new(column)
                    }
                },
                "fixture"
            )
            .is_err()
        );
        assert_eq!(before, serde_json::to_vec(&project).unwrap());
    }
}

#[test]
fn canonical_linux_curve_fixture_reopens_without_rewriting_values() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/framing-arc.json");
    if std::env::var_os("PRUMO_WRITE_FRAMING_FIXTURE").is_some() {
        let (mut project, material, section) = setup();
        let mut value = input(material, section);
        value.roll_degrees = 17.3;
        let second = Uuid::new_v4();
        apply(
            &mut project,
            ModelEdit::PutCatalogEntry {
                entry: entry(second, 0.4, 0.6),
            },
        );
        value.alignment = SectionAlignment::SectionOrigin;
        value.variation = SectionVariation::Linear;
        value.sections.push(SectionStation {
            fraction: 1.0,
            section_id: second,
        });
        value.path = MemberPath::CircularArc {
            start: at([1.3, 4.5, 2.]),
            through: at([2.7, 6.9, 3.1]),
            end: at([4.2, 5.6, 4.8]),
            segments: 31,
        };
        put(&mut project, Uuid::new_v4(), value);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec_pretty(&project).unwrap()).unwrap();
    }
    let bytes = std::fs::read(&path).unwrap();
    let project: Project = serde_json::from_slice(&bytes).unwrap();
    project.validate().unwrap();
    let before = serde_json::to_value(&project).unwrap();
    let id = project.objects[0].id;
    let report = inspect(&project, id).unwrap();
    assert_eq!(report.record.member_ids.len(), 31);
    assert_eq!(before, serde_json::to_value(&project).unwrap());
    let mut forged = project.clone();
    let record = forged.objects[0].extensions.get_mut(FRAMING_KEY).unwrap();
    let old = record["analytical_path_length_m"].as_f64().unwrap();
    record["analytical_path_length_m"] = serde_json::json!(old + 1e-9);
    assert!(forged.validate().is_err());
}

#[test]
fn secondary_section_change_invalidates_registered_fixture_dependency() {
    use prumo_domain::dependencies::{
        Dependency, InspectionPayload, ResultState, SourceKey, SourceKind,
    };
    let (mut project, material, section) = setup();
    let second = Uuid::new_v4();
    apply(
        &mut project,
        ModelEdit::PutCatalogEntry {
            entry: entry(second, 0.3, 0.6),
        },
    );
    let id = Uuid::new_v4();
    let mut value = input(material, section);
    value.variation = SectionVariation::Linear;
    value.sections.push(SectionStation {
        fraction: 1.,
        section_id: second,
    });
    put(&mut project, id, value);
    let sources = object_sources(&project, id).unwrap();
    assert!(sources.contains(&SourceKey {
        kind: SourceKind::CatalogEntry,
        id: second
    }));
    let snapshot = prumo_domain::canonical::model_snapshot(&project).unwrap();
    let index = prumo_domain::canonical::source_index_unchecked(&project).unwrap();
    project
        .dependencies
        .snapshots
        .insert(snapshot.sha256.clone(), snapshot.clone());
    let fixture = project
        .dependencies
        .record(
            "Parametric section fixture, not a calculation".into(),
            "parametric-test".into(),
            project.revision,
            project.revision + 1,
            snapshot.sha256,
            sources
                .into_iter()
                .map(|key| Dependency::source(key, &index).unwrap())
                .collect(),
            InspectionPayload::ExternalEvidence {
                artifact_sha256: "1".repeat(64),
                media_type: "application/json".into(),
            },
        )
        .unwrap();
    stamp(&mut project);
    let mut revised = entry(second, 0.4, 0.6);
    revised.revision = 2;
    revised.source.revision = "2".into();
    apply(&mut project, ModelEdit::PutCatalogEntry { entry: revised });
    assert_eq!(
        project
            .dependencies
            .results
            .iter()
            .find(|r| r.id == fixture)
            .unwrap()
            .state,
        ResultState::Stale
    );
}
