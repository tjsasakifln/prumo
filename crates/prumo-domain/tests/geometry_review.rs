use prumo_domain::geometry::{
    GeometryFilter, GeometryFinding, GeometryRepair, SegmentIntersection, SpatialIntersection,
    TOLERANCES_KEY, TolerancePolicy, audit_project, intersect_segments_2d, intersect_segments_3d,
};
use prumo_domain::model::{ModelEdit, PhysicalRole, apply_model_edit};
use prumo_domain::units::PointInput;
use prumo_domain::{ChangeRecord, Discipline, LengthUnit, PROJECT_SCHEMA_VERSION, Project};
use std::collections::BTreeMap;
use uuid::Uuid;

fn segment_fixture(segments: [[[f64; 3]; 2]; 2], topology_m: f64) -> Project {
    let mut project = Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name: "Geometry diagnostic fixture".into(),
        revision: 1,
        units: LengthUnit::Metre,
        objects: vec![],
        environments: vec![],
        networks: vec![],
        ports: vec![],
        connections: vec![],
        history: vec![ChangeRecord {
            revision: 1,
            actor: "geometry-review".into(),
            operation: "fixture".into(),
            timestamp_unix_ms: 1,
        }],
        normative: Default::default(),
        model: Default::default(),
        dependencies: Default::default(),
        extensions: BTreeMap::from([(
            TOLERANCES_KEY.into(),
            serde_json::to_value(TolerancePolicy {
                topology_m,
                ..Default::default()
            })
            .unwrap(),
        )]),
    };
    for (index, [start, end]) in segments.into_iter().enumerate() {
        apply_model_edit(
            &mut project,
            ModelEdit::AddPhysical {
                name: format!("Fixture {index}"),
                discipline: Discipline::Structure,
                start: PointInput {
                    values: start,
                    unit: LengthUnit::Metre,
                },
                end: PointInput {
                    values: end,
                    unit: LengthUnit::Metre,
                },
                role: PhysicalRole::VisualOnly,
                material_id: None,
                environment_id: None,
            },
            "geometry-review",
        )
        .unwrap();
        project.revision += 1;
        project.history.push(ChangeRecord {
            revision: project.revision,
            actor: "geometry-review".into(),
            operation: "fixture.add".into(),
            timestamp_unix_ms: project.revision,
        });
    }
    project.validate().unwrap();
    project
}

fn only_contact_distance(project: &Project) -> f64 {
    let original = project.clone();
    let report = audit_project(project, &GeometryFilter::default()).unwrap();
    assert_eq!(report.findings.len(), 1, "{:#?}", report.findings);
    let GeometryFinding::NearContact {
        distance_m,
        explicitly_connected,
        ..
    } = &report.findings[0]
    else {
        panic!(
            "Expected only a near-contact diagnostic: {:#?}",
            report.findings
        );
    };
    assert!(!explicitly_connected);
    assert!(!report.engineering_calculation_available);
    assert_eq!(project, &original);
    assert!(project.connections.is_empty() && project.model.links.is_empty());
    let mut reversed_order = project.clone();
    reversed_order.objects.reverse();
    assert_eq!(
        report,
        audit_project(&reversed_order, &GeometryFilter::default()).unwrap()
    );
    *distance_m
}

#[test]
fn a_remote_origin_cannot_erase_the_gap_between_short_parallel_segments() {
    let long = [[-1e9, -1e9], [1e9, 1e9]];
    let short = [[0.0, 1e-8], [1e-4, 1e-4 + 1e-8]];
    for a in [long, [long[1], long[0]]] {
        for b in [short, [short[1], short[0]]] {
            assert_eq!(
                intersect_segments_2d(a, b).unwrap(),
                SegmentIntersection::None
            );
            assert_eq!(
                intersect_segments_2d(b, a).unwrap(),
                SegmentIntersection::None
            );
        }
    }
}

#[test]
fn a_small_endpoint_contact_retains_its_original_coordinate() {
    let long = [[-1e9, -1e9], [1e9, 1e9]];
    let short = [[0.0, 1e-8], [1e-4, 1e-4]];
    let expected = SegmentIntersection::Point {
        point: short[1],
        proper: false,
    };
    assert_eq!(intersect_segments_2d(long, short).unwrap(), expected);
    assert_eq!(intersect_segments_2d(short, long).unwrap(), expected);
}

