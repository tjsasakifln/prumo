//! Reference coordinates and lengths are hand-calculated before adapter execution.

use prumo_components::bim::{
    BimAdapter, BimError, FIXTURE, LENGTH_TOLERANCE_M, MAX_ENTITIES, MAX_INPUT_BYTES, MAX_NESTING,
    RuststepAdapter,
};

#[test]
fn local_geometry_matches_independent_reference_and_preserves_step_identity() {
    let adapter: Box<dyn BimAdapter> = Box::new(RuststepAdapter);
    let result = adapter.inspect(FIXTURE).unwrap();
    assert_eq!(result.record_count, 14);
    assert_eq!(result.schema, "IFC4");
    assert_eq!(result.coordinate_space, "entity_local");
    assert_eq!(result.source_length_unit, "millimetre");
    assert!(!result.complete_ifc_validation);
    let expected_points = [
        (11, [0.0, 0.0, 0.0]),
        (12, [3.0, 4.0, 0.0]),
        (13, [6.0, 4.0, 0.0]),
    ];
    assert_eq!(result.points.len(), expected_points.len());
    for (actual, (id, coordinates)) in result.points.iter().zip(expected_points) {
        assert_eq!(actual.step_id, id);
        for (actual, expected) in actual.coordinates_m.into_iter().zip(coordinates) {
            assert!((actual - expected).abs() <= LENGTH_TOLERANCE_M);
        }
    }
    assert_eq!(result.polylines.len(), 2);
    assert_eq!(result.polylines[0].step_id, 20);
    assert_eq!(result.polylines[0].point_ids, [11, 12]);
    assert!((result.polylines[0].length_m - 5.0).abs() <= LENGTH_TOLERANCE_M);
    assert_eq!(result.polylines[1].step_id, 21);
    assert_eq!(result.polylines[1].point_ids, [12, 13]);
    assert!((result.polylines[1].length_m - 3.0).abs() <= LENGTH_TOLERANCE_M);
    assert!(
        result
            .uninterpreted_entity_types
            .contains(&"IFCLOCALPLACEMENT".into())
    );
    assert!(
        result
            .uninterpreted_entity_types
            .contains(&"IFCBEAM".into())
    );
}

#[test]
fn explicit_metre_and_centimetre_sources_match_the_same_reference() {
    let adapter = RuststepAdapter;
    let reference = adapter.inspect(FIXTURE).unwrap();
    for (prefix, first, second, third) in
        [("$", "3.", "4.", "6."), (".CENTI.", "300.", "400.", "600.")]
    {
        let source = FIXTURE
            .replace(".MILLI.", prefix)
            .replace("3000.", first)
            .replace("4000.", second)
            .replace("6000.", third);
        let result = adapter.inspect(&source).unwrap();
        assert_eq!(result.points, reference.points);
        assert_eq!(result.polylines, reference.polylines);
    }
}

#[test]
fn syntax_schema_identity_reference_and_unit_errors_do_not_poison_next_input() {
    let adapter = RuststepAdapter;
    assert_eq!(
        adapter.inspect("ISO-10303-21;HEADER;"),
        Err(BimError::Syntax)
    );
    assert_eq!(
        adapter.inspect(&(FIXTURE.to_owned() + "unexpected")),
        Err(BimError::Syntax)
    );
    assert_eq!(
        adapter.inspect(&FIXTURE.replace("IFC4", "IFC2X3")),
        Err(BimError::UnsupportedProfile)
    );
    assert_eq!(
        adapter.inspect(&FIXTURE.replace("#13=", "#11=")),
        Err(BimError::DuplicateIdentity(11))
    );
    assert_eq!(
        adapter.inspect(&FIXTURE.replace("(#11,#12)", "(#11,#999)")),
        Err(BimError::DanglingReference(999))
    );
    assert_eq!(
        adapter.inspect(&FIXTURE.replace(".MILLI.", ".KILO.")),
        Err(BimError::UnsupportedUnit)
    );
    assert!(matches!(
        adapter.inspect(&FIXTURE.replace("(#11,#12)", "(#11,#1)")),
        Err(BimError::InvalidEntity { id: 20, .. })
    ));
    assert!(
        adapter
            .inspect(&FIXTURE.replace("3000.", "1.E999"))
            .is_err()
    );
    assert!(adapter.inspect(FIXTURE).is_ok());
}

