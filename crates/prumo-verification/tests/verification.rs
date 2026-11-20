use prumo_verification::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

fn context() -> RunContext {
    static CONTEXT: OnceLock<RunContext> = OnceLock::new();
    CONTEXT
        .get_or_init(|| RunContext::capture(None).unwrap())
        .clone()
}

fn prepare(protocol: CaseProtocol) -> PreparedProtocol {
    let review = ProtocolReview {
        protocol_sha256: protocol.sha256().unwrap(),
        reviewer: "synthetic comparator test".into(),
        competence_and_scope: "Test fixture for software behavior, not engineering evidence".into(),
        record: "crates/prumo-verification/tests/verification.rs".into(),
        kind: ReviewKind::Software,
        independence_confirmed: true,
    };
    PreparedProtocol::new(protocol, review).unwrap()
}

// This helper synthesizes inputs for comparator tests; it is never the reference-suite executor.
fn synthetic_observation(prepared: &PreparedProtocol) -> Result<CaseExecution, String> {
    let outcome = match &prepared.protocol().expected {
        ExpectedOutcome::Results {
            targets,
            classifications,
        } => ObservedOutcome::Results {
            quantities: targets
                .iter()
                .map(|(id, target)| (id.clone(), target.expected))
                .collect(),
            classifications: classifications.clone(),
        },
        ExpectedOutcome::Rejection { code } => ObservedOutcome::Rejection {
            code: code.clone(),
            detail: "Synthetic comparator probe".into(),
        },
    };
    Ok(CaseExecution {
        protocol_sha256: prepared.sha256().into(),
        engine: EngineIdentity {
            name: "synthetic test input".into(),
            version: "1".into(),
            implementation_lineage: BTreeSet::from(["synthetic-test-input".into()]),
        },
        outcome,
    })
}

fn one() -> PreparedProtocol {
    reference_protocols().unwrap().remove(0)
}
fn required(cases: &[PreparedProtocol]) -> BTreeSet<String> {
    cases
        .iter()
        .map(|case| case.protocol().id.clone())
        .collect()
}

#[test]
fn real_matrix_experiments_match_independent_beam_truss_frame_and_mechanism() {
    let report = run_reference_suite(context()).unwrap();
    assert!(report.passed(), "{:#?}", report.evidence());
    assert_eq!(report.evidence().cases.len(), 4);
    assert_eq!(
        report
            .evidence()
            .cases
            .iter()
            .map(|case| case.metrics.len())
            .sum::<usize>(),
        7
    );
    assert_eq!(report.sha256().len(), 64);
    assert!(!report.evidence().engineering_release);
    assert!(
        report
            .gate(&required(&reference_protocols().unwrap()))
            .verification_allowed
    );
    for case in &report.evidence().cases {
        assert_eq!(case.model_sha256.len(), 64);
        assert_eq!(case.expected_results_sha256.len(), 64);
        assert_eq!(case.actual_results_sha256.len(), 64);
        assert_eq!(case.decision, CaseDecision::Passed);
    }
}

#[test]
fn deterministic_cases_and_reports_ignore_case_insertion_order() {
    let mut cases = reference_protocols().unwrap();
    let first = run_suite("determinism", &cases, context(), synthetic_observation).unwrap();
    cases.reverse();
    let second = run_suite("determinism", &cases, context(), synthetic_observation).unwrap();
    assert_eq!(first.sha256(), second.sha256());
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}

#[test]
fn changing_tolerance_requires_a_new_matching_review_before_execution() {
    let prepared = one();
    let old_sha = prepared.sha256().to_owned();
    let mut changed = prepared.protocol().clone();
    if let ExpectedOutcome::Results { targets, .. } = &mut changed.expected {
        targets.values_mut().next().unwrap().tolerance.relative = 0.01;
    }
    assert!(matches!(
        PreparedProtocol::new(changed, prepared.review().clone()),
        Err(VerificationError::InvalidReview(_))
    ));
    assert_eq!(prepared.sha256(), old_sha);
}