#[test]
fn repair_coordinates_preserve_exponents_and_utf8_in_a_typed_command() {
    let raw = r#"{"edit":"apply_geometry_repair","plan_hash":"reviewed-plan","repairs":[{"repair":"snap_endpoint","object_id":"1d875584-0e4a-4de1-adc4-98a013335b8d","endpoint":"start","target_m":[5e-7,-2e-8,0],"reason":"Prévia da ligação térmica"}]}"#;
    let direct: prumo_domain::model::ModelEdit = serde_json::from_str(raw).unwrap();
    let from_value: prumo_domain::model::ModelEdit =
        serde_json::from_value(serde_json::from_str(raw).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&direct).unwrap(),
        serde_json::to_value(from_value).unwrap()
    );
    let prumo_domain::model::ModelEdit::ApplyGeometryRepair { repairs, .. } = direct else {
        unreachable!()
    };
    let GeometryRepair::SnapEndpoint {
        target_m, reason, ..
    } = &repairs[0]
    else {
        unreachable!()
    };
    assert_eq!(*target_m, [5e-7, -2e-8, 0.0]);
    assert_eq!(reason, "Prévia da ligação térmica");
    assert!(
        serde_json::from_str::<prumo_domain::model::ModelEdit>(&raw.replace("5e-7", "1e999"))
            .is_err()
    );
}

#[test]
fn almost_touching_tip_and_segment_interior_are_reported_without_connecting_them() {
    for scale in [1e-3, 1.0, 1e3] {
        for offset in [0.0, 1e-7] {
            let a = [[0.0, 0.0, 0.0], [scale, 0.0, 0.0]];
            let cases = [
                (
                    [
                        [0.5 * scale, offset * scale, 0.0],
                        [0.5 * scale, scale, 0.0],
                    ],
                    offset * scale,
                ),
                (
                    [
                        [0.5 * scale, offset * scale, offset * scale],
                        [0.5 * scale, scale, 0.5 * scale],
                    ],
                    std::f64::consts::SQRT_2 * offset * scale,
                ),
            ];
            for (b, expected) in cases {
                for a in [a, [a[1], a[0]]] {
                    for b in [b, [b[1], b[0]]] {
                        if offset > 0.0 {
                            assert_eq!(
                                intersect_segments_3d(a, b).unwrap(),
                                SpatialIntersection::None
                            );
                        }
                        let project = segment_fixture([a, b], 1e-6 * scale);
                        let gap = only_contact_distance(&project);
                        assert!(
                            (gap - expected).abs() <= expected * 1e-12,
                            "{gap} versus {expected}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn proximity_preserves_small_gaps_at_remote_origins_in_all_three_projections() {
    let long = [[-1e9, -1e9, 0.0], [1e9, 1e9, 0.0]];
    let short = [[0.0, 1e-8, 0.0], [1e-4, 1e-4 + 1e-8, 0.0]];
    let expected = 1e-8 / std::f64::consts::SQRT_2;
    for axes in [[0, 1, 2], [1, 2, 0], [2, 0, 1]] {
        let permute = |point: [f64; 3]| axes.map(|axis| point[axis]);
        let a = long.map(permute);
        let b = short.map(permute);
        for a in [a, [a[1], a[0]]] {
            for b in [b, [b[1], b[0]]] {
                let project = segment_fixture([a, b], 1e-8);
                let actual = only_contact_distance(&project);
                // Fixed before execution; includes the binary64 spelling of 1e-4+1e-8.
                assert!(
                    (actual - expected).abs() <= 1e-9 * expected,
                    "{actual} versus {expected}"
                );
                let mut tighter = project;
                tighter.extensions.insert(
                    TOLERANCES_KEY.into(),
                    serde_json::to_value(TolerancePolicy {
                        selection_m: 1.0,
                        topology_m: 1e-9,
                        ..Default::default()
                    })
                    .unwrap(),
                );
                assert!(
                    audit_project(&tighter, &GeometryFilter::default())
                        .unwrap()
                        .findings
                        .is_empty()
                );
            }
        }
    }
}

#[test]
fn endpoint_proximity_is_clamped_to_the_finite_segment() {
    let a = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
    for x in [-0.01, 1.01] {
        let b = [[x, 1e-7, 0.0], [x, 1.0, 0.0]];
        for a in [a, [a[1], a[0]]] {
            for b in [b, [b[1], b[0]]] {
                let project = segment_fixture([a, b], 1e-6);
                assert!(
                    audit_project(&project, &GeometryFilter::default())
                        .unwrap()
                        .findings
                        .is_empty()
                );
            }
        }
    }
}

#[test]
fn proper_crossings_and_overlaps_do_not_also_become_proximity_diagnostics() {
    let a = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
    let cross = [[5e-7, -1e-7, 0.0], [5e-7, 1.0, 0.0]];
    let overlap = [[1.0 - 5e-7, 0.0, 0.0], [2.0, 0.0, 0.0]];
    let crossing = audit_project(
        &segment_fixture([a, cross], 1e-6),
        &GeometryFilter::default(),
    )
    .unwrap();
    assert!(matches!(
        crossing.findings.as_slice(),
        [GeometryFinding::Crossing { .. }]
    ));
    let overlapping = audit_project(
        &segment_fixture([a, overlap], 1e-6),
        &GeometryFilter::default(),
    )
    .unwrap();
    assert!(matches!(
        overlapping.findings.as_slice(),
        [GeometryFinding::Overlap { .. }]
    ));
}
