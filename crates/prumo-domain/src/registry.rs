//! Versioned normative metadata and qualification contracts.
//!
//! Metadata records do not contain protected normative texts. Qualification
//! evidence identifies the exact method/material/document combination.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Catalog,
    Institutional,
    Manufacturer,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NormAccess {
    PublicMetadataOnly,
    LicensedTextConsulted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LicensedEvidence {
    pub authorization_reference: String,
    pub consulted_by: String,
    pub consulted_on: String,
    pub document_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NormValidity {
    Applicable,
    Superseded,
    Suspended,
    Unresolved,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormResolution {
    pub status: NormValidity,
    pub verified_on: String,
    pub verified_by: String,
    pub catalog_evidence: EvidenceLink,
    pub applicability_evidence: EvidenceLink,
    pub cross_reference_review: EvidenceLink,
    pub cross_reference_norm_ids: Vec<String>,
    pub approved_domains: Vec<String>,
    pub amendments_and_errata_reviewed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormRecord {
    pub id: String,
    pub designation: String,
    pub part: Option<String>,
    pub edition: Option<String>,
    pub amendments: Vec<String>,
    pub errata: Vec<String>,
    pub title: String,
    pub publisher: String,
    pub source_ids: Vec<String>,
    pub sources: Vec<String>,
    pub evidence_class: EvidenceClass,
    pub access: NormAccess,
    pub domains: Vec<String>,
    pub interpretation_owner_role: String,
    pub licensed_evidence: Option<LicensedEvidence>,
    #[serde(default)]
    pub resolution: Option<NormResolution>,
}

impl NormRecord {
    pub fn fingerprint(&self) -> Result<String, RegistryError> {
        canonical_hash(self)
    }

    pub fn validate(&self) -> Result<(), RegistryError> {
        require_identifier(&self.id)?;
        for value in [&self.designation, &self.title, &self.publisher] {
            require_text(value, 1000)?;
        }
        for value in [&self.part, &self.edition].into_iter().flatten() {
            require_text(value, 160)?;
        }
        for value in self
            .amendments
            .iter()
            .chain(&self.errata)
            .chain(&self.source_ids)
            .chain(&self.domains)
        {
            require_text(value, 1000)?;
        }
        for source in &self.sources {
            require_url(source)?;
        }
        match (self.access, &self.licensed_evidence) {
            (NormAccess::PublicMetadataOnly, None) => {}
            (NormAccess::LicensedTextConsulted, Some(evidence)) => {
                for value in [
                    &evidence.authorization_reference,
                    &evidence.consulted_by,
                    &evidence.consulted_on,
                ] {
                    require_text(value, 1000)?;
                }
                require_hash(&evidence.document_sha256)?;
                if self.edition.is_none() || self.evidence_class == EvidenceClass::Unresolved {
                    return Err(RegistryError::UnresolvedNorm(self.id.clone()));
                }
            }
            _ => {
                return Err(RegistryError::InvalidMetadata(
                    "Acesso e evidência de consulta licenciada são incoerentes.".into(),
                ));
            }
        }
        if let Some(resolution) = &self.resolution {
            require_text(&resolution.verified_by, 1000)?;
            require_date(&resolution.verified_on)?;
            resolution.catalog_evidence.validate()?;
            resolution.applicability_evidence.validate()?;
            resolution.cross_reference_review.validate()?;
            for domain in &resolution.approved_domains {
                require_text(domain, 160)?;
            }
            for id in &resolution.cross_reference_norm_ids {
                require_identifier(id)?;
            }
        }
        Ok(())
    }
}

/// User-authored references deliberately have no approval or license fields.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormReferenceInput {
    pub id: String,
    pub designation: String,
    pub part: Option<String>,
    pub edition: Option<String>,
    pub amendments: Vec<String>,
    pub errata: Vec<String>,
    pub title: String,
    pub publisher: String,
    pub source_url: String,
    pub domain: String,
    pub interpretation_owner_role: String,
}

impl From<&NormRecord> for NormReferenceInput {
    fn from(norm: &NormRecord) -> Self {
        Self {
            id: norm.id.clone(),
            designation: norm.designation.clone(),
            part: norm.part.clone(),
            edition: norm.edition.clone(),
            amendments: norm.amendments.clone(),
            errata: norm.errata.clone(),
            title: norm.title.clone(),
            publisher: norm.publisher.clone(),
            source_url: norm.sources.first().cloned().unwrap_or_default(),
            domain: norm.domains.first().cloned().unwrap_or_default(),
            interpretation_owner_role: norm.interpretation_owner_role.clone(),
        }
    }
}

impl NormReferenceInput {
    pub fn into_record(self) -> Result<NormRecord, RegistryError> {
        require_url(&self.source_url)?;
        require_text(&self.domain, 160)?;
        require_text(&self.interpretation_owner_role, 1000)?;
        let record = NormRecord {
            source_ids: vec![format!("project:{}", self.id)],
            id: self.id,
            designation: self.designation,
            part: self.part,
            edition: self.edition,
            amendments: self.amendments,
            errata: self.errata,
            title: self.title,
            publisher: self.publisher,
            sources: vec![self.source_url],
            evidence_class: EvidenceClass::Unresolved,
            access: NormAccess::PublicMetadataOnly,
            domains: vec![self.domain],
            interpretation_owner_role: self.interpretation_owner_role,
            licensed_evidence: None,
            resolution: None,
        };
        record.validate()?;
        Ok(record)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityOperation {
    Model,
    Calculate,
    Verify,
    Emit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Planned,
    Experimental,
    Verified,
    Released,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceLink {
    pub reference: String,
    pub sha256: String,
}

impl EvidenceLink {
    pub fn validate(&self) -> Result<(), RegistryError> {
        require_text(&self.reference, 1000)?;
        require_hash(&self.sha256)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DomainApproval {
    pub method: String,
    pub material: String,
    pub norm_fingerprints: BTreeMap<String, String>,
    pub domain_description: String,
    pub reviewer: String,
    pub professional_registration: String,
    pub approved_on: String,
    pub approval_reference: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseEvidence {
    pub version: String,
    pub artifact_sha256: String,
    pub approval_reference: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationEvidence {
    pub implementation_revision: String,
    pub validation_report: EvidenceLink,
    pub independent_reference: EvidenceLink,
    pub domain_approval: DomainApproval,
    pub release: Option<ReleaseEvidence>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRecord {
    pub id: String,
    pub family: String,
    pub title: String,
    pub product: String,
    pub module: String,
    pub reference_version: Option<String>,
    pub version_note: String,
    pub source_ids: Vec<String>,
    pub sources: Vec<String>,
    pub owner_role: String,
    pub implementation_issues: Vec<u16>,
    pub operation: CapabilityOperation,
    pub discipline: String,
    pub method: String,
    pub material: String,
    pub norm_ids: Vec<String>,
    pub state: CapabilityState,
    pub qualification: Option<QualificationEvidence>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryCatalog {
    pub schema_version: u32,
    pub version: String,
    pub norms: Vec<NormRecord>,
    pub capabilities: Vec<CapabilityRecord>,
}

impl RegistryCatalog {
    /// Includes referenced documents transitively. A visited set handles cycles
    /// without dropping dependencies or recursing indefinitely.
    pub fn norm_dependency_ids(&self, roots: &[String]) -> BTreeSet<String> {
        let mut pending = roots.to_vec();
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id.clone()) {
                continue;
            }
            if let Some(resolution) = self
                .norms
                .iter()
                .find(|norm| norm.id == id)
                .and_then(|norm| norm.resolution.as_ref())
            {
                pending.extend(resolution.cross_reference_norm_ids.iter().cloned());
            }
        }
        visited
    }

    pub fn validate_integrity(&self) -> Result<(), RegistryError> {
        if self.schema_version != REGISTRY_SCHEMA_VERSION {
            return Err(RegistryError::UnsupportedSchema(self.schema_version));
        }
        require_text(&self.version, 160)?;
        if self.norms.len() > 4096 || self.capabilities.len() > 16384 {
            return Err(RegistryError::InvalidMetadata(
                "Catálogo excede os limites de registros.".into(),
            ));
        }
        let mut norm_ids = BTreeSet::new();
        for norm in &self.norms {
            norm.validate()?;
            if !norm_ids.insert(&norm.id) {
                return Err(RegistryError::DuplicateId(norm.id.clone()));
            }
        }
        let mut capability_ids = BTreeSet::new();
        for capability in &self.capabilities {
            require_identifier(&capability.id)?;
            if !capability_ids.insert(&capability.id) {
                return Err(RegistryError::DuplicateId(capability.id.clone()));
            }
            for value in [
                &capability.family,
                &capability.title,
                &capability.product,
                &capability.module,
                &capability.discipline,
                &capability.method,
                &capability.material,
            ] {
                require_text(value, 1000)?;
            }
            for source in &capability.sources {
                require_url(source)?;
            }
            if matches!(
                capability.state,
                CapabilityState::Verified | CapabilityState::Released
            ) {
                self.validate_qualification(capability)?;
            }
        }
        Ok(())
    }

    pub fn norm_fingerprints(
        &self,
        capability: &CapabilityRecord,
    ) -> Result<BTreeMap<String, String>, RegistryError> {
        self.norm_dependency_ids(&capability.norm_ids)
            .into_iter()
            .map(|id| {
                let norm = self
                    .norms
                    .iter()
                    .find(|norm| norm.id == id)
                    .ok_or_else(|| RegistryError::UnresolvedNorm(id.clone()))?;
                Ok((id.clone(), norm.fingerprint()?))
            })
            .collect()
    }

    pub fn validate_qualification(
        &self,
        capability: &CapabilityRecord,
    ) -> Result<(), RegistryError> {
        if ["pending", "unspecified", "planned"].contains(&capability.method.as_str())
            || ["pending", "unspecified", "planned"].contains(&capability.material.as_str())
        {
            return Err(RegistryError::QualificationRequired(
                "Método e material precisam estar delimitados.".into(),
            ));
        }
        require_text(&capability.owner_role, 1000)?;
        if capability.implementation_issues.is_empty()
            || capability
                .implementation_issues
                .iter()
                .any(|issue| !(1..=250).contains(issue))
        {
            return Err(RegistryError::QualificationRequired(
                "Capacidade sem issue de implementação válida.".into(),
            ));
        }
        if capability.norm_ids.is_empty() && capability.operation != CapabilityOperation::Model {
            return Err(RegistryError::QualificationRequired("Uma capacidade de cálculo, verificação ou emissão precisa de referência técnica resolvida.".into()));
        }
        for id in &self.norm_dependency_ids(&capability.norm_ids) {
            let norm = self
                .norms
                .iter()
                .find(|norm| &norm.id == id)
                .ok_or_else(|| RegistryError::UnresolvedNorm(id.clone()))?;
            norm.validate()?;
            if norm.edition.is_none() || norm.evidence_class == EvidenceClass::Unresolved {
                return Err(RegistryError::UnresolvedNorm(id.clone()));
            }
            if norm.access != NormAccess::LicensedTextConsulted || norm.licensed_evidence.is_none()
            {
                return Err(RegistryError::LicensedSourceRequired(id.clone()));
            }
            let resolution = norm.resolution.as_ref().ok_or_else(|| RegistryError::QualificationRequired(format!("Vigência, aplicabilidade e referências cruzadas ainda não foram resolvidas: {id}.")))?;
            if resolution.status != NormValidity::Applicable
                || !resolution.amendments_and_errata_reviewed
                || !resolution.approved_domains.contains(&capability.discipline)
                || !norm.domains.contains(&capability.discipline)
            {
                return Err(RegistryError::QualificationRequired(format!(
                    "A resolução normativa não aprova este domínio: {id}."
                )));
            }
            for reference_id in &resolution.cross_reference_norm_ids {
                let reference = self
                    .norms
                    .iter()
                    .find(|item| &item.id == reference_id)
                    .ok_or_else(|| RegistryError::UnresolvedNorm(reference_id.clone()))?;
                if reference.edition.is_none()
                    || reference.access != NormAccess::LicensedTextConsulted
                    || reference
                        .resolution
                        .as_ref()
                        .is_none_or(|item| item.status != NormValidity::Applicable)
                {
                    return Err(RegistryError::UnresolvedNorm(reference_id.clone()));
                }
            }
            require_text(&norm.interpretation_owner_role, 1000)?;
        }
        let qualification = capability.qualification.as_ref().ok_or_else(|| {
            RegistryError::QualificationRequired("Falta evidência de qualificação.".into())
        })?;
        require_text(&qualification.implementation_revision, 160)?;
        qualification.validation_report.validate()?;
        qualification.independent_reference.validate()?;
        if qualification.validation_report.sha256 == qualification.independent_reference.sha256 {
            return Err(RegistryError::QualificationRequired(
                "A referência independente não pode ser o próprio relatório de validação.".into(),
            ));
        }
        let approval = &qualification.domain_approval;
        if approval.method != capability.method
            || approval.material != capability.material
            || approval.norm_fingerprints != self.norm_fingerprints(capability)?
        {
            return Err(RegistryError::QualificationRequired("A aprovação não corresponde à combinação exata de método, material e versões normativas.".into()));
        }
        for value in [
            &approval.domain_description,
            &approval.reviewer,
            &approval.professional_registration,
            &approval.approved_on,
            &approval.approval_reference,
        ] {
            require_text(value, 1000)?;
        }
        if capability.state == CapabilityState::Released {
            let release = qualification.release.as_ref().ok_or_else(|| {
                RegistryError::QualificationRequired(
                    "Falta evidência de liberação do artefato.".into(),
                )
            })?;
            require_text(&release.version, 160)?;
            require_text(&release.approval_reference, 1000)?;
            require_hash(&release.artifact_sha256)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormativePackage {
    pub schema_version: u32,
    pub hash: String,
    pub catalog: RegistryCatalog,
}

impl NormativePackage {
    pub fn from_catalog(catalog: &RegistryCatalog) -> Result<Self, RegistryError> {
        catalog.validate_integrity()?;
        let mut canonical = catalog.clone();
        canonical.norms.sort_by(|a, b| a.id.cmp(&b.id));
        canonical.capabilities.sort_by(|a, b| a.id.cmp(&b.id));
        let hash = canonical_hash(&canonical)?;
        Ok(Self {
            schema_version: REGISTRY_SCHEMA_VERSION,
            hash,
            catalog: canonical,
        })
    }

    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.schema_version != REGISTRY_SCHEMA_VERSION {
            return Err(RegistryError::UnsupportedSchema(self.schema_version));
        }
        require_hash(&self.hash)?;
        let canonical = Self::from_catalog(&self.catalog)?;
        if canonical.hash != self.hash || canonical.catalog != self.catalog {
            return Err(RegistryError::HashMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationImpact {
    pub revision: u64,
    pub from_hash: String,
    pub to_hash: String,
    pub changed_norm_ids: Vec<String>,
    pub affected_capability_ids: Vec<String>,
    pub recalculation_required: bool,
}

pub fn compare_packages(
    from: &NormativePackage,
    to: &NormativePackage,
    revision: u64,
) -> Result<MigrationImpact, RegistryError> {
    from.validate()?;
    to.validate()?;
    Ok(compare_package_content(from, to, revision))
}

// Callers in this module validate both immutable packages in the same call.
// No validation result is cached between operations or mutable catalog edits.
fn compare_package_content(
    from: &NormativePackage,
    to: &NormativePackage,
    revision: u64,
) -> MigrationImpact {
    let mut changed_norms = BTreeSet::new();
    for norm in from.catalog.norms.iter().chain(&to.catalog.norms) {
        let left = from.catalog.norms.iter().find(|item| item.id == norm.id);
        let right = to.catalog.norms.iter().find(|item| item.id == norm.id);
        if left != right {
            changed_norms.insert(norm.id.clone());
        }
    }
    let mut affected = BTreeSet::new();
    for capability in from
        .catalog
        .capabilities
        .iter()
        .chain(&to.catalog.capabilities)
    {
        let left = from
            .catalog
            .capabilities
            .iter()
            .find(|item| item.id == capability.id);
        let right = to
            .catalog
            .capabilities
            .iter()
            .find(|item| item.id == capability.id);
        let left_dependencies = left
            .map(|record| from.catalog.norm_dependency_ids(&record.norm_ids))
            .unwrap_or_default();
        let right_dependencies = right
            .map(|record| to.catalog.norm_dependency_ids(&record.norm_ids))
            .unwrap_or_default();
        if left != right
            || left_dependencies
                .iter()
                .chain(&right_dependencies)
                .any(|id| changed_norms.contains(id))
        {
            affected.insert(capability.id.clone());
        }
    }
    MigrationImpact {
        revision,
        from_hash: from.hash.clone(),
        to_hash: to.hash.clone(),
        changed_norm_ids: changed_norms.into_iter().collect(),
        affected_capability_ids: affected.into_iter().collect(),
        recalculation_required: true,
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryChange {
    pub revision: u64,
    pub catalog_version: String,
    pub previous: Option<NormRecord>,
    pub current: NormRecord,
    pub impact: MigrationImpact,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NormativePackagePreview {
    pub project_revision: Option<u64>,
    pub active_hash: Option<String>,
    pub candidate: NormativePackage,
    pub impact: Option<MigrationImpact>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormativeState {
    pub catalog: Option<RegistryCatalog>,
    pub active: Option<NormativePackage>,
    pub previous: Vec<NormativePackage>,
    pub migrations: Vec<MigrationImpact>,
    pub registry_changes: Vec<RegistryChange>,
    pub recalculation_required: bool,
}

impl NormativeState {
    pub fn validate(&self, project_revision: u64) -> Result<(), RegistryError> {
        if let Some(catalog) = &self.catalog {
            catalog.validate_integrity()?;
        }
        if self.previous.len() != self.migrations.len()
            || (self.active.is_none() && !self.previous.is_empty())
        {
            return Err(RegistryError::InvalidHistory);
        }
        for package in self.active.iter().chain(&self.previous) {
            package.validate()?;
        }
        for (index, old) in self.previous.iter().enumerate() {
            let next = self
                .previous
                .get(index + 1)
                .or(self.active.as_ref())
                .ok_or(RegistryError::InvalidHistory)?;
            let migration = &self.migrations[index];
            if migration.revision == 0
                || migration.revision > project_revision
                || migration.from_hash == migration.to_hash
                || (index > 0 && migration.revision <= self.migrations[index - 1].revision)
                || compare_package_content(old, next, migration.revision) != *migration
            {
                return Err(RegistryError::InvalidHistory);
            }
        }
        if !self.migrations.is_empty() && !self.recalculation_required {
            return Err(RegistryError::InvalidHistory);
        }
        for (index, change) in self.registry_changes.iter().enumerate() {
            change.current.validate()?;
            if let Some(previous) = &change.previous {
                previous.validate()?;
                if previous.id != change.current.id {
                    return Err(RegistryError::InvalidHistory);
                }
            }
            require_hash(&change.impact.from_hash)?;
            require_hash(&change.impact.to_hash)?;
            if self.catalog.is_none()
                || change.revision == 0
                || change.revision > project_revision
                || change.impact.revision != change.revision
                || change.impact.from_hash == change.impact.to_hash
                || (index > 0 && change.revision <= self.registry_changes[index - 1].revision)
            {
                return Err(RegistryError::InvalidHistory);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageFinding {
    pub code: String,
    pub subject: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoverageAudit {
    pub expected_families: usize,
    pub present_families: usize,
    pub capabilities: usize,
    pub norms: usize,
    pub errors: Vec<CoverageFinding>,
    pub pending: Vec<CoverageFinding>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GateReason {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GateReport {
    pub allowed: bool,
    pub capability_id: String,
    pub reasons: Vec<GateReason>,
}

pub fn canonical_hash(value: &impl Serialize) -> Result<String, RegistryError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| RegistryError::InvalidMetadata(error.to_string()))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub fn require_hash(value: &str) -> Result<(), RegistryError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RegistryError::InvalidMetadata(
            "Identificador SHA-256 inválido.".into(),
        ));
    }
    Ok(())
}

fn require_identifier(value: &str) -> Result<(), RegistryError> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
    {
        return Err(RegistryError::InvalidMetadata(
            "Identificador de registro inválido.".into(),
        ));
    }
    Ok(())
}

fn require_text(value: &str, maximum: usize) -> Result<(), RegistryError> {
    if value.trim().is_empty()
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        return Err(RegistryError::InvalidMetadata(
            "Campo obrigatório ausente ou inválido no registro.".into(),
        ));
    }
    Ok(())
}

fn require_url(value: &str) -> Result<(), RegistryError> {
    require_text(value, 2048)?;
    if !(value.starts_with("https://") || value.starts_with("http://"))
        || value.chars().any(char::is_whitespace)
    {
        return Err(RegistryError::InvalidMetadata(
            "A referência pública deve usar URL HTTP ou HTTPS.".into(),
        ));
    }
    Ok(())
}

fn require_date(value: &str) -> Result<(), RegistryError> {
    let parts: Vec<_> = value.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(RegistryError::InvalidMetadata(
            "Informe a data de verificação como AAAA-MM-DD.".into(),
        ));
    }
    let year: u32 = parts[0]
        .parse()
        .map_err(|_| RegistryError::InvalidMetadata("Ano inválido.".into()))?;
    let month: usize = parts[1]
        .parse()
        .map_err(|_| RegistryError::InvalidMetadata("Mês inválido.".into()))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| RegistryError::InvalidMetadata("Dia inválido.".into()))?;
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if year < 1900 || !(1..=12).contains(&month) || day == 0 || day > days[month - 1] {
        return Err(RegistryError::InvalidMetadata(
            "Data de verificação inválida.".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Metadados inválidos: {0}")]
    InvalidMetadata(String),
    #[error("Versão {0} do registro normativo não suportada.")]
    UnsupportedSchema(u32),
    #[error("Identificador repetido: {0}.")]
    DuplicateId(String),
    #[error("Norma não resolvida: {0}.")]
    UnresolvedNorm(String),
    #[error("Falta evidência de consulta autorizada ao texto normativo: {0}.")]
    LicensedSourceRequired(String),
    #[error("Qualificação insuficiente: {0}")]
    QualificationRequired(String),
    #[error("O conteúdo do pacote normativo não corresponde ao hash registrado.")]
    HashMismatch,
    #[error("O histórico de pacotes e migrações normativas é inválido.")]
    InvalidHistory,
    #[error("A mudança de estado solicitada não é permitida.")]
    InvalidTransition,
    #[error("Capacidade não encontrada: {0}.")]
    CapabilityNotFound(String),
}
