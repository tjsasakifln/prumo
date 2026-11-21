//! Project-authorized geometry and reproducible verification tools.
use super::{decode, definition};
use prumo_app::{
    AppError, ProjectService,
    access::{AccessScope, ProjectGrant},
};
use prumo_domain::catalogue::{PhysicalQuantity, PhysicalValue};
use prumo_domain::geometry::{FaceInput, GeometryFilter, GeometryRepair};
use prumo_domain::{layout::SnapRequest, view::ViewCommand};
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ViewArgs {
    command: ViewCommand,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeasureArgs {
    expression: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapArgs {
    request: SnapRequest,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectArgs {
    object_id: Uuid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SurfaceArgs {
    surface: prumo_domain::surfaces::SurfaceSpec,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssemblyArgs {
    assembly: prumo_domain::assemblies::AssemblySpec,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisTrialArgs {
    trial: Value,
}

pub(super) enum Operation {
    AnalysisTrial(Value),
    PrecalculationState,
    PrecalculationReview,
    ViewRead,
    ViewExport,
    View(ViewCommand),
    Layout,
    Measure(String),
    Snap(SnapRequest),
    Framing(Uuid),
    Surface(Uuid),
    SurfacePreview(Box<prumo_domain::surfaces::SurfaceSpec>),
    Assembly(Uuid),
    AssemblyPreview(Box<prumo_domain::assemblies::AssemblySpec>),
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
            "prumo.analysis.trial" => decode::<AnalysisTrialArgs>(args).map(|input| Self::AnalysisTrial(input.trial)),
            "prumo.precalculation.read" => decode::<Empty>(args).map(|_| Self::PrecalculationState),
            "prumo.precalculation.review" => decode::<Empty>(args).map(|_| Self::PrecalculationReview),
            "prumo.view.export" => decode::<Empty>(args).map(|_| Self::ViewExport),
            "prumo.surfaces.preview" => {
                decode::<SurfaceArgs>(args).map(|a| Self::SurfacePreview(Box::new(a.surface)))
            }
            "prumo.assemblies.inspect" => {
                decode::<ObjectArgs>(args).map(|a| Self::Assembly(a.object_id))
            }
            "prumo.assemblies.preview" => {
                decode::<AssemblyArgs>(args).map(|a| Self::AssemblyPreview(Box::new(a.assembly)))
            }
            "prumo.framing.inspect" => {
                decode::<ObjectArgs>(args).map(|a| Self::Framing(a.object_id))
            }
            "prumo.surfaces.inspect" => {
                decode::<ObjectArgs>(args).map(|a| Self::Surface(a.object_id))
            }
            "prumo.view.read" => decode::<Empty>(args).map(|_| Self::ViewRead),
            "prumo.view.command" => decode::<ViewArgs>(args).map(|a| Self::View(a.command)),
            "prumo.layout.read" => decode::<Empty>(args).map(|_| Self::Layout),
            "prumo.layout.measure" => {
                decode::<MeasureArgs>(args).map(|a| Self::Measure(a.expression))
            }
            "prumo.layout.snap" => decode::<SnapArgs>(args).map(|a| Self::Snap(a.request)),
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
            Self::ViewExport => AccessScope::Model,
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
            Self::AnalysisTrial(input) => service.analysis_trial(input, Some(cancel))?,
            Self::PrecalculationState => service.precalculation_state()?,
            Self::PrecalculationReview => service.precalculation_review()?,
            Self::ViewExport => serde_json::to_value(service.export_web_view()?)?,
            Self::SurfacePreview(spec) => service.surface_preview(&spec)?,
            Self::Assembly(id) => service.assembly_inspection(id)?,
            Self::AssemblyPreview(spec) => service.assembly_preview(&spec)?,
            Self::Framing(id) => serde_json::to_value(service.framing_inspection(id)?)?,
            Self::Surface(id) => serde_json::to_value(service.surface_inspection(id)?)?,
            Self::ViewRead => serde_json::to_value(service.view_report()?)?,
            Self::View(command) => serde_json::to_value(service.view_command(command)?)?,
            Self::Layout => service.layout_view()?,
            Self::Measure(expression) => service.layout_measure(&expression)?,
            Self::Snap(request) => serde_json::to_value(service.layout_snap(&request)?)?,
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
            "prumo.analysis.trial",
            "Executa ensaio analítico Rust sem mutar o projeto. Forneça unidades SI explícitas. As saídas não aprovam projeto, estabilidade, norma nem dimensionamento.",
            json!({"trial":{"type":"object","required":["kind"],"properties":{
                "kind":{"type":"string","enum":["linear","truss","frame2d","mesh_inspect","membrane","mesh_refine","mesh_convergence","multipoint","rigid_diaphragm","load_ledger","path_audit","load_equilibrium"]}
            }}}),
            &["trial"],true,
        ),
        definition("prumo.precalculation.read", "Consulta vínculos, referências calibradas, candidatos IFC e instâncias em sua revisão, sem declarar dados como calculados.", json!({}), &[], true),
        definition("prumo.precalculation.review", "Revisa geometria, vínculos, materiais e ações; retorna impedimentos localizados. Não aprova cálculo de engenharia.", json!({}), &[], true),
        definition(
            "prumo.surfaces.preview",
            "Confere contorno, aberturas, espessuras, nervuras, área material e quantidade por componente antes de salvar. Não produz malha ou cálculo de engenharia.",
            json!({"surface":super::surface_assembly_schema::surface_spec()}),
            &["surface"],
            true,
        ),
        definition(
            "prumo.assemblies.inspect",
            "Consulta partes, apoios e continuidade explicitamente declarados de parede, núcleo, escada ou rampa. Referência visual e elemento estrutural permanecem distintos.",
            json!({"object_id":id}),
            &["object_id"],
            true,
        ),
        definition(
            "prumo.assemblies.preview",
            "Prepara as partes físicas e regiões candidatas de parede, núcleo, escada ou rampa. Preserva orientação e apoios explícitos; não calcula transferência de cargas.",
            json!({"assembly":super::surface_assembly_schema::assembly_spec()}),
            &["assembly"],
            true,
        ),
        definition(
            "prumo.framing.inspect",
            "Inspeciona material, seções, traçado, comprimento, excentricidade, identidades e comportamento analítico explícito de uma peça paramétrica. Não calcula esforços ou resistência.",
            json!({"object_id":id}),
            &["object_id"],
            true,
        ),
        definition(
            "prumo.surfaces.inspect",
            "Inspeciona contornos, aberturas, regiões candidatas, apoios e peso próprio por componente físico. Não calcula esforços, malha definitiva ou punção.",
            json!({"object_id":id}),
            &["object_id"],
            true,
        ),
        definition(
            "prumo.view.read",
            "Consulta seleção, ocultação, corte, câmera e identidades sincronizadas das vistas. Estado de sessão, sem alterar o projeto.",
            json!({}),
            &[],
            true,
        ),
        definition(
            "prumo.view.command",
            "Navega, seleciona, localiza, oculta e isola objetos com os mesmos comandos da interface. Não muda revisão, ligações ou resultados técnicos.",
            json!({"command":super::authoring_schema::view_command()}),
            &["command"],
            true,
        ),
        definition(
            "prumo.layout.read",
            "Consulta eixos, níveis, sistemas locais e vínculos dimensionais com unidade em metros e identidade da revisão.",
            json!({}),
            &[],
            true,
        ),
        definition(
            "prumo.layout.measure",
            "Interpreta uma medida com unidade explícita, vírgula ou ponto e operações aritméticas dimensionais controladas. Não modifica o projeto.",
            json!({"expression":super::authoring_schema::measure()}),
            &["expression"],
            true,
        ),
        definition(
            "prumo.layout.snap",
            "Propõe extremidade, alinhamento, interseção ou distância com fontes e unidade explícitas. O encaixe nunca cria conexão resistente por aparência.",
            json!({"request":super::authoring_schema::snap_request()}),
            &["request"],
            true,
        ),
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
        result.push(definition("prumo.view.export","Exporta o modelo JSON completo para o visualizador web Rust, ao lado do arquivo do projeto, com revisão e SHA no nome. Não recebe caminhos nem substitui arquivo existente diferente.",json!({}),&[],false));
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