#[test]
fn sign_mutation_is_rejected_even_when_magnitude_is_correct() {
    let cases = [one()];
    let report = run_suite("wrong-sign", &cases, context(), |case| {
        let mut result = synthetic_observation(case)?;
        if let ObservedOutcome::Results { quantities, .. } = &mut result.outcome {
            for value in quantities.values_mut() {
                value.value = -value.value;
            }
        }
        Ok(result)
    })
    .unwrap();
    assert!(!report.passed());
    assert_eq!(report.evidence().cases[0].decision, CaseDecision::Failed);
    assert!(!report.gate(&required(&cases)).verification_allowed);
}

#[test]
fn same_dimension_different_unit_is_not_silently_converted() {
    let cases = [one()];
    let report = run_suite("wrong-unit", &cases, context(), |case| {
        let mut result = synthetic_observation(case)?;
        if let ObservedOutcome::Results { quantities, .. } = &mut result.outcome {
            let length = quantities
                .values_mut()
                .find(|value| value.unit == Unit::Metre)
                .unwrap();
            length.unit = Unit::Millimetre;
            length.value *= 1000.0;
        }
        Ok(result)
    })
    .unwrap();
    assert!(!report.passed());
    assert!(
        report.evidence().cases[0]
            .metrics
            .iter()
            .any(|metric| metric.diagnostic.contains("Unidade diferente"))
    );
}

#[test]
fn two_interfaces_of_the_same_kernel_are_not_independent() {
    let mut protocol = one().protocol().clone();
    protocol.reference.implementation_lineage = BTreeSet::from(["shared-solver-kernel".into()]);
    let cases = [prepare(protocol)];
    let report = run_suite("not-independent", &cases, context(), |case| {
        let mut result = synthetic_observation(case)?;
        result.engine.name = "a completely different interface name".into();
        result.engine.implementation_lineage =
            BTreeSet::from(["wrapper-b".into(), "shared-solver-kernel".into()]);
        Ok(result)
    })
    .unwrap();
    assert_eq!(report.evidence().cases[0].decision, CaseDecision::Blocked);
    assert!(!report.gate(&required(&cases)).verification_allowed);
}

#[test]
fn missing_result_and_missing_execution_block_the_gate() {
    let cases = [one()];
    let empty = run_suite("missing", &cases, context(), |case| {
        let mut result = synthetic_observation(case)?;
        result.outcome = ObservedOutcome::Results {
            quantities: BTreeMap::new(),
            classifications: BTreeMap::new(),
        };
        Ok(result)
    })
    .unwrap();
    assert!(!empty.passed());
    assert!(
        empty.evidence().cases[0]
            .metrics
            .iter()
            .all(|metric| metric.actual.is_none())
    );
    let failed = run_suite("failed-process", &cases, context(), |_| {
        Err("Process disconnected".into())
    })
    .unwrap();
    assert_eq!(failed.evidence().cases[0].decision, CaseDecision::Blocked);
    assert!(!failed.gate(&required(&cases)).verification_allowed);
    assert_eq!(failed.evidence().cases[0].actual_results_sha256.len(), 64);
}

#[test]
fn omitting_a_required_case_cannot_produce_release() {
    let cases = [one()];
    let report = run_suite("missing-case", &cases, context(), synthetic_observation).unwrap();
    let mut ids = required(&cases);
    ids.insert("mandatory-case-not-executed".into());
    let gate = report.gate(&ids);
    assert!(!gate.verification_allowed);
    assert_eq!(
        gate.missing_cases,
        BTreeSet::from(["mandatory-case-not-executed".into()])
    );
    assert!(!gate.engineering_release);
    assert!(!report.gate(&BTreeSet::new()).verification_allowed);
}

#[test]
fn a_critical_failure_blocks_even_when_not_in_selected_required_subset() {
    let cases = reference_protocols().unwrap();
    let failed_id = cases[0].protocol().id.clone();
    let report = run_suite("critical", &cases, context(), |case| {
        if case.protocol().id == failed_id {
            Err("deliberate failure".into())
        } else {
            synthetic_observation(case)
        }
    })
    .unwrap();
    let mut subset = required(&cases);
    subset.remove(&failed_id);
    assert!(!report.gate(&subset).verification_allowed);
}

#[test]
fn obsolete_protocol_hash_is_never_accepted() {
    let cases = [one()];
    let report = run_suite("old-protocol", &cases, context(), |case| {
        let mut result = synthetic_observation(case)?;
        result.protocol_sha256 = "0".repeat(64);
        Ok(result)
    })
    .unwrap();
    assert_eq!(report.evidence().cases[0].decision, CaseDecision::Blocked);
}