#[test]
fn resource_limits_precede_recursive_parsing_and_unbounded_entity_processing() {
    let adapter = RuststepAdapter;
    assert_eq!(
        adapter.inspect(&" ".repeat(MAX_INPUT_BYTES + 1)),
        Err(BimError::ResourceLimit)
    );
    assert_eq!(
        adapter.inspect(&format!(
            "{}{}",
            "(".repeat(MAX_NESTING + 1),
            ")".repeat(MAX_NESTING + 1)
        )),
        Err(BimError::ResourceLimit)
    );
    let mut source = FIXTURE.split("DATA;").next().unwrap().to_owned() + "DATA;\n";
    for id in 1..=MAX_ENTITIES + 1 {
        source.push_str(&format!("#{id}=IFCCARTESIANPOINT((0.,0.,0.));\n"));
    }
    source.push_str("ENDSEC;\nEND-ISO-10303-21;");
    assert!(source.len() < MAX_INPUT_BYTES);
    assert_eq!(adapter.inspect(&source), Err(BimError::ResourceLimit));
    // Delimiters inside quoted text and comments do not increase syntax depth.
    let harmless = FIXTURE
        .replace(
            "Fixture beam",
            &format!("Beam {}", "(".repeat(MAX_NESTING + 1)),
        )
        .replace(
            "#20=",
            &format!("/* {} */ #20=", "(".repeat(MAX_NESTING + 1)),
        );
    assert!(adapter.inspect(&harmless).is_ok());
    // Upstream 0.4.0 does not implement STEP apostrophe escaping. Refuse the
    // unsupported profile explicitly, without silently rewriting source text.
    let escaped = FIXTURE.replace("Fixture beam", "Owner''s beam");
    assert_eq!(
        adapter.inspect(&escaped),
        Err(BimError::UnsupportedTextEncoding)
    );
}

#[test]
fn profile_rejects_ambiguous_units_and_degenerate_geometry() {
    let adapter = RuststepAdapter;
    let ambiguous = FIXTURE.replace(
        "#2=IFCUNITASSIGNMENT((#1));",
        "#2=IFCUNITASSIGNMENT((#1));\n#3=IFCUNITASSIGNMENT((#1));",
    );
    assert_eq!(adapter.inspect(&ambiguous), Err(BimError::UnsupportedUnit));
    let unassigned = FIXTURE.replace(",(#31),#2);", ",(#31),$);");
    assert_eq!(adapter.inspect(&unassigned), Err(BimError::UnsupportedUnit));
    assert!(matches!(
        adapter.inspect(&FIXTURE.replace("(#11,#12)", "(#11,#11)")),
        Err(BimError::InvalidEntity { id: 20, .. })
    ));
    assert!(matches!(
        adapter.inspect(&FIXTURE.replace("(3000.,4000.,0.)", "(3000.,4000.)")),
        Err(BimError::InvalidEntity { id: 12, .. })
    ));
}

#[test]
fn ascii_controls_are_rejected_inside_text_and_formatting_whitespace_is_supported() {
    let adapter = RuststepAdapter;
    for control in ['\0', '\u{0007}', '\u{007f}', '\t', '\r', '\n'] {
        let source = FIXTURE.replace("Fixture beam", &format!("Beam {control} label"));
        assert_eq!(adapter.inspect(&source), Err(BimError::Syntax));
    }
    for control in ['\0', '\u{0007}', '\u{007f}'] {
        assert_eq!(
            adapter.inspect(&format!("{control}{FIXTURE}")),
            Err(BimError::Syntax)
        );
    }
    assert!(adapter.inspect(&FIXTURE.replace('\n', "\r\n\t")).is_ok());
    assert!(
        adapter
            .inspect(&FIXTURE.replace("Fixture beam", "Normal ASCII label"))
            .is_ok()
    );
}

#[test]
fn experiment_reports_actual_checks_without_engineering_approval() {
    let report = prumo_components::bim::run_experiment();
    assert!(report.passed(), "{report:?}");
    assert!(report.elapsed_ns > 0);
    assert_eq!(report.iterations, 20);
    assert!(report.artifact.is_none());
}
