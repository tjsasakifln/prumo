use prumo_domain::{
    ChangeRecord, Discipline, LengthUnit, PROJECT_SCHEMA_VERSION, Project,
    layout::*,
    model::{ModelEdit, PhysicalRole, apply_model_edit},
    units::PointInput,
};
use std::collections::BTreeMap;
use uuid::Uuid;
fn project() -> Project {
    Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name: "Layout fixture".into(),
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
fn apply(project: &mut Project, command: LayoutEdit) {
    apply_model_edit(project, ModelEdit::EditLayout { command }, "fixture").unwrap();
    stamp(project);
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
fn p(values: [f64; 3]) -> PointInput {
    PointInput {
        values,
        unit: LengthUnit::Metre,
    }
}
fn axis(project: &mut Project, name: &str, origin: [f64; 3], angle_degrees: f64) -> Uuid {
    let id = Uuid::new_v4();
    apply(
        project,
        LayoutEdit::PutAxis {
            id,
            input: AxisInput {
                name: name.into(),
                origin: p(origin),
                angle_degrees,
                offset: "0 m".into(),
                extent: "20 m".into(),
                frame_id: None,
            },
        },
    );
    id
}
fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected} at {tolerance}"
    );
}

#[test]
fn dimensional_arithmetic_accepts_decimal_comma_and_mixed_explicit_units() {
    for (input, expected) in [
        ("3,2 m", 3.2),
        ("25 cm", 0.25),
        ("250 mm", 0.25),
        ("(3 m + 20 cm) / 2", 1.6),
        ("2 * 1,5 m - 250 mm", 2.75),
        ("-1 m + 200 cm", 1.0),
        ("2 m / 4 m * 30 cm", 0.15),
    ] {
        near(parse_length(input).unwrap(), expected, 1e-12);
    }
}

