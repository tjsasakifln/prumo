//! Qualification records in this file are synthetic fixtures, never production
//! approvals, normative interpretations or engineering calculation results.

use prumo_registry::*;
use serde_json::Value;
use std::collections::BTreeSet;

fn evidence(label: &str, digit: char) -> EvidenceLink {
    EvidenceLink {
        reference: format!("fixture-only:{label}"),
        sha256: digit.to_string().repeat(64),
    }
}

fn fixture_catalog() -> RegistryCatalog {
    RegistryCatalog {
        schema_version: 1,
        version: "fixture-1".into(),
        norms: vec![NormRecord {
            id: "fixture-norm".into(),
            designation: "FIXTURE 001".into(),
            part: Some("1".into()),
            edition: Some("2020".into()),
            amendments: Vec::new(),
            errata: Vec::new(),
            title: "Documento fictício sem valor normativo".into(),
            publisher: "Teste".into(),
            source_ids: vec!["FIXTURE".into()],
            sources: vec!["https://example.invalid/fixture".into()],
            evidence_class: EvidenceClass::Catalog,
            access: NormAccess::LicensedTextConsulted,
            domains: vec!["structure".into()],
            interpretation_owner_role: "Responsável fictício".into(),
            licensed_evidence: Some(LicensedEvidence {
                authorization_reference: "fixture-only:authorization".into(),
                consulted_by: "Revisor fictício".into(),
                consulted_on: "2026-11-14".into(),
                document_sha256: "a".repeat(64),
            }),
            resolution: Some(NormResolution {
                status: NormValidity::Applicable,
                verified_on: "2026-11-14".into(),
                verified_by: "Revisor fictício".into(),
                catalog_evidence: evidence("catalog", 'b'),
                applicability_evidence: evidence("applicability", 'c'),
                cross_reference_review: evidence("cross-reference-review", 'd'),
                cross_reference_norm_ids: Vec::new(),
                approved_domains: vec!["structure".into()],
                amendments_and_errata_reviewed: true,
            }),
        }],
        capabilities: vec![CapabilityRecord {
            id: "PAR-06-fixture".into(),
            family: "PAR-06".into(),
            title: "Capacidade sintética de ensaio".into(),
            product: "Fixture".into(),
            module: "Fixture".into(),
            reference_version: Some("fixture-1".into()),
            version_note: "Somente para teste do bloqueio, sem cálculo".into(),
            source_ids: vec!["FIXTURE".into()],
            sources: Vec::new(),
            owner_role: "Equipe fictícia de teste".into(),
            implementation_issues: vec![41],
            operation: CapabilityOperation::Verify,
            discipline: "structure".into(),
            method: "fixture-linear".into(),
            material: "fixture-concrete".into(),
            norm_ids: vec!["fixture-norm".into()],
            state: CapabilityState::Planned,
            qualification: None,
        }],
    }
}

fn qualification(catalog: &RegistryCatalog) -> QualificationEvidence {
    let capability = &catalog.capabilities[0];
    QualificationEvidence {
        implementation_revision: "fixture-only:implementation".into(),
        validation_report: evidence("validation", '1'),
        independent_reference: evidence("independent", '2'),
        domain_approval: DomainApproval {
            method: capability.method.clone(),
            material: capability.material.clone(),
            norm_fingerprints: catalog.norm_fingerprints(capability).unwrap(),
            domain_description: "Somente fixture do mecanismo de estados".into(),
            reviewer: "Revisor fictício".into(),
            professional_registration: "FIXTURE-NOT-A-REGISTRATION".into(),
            approved_on: "2026-11-14".into(),
            approval_reference: "fixture-only:domain-approval".into(),
        },
        release: Some(ReleaseEvidence {
            version: "fixture-only-release".into(),
            artifact_sha256: "3".repeat(64),
            approval_reference: "fixture-only:release".into(),
        }),
    }
}

fn experimental(mut catalog: RegistryCatalog) -> RegistryCatalog {
    let id = catalog.capabilities[0].id.clone();
    transition_capability(
        &mut catalog,
        &id,
        CapabilityState::Experimental,
        None,
        "fixture-2".into(),
    )
    .unwrap();
    catalog
}

fn released_fixture() -> RegistryCatalog {
    let mut catalog = experimental(fixture_catalog());
    let id = catalog.capabilities[0].id.clone();
    let proof = qualification(&catalog);
    transition_capability(
        &mut catalog,
        &id,
        CapabilityState::Verified,
        Some(proof.clone()),
        "fixture-3".into(),
    )
    .unwrap();
    transition_capability(
        &mut catalog,
        &id,
        CapabilityState::Released,
        Some(proof),
        "fixture-4".into(),
    )
    .unwrap();
    catalog
}

