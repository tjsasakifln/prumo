use prumo_domain::geometry::*;

#[test]
fn typed_commands_preserve_exponent_numbers_across_json_boundaries() {
    let input = r#"{"edit":"set_geometry_tolerances","tolerances":{"selection_m":0.01,"topology_m":1e-6,"mesh_m":0.0001,"analysis_m":1e-8}}"#;
    let value: serde_json::Value = serde_json::from_str(input).unwrap();
    let direct: prumo_domain::model::ModelEdit = serde_json::from_str(input).unwrap();
    let converted: prumo_domain::model::ModelEdit = serde_json::from_value(value).unwrap();
    let prumo_domain::model::ModelEdit::SetGeometryTolerances { tolerances: a } = direct else {
        panic!("wrong command")
    };
    let prumo_domain::model::ModelEdit::SetGeometryTolerances { tolerances: b } = converted else {
        panic!("wrong command")
    };
    assert_eq!(a, b);
    assert_eq!(a.topology_m, 1e-6);
}

#[test]
fn tilted_spatial_crossings_and_skew_lines_are_distinguished_exactly() {
    let a = [[0., 0., 0.], [2., 2., 2.]];
    let b = [[0., 2., 1.], [2., 0., 1.]];
    assert_eq!(
        intersect_segments_3d(a, b).unwrap(),
        SpatialIntersection::Point {
            point_m: [1., 1., 1.],
            proper: true
        }
    );
    let skew = [[0., 2., 1. + 1e-10], [2., 0., 1. + 1e-10]];
    assert_eq!(
        intersect_segments_3d(a, skew).unwrap(),
        SpatialIntersection::None
    );
    assert_eq!(
        intersect_segments_3d(a, [[1., 1., 1.], [3., 3., 3.]]).unwrap(),
        SpatialIntersection::Overlap {
            start_m: [1., 1., 1.],
            end_m: [2., 2., 2.]
        }
    );
    assert_eq!(
        intersect_segments_3d(a, [[0., 0., 1.], [2., 2., 3.]]).unwrap(),
        SpatialIntersection::None
    );
}

#[test]
fn robust_orientation_keeps_small_signs_and_rejects_nonfinite_coordinates() {
    assert_eq!(
        orientation_2d([[0., 0.], [1., 1.], [2., 2. + 1e-15]]).unwrap(),
        1
    );
    assert_eq!(
        orientation_2d([[0., 0.], [2., 2. + 1e-15], [1., 1.]]).unwrap(),
        -1
    );
    assert_eq!(orientation_2d([[0., 0.], [1., 1.], [2., 2.]]).unwrap(), 0);
    assert!(orientation_2d([[0., 0.], [f64::NAN, 1.], [2., 2.]]).is_err());
}

#[test]
fn intersections_distinguish_crossing_contact_overlap_and_degeneracy() {
    assert_eq!(
        intersect_segments_2d([[0., 0.], [2., 2.]], [[0., 2.], [2., 0.]]).unwrap(),
        SegmentIntersection::Point {
            point: [1., 1.],
            proper: true
        }
    );
    assert_eq!(
        intersect_segments_2d([[0., 0.], [1., 0.]], [[1., 0.], [2., 1.]]).unwrap(),
        SegmentIntersection::Point {
            point: [1., 0.],
            proper: false
        }
    );
    assert!(matches!(
        intersect_segments_2d([[0., 0.], [3., 0.]], [[1., 0.], [2., 0.]]).unwrap(),
        SegmentIntersection::Overlap { .. }
    ));
    assert_eq!(
        intersect_segments_2d([[0., 0.], [1., 0.]], [[0., 1e-12], [1., 1e-12]]).unwrap(),
        SegmentIntersection::None
    );
    assert!(intersect_segments_2d([[1., 1.], [1., 1.]], [[0., 0.], [2., 2.]]).is_err());
}

#[test]
fn large_coordinates_preserve_local_intersection_and_face_area() {
    let x = 100_000_000.;
    assert_eq!(
        intersect_segments_2d([[x, x], [x + 2., x + 2.]], [[x, x + 2.], [x + 2., x]]).unwrap(),
        SegmentIntersection::Point {
            point: [x + 1., x + 1.],
            proper: true
        }
    );
    let face = FaceInput {
        vertices_m: vec![
            [x, x, 0.],
            [x + 2., x, 0.],
            [x + 2., x + 3., 0.],
            [x, x + 3., 0.],
        ],
        expected_normal: [0., 0., 1.],
    };
    let inspected = inspect_face(&face, TolerancePolicy::default()).unwrap();
    assert!(inspected.valid);
    assert_eq!(inspected.area_m2, 6.);
    let mut reverse = face;
    reverse.vertices_m.reverse();
    let inverted = inspect_face(&reverse, TolerancePolicy::default()).unwrap();
    assert!(inverted.inverted && !inverted.valid);
    assert_eq!(inverted.area_m2, 6.);
}

#[test]
fn faces_detect_nonplanarity_repeated_vertices_and_self_intersection() {
    for vertices in [
        vec![[0., 0., 0.], [2., 0., 0.], [2., 2., 0.01], [0., 2., 0.]],
        vec![[0., 0., 0.], [2., 0., 0.], [2., 0., 0.], [0., 2., 0.]],
        vec![[0., 0., 0.], [2., 2., 0.], [0., 2., 0.], [2., 0., 0.]],
        vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.]],
    ] {
        let r = inspect_face(
            &FaceInput {
                vertices_m: vertices,
                expected_normal: [0., 0., 1.],
            },
            TolerancePolicy::default(),
        )
        .unwrap();
        assert!(!r.valid, "{r:?}");
    }
}

#[test]
fn tolerances_are_distinct_finite_and_cannot_be_silently_widened() {
    let t = TolerancePolicy::default();
    assert_ne!(t.selection_m, t.topology_m);
    assert_ne!(t.mesh_m, t.analysis_m);
    for bad in [f64::NAN, f64::INFINITY, 0., -1., 11.] {
        assert!(
            TolerancePolicy {
                topology_m: bad,
                ..t
            }
            .validate()
            .is_err()
        );
    }
}
