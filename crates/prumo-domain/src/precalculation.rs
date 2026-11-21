//! PRU-016..020: mechanical hypotheses, traceable references, staged BIM input,
//! controlled repetition and pre-calculation review. No structural solver is implied.
use crate::identity::{IfcGuid, SourceOrigin};
use crate::model::{PhysicalRole, Point3, PointInput};
use crate::{Discipline, DomainError, ModelState, PhysicalObject, Project, validate_name};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const KEY: &str = "prumo.precalculation.v1";
const MAX_SOURCES: usize = 100;
const MAX_ELEMENTS: usize = 2_000;
const MAX_BATCH: usize = 500;

fn invalid(message: impl Into<String>) -> DomainError {
    DomainError::InvalidModel(message.into())
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Freedom {
    Free {},
    Fixed {},
    Spring { stiffness_si: f64 },
    CompressionOnly { stiffness_si: f64 },
}

impl Freedom {
    fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::Spring { stiffness_si } | Self::CompressionOnly { stiffness_si }
                if !stiffness_si.is_finite() || *stiffness_si <= 0.0 =>
            {
                Err(invalid("A mola precisa de rigidez SI positiva e finita."))
            }
            _ => Ok(()),
        }
    }
    fn restrained(&self) -> bool {
        !matches!(self, Self::Free {})
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeSupport {
    pub id: Uuid,
    pub node_id: Uuid,
    /// Proper rotation: local degrees of freedom to engineering coordinates.
    pub rotation_to_engineering: [[f64; 3]; 3],
    /// Ux, Uy, Uz in N/m; Rx, Ry, Rz in N.m/rad when springs.
    pub freedoms: [Freedom; 6],
    pub justification: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberEnd {
    Start,
    End,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MemberRelease {
    pub id: Uuid,
    pub member_id: Uuid,
    pub end: MemberEnd,
    pub freedoms: [Freedom; 6],
    pub justification: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceFormat {
    Dxf,
    PdfVector,
    PdfRaster,
    Image,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Calibration {
    /// First declared distance in drawing coordinates, not pixels guessed as metres.
    pub measured_a: f64,
    pub actual_a_m: f64,
    pub measured_b: f64,
    pub actual_b_m: f64,
}

impl Calibration {
    pub fn factors(&self) -> Result<(f64, f64, f64), DomainError> {
        if [self.measured_a, self.measured_b, self.actual_a_m, self.actual_b_m]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err(invalid("A calibração exige duas medidas e cotas positivas."));
        }
        let a = self.actual_a_m / self.measured_a;
        let b = self.actual_b_m / self.measured_b;
        if !a.is_finite() || !b.is_finite() || a > 1e6 || b > 1e6 || a < 1e-9 || b < 1e-9 {
            return Err(invalid("A escala da referência está fora dos limites."));
        }
        let discrepancy = (a - b).abs() / a.max(b);
        Ok((a, b, discrepancy))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceTrace {
    pub id: Uuid,
    pub layer: String,
    pub a: [f64; 3],
    pub b: [f64; 3],
    pub visible: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DrawingReference {
    pub id: Uuid,
    pub label: String,
    pub format: ReferenceFormat,
    pub sha256: String,
    pub calibration: Calibration,
    pub origin_m: [f64; 3],
    /// Caller-submitted candidate geometry. No PDF/DXF/image parsing is claimed.
    pub traces: Vec<ReferenceTrace>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IfcSchema {
    Ifc2x3,
    Ifc4,
    Ifc4x3,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IfcCandidate {
    pub guid: String,
    pub name: String,
    pub entity_type: String,
    pub start_m: [f64; 3],
    pub end_m: [f64; 3],
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IfcReference {
    pub id: Uuid,
    pub label: String,
    pub sha256: String,
    pub schema: IfcSchema,
    pub candidates: Vec<IfcCandidate>,
    /// Staging manifests never certify schema, placement or analytical connectivity.
    pub unknown_entity_count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    pub source_id: Uuid,
    pub object_id: Uuid,
    pub linked: bool,
    pub overridden: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrecalculationState {
    pub supports: Vec<NodeSupport>,
    pub releases: Vec<MemberRelease>,
    pub drawings: Vec<DrawingReference>,
    pub ifc: Vec<IfcReference>,
    pub instances: Vec<Instance>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum PrecalculationEdit {
    PutSupport { support: NodeSupport },
    RemoveSupport { id: Uuid },
    PutRelease { release: MemberRelease },
    RemoveRelease { id: Uuid },
    PutDrawing { reference: DrawingReference, expected_sha256: Option<String> },
    PutIfc { reference: IfcReference, expected_sha256: Option<String> },
    PromoteDrawing {
        reference_id: Uuid,
        trace_id: Uuid,
        name: String,
        material_id: Option<Uuid>,
        confirm_dimensions: bool,
    },
    PromoteIfc {
        reference_id: Uuid,
        guid: String,
        material_id: Option<Uuid>,
        confirm_hypotheses: bool,
    },
    CopyBatch {
        object_ids: Vec<Uuid>,
        translation: PointInput,
        linked: bool,
    },
    SetMaterialBatch {
        object_ids: Vec<Uuid>,
        material_id: Option<Uuid>,
    },
}

pub fn state(project: &Project) -> Result<PrecalculationState, DomainError> {
    project.extensions.get(KEY).map_or_else(
        || Ok(PrecalculationState::default()),
        |value| serde_json::from_value(value.clone())
            .map_err(|e| invalid(format!("Metadados de pré-cálculo inválidos: {e}"))),
    )
}

fn sha_ok(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn good_point(p: [f64; 3]) -> Result<(), DomainError> {
    crate::validate_point(p)
}

fn member_geometry_ok(a: [f64; 3], b: [f64; 3]) -> Result<(), DomainError> {
    good_point(a)?;
    good_point(b)?;
    if (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>() <= 1e-12 {
        return Err(invalid("A candidata tem comprimento nulo ou inferior à tolerância."));
    }
    Ok(())
}

pub fn validate_project(project: &Project) -> Result<(), DomainError> {
    let s = state(project)?;
    if s.supports.len() > MAX_ELEMENTS || s.releases.len() > MAX_ELEMENTS
        || s.drawings.len() > MAX_SOURCES || s.ifc.len() > MAX_SOURCES
        || s.instances.len() > MAX_ELEMENTS
    {
        return Err(DomainError::ModelLimit);
    }
    let mut ids = BTreeSet::new();
    let mut target_nodes = BTreeSet::new();
    for support in &s.supports {
        if support.id.is_nil() || !ids.insert(support.id) {
            return Err(invalid("Identidade repetida de apoio."));
        }
        if !target_nodes.insert(support.node_id) {
            return Err(invalid(format!("Apoios contraditórios no nó {}.", support.node_id)));
        }
        if !project.model.analytical_nodes.iter().any(|n| n.id == support.node_id) {
            return Err(DomainError::UnknownReference(support.node_id));
        }
        crate::model::validate_rotation(support.rotation_to_engineering)?;
        validate_name(&support.justification)?;
        for freedom in &support.freedoms { freedom.validate()?; }
        if support.freedoms.iter().all(|f| !f.restrained()) {
            return Err(invalid("Apoio integralmente livre não é um vínculo."));
        }
    }
    let mut end_pairs = BTreeSet::new();
    for release in &s.releases {
        if release.id.is_nil() || !ids.insert(release.id) {
            return Err(invalid("Identidade repetida de liberação."));
        }
        if !project.model.analytical_members.iter().any(|m| m.id == release.member_id) {
            return Err(DomainError::UnknownReference(release.member_id));
        }
        if !end_pairs.insert((release.member_id, release.end as u8)) {
            return Err(invalid("Liberações conflitantes na mesma extremidade."));
        }
        validate_name(&release.justification)?;
        for freedom in &release.freedoms { freedom.validate()?; }
    }
    for drawing in &s.drawings {
        if drawing.id.is_nil() || !ids.insert(drawing.id) || !sha_ok(&drawing.sha256)
            || drawing.traces.len() > MAX_ELEMENTS
        {
            return Err(invalid("Identidade, SHA ou limite de referência inválido."));
        }
        validate_name(&drawing.label)?;
        good_point(drawing.origin_m)?;
        drawing.calibration.factors()?;
        let mut local = BTreeSet::new();
        for trace in &drawing.traces {
            if trace.id.is_nil() || !local.insert(trace.id) { return Err(invalid("Traço repetido.")); }
            validate_name(&trace.layer)?;
            member_geometry_ok(trace.a, trace.b)?;
        }
    }
    for source in &s.ifc {
        if source.id.is_nil() || !ids.insert(source.id) || !sha_ok(&source.sha256)
            || source.candidates.len() > MAX_ELEMENTS || source.unknown_entity_count > 100_000
        {
            return Err(invalid("Identidade, SHA ou limite IFC inválido."));
        }
        validate_name(&source.label)?;
        let mut guids = BTreeSet::new();
        for candidate in &source.candidates {
            IfcGuid::try_from(candidate.guid.clone())?;
            validate_name(&candidate.name)?;
            validate_name(&candidate.entity_type)?;
            member_geometry_ok(candidate.start_m, candidate.end_m)?;
            if !guids.insert(&candidate.guid) {
                return Err(invalid("GUID IFC duplicado dentro da referência."));
            }
        }
    }
    let mut children = BTreeSet::new();
    for instance in &s.instances {
        if !children.insert(instance.object_id) || instance.source_id == instance.object_id
            || !project.objects.iter().any(|x| x.id == instance.object_id)
            || !project.model.identities.contains_key(&instance.source_id)
        {
            return Err(invalid("Relação de instância inválida ou repetida."));
        }
    }
    Ok(())
}

fn upsert<T: Clone>(items: &mut Vec<T>, id: Uuid, mut identifier: impl FnMut(&T) -> Uuid, value: T) {
    if let Some(existing) = items.iter_mut().find(|item| identifier(item) == id) {
        *existing = value;
    } else {
        items.push(value);
    }
}

fn update_source<T>(items: &mut Vec<T>, id: Uuid, expected_sha: Option<&str>,
    identifier: impl Fn(&T) -> Uuid, sha: impl Fn(&T) -> &str,
    replacement: T) -> Result<(), DomainError>
{
    if let Some(old) = items.iter_mut().find(|item| identifier(item) == id) {
        if Some(sha(old)) != expected_sha {
            return Err(invalid("A referência mudou. Confirme o SHA anterior para substituir."));
        }
        *old = replacement;
    } else if expected_sha.is_some() {
        return Err(invalid("Referência não encontrada para a revisão informada."));
    } else {
        items.push(replacement);
    }
    Ok(())
}

fn insert_promoted(project: &mut Project, name: String,
    points: ([f64; 3], [f64; 3]), material_id: Option<Uuid>,
    provenance: (SourceOrigin, Option<IfcGuid>), actor: &str, revision: u64,
) -> Result<(), DomainError> {
    let (a, b) = points;
    let (origin, guid) = provenance;
    validate_name(&name)?;
    member_geometry_ok(a, b)?;
    if let Some(id) = material_id
        && !project.model.materials.iter().any(|m| m.id == id)
    {
        return Err(DomainError::UnknownReference(id));
    }
    let id = Uuid::new_v4();
    project.objects.push(PhysicalObject {
        id, name, discipline: Discipline::Structure,
        start_m: Point3::try_from(a)?, end_m: Point3::try_from(b)?,
        state: ModelState::Modelled, role: PhysicalRole::AnalyticalSource,
        material_id, environment_id: None, extensions: BTreeMap::new(),
    });
    crate::model::record_identity(project, id, actor, revision, true)?;
    let identity = project.model.identities.get_mut(&id)
        .ok_or(DomainError::UnknownReference(id))?;
    identity.origin = origin;
    if let Some(guid) = guid { identity.ifc_guid = Some(guid); }
    Ok(())
}

pub fn apply(project: &mut Project, command: PrecalculationEdit,
    actor: &str, revision: u64) -> Result<(), DomainError>
{
    validate_name(actor)?;
    let mut state = state(project)?;
    match command {
        PrecalculationEdit::PutSupport { support } => {
            upsert(&mut state.supports, support.id, |item| item.id, support);
        }
        PrecalculationEdit::RemoveSupport { id } => {
            if !state.supports.iter().any(|item| item.id == id) { return Err(DomainError::UnknownReference(id)); }
            state.supports.retain(|item| item.id != id);
        }
        PrecalculationEdit::PutRelease { release } => {
            upsert(&mut state.releases, release.id, |item| item.id, release);
        }
        PrecalculationEdit::RemoveRelease { id } => {
            if !state.releases.iter().any(|item| item.id == id) { return Err(DomainError::UnknownReference(id)); }
            state.releases.retain(|item| item.id != id);
        }
        PrecalculationEdit::PutDrawing { reference, expected_sha256 } => {
            update_source(&mut state.drawings, reference.id, expected_sha256.as_deref(),
                |r| r.id, |r| r.sha256.as_str(), reference)?;
        }
        PrecalculationEdit::PutIfc { reference, expected_sha256 } => {
            update_source(&mut state.ifc, reference.id, expected_sha256.as_deref(),
                |r| r.id, |r| r.sha256.as_str(), reference)?;
        }
        PrecalculationEdit::PromoteDrawing {
            reference_id, trace_id, name, material_id, confirm_dimensions,
        } => {
            if !confirm_dimensions { return Err(invalid("Confira duas cotas antes de promover o traço.")); }
            let drawing = state.drawings.iter().find(|r| r.id == reference_id)
                .ok_or(DomainError::UnknownReference(reference_id))?;
            let (_, _, discrepancy) = drawing.calibration.factors()?;
            if discrepancy > 0.01 {
                return Err(invalid("Escalas discrepantes em mais de 1%. Corrija a referência antes da promoção."));
            }
            let trace = drawing.traces.iter().find(|r| r.id == trace_id && r.visible)
                .ok_or(DomainError::UnknownReference(trace_id))?;
            let (scale, _, _) = drawing.calibration.factors()?;
            let mapped = |p: [f64; 3]| [
                drawing.origin_m[0] + p[0] * scale,
                drawing.origin_m[1] + p[1] * scale,
                drawing.origin_m[2] + p[2] * scale,
            ];
            insert_promoted(project, name, (mapped(trace.a), mapped(trace.b)), material_id,
                (SourceOrigin::Imported {
                    system: format!("{:?}", drawing.format),
                    reference: drawing.sha256.clone(),
                    external_id: trace.id.to_string(),
                }, None), actor, revision)?;
        }
        PrecalculationEdit::PromoteIfc {
            reference_id, guid, material_id, confirm_hypotheses,
        } => {
            if !confirm_hypotheses {
                return Err(invalid("A promoção IFC requer confirmação explícita das hipóteses físicas."));
            }
            let source = state.ifc.iter().find(|r| r.id == reference_id)
                .ok_or(DomainError::UnknownReference(reference_id))?;
            let c = source.candidates.iter().find(|c| c.guid == guid)
                .ok_or_else(|| invalid("GUID não encontrado no IFC importado."))?;
            if !["IFCBEAM", "IFCCOLUMN", "IFCMEMBER"].iter()
                .any(|ty| c.entity_type.eq_ignore_ascii_case(ty)) {
                return Err(invalid("O tipo IFC não representa uma barra estrutural suportada. Mantenha-o como referência."));
            }
            if project.model.identities.values().any(|identity|
                matches!(&identity.origin, SourceOrigin::Imported {reference,external_id,..}
                    if reference == &source.sha256 && external_id == &guid))
            {
                return Err(invalid("Elemento IFC já foi promovido. Reimportação não duplica objetos."));
            }
            insert_promoted(project, c.name.clone(), (c.start_m, c.end_m), material_id,
                (SourceOrigin::Imported {
                    system: "IFC".into(), reference: source.sha256.clone(), external_id: guid.clone(),
                }, Some(IfcGuid::try_from(guid)?)), actor, revision)?;
        }
        PrecalculationEdit::CopyBatch { object_ids, translation, linked } => {
            if object_ids.is_empty() || object_ids.len() > MAX_BATCH
                || object_ids.iter().collect::<BTreeSet<_>>().len() != object_ids.len()
            {
                return Err(invalid("Selecione de 1 a 500 peças distintas para a cópia."));
            }
            let delta = translation.to_si()?.as_array();
            for source_id in object_ids {
                let mut object = project.objects.iter().find(|x| x.id == source_id)
                    .ok_or(DomainError::UnknownReference(source_id))?.clone();
                if !object.extensions.is_empty() {
                    return Err(invalid("Peça com gerador paramétrico: copie pelo comando específico da geometria."));
                }
                let id = Uuid::new_v4();
                object.id = id;
                object.name = format!("{} cópia", object.name);
                validate_name(&object.name)?;
                let offset = |p: Point3| -> Result<Point3, DomainError> {
                    let a = p.as_array();
                    Point3::try_from([a[0] + delta[0], a[1] + delta[1], a[2] + delta[2]])
                };
                object.start_m = offset(object.start_m)?;
                object.end_m = offset(object.end_m)?;
                project.objects.push(object);
                crate::model::record_identity(project, id, actor, revision, true)?;
                project.model.identities.get_mut(&id).expect("registered")
                    .origin = SourceOrigin::Derived { operation: format!("copy_from:{source_id}") };
                state.instances.push(Instance {
                    source_id, object_id: id, linked, overridden: false,
                });
            }
        }
        PrecalculationEdit::SetMaterialBatch { object_ids, material_id } => {
            if object_ids.is_empty() || object_ids.len() > MAX_BATCH
                || object_ids.iter().collect::<BTreeSet<_>>().len() != object_ids.len()
            {
                return Err(invalid("Selecione de 1 a 500 peças distintas para editar."));
            }
            if let Some(id) = material_id
                && !project.model.materials.iter().any(|m| m.id == id)
            {
                return Err(DomainError::UnknownReference(id));
            }
            for id in object_ids {
                let target = project.objects.iter_mut().find(|o| o.id == id)
                    .ok_or(DomainError::UnknownReference(id))?;
                target.material_id = material_id;
                if let Some(instance) = state.instances.iter_mut().find(|x| x.object_id == id) {
                    instance.overridden = true;
                }
            }
        }
    }
    project.extensions.insert(KEY.into(), serde_json::to_value(state)
        .map_err(|e| invalid(e.to_string()))?);
    validate_project(project)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity { Blocker, Warning, Info }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub stage: String,
    pub code: String,
    pub severity: Severity,
    pub entity_id: Option<Uuid>,
    pub explanation: String,
    pub consequence: String,
    pub suggested_action: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub project_id: Uuid,
    pub revision: u64,
    pub stages: Vec<String>,
    pub findings: Vec<Finding>,
    pub ready_for_engineering_calculation: bool,
    pub calculation_approved: bool,
}

fn finding(stage: &str, code: &str, severity: Severity, entity_id: Option<Uuid>,
    explanation: &str, consequence: &str, suggested_action: &str) -> Finding
{
    Finding {
        stage: stage.into(), code: code.into(), severity, entity_id,
        explanation: explanation.into(), consequence: consequence.into(),
        suggested_action: suggested_action.into(),
    }
}

pub fn review(project: &Project) -> Result<Review, DomainError> {
    let state = state(project)?;
    let mut findings = Vec::new();
    if project.objects.is_empty() {
        findings.push(finding("projeto","empty_model",Severity::Blocker,None,
            "Nenhuma peça foi modelada.", "Não há estrutura para avaliar.",
            "Insira as peças com geometria e identidades explícitas."));
    }
    for obj in &project.objects {
        if obj.discipline == Discipline::Structure
            && obj.role == PhysicalRole::AnalyticalSource && obj.material_id.is_none()
        {
            findings.push(finding("materiais","missing_material",Severity::Blocker,Some(obj.id),
                "Peça estrutural sem material associado.",
                "Rigidez e peso próprio não podem ser determinados.",
                "Escolha um material do catálogo e registre sua origem."));
        }
    }
    if project.objects.iter().any(|o| o.discipline == Discipline::Structure
        && o.role == PhysicalRole::AnalyticalSource)
        && project.model.analytical_members.is_empty()
    {
        findings.push(finding("geometria", "no_analytical_members", Severity::Blocker,
            None, "Peças estruturais físicas não possuem barras analíticas conectadas.",
            "Não é possível conferir a transferência de esforços entre as peças.",
            "Defina os nós, barras e vínculos analíticos correspondentes."));
    }
    for obj in &project.objects {
        if obj.discipline != Discipline::Structure || obj.role != PhysicalRole::AnalyticalSource {
            continue;
        }
        if !project.model.links.iter().any(|link| link.physical_id == obj.id) {
            findings.push(finding("geometria", "unlinked_physical", Severity::Blocker, Some(obj.id),
                "Peça física sem vínculo explícito com elemento ou região analítica.",
                "A transferência de esforços desta peça não está definida.",
                "Associe a peça ao elemento analítico correspondente."));
        }
        if let Some(surface) = crate::surfaces::metadata(obj)?
            && surface.surface.supports.is_empty()
            && surface.surface.edge_conditions.iter().all(|edge|
                matches!(&edge.behavior, crate::surfaces::EdgeBehavior::Free {}))
        {
            findings.push(finding("vínculos", "unsupported_surface", Severity::Blocker, Some(obj.id),
                "Laje ou painel sem apoio explícito registrado.",
                "Não há caminho de cargas declarado para o contorno.",
                "Cadastre apoios de superfície ou vínculos de borda com peças existentes."));
        }
    }
    let mut adjacency: BTreeMap<Uuid, Vec<Uuid>> = BTreeMap::new();
    for member in &project.model.analytical_members {
        adjacency.entry(member.start_node_id).or_default().push(member.end_node_id);
        adjacency.entry(member.end_node_id).or_default().push(member.start_node_id);
    }
    let mut reached = BTreeSet::new();
    let supports: BTreeSet<_> = state.supports.iter().map(|item| item.node_id).collect();
    for node in &project.model.analytical_nodes {
        if !reached.insert(node.id) {
            continue;
        }
        let mut pending = vec![node.id];
        let mut anchored = false;
        while let Some(current) = pending.pop() {
            anchored |= supports.contains(&current);
            for next in adjacency.get(&current).into_iter().flatten() {
                if reached.insert(*next) { pending.push(*next); }
            }
        }
        if !anchored {
            findings.push(finding("vínculos", "unsupported_component", Severity::Blocker, Some(node.id),
                "Conjunto de nós e barras sem apoio em sua componente conectada.",
                "O subconjunto pode se deslocar sem resistência de contorno.",
                "Defina os apoios físicos ou conexões analíticas faltantes."));
        }
    }
    let mut coordinates = BTreeMap::new();
    for node in &project.model.analytical_nodes {
        let pos = node.position_m.as_array();
        let key = [pos[0].to_bits(),pos[1].to_bits(),pos[2].to_bits()];
        if let Some(previous) = coordinates.insert(key, node.id) {
            findings.push(finding("geometria","coincident_nodes",Severity::Warning,Some(node.id),
                "Nós diferentes ocupam as mesmas coordenadas.",
                "Peças podem parecer conectadas sem compartilhar nó.",
                &format!("Inspecione os nós {previous} e {} e vincule-os explicitamente.",node.id)));
        }
        if !project.model.analytical_members.iter().any(|m|
            m.start_node_id == node.id || m.end_node_id == node.id)
        {
            findings.push(finding("geometria","disconnected_node",Severity::Blocker,Some(node.id),
                "Nó analítico sem nenhuma barra conectada.",
                "A transferência de esforços está indefinida.",
                "Conecte o nó explicitamente à geometria estrutural pertinente."));
        }
    }
    let mut actions = BTreeSet::new();
    for action in &project.model.actions {
        let signature = serde_json::to_string(&(action.application_node_id, &action.force,
            &action.couple, &action.frame)).map_err(|e| invalid(e.to_string()))?;
        if !actions.insert(signature) {
            findings.push(finding("ações","duplicate_action",Severity::Blocker,Some(action.id),
                "A mesma ação nodal foi informada mais de uma vez.",
                "Há risco de somar cargas duplicadas.",
                "Remova a carga duplicada ou documente situações distintas."));
        }
    }
    for support in &state.supports {
        if support.freedoms.iter().any(|f| matches!(f,Freedom::CompressionOnly{..})) {
            findings.push(finding("vínculos","unilateral_support",Severity::Warning,Some(support.node_id),
                "Apoio unilateral declarado.", "Exige análise não linear ainda indisponível.",
                "Não interprete o vínculo como apoio bilateral linear."));
        }
    }
    for member in &project.model.analytical_members {
        let ends: Vec<_> = state.releases.iter().filter(|r| r.member_id == member.id).collect();
        if ends.len() == 2 && ends.iter().all(|r| r.freedoms.iter().all(|d| !d.restrained())) {
            findings.push(finding("vínculos","double_release_mechanism",Severity::Blocker,
                Some(member.id), "As duas extremidades foram liberadas em todos os graus de liberdade.",
                "O membro pode produzir um mecanismo sem rigidez.",
                "Defina vínculos físicos consistentes; não estabilize artificialmente."));
        }
    }
    if !project.model.analytical_nodes.is_empty() && state.supports.is_empty() {
        findings.push(finding("vínculos","no_support",Severity::Blocker,None,
            "Nenhum apoio analítico foi declarado.",
            "A estrutura não possui condições de contorno verificáveis.",
            "Informe apoios e suas restrições por grau de liberdade."));
    }
    for drawing in &state.drawings {
        if drawing.calibration.factors()?.2 > 0.01 {
            findings.push(finding("geometria","distorted_reference",Severity::Blocker,Some(drawing.id),
                "As duas cotas de calibração da referência divergem mais de 1%.",
                "A escala pode distorcer dimensões de peças promovidas.",
                "Corrija orientação, escala ou a origem do desenho antes da promoção."));
        }
    }
    for source in &state.ifc {
        findings.push(finding("revisão","ifc_staging_only",Severity::Info,Some(source.id),
            "Candidatos IFC em estágio de referência, sem validação completa de schema.",
            "A geometria recebida não prova conectividade ou modelo analítico.",
            "Confira GUIDs, unidades, localização e materiais antes da promoção."));
    }
    findings.push(finding("revisão","solver_unavailable",Severity::Info,None,
        "Cálculo estrutural de projeto ainda não foi implementado.",
        "Concluir as verificações de modelagem não certifica esforços nem dimensionamento.",
        "Somente uma etapa futura de solver qualificado poderá produzir resultados."));
    Ok(Review {
        project_id: project.id, revision: project.revision,
        stages: ["projeto","geometria","vínculos","materiais","ações","revisão"]
            .iter().map(|s| s.to_string()).collect(),
        ready_for_engineering_calculation: !findings.iter().any(|f|
            f.severity == Severity::Blocker) && !project.objects.is_empty(),
        calculation_approved: false,
        findings,
    })
}