#[test]
fn bundled_inventory_matches_every_planned_family_and_has_no_fabricated_approval() {
    let catalog = bundled_catalog().unwrap();
    let coverage: Value =
        serde_json::from_str(include_str!("../../../planning/coverage.json")).unwrap();
    let expected: BTreeSet<_> = coverage["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["capability_id"].as_str().unwrap().to_owned())
        .collect();
    let actual: BTreeSet<_> = catalog
        .capabilities
        .iter()
        .map(|record| record.family.clone())
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 72);
    let backlog: Value =
        serde_json::from_str(include_str!("../../../planning/backlog.json")).unwrap();
    let issues: BTreeSet<u16> = backlog["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| {
            issue["key"]
                .as_str()
                .unwrap()
                .trim_start_matches("PRU-")
                .parse()
                .unwrap()
        })
        .collect();
    assert!(
        catalog
            .capabilities
            .iter()
            .flat_map(|record| &record.implementation_issues)
            .all(|issue| issues.contains(issue))
    );
    for family in coverage["capabilities"].as_array().unwrap() {
        let id = family["capability_id"].as_str().unwrap();
        let expected_issues: BTreeSet<u16> = family["implementation_issues"]
            .as_array()
            .unwrap()
            .iter()
            .map(|issue| {
                issue
                    .as_str()
                    .unwrap()
                    .trim_start_matches("PRU-")
                    .parse()
                    .unwrap()
            })
            .collect();
        let recorded: BTreeSet<_> = catalog
            .capabilities
            .iter()
            .filter(|record| record.family == id)
            .flat_map(|record| record.implementation_issues.iter().copied())
            .collect();
        assert!(
            expected_issues.is_subset(&recorded),
            "{id}: implementation issues lost"
        );
    }
    assert_eq!(catalog.capabilities.len(), 368);
    assert_eq!(catalog.norms.len(), 171);
    let audit = audit_coverage(&catalog);
    assert!(audit.errors.is_empty(), "{:?}", audit.errors);
    assert!(!audit.pending.is_empty());
    assert!(
        catalog.capabilities.iter().all(
            |record| record.state == CapabilityState::Planned && record.qualification.is_none()
        )
    );
    assert!(
        catalog
            .norms
            .iter()
            .all(|norm| norm.access == NormAccess::PublicMetadataOnly
                && norm.licensed_evidence.is_none()
                && norm.resolution.is_none())
    );
    for family in ["nbr-7190", "nbr-5419", "nbr-15575"] {
        let count = catalog
            .norms
            .iter()
            .filter(|norm| norm.id.starts_with(&format!("{family}-")) && norm.part.is_some())
            .count();
        assert!(
            count
                >= if family == "nbr-7190" {
                    7
                } else if family == "nbr-5419" {
                    4
                } else {
                    6
                }
        );
    }
    for discipline in [
        "water_sanitation",
        "gas",
        "fire_protection",
        "electrical",
        "photovoltaic",
        "lightning_protection",
        "cabling",
        "hvac",
    ] {
        assert!(
            catalog
                .capabilities
                .iter()
                .any(|capability| capability.discipline == discipline),
            "{discipline}"
        );
    }
}

#[test]
fn coverage_audit_finds_missing_family_owner_issue_and_norm_resolution() {
    let mut catalog = bundled_catalog().unwrap();
    catalog
        .capabilities
        .retain(|record| record.family != "MEP-36");
    catalog.capabilities[0].owner_role.clear();
    catalog.capabilities[0].implementation_issues.clear();
    catalog.capabilities[0]
        .norm_ids
        .push("does-not-exist".into());
    let audit = audit_coverage(&catalog);
    for code in [
        "missing_family",
        "missing_owner",
        "missing_implementation_issue",
        "unresolved_norm_reference",
    ] {
        assert!(audit.errors.iter().any(|item| item.code == code), "{code}");
    }
    assert!(
        audit
            .pending
            .iter()
            .any(|item| item.code == "norm_edition_unresolved")
    );
    assert!(
        audit
            .pending
            .iter()
            .any(|item| item.code == "normative_resolution_pending")
    );
}