#[test]
fn numeric_bounds_never_erase_a_nonzero_measure_into_zero() {
    // The 256-character grammar does not permit exponent notation. Even the
    // smallest admissible decimal literal stays representable as binary64.
    let tiny = format!("0.{}1 m", "0".repeat(249));
    let parsed = parse_length(&tiny).unwrap();
    assert!(parsed > 0.0);
    assert!((parsed / 1e-250 - 1.0).abs() < 1e-14);
    let underflow_literal = format!("0.{}1 m", "0".repeat(330));
    assert!(parse_length(&underflow_literal).is_err());
    assert!(parse_length("1e-300 m * 1e-300").is_err());
    assert_eq!(parse_length("0 mm * 1000").unwrap(), 0.0);
}
#[test]
fn ambiguous_units_separators_and_unbounded_operations_are_rejected() {
    for input in [
        "3",
        "1,000.00 m",
        "1.000,00 m",
        "1 000 m",
        "1 m + 2",
        "5 mm / 0",
        "nan m",
        "inf m",
        "1 km",
        "sqrt(4) m",
        "2 ** 3 m",
        "2 m * 2 m",
        "(3 m",
        "3 m)",
        "1e9 m",
        "1 m; rm -rf",
        "1 m +",
    ] {
        assert!(parse_length(input).is_err(), "accepted {input}");
    }
    assert!(parse_length(&format!("{}1 m{}", "(".repeat(9), ")".repeat(9))).is_err());
}
#[test]
fn inclined_axes_intersection_has_explicit_level_and_no_structural_connection() {
    let mut project = project();
    let level = Uuid::new_v4();
    apply(
        &mut project,
        LayoutEdit::PutLevel {
            id: level,
            name: "Meio nível".into(),
            elevation: "3 m + 45 cm".into(),
        },
    );
    let a = axis(&mut project, "A", [0., 0., 0.], 45.);
    let b = axis(&mut project, "B", [0., 2., 0.], -45.);
    let before = serde_json::to_vec(&project).unwrap();
    let proposal = propose_snap(
        &project,
        &SnapRequest::Intersection {
            first_axis_id: a,
            second_axis_id: b,
            level_id: level,
            offset: "15 cm".into(),
        },
    )
    .unwrap();
    near(proposal.point_m[0], 1., 1e-12);
    near(proposal.point_m[1], 1., 1e-12);
    near(proposal.point_m[2], 3.6, 1e-12);
    assert!(!proposal.creates_structural_connection);
    assert_eq!(before, serde_json::to_vec(&project).unwrap());
    assert!(project.model.analytical_nodes.is_empty());
}
#[test]
fn local_frames_and_perpendicular_axis_offsets_use_engineering_coordinates() {
    let mut project = project();
    let frame = Uuid::new_v4();
    apply(
        &mut project,
        LayoutEdit::PutFrame {
            id: frame,
            name: "Ala inclinada".into(),
            origin: p([10., 20., 0.]),
            angle_degrees: 90.,
        },
    );
    let id = Uuid::new_v4();
    apply(
        &mut project,
        LayoutEdit::PutAxis {
            id,
            input: AxisInput {
                name: "A".into(),
                origin: p([2., 0., 0.]),
                angle_degrees: 0.,
                offset: "50 cm".into(),
                extent: "5 m".into(),
                frame_id: Some(frame),
            },
        },
    );
    let resolved = resolve_axis(&project, id).unwrap();
    near(resolved.origin_m[0], 9.5, 1e-12);
    near(resolved.origin_m[1], 22., 1e-12);
    near(resolved.direction[0], 0., 1e-12);
    near(resolved.direction[1], 1., 1e-12);
    let aligned = propose_snap(
        &project,
        &SnapRequest::Alignment {
            axis_id: id,
            position: p([12., 25., 3.]),
        },
    )
    .unwrap();
    near(aligned.point_m[0], 9.5, 1e-12);
    near(aligned.point_m[1], 25., 1e-12);
    assert_eq!(aligned.point_m[2], 3.);
}
#[test]
fn large_survey_coordinates_keep_local_snap_accuracy() {
    let mut project = project();
    let level = Uuid::new_v4();
    apply(
        &mut project,
        LayoutEdit::PutLevel {
            id: level,
            name: "Nível".into(),
            elevation: "3 m".into(),
        },
    );
    let a = axis(&mut project, "A", [900_000_000., 900_000_000., 0.], 45.);
    let b = axis(&mut project, "B", [900_000_000., 900_000_002., 0.], -45.);
    let snap = propose_snap(
        &project,
        &SnapRequest::Intersection {
            first_axis_id: a,
            second_axis_id: b,
            level_id: level,
            offset: "0 m".into(),
        },
    )
    .unwrap();
    near(snap.point_m[0], 900_000_001., 1.2e-7);
    near(snap.point_m[1], 900_000_001., 1.2e-7);
}
#[test]
fn parallel_axes_and_repeated_identity_are_refused_atomically() {
    let mut project = project();
    let a = axis(&mut project, "A", [0.; 3], 0.);
    let b = axis(&mut project, "B", [0., 1., 0.], 0.);
    let level = Uuid::new_v4();
    apply(
        &mut project,
        LayoutEdit::PutLevel {
            id: level,
            name: "Térreo".into(),
            elevation: "0 m".into(),
        },
    );
    assert!(
        propose_snap(
            &project,
            &SnapRequest::Intersection {
                first_axis_id: a,
                second_axis_id: b,
                level_id: level,
                offset: "0 m".into()
            }
        )
        .is_err()
    );
    let before = serde_json::to_vec(&project).unwrap();
    assert!(
        apply_model_edit(
            &mut project,
            ModelEdit::EditLayout {
                command: LayoutEdit::PutLevel {
                    id: a,
                    name: "ID repetido".into(),
                    elevation: "0 m".into()
                }
            },
            "fixture"
        )
        .is_err()
    );
    assert_eq!(before, serde_json::to_vec(&project).unwrap());
}
#[test]
fn whole_object_binding_moves_level_dependents_and_rejects_detached_geometry() {
    let mut project = project();
    let level = Uuid::new_v4();
    apply(
        &mut project,
        LayoutEdit::PutLevel {
            id: level,
            name: "L1".into(),
            elevation: "3 m".into(),
        },
    );
    apply_model_edit(
        &mut project,
        ModelEdit::AddPhysical {
            name: "Referência".into(),
            discipline: Discipline::Structure,
            start: p([0., 0., 0.]),
            end: p([4., 0., 0.]),
            role: PhysicalRole::VisualOnly,
            material_id: None,
            environment_id: None,
        },
        "fixture",
    )
    .unwrap();
    stamp(&mut project);
    let object_id = project.objects[0].id;
    apply(
        &mut project,
        LayoutEdit::BindObject {
            object_id,
            level_id: level,
            offset: "20 cm".into(),
        },
    );
    near(project.objects[0].start_m[2], 3.2, 1e-12);
    apply(
        &mut project,
        LayoutEdit::PutLevel {
            id: level,
            name: "L1".into(),
            elevation: "4 m".into(),
        },
    );
    near(project.objects[0].start_m[2], 4.2, 1e-12);
    near(project.objects[0].end_m[2], 4.2, 1e-12);
    assert!(
        apply_model_edit(
            &mut project,
            ModelEdit::SetPhysicalGeometry {
                object_id,
                start: p([0., 0., 6.]),
                end: p([4., 0., 6.])
            },
            "fixture"
        )
        .is_err()
    );
    apply(&mut project, LayoutEdit::UnbindObject { object_id });
    assert!(state(&project).unwrap().bindings.is_empty());
}
#[test]
fn distance_snap_reports_explicit_geometry_without_mutation() {
    let project = project();
    let proposal = propose_snap(
        &project,
        &SnapRequest::Distance {
            from: LayoutPoint::Coordinates {
                position: p([0.; 3]),
            },
            towards: LayoutPoint::Coordinates {
                position: p([3., 4., 0.]),
            },
            distance: "250 cm".into(),
        },
    )
    .unwrap();
    near(proposal.point_m[0], 1.5, 1e-12);
    near(proposal.point_m[1], 2., 1e-12);
    assert!(!proposal.creates_structural_connection);
}