#[test]
fn nonfinite_observations_are_blocked_and_the_diagnostic_is_serializable() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let report = run_suite("not-finite", &[one()], context(), |case| {
            let mut result = synthetic_observation(case)?;
            if let ObservedOutcome::Results { quantities, .. } = &mut result.outcome {
                quantities.values_mut().next().unwrap().value = invalid;
            }
            Ok(result)
        })
        .unwrap();
        assert_eq!(report.evidence().cases[0].decision, CaseDecision::Blocked);
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("Resultado inválido"));
        assert!(!json.contains("\"value\":null"));
    }
}

#[test]
fn normative_boundary_decisions_are_exact_despite_numeric_tolerance() {
    for (id, utilization, expected) in [
        ("below", 0.999999999, "meets"),
        ("on", 1.0, "meets"),
        ("above", 1.000000001, "does_not_meet"),
    ] {
        let mut protocol = one().protocol().clone();
        protocol.id = format!("synthetic-boundary-{id}");
        protocol.domain = "synthetic_comparator_only_not_a_normative_rule".into();
        protocol.expected = ExpectedOutcome::Results {
            targets: BTreeMap::from([("utilization".into(), Target {
                quantity: QuantityKind::Dimensionless, expected: QuantityValue { value: utilization, unit: Unit::One },
                tolerance: Tolerance { absolute: 0.001, relative: 0.0, reference_scale: 1.0, rationale: "Deliberately loose numeric metric tests that exact categorical decisions remain strict.".into() },
            })]), classifications: BTreeMap::from([("decision".into(), expected.into())]),
        };
        let case = prepare(protocol);
        let correct = run_suite(
            "boundary",
            std::slice::from_ref(&case),
            context(),
            synthetic_observation,
        )
        .unwrap();
        assert!(correct.passed());
        let wrong = run_suite("boundary", &[case], context(), |case| {
            let mut result = synthetic_observation(case)?;
            if let ObservedOutcome::Results {
                quantities,
                classifications,
            } = &mut result.outcome
            {
                quantities.get_mut("utilization").unwrap().value = 1.0;
                classifications.insert(
                    "decision".into(),
                    if expected == "meets" {
                        "does_not_meet"
                    } else {
                        "meets"
                    }
                    .into(),
                );
            }
            Ok(result)
        })
        .unwrap();
        assert!(!wrong.passed());
        assert!(wrong.evidence().cases[0].metrics[0].passed);
        assert!(!wrong.evidence().cases[0].classifications[0].passed);
    }
}

#[test]
fn installation_quantities_have_explicit_domains_and_units() {
    let quantities = [
        (Unit::Pascal, QuantityKind::Pressure),
        (Unit::CubicMetrePerSecond, QuantityKind::VolumeFlow),
        (Unit::Ampere, QuantityKind::Current),
        (Unit::Volt, QuantityKind::Voltage),
        (Unit::Watt, QuantityKind::Power),
        (Unit::Joule, QuantityKind::Energy),
        (Unit::Kelvin, QuantityKind::AbsoluteTemperature),
        (Unit::KelvinDifference, QuantityKind::TemperatureDifference),
    ];
    for (unit, quantity) in quantities {
        let mut protocol = one().protocol().clone();
        protocol.expected = ExpectedOutcome::Results {
            targets: BTreeMap::from([("installation-quantity".into(), Target {
                quantity, expected: QuantityValue { value: 5.0, unit },
                tolerance: Tolerance { absolute: 1e-9, relative: 0.0, reference_scale: 5.0, rationale: "Synthetic protocol-schema test, not qualification of an installation engine.".into() },
            })]), classifications: BTreeMap::new(),
        };
        let prepared = prepare(protocol.clone());
        assert!(
            run_suite(
                "installation",
                &[prepared],
                context(),
                synthetic_observation
            )
            .unwrap()
            .passed()
        );
        if let ExpectedOutcome::Results { targets, .. } = &mut protocol.expected {
            targets.values_mut().next().unwrap().expected.unit = Unit::Newton;
        }
        assert!(protocol.validate().is_err());
    }
}