#[test]
fn package_hash_covers_actual_metadata_and_is_canonical_across_record_order() {
    let catalog = bundled_catalog().unwrap();
    let package = NormativePackage::from_catalog(&catalog).unwrap();
    let mut reordered = catalog;
    reordered.norms.reverse();
    reordered.capabilities.reverse();
    assert_eq!(NormativePackage::from_catalog(&reordered).unwrap(), package);
    let mut tampered = package.clone();
    tampered.catalog.norms[0].edition = Some("2099".into());
    assert!(matches!(
        tampered.validate(),
        Err(RegistryError::HashMismatch)
    ));
    let roundtrip: NormativePackage =
        serde_json::from_slice(&serde_json::to_vec(&package).unwrap()).unwrap();
    assert_eq!(roundtrip, package);
    roundtrip.validate().unwrap();
}

#[test]
fn a_public_source_never_qualifies_a_normative_rule() {
    let mut catalog = experimental(fixture_catalog());
    catalog.norms[0].access = NormAccess::PublicMetadataOnly;
    catalog.norms[0].licensed_evidence = None;
    let before = catalog.clone();
    let proof = qualification(&catalog);
    assert!(matches!(
        transition_capability(
            &mut catalog,
            "PAR-06-fixture",
            CapabilityState::Verified,
            Some(proof),
            "fixture-3".into()
        ),
        Err(RegistryError::LicensedSourceRequired(_))
    ));
    assert_eq!(catalog, before);
}

#[test]
fn licensed_access_alone_does_not_resolve_validity_applicability_or_cross_references() {
    for change in 0..5 {
        let mut catalog = experimental(fixture_catalog());
        match change {
            0 => catalog.norms[0].resolution = None,
            1 => catalog.norms[0].resolution.as_mut().unwrap().status = NormValidity::Suspended,
            2 => {
                catalog.norms[0]
                    .resolution
                    .as_mut()
                    .unwrap()
                    .amendments_and_errata_reviewed = false
            }
            3 => {
                catalog.norms[0]
                    .resolution
                    .as_mut()
                    .unwrap()
                    .approved_domains = vec!["gas".into()]
            }
            4 => {
                catalog.norms[0]
                    .resolution
                    .as_mut()
                    .unwrap()
                    .cross_reference_norm_ids = vec!["unresolved-reference".into()]
            }
            _ => unreachable!(),
        }
        if change == 4 {
            assert!(catalog.norm_fingerprints(&catalog.capabilities[0]).is_err());
            continue;
        }
        let proof = qualification(&catalog);
        assert!(
            transition_capability(
                &mut catalog,
                "PAR-06-fixture",
                CapabilityState::Verified,
                Some(proof),
                "fixture-3".into()
            )
            .is_err()
        );
    }
    let seed = bundled_catalog().unwrap();
    for id in ["nbr-6122", "nbr-16401-3"] {
        let norm = seed.norms.iter().find(|norm| norm.id == id).unwrap();
        assert!(
            norm.resolution.is_none(),
            "{id} must retain the documented unresolved applicability"
        );
        assert!(
            audit_coverage(&seed)
                .pending
                .iter()
                .any(|item| item.subject == id && item.code == "normative_resolution_pending")
        );
    }
}

#[test]
fn qualification_binds_exact_method_material_and_normative_version() {
    for change in 0..4 {
        let mut catalog = experimental(fixture_catalog());
        let mut proof = qualification(&catalog);
        match change {
            0 => proof.domain_approval.method = "another-method".into(),
            1 => proof.domain_approval.material = "another-material".into(),
            2 => {
                proof
                    .domain_approval
                    .norm_fingerprints
                    .insert("fixture-norm".into(), "f".repeat(64));
            }
            3 => {
                proof.domain_approval.professional_registration.clear();
            }
            _ => unreachable!(),
        }
        assert!(
            transition_capability(
                &mut catalog,
                "PAR-06-fixture",
                CapabilityState::Verified,
                Some(proof),
                "fixture-3".into()
            )
            .is_err()
        );
    }
}

#[test]
fn the_same_artifact_with_a_different_url_is_not_an_independent_reference() {
    let mut catalog = experimental(fixture_catalog());
    let mut proof = qualification(&catalog);
    proof.independent_reference.reference = "fixture-only:another-location".into();
    proof.independent_reference.sha256 = proof.validation_report.sha256.clone();
    assert!(
        transition_capability(
            &mut catalog,
            "PAR-06-fixture",
            CapabilityState::Verified,
            Some(proof),
            "fixture-3".into()
        )
        .is_err()
    );
}

