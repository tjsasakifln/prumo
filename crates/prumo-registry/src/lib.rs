#![forbid(unsafe_code)]

//! Installed inventory, normative revisions, qualification and final-result gates.

pub use prumo_domain::registry::*;
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub fn bundled_catalog() -> Result<RegistryCatalog, RegistryError> {
    static CATALOG: OnceLock<Result<RegistryCatalog, String>> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            let catalog: RegistryCatalog =
                serde_json::from_str(include_str!("../data/catalogue.json"))
                    .map_err(|error| error.to_string())?;
            catalog
                .validate_integrity()
                .map_err(|error| error.to_string())?;
            Ok(catalog)
        })
        .clone()
        .map_err(RegistryError::InvalidMetadata)
}

pub fn audit_coverage(catalog: &RegistryCatalog) -> CoverageAudit {
    let expected: BTreeSet<_> = ["PAR", "MEP"]
        .into_iter()
        .flat_map(|prefix| (1..=36).map(move |number| format!("{prefix}-{number:02}")))
        .collect();
    let present: BTreeSet<_> = catalog
        .capabilities
        .iter()
        .map(|capability| capability.family.clone())
        .collect();
    let mut audit = CoverageAudit {
        expected_families: expected.len(),
        present_families: present.intersection(&expected).count(),
        capabilities: catalog.capabilities.len(),
        norms: catalog.norms.len(),
        errors: Vec::new(),
        pending: Vec::new(),
    };
    if let Err(error) = catalog.validate_integrity() {
        audit.errors.push(finding(
            "invalid_catalog",
            &catalog.version,
            &error.to_string(),
        ));
    }
    for family in expected.difference(&present) {
        audit.errors.push(finding(
            "missing_family",
            family,
            "Família obrigatória sem capacidade cadastrada.",
        ));
    }
    for family in present.difference(&expected) {
        audit.errors.push(finding(
            "unknown_family",
            family,
            "Família fora do inventário obrigatório; mantenha o vínculo explícito de cobertura.",
        ));
    }
    for capability in &catalog.capabilities {
        if capability.owner_role.trim().is_empty() {
            audit.errors.push(finding(
                "missing_owner",
                &capability.id,
                "Capacidade sem responsável funcional identificado.",
            ));
        }
        if capability.implementation_issues.is_empty()
            || capability
                .implementation_issues
                .iter()
                .any(|issue| !(1..=250).contains(issue))
        {
            audit.errors.push(finding(
                "missing_implementation_issue",
                &capability.id,
                "Capacidade sem issue de implementação válida.",
            ));
        }
        for id in &catalog.norm_dependency_ids(&capability.norm_ids) {
            if !catalog.norms.iter().any(|norm| &norm.id == id) {
                audit.errors.push(finding(
                    "unresolved_norm_reference",
                    &capability.id,
                    &format!("Referência normativa inexistente: {id}."),
                ));
            }
        }
        if capability.reference_version.is_none() {
            audit.pending.push(finding(
                "reference_version_unconfirmed",
                &capability.id,
                "Release do produto de referência ainda não identificada na fonte pública.",
            ));
        }
        if capability.state != CapabilityState::Released {
            audit.pending.push(finding(
                "capability_not_released",
                &capability.id,
                "A capacidade ainda não foi liberada para resultados finais.",
            ));
        }
    }
    for norm in &catalog.norms {
        if norm.edition.is_none() || norm.evidence_class == EvidenceClass::Unresolved {
            audit.pending.push(finding(
                "norm_edition_unresolved",
                &norm.id,
                "Edição, correções ou aplicabilidade exigem confirmação na fonte autorizada.",
            ));
        }
        if norm.access != NormAccess::LicensedTextConsulted {
            audit.pending.push(finding("normative_text_unconsulted", &norm.id, "Metadados públicos não substituem consulta autorizada ao texto e às referências cruzadas."));
        }
        if norm.resolution.as_ref().is_none_or(|resolution| {
            resolution.status != NormValidity::Applicable
                || !resolution.amendments_and_errata_reviewed
        }) {
            audit.pending.push(finding("normative_resolution_pending", &norm.id, "Vigência, aplicabilidade, emendas, erratas e referências cruzadas aguardam revisão documentada."));
        }
        if norm.interpretation_owner_role.trim().is_empty() {
            audit.errors.push(finding(
                "missing_interpretation_owner",
                &norm.id,
                "Norma sem atribuição de responsabilidade pela interpretação.",
            ));
        }
    }
    audit
}

fn finding(code: &str, subject: &str, message: &str) -> CoverageFinding {
    CoverageFinding {
        code: code.into(),
        subject: subject.into(),
        message: message.into(),
    }
}

