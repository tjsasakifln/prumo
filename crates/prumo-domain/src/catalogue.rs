//! Project-local catalogue versions, explicit assignments and physical properties.
use crate::{DomainError, Project, validate_name};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

pub mod quantities;
pub mod sections;
pub use quantities::*;
pub use sections::{SectionAnalysis, SectionGeometry, SectionShape, analyse_section};

pub const CATALOGUE_KEY: &str = "prumo.catalogue.v1";
pub const ASSIGNMENTS_KEY: &str = "prumo.catalogue.assignments.v1";
pub const MAX_CATALOGUE_ENTRIES: usize = 10_000;
pub const MAX_ENTRY_VERSIONS: usize = 256;

pub(crate) fn invalid(message: &str) -> DomainError {
    DomainError::InvalidModel(message.into())
}
pub(crate) fn finite<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let number = serde_json::Number::deserialize(deserializer)?;
    number
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| serde::de::Error::custom("Valor não finito ou fora do intervalo f64."))
}
fn text(value: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() || value.len() > 16_384 || value.contains('\0') {
        Err(invalid(
            "Identificação ou origem de catálogo ausente/inválida.",
        ))
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSource {
    pub publisher: String,
    pub reference: String,
    pub edition: String,
    pub revision: String,
    pub data_license: String,
    /// Original supplied record, kept as text to preserve all decimal digits and units.
    pub original_record: Option<String>,
}
impl CatalogSource {
    fn validate(&self) -> Result<(), DomainError> {
        for value in [
            &self.publisher,
            &self.reference,
            &self.edition,
            &self.revision,
            &self.data_license,
        ] {
            text(value)?;
        }
        if self
            .original_record
            .as_ref()
            .is_some_and(|record| record.len() > 1_048_576)
        {
            return Err(invalid("Registro de origem excede 1 MiB."));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialState {
    pub name: String,
    pub conditions: Vec<ConditionRange>,
    pub elastic: ElasticLaw,
    pub density: PhysicalValue,
    pub properties: BTreeMap<String, PhysicalValue>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogMaterial {
    pub class: String,
    pub states: Vec<MaterialState>,
}
impl CatalogMaterial {
    pub fn validate(&self) -> Result<(), DomainError> {
        text(&self.class)?;
        if self.states.is_empty() || self.states.len() > 128 {
            return Err(invalid("Material exige de 1 a 128 estados de condição."));
        }
        let mut names = std::collections::BTreeSet::new();
        for state in &self.states {
            text(&state.name)?;
            if !names.insert(&state.name) {
                return Err(invalid("Estados de material com nome repetido."));
            }
            validate_conditions(&state.conditions)?;
            state.elastic.validate()?;
            state.density.require(PhysicalQuantity::Density)?;
            validate_properties(&state.properties)?;
        }
        Ok(())
    }

    pub fn state_for(
        &self,
        conditions: &BTreeMap<PhysicalQuantity, PhysicalValue>,
    ) -> Result<&MaterialState, DomainError> {
        self.validate()?;
        for (quantity, value) in conditions {
            value.require(*quantity)?;
        }
        let mut matches = Vec::new();
        for state in &self.states {
            let mut matches_all = true;
            for range in &state.conditions {
                let Some(value) = conditions.get(&range.quantity) else {
                    matches_all = false;
                    break;
                };
                if !range.contains(value)? {
                    matches_all = false;
                    break;
                }
            }
            if matches_all {
                matches.push(state);
            }
        }
        if matches.len() != 1 {
            return Err(invalid(
                "Condição fora do domínio ou ambígua; não há interpolação nem estado padrão silencioso.",
            ));
        }
        Ok(matches[0])
    }
}

fn validate_conditions(conditions: &[ConditionRange]) -> Result<(), DomainError> {
    if conditions.len() > 32 {
        return Err(invalid("Excesso de condições de aplicação."));
    }
    let mut quantities = std::collections::BTreeSet::new();
    for range in conditions {
        range.validate()?;
        if !quantities.insert(range.quantity) {
            return Err(invalid("Grandeza repetida nos intervalos de aplicação."));
        }
    }
    Ok(())
}
fn validate_properties(properties: &BTreeMap<String, PhysicalValue>) -> Result<(), DomainError> {
    if properties.len() > 256 {
        return Err(invalid("Excesso de propriedades de catálogo."));
    }
    for (name, value) in properties {
        text(name)?;
        value.si()?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogKind {
    Material,
    Section,
    Properties,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CatalogPayload {
    Material {
        material: CatalogMaterial,
    },
    Section {
        shape: SectionShape,
    },
    Properties {
        values: BTreeMap<String, PhysicalValue>,
    },
}
impl CatalogPayload {
    pub fn kind(&self) -> CatalogKind {
        match self {
            Self::Material { .. } => CatalogKind::Material,
            Self::Section { .. } => CatalogKind::Section,
            Self::Properties { .. } => CatalogKind::Properties,
        }
    }
    fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::Material { material } => material.validate(),
            Self::Section { shape } => shape.geometry().map(|_| ()),
            Self::Properties { values } => {
                if values.is_empty() {
                    return Err(invalid("Catálogo de propriedades vazio."));
                }
                validate_properties(values)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogCustomization {
    pub base_id: Uuid,
    pub base_revision: u64,
    pub base_sha256: String,
    pub reason: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    pub id: Uuid,
    pub revision: u64,
    pub name: String,
    pub source: CatalogSource,
    pub applicability: Vec<ConditionRange>,
    pub customization: Option<CatalogCustomization>,
    pub payload: CatalogPayload,
}
impl CatalogEntry {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.id.is_nil() || self.revision == 0 {
            return Err(invalid("Identidade ou revisão de catálogo inválida."));
        }
        validate_name(&self.name)?;
        self.source.validate()?;
        validate_conditions(&self.applicability)?;
        self.payload.validate()?;
        if let Some(custom) = &self.customization {
            if custom.base_id.is_nil() || custom.base_id == self.id || custom.base_revision == 0 {
                return Err(invalid(
                    "Customização deve ter nova identidade e base histórica explícita.",
                ));
            }
            if custom.base_sha256.len() != 64
                || !custom
                    .base_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(invalid(
                    "Identidade SHA-256 da origem de catálogo inválida.",
                ));
            }
            text(&custom.reason)?;
        }
        Ok(())
    }
    pub fn sha256(&self) -> Result<String, DomainError> {
        self.validate()?;
        crate::canonical::digest(self)
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, DomainError> {
        if bytes.len() > 1_048_576 {
            return Err(invalid("Importação de catálogo excede 1 MiB."));
        }
        crate::canonical::validate_json_representation(bytes)
            .map_err(|e| invalid(&e.to_string()))?;
        let mut entry: Self = serde_json::from_slice(bytes).map_err(|e| invalid(&e.to_string()))?;
        if entry.source.original_record.is_none() {
            entry.source.original_record =
                Some(String::from_utf8(bytes.to_vec()).map_err(|e| invalid(&e.to_string()))?);
        }
        entry.validate()?;
        Ok(entry)
    }
    pub fn material_state_for(
        &self,
        conditions: &BTreeMap<PhysicalQuantity, PhysicalValue>,
    ) -> Result<&MaterialState, DomainError> {
        self.validate()?;
        for range in &self.applicability {
            let value = conditions
                .get(&range.quantity)
                .ok_or_else(|| invalid("Condição de aplicação do catálogo não informada."))?;
            if !range.contains(value)? {
                return Err(invalid(
                    "Condição fora do intervalo de aplicação do catálogo.",
                ));
            }
        }
        match &self.payload {
            CatalogPayload::Material { material } => material.state_for(conditions),
            _ => Err(invalid("Entrada selecionada não é um material.")),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogRevision {
    pub entry: CatalogEntry,
    pub entry_sha256: String,
    pub project_revision: u64,
    pub actor: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectCatalogue {
    pub schema_version: u32,
    pub entries: BTreeMap<Uuid, Vec<CatalogRevision>>,
}
impl Default for ProjectCatalogue {
    fn default() -> Self {
        Self {
            schema_version: 1,
            entries: BTreeMap::new(),
        }
    }
}
impl ProjectCatalogue {
    pub fn latest(&self, id: Uuid) -> Result<&CatalogEntry, DomainError> {
        self.entries
            .get(&id)
            .and_then(|versions| versions.last())
            .map(|version| &version.entry)
            .ok_or(DomainError::UnknownReference(id))
    }
    pub fn validate(&self, project_revision: u64) -> Result<(), DomainError> {
        if self.schema_version != 1 || self.entries.len() > MAX_CATALOGUE_ENTRIES {
            return Err(invalid(
                "Schema ou quantidade de entradas de catálogo inválida.",
            ));
        }
        for (id, versions) in &self.entries {
            if versions.is_empty() || versions.len() > MAX_ENTRY_VERSIONS {
                return Err(invalid("Histórico de catálogo vazio ou excedido."));
            }
            let mut previous_project_revision = 0;
            let kind = versions[0].entry.payload.kind();
            for (index, version) in versions.iter().enumerate() {
                version.entry.validate()?;
                if version.entry.id != *id
                    || version.entry.revision != index as u64 + 1
                    || version.entry.payload.kind() != kind
                    || version.entry.sha256()? != version.entry_sha256
                    || version.project_revision < previous_project_revision
                    || version.project_revision == 0
                    || version.project_revision > project_revision
                {
                    return Err(invalid(
                        "Histórico de catálogo perdeu identidade, ordem ou integridade.",
                    ));
                }
                validate_name(&version.actor)?;
                previous_project_revision = version.project_revision;
                if let Some(custom) = &version.entry.customization {
                    let base = self
                        .entries
                        .get(&custom.base_id)
                        .and_then(|items| {
                            items
                                .iter()
                                .find(|item| item.entry.revision == custom.base_revision)
                        })
                        .ok_or_else(|| invalid("Origem histórica da customização ausente."))?;
                    if base.entry_sha256 != custom.base_sha256
                        || base.project_revision > version.project_revision
                        || base.entry.payload.kind() != kind
                    {
                        return Err(invalid(
                            "Customização não corresponde à versão da origem declarada.",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn project_catalogue(project: &Project) -> Result<ProjectCatalogue, DomainError> {
    match project.extensions.get(CATALOGUE_KEY) {
        None => Ok(ProjectCatalogue::default()),
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| invalid(&e.to_string())),
    }
}
pub fn put_entry(
    project: &mut Project,
    entry: CatalogEntry,
    actor: &str,
    revision: u64,
) -> Result<(), DomainError> {
    entry.validate()?;
    validate_name(actor)?;
    let mut catalogue = project_catalogue(project)?;
    let versions = catalogue.entries.entry(entry.id).or_default();
    if entry.revision != versions.len() as u64 + 1 {
        return Err(invalid(
            "Revisão de catálogo obsoleta ou fora de sequência.",
        ));
    }
    versions.push(CatalogRevision {
        entry_sha256: entry.sha256()?,
        entry,
        project_revision: revision,
        actor: actor.into(),
    });
    catalogue.validate(revision)?;
    project.extensions.insert(
        CATALOGUE_KEY.into(),
        serde_json::to_value(catalogue).map_err(|e| invalid(&e.to_string()))?,
    );
    Ok(())
}
pub fn assignments(
    object: &crate::PhysicalObject,
) -> Result<BTreeMap<CatalogKind, Uuid>, DomainError> {
    match object.extensions.get(ASSIGNMENTS_KEY) {
        None => Ok(BTreeMap::new()),
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| invalid(&e.to_string())),
    }
}
pub fn assign_entry(
    project: &mut Project,
    object_id: Uuid,
    entry_id: Uuid,
) -> Result<(), DomainError> {
    let catalogue = project_catalogue(project)?;
    let kind = catalogue.latest(entry_id)?.payload.kind();
    let object = project
        .objects
        .iter_mut()
        .find(|object| object.id == object_id)
        .ok_or(DomainError::UnknownReference(object_id))?;
    if kind == CatalogKind::Material && object.material_id.is_some() {
        return Err(invalid(
            "A peça já usa material do modelo. Remova essa atribuição explicitamente antes de escolher material do catálogo.",
        ));
    }
    let mut references = assignments(object)?;
    references.insert(kind, entry_id);
    object.extensions.insert(
        ASSIGNMENTS_KEY.into(),
        serde_json::to_value(references).map_err(|e| invalid(&e.to_string()))?,
    );
    Ok(())
}
pub fn clear_assignment(
    project: &mut Project,
    object_id: Uuid,
    kind: CatalogKind,
) -> Result<(), DomainError> {
    let object = project
        .objects
        .iter_mut()
        .find(|object| object.id == object_id)
        .ok_or(DomainError::UnknownReference(object_id))?;
    let mut references = assignments(object)?;
    if references.remove(&kind).is_none() {
        return Err(invalid("A peça não possui essa atribuição de catálogo."));
    }
    if references.is_empty() {
        object.extensions.remove(ASSIGNMENTS_KEY);
    } else {
        object.extensions.insert(
            ASSIGNMENTS_KEY.into(),
            serde_json::to_value(references).map_err(|e| invalid(&e.to_string()))?,
        );
    }
    Ok(())
}
pub fn validate_catalogue(
    project: &Project,
    revision: u64,
    ids: &mut std::collections::BTreeSet<Uuid>,
) -> Result<(), DomainError> {
    let catalogue = project_catalogue(project)?;
    catalogue.validate(revision)?;
    for id in catalogue.entries.keys() {
        crate::insert_id(ids, *id)?;
    }
    for object in &project.objects {
        for (kind, id) in assignments(object)? {
            if kind == CatalogKind::Material && object.material_id.is_some() {
                return Err(invalid("A peça não pode manter dois materiais efetivos."));
            }
            if catalogue.latest(id)?.payload.kind() != kind {
                return Err(invalid(
                    "Tipo de catálogo não corresponde à atribuição no objeto.",
                ));
            }
        }
    }
    Ok(())
}

pub fn object_catalogue_sources(
    project: &Project,
    object_id: Uuid,
) -> Result<Vec<crate::dependencies::SourceKey>, DomainError> {
    let object = project
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .ok_or(DomainError::UnknownReference(object_id))?;
    let mut result = vec![crate::dependencies::SourceKey {
        kind: crate::dependencies::SourceKind::Physical,
        id: object_id,
    }];
    result.extend(
        assignments(object)?
            .values()
            .map(|id| crate::dependencies::SourceKey {
                kind: crate::dependencies::SourceKind::CatalogEntry,
                id: *id,
            }),
    );
    Ok(result)
}