#[test]
fn state_progression_and_release_require_separate_evidence_without_promoting_the_family() {
    let mut catalog = fixture_catalog();
    let mut sibling = catalog.capabilities[0].clone();
    sibling.id = "PAR-06-fixture-other-material".into();
    sibling.material = "fixture-steel".into();
    catalog.capabilities.push(sibling);
    let proof = qualification(&catalog);
    assert!(matches!(
        transition_capability(
            &mut catalog,
            "PAR-06-fixture",
            CapabilityState::Verified,
            Some(proof.clone()),
            "fixture-2".into()
        ),
        Err(RegistryError::InvalidTransition)
    ));
    catalog = experimental(catalog);
    transition_capability(
        &mut catalog,
        "PAR-06-fixture",
        CapabilityState::Verified,
        Some(proof.clone()),
        "fixture-3".into(),
    )
    .unwrap();
    let mut missing_release = proof.clone();
    missing_release.release = None;
    assert!(
        transition_capability(
            &mut catalog,
            "PAR-06-fixture",
            CapabilityState::Released,
            Some(missing_release),
            "fixture-4".into()
        )
        .is_err()
    );
    transition_capability(
        &mut catalog,
        "PAR-06-fixture",
        CapabilityState::Released,
        Some(proof),
        "fixture-4".into(),
    )
    .unwrap();
    assert_eq!(catalog.capabilities[1].state, CapabilityState::Planned);
    assert!(catalog.capabilities[1].qualification.is_none());
}

#[test]
fn final_result_gate_rejects_incomplete_unreleased_wrong_domain_and_stale_results() {
    let package = NormativePackage::from_catalog(&released_fixture()).unwrap();
    let valid_fixture = FinalResultContext {
        project_revision: 7,
        result_revision: Some(7),
        result_package_hash: Some(package.hash.clone()),
        method: "fixture-linear".into(),
        material: "fixture-concrete".into(),
        recalculation_required: false,
        calculation_evidence: Some(evidence("synthetic-result", '4')),
    };
    assert!(evaluate_final_result(Some(&package), "PAR-06-fixture", &valid_fixture).allowed);
    for change in 0..6 {
        let mut context = valid_fixture.clone();
        match change {
            0 => context.calculation_evidence = None,
            1 => context.result_revision = Some(6),
            2 => context.result_package_hash = Some("e".repeat(64)),
            3 => context.method = "other-method".into(),
            4 => context.material = "other-material".into(),
            5 => context.recalculation_required = true,
            _ => unreachable!(),
        }
        assert!(!evaluate_final_result(Some(&package), "PAR-06-fixture", &context).allowed);
    }
    let experimental = NormativePackage::from_catalog(&experimental(fixture_catalog())).unwrap();
    assert!(!evaluate_final_result(Some(&experimental), "PAR-06-fixture", &valid_fixture).allowed);
    assert!(!evaluate_final_result(None, "PAR-06-fixture", &valid_fixture).allowed);
}

#[test]
fn registered_revision_does_not_inherit_license_approval_or_change_the_old_package() {
    let catalog = released_fixture();
    let original_package = NormativePackage::from_catalog(&catalog).unwrap();
    let mut updated_reference = NormReferenceInput::from(&catalog.norms[0]);
    updated_reference.edition = Some("2026".into());
    let (updated, change) =
        register_reference(&catalog, updated_reference, "fixture-new-edition".into(), 8).unwrap();
    assert_eq!(
        original_package.catalog.norms[0].edition.as_deref(),
        Some("2020")
    );
    assert_eq!(updated.norms[0].edition.as_deref(), Some("2026"));
    assert_eq!(updated.norms[0].access, NormAccess::PublicMetadataOnly);
    assert!(updated.norms[0].resolution.is_none());
    assert!(updated.norms[0].licensed_evidence.is_none());
    assert_eq!(updated.capabilities[0].state, CapabilityState::Planned);
    assert!(updated.capabilities[0].qualification.is_none());
    assert_eq!(change.previous.unwrap().edition.as_deref(), Some("2020"));
    assert_eq!(change.impact.changed_norm_ids, ["fixture-norm"]);
    assert_eq!(change.impact.affected_capability_ids, ["PAR-06-fixture"]);
    assert!(change.impact.recalculation_required);
}