fn deduplicate(values: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

/// Build a new local registry version without modifying any existing package.
pub fn register_reference(
    catalog: &RegistryCatalog,
    input: NormReferenceInput,
    new_version: String,
    revision: u64,
) -> Result<(RegistryCatalog, RegistryChange), RegistryError> {
    if new_version == catalog.version || revision == 0 {
        return Err(RegistryError::InvalidMetadata(
            "A alteração exige uma nova versão e revisão.".into(),
        ));
    }
    let before = NormativePackage::from_catalog(catalog)?;
    let mut current = input.into_record()?;
    let mut updated = catalog.clone();
    let previous = updated
        .norms
        .iter()
        .find(|norm| norm.id == current.id)
        .cloned();
    if let Some(previous) = &previous {
        current
            .sources
            .extend(previous.sources.iter().skip(1).cloned());
        current
            .domains
            .extend(previous.domains.iter().skip(1).cloned());
        current
            .source_ids
            .splice(0..0, previous.source_ids.iter().cloned());
        deduplicate(&mut current.sources);
        deduplicate(&mut current.domains);
        deduplicate(&mut current.source_ids);
    }
    if previous.as_ref() == Some(&current) {
        return Err(RegistryError::InvalidMetadata(
            "A referência não contém alteração.".into(),
        ));
    }
    if let Some(existing) = updated.norms.iter_mut().find(|norm| norm.id == current.id) {
        *existing = current.clone();
    } else {
        updated.norms.push(current.clone());
    }
    for capability in &mut updated.capabilities {
        if catalog
            .norm_dependency_ids(&capability.norm_ids)
            .contains(&current.id)
        {
            capability.state = CapabilityState::Planned;
            capability.qualification = None;
        }
    }
    updated.version = new_version;
    updated.validate_integrity()?;
    let after = NormativePackage::from_catalog(&updated)?;
    let impact = compare_packages(&before, &after, revision)?;
    let change = RegistryChange {
        revision,
        catalog_version: updated.version.clone(),
        previous,
        current,
        impact,
    };
    Ok((updated, change))
}

/// Qualification is an installed-registry maintenance operation, not an MCP tool.
/// It requires reviewed evidence; it never assigns an approval to a whole family.
pub fn transition_capability(
    catalog: &mut RegistryCatalog,
    capability_id: &str,
    target: CapabilityState,
    qualification: Option<QualificationEvidence>,
    new_version: String,
) -> Result<(), RegistryError> {
    let original = catalog
        .capabilities
        .iter()
        .find(|capability| capability.id == capability_id)
        .ok_or_else(|| RegistryError::CapabilityNotFound(capability_id.into()))?;
    let allowed = matches!(
        (original.state, target),
        (CapabilityState::Planned, CapabilityState::Experimental)
            | (CapabilityState::Experimental, CapabilityState::Verified)
            | (CapabilityState::Verified, CapabilityState::Released)
            | (
                CapabilityState::Experimental
                    | CapabilityState::Verified
                    | CapabilityState::Released,
                CapabilityState::Planned
            )
            | (
                CapabilityState::Verified | CapabilityState::Released,
                CapabilityState::Experimental
            )
    );
    if !allowed || new_version == catalog.version {
        return Err(RegistryError::InvalidTransition);
    }
    let mut candidate = catalog.clone();
    let record = candidate
        .capabilities
        .iter_mut()
        .find(|capability| capability.id == capability_id)
        .ok_or_else(|| RegistryError::CapabilityNotFound(capability_id.into()))?;
    record.state = target;
    record.qualification = if matches!(
        target,
        CapabilityState::Verified | CapabilityState::Released
    ) {
        qualification
    } else {
        None
    };
    candidate.version = new_version;
    candidate.validate_integrity()?;
    *catalog = candidate;
    Ok(())
}

/// Inputs are supplied by a calculation pipeline, never accepted as an approval
/// from the UI/MCP. The current product has no calculation evidence to supply.
#[derive(Clone, Debug, Default)]
pub struct FinalResultContext {
    pub project_revision: u64,
    pub result_revision: Option<u64>,
    pub result_package_hash: Option<String>,
    pub method: String,
    pub material: String,
    pub recalculation_required: bool,
    pub calculation_evidence: Option<EvidenceLink>,
}

pub fn evaluate_final_result(
    package: Option<&NormativePackage>,
    capability_id: &str,
    context: &FinalResultContext,
) -> GateReport {
    let mut report = GateReport {
        allowed: false,
        capability_id: capability_id.into(),
        reasons: Vec::new(),
    };
    if context
        .calculation_evidence
        .as_ref()
        .is_none_or(|evidence| evidence.validate().is_err())
    {
        report.reasons.push(reason(
            "result_missing",
            "Não existe resultado calculado e rastreável para esta capacidade.",
        ));
    }
    let Some(package) = package else {
        report.reasons.push(reason(
            "package_missing",
            "O projeto ainda não fixou seu pacote normativo.",
        ));
        return report;
    };
    if package.validate().is_err() {
        report.reasons.push(reason(
            "package_invalid",
            "A integridade do pacote normativo não foi confirmada.",
        ));
        return report;
    }
    if context.recalculation_required || context.result_revision != Some(context.project_revision) {
        report.reasons.push(reason(
            "result_stale",
            "É necessário calcular a revisão atual antes de uma conclusão final.",
        ));
    }
    if context.result_package_hash.as_deref() != Some(package.hash.as_str()) {
        report.reasons.push(reason(
            "result_package_mismatch",
            "O resultado não corresponde ao pacote normativo fixado.",
        ));
    }
    let Some(capability) = package
        .catalog
        .capabilities
        .iter()
        .find(|capability| capability.id == capability_id)
    else {
        report.reasons.push(reason(
            "capability_missing",
            "A capacidade solicitada não pertence ao pacote fixado.",
        ));
        return report;
    };
    if capability.state != CapabilityState::Released {
        report.reasons.push(reason(
            "capability_not_released",
            "A combinação de método, material e norma ainda não foi liberada.",
        ));
    }
    if capability.method != context.method || capability.material != context.material {
        report.reasons.push(reason(
            "domain_mismatch",
            "O resultado usa método ou material diferente do domínio qualificado.",
        ));
    }
    if package.catalog.validate_qualification(capability).is_err() {
        report.reasons.push(reason(
            "qualification_incomplete",
            "Faltam fontes autorizadas, domínio aprovado ou evidências para esta combinação.",
        ));
    }
    report.allowed = report.reasons.is_empty();
    report
}

pub fn reason(code: &str, message: &str) -> GateReason {
    GateReason {
        code: code.into(),
        message: message.into(),
    }
}
