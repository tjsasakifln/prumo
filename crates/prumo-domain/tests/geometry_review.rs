use prumo_domain::geometry::{GeometryRepair, SegmentIntersection, intersect_segments_2d};

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