#[test]
fn cross_references_invalidate_transitively_and_cycles_do_not_hide_dependencies() {
    let mut catalog = fixture_catalog();
    let mut second = catalog.norms[0].clone();
    second.id = "fixture-b".into();
    let mut third = catalog.norms[0].clone();
    third.id = "fixture-c".into();
    catalog.norms[0]
        .resolution
        .as_mut()
        .unwrap()
        .cross_reference_norm_ids = vec![second.id.clone()];
    second.resolution.as_mut().unwrap().cross_reference_norm_ids = vec![third.id.clone()];
    third.resolution.as_mut().unwrap().cross_reference_norm_ids = vec!["fixture-norm".into()];
    catalog.norms.extend([second, third]);
    assert_eq!(
        catalog
            .norm_dependency_ids(&catalog.capabilities[0].norm_ids)
            .len(),
        3
    );
    catalog = experimental(catalog);
    let proof = qualification(&catalog);
    assert_eq!(proof.domain_approval.norm_fingerprints.len(), 3);
    transition_capability(
        &mut catalog,
        "PAR-06-fixture",
        CapabilityState::Verified,
        Some(proof.clone()),
        "fixture-3".into(),
    )
    .unwrap();
    transition_capability(
        &mut catalog,
        "PAR-06-fixture",
        CapabilityState::Released,
        Some(proof),
        "fixture-4".into(),
    )
    .unwrap();
    let before = NormativePackage::from_catalog(&catalog).unwrap();
    let mut reference = NormReferenceInput::from(
        catalog
            .norms
            .iter()
            .find(|norm| norm.id == "fixture-c")
            .unwrap(),
    );
    reference.edition = Some("2030".into());
    let (updated, change) =
        register_reference(&catalog, reference, "fixture-transitive-change".into(), 9).unwrap();
    assert_eq!(updated.capabilities[0].state, CapabilityState::Planned);
    assert_eq!(change.impact.changed_norm_ids, ["fixture-c"]);
    assert_eq!(change.impact.affected_capability_ids, ["PAR-06-fixture"]);
    let after = NormativePackage::from_catalog(&updated).unwrap();
    assert_eq!(compare_packages(&before, &after, 9).unwrap(), change.impact);

    let mut stale_proof = catalog.clone();
    stale_proof
        .norms
        .iter_mut()
        .find(|norm| norm.id == "fixture-c")
        .unwrap()
        .edition = Some("2030".into());
    assert!(stale_proof.validate_integrity().is_err());
    let mut unresolved = experimental(fixture_catalog());
    unresolved.norms[0]
        .resolution
        .as_mut()
        .unwrap()
        .cross_reference_norm_ids = vec!["fixture-missing-transitive".into()];
    assert!(
        unresolved
            .norm_fingerprints(&unresolved.capabilities[0])
            .is_err()
    );
    unresolved.capabilities[0].state = CapabilityState::Planned;
    assert!(
        audit_coverage(&unresolved)
            .errors
            .iter()
            .any(|finding| finding.code == "unresolved_norm_reference"
                && finding.message.contains("fixture-missing-transitive"))
    );
}

#[test]
fn changing_the_primary_reference_preserves_secondary_sources_domains_and_provenance() {
    let mut catalog = bundled_catalog().unwrap();
    let norm = catalog
        .norms
        .iter_mut()
        .find(|norm| norm.id == "nbr-5410")
        .unwrap();
    norm.sources.extend([
        "https://example.invalid/secondary-a".into(),
        "https://example.invalid/secondary-b".into(),
    ]);
    norm.domains
        .extend(["fixture-secondary-a".into(), "fixture-secondary-b".into()]);
    norm.source_ids
        .extend(["FIXTURE-A".into(), "FIXTURE-B".into()]);
    let previous = norm.clone();
    let mut input = NormReferenceInput::from(&previous);
    input.edition = Some("fixture-updated".into());
    input.source_url = "https://example.invalid/new-primary".into();
    input.domain = "fixture-primary".into();
    let (updated, change) =
        register_reference(&catalog, input, "fixture-source-preservation".into(), 2).unwrap();
    let current = updated
        .norms
        .iter()
        .find(|norm| norm.id == "nbr-5410")
        .unwrap();
    assert_eq!(current.sources[0], "https://example.invalid/new-primary");
    assert_eq!(current.domains[0], "fixture-primary");
    assert!(
        previous
            .sources
            .iter()
            .skip(1)
            .all(|value| current.sources.contains(value))
    );
    assert!(
        previous
            .domains
            .iter()
            .skip(1)
            .all(|value| current.domains.contains(value))
    );
    assert!(
        previous
            .source_ids
            .iter()
            .all(|value| current.source_ids.contains(value))
    );
    assert_eq!(
        current.source_ids.iter().collect::<BTreeSet<_>>().len(),
        current.source_ids.len()
    );
    assert_eq!(change.previous.as_ref(), Some(&previous));
    assert_eq!(current.edition.as_deref(), Some("fixture-updated"));
}
