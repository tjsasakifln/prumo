//! Project-authorized geometry and reproducible verification tools.
use super::{decode, definition};
use prumo_app::{
    AppError, ProjectService,
    access::{AccessScope, ProjectGrant},
};
use prumo_domain::catalogue::{PhysicalQuantity, PhysicalValue};
use prumo_domain::geometry::{FaceInput, GeometryFilter, GeometryRepair};
use rmcp::{ErrorData, model::Tool};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditArgs {
    #[serde(default)]
    filter: GeometryFilter,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairArgs {
    repairs: Vec<GeometryRepair>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FaceArgs {
    face: FaceInput,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryArgs {
    entry_id: Uuid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MaterialArgs {
    entry_id: Uuid,
    conditions: BTreeMap<PhysicalQuantity, PhysicalValue>,
}

pub(super) enum Operation {
    Audit(GeometryFilter),
    Preview(Vec<GeometryRepair>),
    Face(FaceInput),
    Protocols,
    Verify,
    Catalogue,
    Section(Uuid),
    Material(Uuid, BTreeMap<PhysicalQuantity, PhysicalValue>),
}
impl Operation {
    pub(super) fn parse(name: &str, args: Value) -> Option<Result<Self, ErrorData>> {
        Some(match name {
            "prumo.geometry.audit" => decode::<AuditArgs>(args).map(|a| Self::Audit(a.filter)),
            "prumo.geometry.preview" => {
                decode::<RepairArgs>(args).map(|a| Self::Preview(a.repairs))
            }
            "prumo.geometry.face" => decode::<FaceArgs>(args).map(|a| Self::Face(a.face)),
            "prumo.verification.protocols" => decode::<Empty>(args).map(|_| Self::Protocols),
            "prumo.verification.run" => decode::<Empty>(args).map(|_| Self::Verify),
            "prumo.catalogue.read" => decode::<Empty>(args).map(|_| Self::Catalogue),
            "prumo.catalogue.section" => {
                decode::<EntryArgs>(args).map(|a| Self::Section(a.entry_id))
            }
            "prumo.catalogue.material" => {
                decode::<MaterialArgs>(args).map(|a| Self::Material(a.entry_id, a.conditions))
            }
            _ => return None,
        })
    }
    pub(super) fn required_scope(&self) -> AccessScope {
        match self {
            Self::Preview(_) => AccessScope::Model,
            Self::Verify | Self::Section(_) => AccessScope::Calculation,
            _ => AccessScope::Read,
        }
    }
    pub(super) fn needs_all_disciplines(&self) -> bool {
        true
    }
    pub(super) fn run(
        self,
        service: &ProjectService,
        cancel: &AtomicBool,
    ) -> Result<Value, AppError> {
        if cancel.load(Ordering::Acquire) {
            return Err(AppError::InvalidInput(
                "Operação cancelada antes da execução.".into(),
            ));
        }
        Ok(match self {
            Self::Audit(filter) => serde_json::to_value(service.geometry_report(&filter)?)?,
            Self::Preview(repairs) => {
                serde_json::to_value(service.geometry_repair_preview(&repairs)?)?
            }
            Self::Face(face) => serde_json::to_value(service.inspect_geometry_face(&face)?)?,
            Self::Protocols => serde_json::to_value(service.verification_protocols()?)?,
            Self::Verify => serde_json::to_value(service.run_verification_suite()?)?,
            Self::Catalogue => serde_json::to_value(service.catalogue_view()?)?,
            Self::Section(id) => serde_json::to_value(service.catalogue_section(id)?)?,
            Self::Material(id, conditions) => {
                serde_json::to_value(service.catalogue_material(id, &conditions)?)?
            }
        })
    }
}
pub(super) fn tool_catalog(grant: &ProjectGrant) -> Vec<Tool> {
    if !grant.allows(AccessScope::Read) || !grant.all_disciplines() {
        return vec![];
    }
    let id = json!({"type":"string","format":"uuid"});
    let nullable_id = json!({"oneOf":[{"type":"null"},id]});
    let filter = json!({"type":"object","additionalProperties":false,"properties":{
        "discipline":{"oneOf":[{"type":"null"},{"type":"string","enum":["structure","water_sanitation","gas","fire_protection","electrical","photovoltaic","lightning_protection","cabling","hvac"]}]},
        "environment_id":nullable_id,"network_id":nullable_id
    }});
    let point = json!({"type":"array","minItems":3,"maxItems":3,"items":{"type":"number"}});
    let face = json!({"type":"object","additionalProperties":false,"required":["vertices_m","expected_normal"],"properties":{
        "vertices_m":{"type":"array","minItems":3,"maxItems":512,"items":point},"expected_normal":point
    }});
    let mut result = vec![
        definition(
            "prumo.geometry.audit",
            "Inspeciona duplicidades, vãos curtos, contatos e cruzamentos da seleção, mantendo conexões explícitas. Retorna revisão e tolerâncias em metros.",
            json!({"filter":filter}),
            &[],
            true,
        ),
        definition(
            "prumo.geometry.face",
            "Confere orientação, degenerescência, planicidade e área de uma face informada em metros. Não insere a face no modelo.",
            json!({"face":face}),
            &["face"],
            true,
        ),
        definition(
            "prumo.verification.protocols",
            "Consulta casos, referências independentes e tolerâncias registradas antes de executar a bateria. Não libera cálculo de projeto.",
            json!({}),
            &[],
            true,
        ),
        definition(
            "prumo.catalogue.read",
            "Consulta entradas, versões, origens, unidades e customizações do catálogo local, com revisão e SHA do projeto.",
            json!({}),
            &[],
            true,
        ),
        definition(
            "prumo.catalogue.material",
            "Seleciona um estado de material nas condições explícitas e dentro da aplicação da fonte. Rejeita condições ambíguas; mantém valores nominais, característicos e de projeto distintos.",
            json!({"entry_id":id,"conditions":super::catalogue_schema::conditions_values()}),
            &["entry_id", "conditions"],
            true,
        ),
    ];
    if grant.allows(AccessScope::Model) {
        result.push(definition("prumo.geometry.preview","Prepara correções explícitas e mostra alteração de dimensões e conexões. Para aplicar use model.edit apply_geometry_repair com o mesmo hash, na mesma revisão e ator.",json!({"repairs":super::model_schema::repairs_schema()}),&["repairs"],true));
    }
    if grant.allows(AccessScope::Calculation) {
        result.push(definition("prumo.verification.run","Executa casos analíticos de viga, treliça e pórtico e rejeição de mecanismo. Retorna evidência SHA, unidade, plataforma, build, dependências e estado. Não calcula o projeto nem libera engenharia.",json!({}),&[],true));
    }
    if grant.allows(AccessScope::Calculation) {
        result.push(definition("prumo.catalogue.section","Calcula área, centroide, inércias e torção de Saint-Venant para seção homogênea isotrópica na versão atual do catálogo. Retorna método e convergência; torção sem convergência não possui valor liberado. Não dimensiona o projeto.",json!({"entry_id":id}),&["entry_id"],true));
    }
    result
}