#[test]
fn invalid_tolerances_and_incompatible_quantity_are_rejected_before_execution() {
    for bad in [-1.0, f64::NAN, f64::INFINITY] {
        let mut protocol = one().protocol().clone();
        if let ExpectedOutcome::Results { targets, .. } = &mut protocol.expected {
            targets.values_mut().next().unwrap().tolerance.absolute = bad;
        }
        assert!(protocol.sha256().is_err());
    }
    let mut protocol = one().protocol().clone();
    if let ExpectedOutcome::Results { targets, .. } = &mut protocol.expected {
        targets
            .values_mut()
            .next()
            .unwrap()
            .tolerance
            .reference_scale = 0.0;
    }
    assert!(protocol.validate().is_err());
}

#[test]
fn unknown_json_fields_missing_provenance_duplicates_and_empty_suites_are_refused() {
    let mut json = serde_json::to_value(one().protocol()).unwrap();
    json["pretend_approved"] = serde_json::json!(true);
    assert!(serde_json::from_value::<CaseProtocol>(json).is_err());
    assert!(run_suite("empty", &[], context(), synthetic_observation).is_err());
    assert!(
        run_suite(
            "duplicates",
            &[one(), one()],
            context(),
            synthetic_observation
        )
        .is_err()
    );
    let mut bad = context();
    bad.build.prumo_commit.clear();
    let ran = std::cell::Cell::new(false);
    assert!(
        run_suite("no-provenance", &[one()], bad, |case| {
            ran.set(true);
            synthetic_observation(case)
        })
        .is_err()
    );
    assert!(!ran.get());
}

#[test]
fn a_snapshot_requires_a_real_project_identity_and_positive_revision() {
    let mut captured = context();
    captured.basis = Some(prumo_domain::components::ExperimentBasis {
        project_id: "90f3e6c7-ef6e-42e6-962f-e851b2669fc9".parse().unwrap(),
        revision: 1,
        content_sha256: "a".repeat(64),
    });
    assert!(captured.validate().is_ok());
    let mut nil_project = captured.clone();
    nil_project.basis.as_mut().unwrap().project_id = Default::default();
    assert!(nil_project.validate().is_err());
    let mut zero_revision = captured;
    zero_revision.basis.as_mut().unwrap().revision = 0;
    assert!(zero_revision.validate().is_err());
}

#[test]
fn imported_protocol_preserves_values_and_rejects_reserved_numeric_transport_keys() {
    let protocol = one().protocol().clone();
    let bytes = serde_json::to_vec(&protocol).unwrap();
    let imported = CaseProtocol::from_json(&bytes).unwrap();
    assert_eq!(imported, protocol);
    assert_eq!(imported.sha256().unwrap(), protocol.sha256().unwrap());
    let mut value = serde_json::to_value(&protocol).unwrap();
    value["inputs"]["length"]["value"] = serde_json::json!({"$serde_json::private::Number":"2"});
    assert!(CaseProtocol::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn regression_report_preserves_failures_missing_cases_and_changed_protocols() {
    let cases = reference_protocols().unwrap();
    let previous = run_suite("regression", &cases, context(), synthetic_observation).unwrap();
    let current = run_suite("regression", &cases[..3], context(), |case| {
        if case.protocol().id == cases[0].protocol().id {
            Err("injected regression".into())
        } else {
            synthetic_observation(case)
        }
    })
    .unwrap();
    let diff = compare_regression(&previous, &current);
    assert_eq!(
        diff.regressions,
        BTreeSet::from([cases[0].protocol().id.clone()])
    );
    assert_eq!(
        diff.removed,
        BTreeSet::from([cases[3].protocol().id.clone()])
    );
    assert!(diff.changed_outputs.contains(&cases[0].protocol().id));
    let repaired = compare_regression(&current, &previous);
    assert_eq!(repaired.fixes, diff.regressions);
    let mut changed = cases[0].protocol().clone();
    changed
        .assumptions
        .push("Protocol revision for a new independently reviewed domain.".into());
    let new_protocol = run_suite(
        "regression",
        &[prepare(changed)],
        context(),
        synthetic_observation,
    )
    .unwrap();
    assert!(
        compare_regression(&previous, &new_protocol)
            .changed_protocols
            .contains(&cases[0].protocol().id)
    );
}
