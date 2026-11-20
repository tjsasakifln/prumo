//! Explicit project axes, levels and dimensional construction points.
//!
//! A snap is a geometric proposal. It never adds a structural connection.
use crate::{
    DomainError, Project,
    model::record_identity,
    units::{Point3, PointInput},
    validate_name,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const LAYOUT_KEY: &str = "prumo.layout.v1";
const LIMIT: usize = 10_000;
fn invalid(message: impl Into<String>) -> DomainError {
    DomainError::InvalidModel(message.into())
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutState {
    pub levels: Vec<Level>,
    pub axes: Vec<Axis>,
    pub frames: Vec<PlanFrame>,
    pub bindings: Vec<LevelBinding>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Level {
    pub id: Uuid,
    pub name: String,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub elevation_m: f64,
    pub original_expression: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanFrame {
    pub id: Uuid,
    pub name: String,
    pub origin_m: Point3,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub angle_degrees: f64,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AxisInput {
    pub name: String,
    pub origin: PointInput,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub angle_degrees: f64,
    pub offset: String,
    pub extent: String,
    pub frame_id: Option<Uuid>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Axis {
    pub id: Uuid,
    pub input: AxisInput,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LevelBinding {
    pub object_id: Uuid,
    pub level_id: Uuid,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub offset_m: f64,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedAxis {
    pub id: Uuid,
    pub name: String,
    pub origin_m: Point3,
    #[serde(deserialize_with = "crate::numeric::vector3")]
    pub direction: [f64; 3],
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub extent_m: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "point", rename_all = "snake_case", deny_unknown_fields)]
pub enum LayoutPoint {
    Coordinates {
        position: PointInput,
    },
    Local {
        frame_id: Uuid,
        position: PointInput,
    },
    AxisStation {
        axis_id: Uuid,
        station: String,
        level_id: Uuid,
        offset: String,
    },
    GridIntersection {
        first_axis_id: Uuid,
        second_axis_id: Uuid,
        level_id: Uuid,
        offset: String,
    },
    Node {
        node_id: Uuid,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum LayoutEdit {
    PutLevel {
        id: Uuid,
        name: String,
        elevation: String,
    },
    PutAxis {
        id: Uuid,
        input: AxisInput,
    },
    PutFrame {
        id: Uuid,
        name: String,
        origin: PointInput,
        #[serde(deserialize_with = "crate::numeric::scalar")]
        angle_degrees: f64,
    },
    BindObject {
        object_id: Uuid,
        level_id: Uuid,
        offset: String,
    },
    UnbindObject {
        object_id: Uuid,
    },
}

/// Bounded arithmetic with dimensions. Only +, -, *, / and parentheses exist.
/// Lengths require an explicit m, cm or mm. Scalars are allowed as factors.
pub fn parse_length(expression: &str) -> Result<f64, DomainError> {
    if expression.is_empty() || expression.len() > 256 || !expression.is_ascii() {
        return Err(invalid(
            "Informe medida com unidade m, cm ou mm, em até 256 caracteres.",
        ));
    }
    let mut parser = Expression {
        bytes: expression.as_bytes(),
        at: 0,
        depth: 0,
        tokens: 0,
    };
    let value = parser.sum()?;
    parser.space();
    if parser.at != parser.bytes.len() || value.dimension != 1 {
        return Err(invalid(
            "Medida ambígua: informe unidade explícita e use apenas +, -, *, / e parênteses. Não use separador de milhares.",
        ));
    }
    if !value.value.is_finite() || value.value.abs() > crate::MAX_COORDINATE_M {
        return Err(DomainError::InvalidQuantity);
    }
    Ok(if value.value == 0.0 { 0.0 } else { value.value })
}
#[derive(Clone, Copy)]
struct Quantity {
    value: f64,
    dimension: i8,
}
struct Expression<'a> {
    bytes: &'a [u8],
    at: usize,
    depth: usize,
    tokens: usize,
}
impl Expression<'_> {
    fn space(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn consume(&mut self, byte: u8) -> bool {
        self.space();
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn sum(&mut self) -> Result<Quantity, DomainError> {
        let mut value = self.product()?;
        loop {
            let sign = if self.consume(b'+') {
                1.0
            } else if self.consume(b'-') {
                -1.0
            } else {
                break;
            };
            let other = self.product()?;
            if value.dimension != other.dimension {
                return Err(invalid(
                    "Não é possível somar medida com número sem unidade.",
                ));
            }
            value.value += sign * other.value;
        }
        Ok(value)
    }
    fn product(&mut self) -> Result<Quantity, DomainError> {
        let mut value = self.factor()?;
        loop {
            let divide = if self.consume(b'*') {
                false
            } else if self.consume(b'/') {
                true
            } else {
                break;
            };
            let other = self.factor()?;
            let both_nonzero = value.value != 0.0 && other.value != 0.0;
            if divide {
                if other.value == 0.0 {
                    return Err(invalid("Divisão por zero na medida."));
                }
                value.value /= other.value;
                value.dimension -= other.dimension;
            } else {
                value.value *= other.value;
                value.dimension += other.dimension;
            }
            if value.dimension.abs() > 2
                || !value.value.is_finite()
                || (both_nonzero && value.value == 0.0)
            {
                return Err(DomainError::InvalidQuantity);
            }
        }
        Ok(value)
    }
    fn factor(&mut self) -> Result<Quantity, DomainError> {
        self.tokens += 1;
        if self.tokens > 64 {
            return Err(invalid("Expressão excede 64 parcelas."));
        }
        let sign = if self.consume(b'-') {
            -1.0
        } else {
            self.consume(b'+');
            1.0
        };
        if self.consume(b'(') {
            self.depth += 1;
            if self.depth > 8 {
                return Err(invalid("Expressão excede oito níveis de parênteses."));
            }
            let mut value = self.sum()?;
            if !self.consume(b')') {
                return Err(invalid("Parêntese sem fechamento na medida."));
            }
            self.depth -= 1;
            value.value *= sign;
            return Ok(value);
        }
        self.space();
        let start = self.at;
        let mut separators = 0;
        while let Some(byte) = self.bytes.get(self.at) {
            if byte.is_ascii_digit() {
                self.at += 1;
            } else if *byte == b'.' || *byte == b',' {
                separators += 1;
                self.at += 1;
            } else {
                break;
            }
        }
        if start == self.at
            || separators > 1
            || !self.bytes[start..self.at].iter().any(u8::is_ascii_digit)
        {
            return Err(invalid(
                "Número ambíguo: use vírgula ou ponto decimal, sem separador de milhares.",
            ));
        }
        let text = std::str::from_utf8(&self.bytes[start..self.at])
            .map_err(|_| invalid("Número inválido."))?
            .replace(',', ".");
        let number: f64 = text
            .parse()
            .map_err(|_| invalid("Número inválido na medida."))?;
        if number == 0.0 && text.bytes().any(|byte| (b'1'..=b'9').contains(&byte)) {
            return Err(invalid(
                "A medida é pequena demais para preservar seu valor numérico.",
            ));
        }
        self.space();
        let unit_start = self.at;
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_alphabetic) {
            self.at += 1;
        }
        let (factor, dimension) = match &self.bytes[unit_start..self.at] {
            b"" => (1.0, 0),
            b"m" => (1.0, 1),
            b"cm" => (0.01, 1),
            b"mm" => (0.001, 1),
            _ => return Err(invalid("Unidade desconhecida. Use m, cm ou mm.")),
        };
        let value = number * sign * factor;
        if !value.is_finite() || (number != 0.0 && value == 0.0) {
            return Err(DomainError::InvalidQuantity);
        }
        Ok(Quantity { value, dimension })
    }
}

pub fn state(project: &Project) -> Result<LayoutState, DomainError> {
    project.extensions.get(LAYOUT_KEY).map_or_else(
        || Ok(LayoutState::default()),
        |value| serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string())),
    )
}
fn save(project: &mut Project, state: &LayoutState) -> Result<(), DomainError> {
    project.extensions.insert(
        LAYOUT_KEY.into(),
        serde_json::to_value(state).map_err(|e| invalid(e.to_string()))?,
    );
    Ok(())
}
fn angle(value: f64) -> Result<f64, DomainError> {
    if !value.is_finite() || value.abs() > 360.0 {
        return Err(invalid(
            "Ângulo deve ser finito, em graus, entre -360 e 360.",
        ));
    }
    Ok(value.to_radians())
}
fn rotate(point: [f64; 3], radians: f64) -> [f64; 3] {
    let (s, c) = radians.sin_cos();
    [
        c * point[0] - s * point[1],
        s * point[0] + c * point[1],
        point[2],
    ]
}
fn local_to_world(
    state: &LayoutState,
    frame_id: Option<Uuid>,
    p: Point3,
) -> Result<Point3, DomainError> {
    if let Some(id) = frame_id {
        let frame = state
            .frames
            .iter()
            .find(|f| f.id == id)
            .ok_or(DomainError::UnknownReference(id))?;
        let rotated = rotate(p.as_array(), angle(frame.angle_degrees)?);
        Point3::try_from(std::array::from_fn(|i| rotated[i] + frame.origin_m[i]))
    } else {
        Ok(p)
    }
}
pub fn resolve_axis(project: &Project, id: Uuid) -> Result<ResolvedAxis, DomainError> {
    resolve_axis_in(&state(project)?, id)
}
fn resolve_axis_in(state: &LayoutState, id: Uuid) -> Result<ResolvedAxis, DomainError> {
    let axis = state
        .axes
        .iter()
        .find(|a| a.id == id)
        .ok_or(DomainError::UnknownReference(id))?;
    validate_name(&axis.input.name)?;
    let local_angle = angle(axis.input.angle_degrees)?;
    let frame_angle = axis
        .input
        .frame_id
        .map(|id| {
            state
                .frames
                .iter()
                .find(|f| f.id == id)
                .ok_or(DomainError::UnknownReference(id))
                .and_then(|f| angle(f.angle_degrees))
        })
        .transpose()?
        .unwrap_or(0.0);
    let global_angle = local_angle + frame_angle;
    let (s, c) = global_angle.sin_cos();
    let offset = parse_length(&axis.input.offset)?;
    let origin = local_to_world(state, axis.input.frame_id, axis.input.origin.to_si()?)?;
    let origin = Point3::try_from([origin[0] - s * offset, origin[1] + c * offset, origin[2]])?;
    let extent_m = parse_length(&axis.input.extent)?;
    if extent_m <= crate::MIN_MEMBER_LENGTH_M {
        return Err(DomainError::DegenerateMember);
    }
    Ok(ResolvedAxis {
        id,
        name: axis.input.name.clone(),
        origin_m: origin,
        direction: [c, s, 0.0],
        extent_m,
    })
}
fn intersection(a: &ResolvedAxis, b: &ResolvedAxis) -> Result<Point3, DomainError> {
    let cross = a.direction[0] * b.direction[1] - a.direction[1] * b.direction[0];
    if cross.abs() < 1.0e-10 {
        return Err(invalid(
            "Eixos paralelos ou quase paralelos não definem interseção estável.",
        ));
    }
    let delta = [b.origin_m[0] - a.origin_m[0], b.origin_m[1] - a.origin_m[1]];
    let station = (delta[0] * b.direction[1] - delta[1] * b.direction[0]) / cross;
    Point3::try_from([
        a.origin_m[0] + station * a.direction[0],
        a.origin_m[1] + station * a.direction[1],
        a.origin_m[2],
    ])
}
pub fn resolve_point(project: &Project, input: &LayoutPoint) -> Result<Point3, DomainError> {
    let layout = state(project)?;
    let level_z = |id: Uuid, offset: &str| -> Result<f64, DomainError> {
        Ok(layout
            .levels
            .iter()
            .find(|l| l.id == id)
            .ok_or(DomainError::UnknownReference(id))?
            .elevation_m
            + parse_length(offset)?)
    };
    match input {
        LayoutPoint::Coordinates { position } => position.to_si(),
        LayoutPoint::Local { frame_id, position } => {
            local_to_world(&layout, Some(*frame_id), position.to_si()?)
        }
        LayoutPoint::AxisStation {
            axis_id,
            station,
            level_id,
            offset,
        } => {
            let axis = resolve_axis_in(&layout, *axis_id)?;
            let station = parse_length(station)?;
            Point3::try_from([
                axis.origin_m[0] + axis.direction[0] * station,
                axis.origin_m[1] + axis.direction[1] * station,
                level_z(*level_id, offset)?,
            ])
        }
        LayoutPoint::GridIntersection {
            first_axis_id,
            second_axis_id,
            level_id,
            offset,
        } => {
            if first_axis_id == second_axis_id {
                return Err(invalid("Escolha dois eixos distintos."));
            }
            let p = intersection(
                &resolve_axis_in(&layout, *first_axis_id)?,
                &resolve_axis_in(&layout, *second_axis_id)?,
            )?;
            Point3::try_from([p[0], p[1], level_z(*level_id, offset)?])
        }
        LayoutPoint::Node { node_id } => project
            .model
            .analytical_nodes
            .iter()
            .find(|node| node.id == *node_id)
            .map(|node| node.position_m)
            .ok_or(DomainError::UnknownReference(*node_id)),
    }
}
pub fn point_sources(point: &LayoutPoint) -> Vec<Uuid> {
    match point {
        LayoutPoint::Coordinates { .. } => vec![],
        LayoutPoint::Local { frame_id, .. } => vec![*frame_id],
        LayoutPoint::AxisStation {
            axis_id, level_id, ..
        } => vec![*axis_id, *level_id],
        LayoutPoint::GridIntersection {
            first_axis_id,
            second_axis_id,
            level_id,
            ..
        } => vec![*first_axis_id, *second_axis_id, *level_id],
        LayoutPoint::Node { node_id } => vec![*node_id],
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SnapRequest {
    Endpoint {
        object_id: Uuid,
        end: bool,
    },
    Alignment {
        axis_id: Uuid,
        position: PointInput,
    },
    Intersection {
        first_axis_id: Uuid,
        second_axis_id: Uuid,
        level_id: Uuid,
        offset: String,
    },
    Distance {
        from: LayoutPoint,
        towards: LayoutPoint,
        distance: String,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapProposal {
    pub project_id: Uuid,
    pub revision: u64,
    pub point_m: Point3,
    pub sources: Vec<Uuid>,
    pub creates_structural_connection: bool,
    pub explanation: String,
}
pub fn propose_snap(project: &Project, request: &SnapRequest) -> Result<SnapProposal, DomainError> {
    let (point_m, sources, kind) = match request {
        SnapRequest::Endpoint { object_id, end } => {
            let object = project
                .objects
                .iter()
                .find(|o| o.id == *object_id)
                .ok_or(DomainError::UnknownReference(*object_id))?;
            (
                if *end { object.end_m } else { object.start_m },
                vec![*object_id],
                "Extremidade",
            )
        }
        SnapRequest::Alignment { axis_id, position } => {
            let p = position.to_si()?;
            let axis = resolve_axis(project, *axis_id)?;
            let station = (p[0] - axis.origin_m[0]) * axis.direction[0]
                + (p[1] - axis.origin_m[1]) * axis.direction[1];
            (
                Point3::try_from([
                    axis.origin_m[0] + station * axis.direction[0],
                    axis.origin_m[1] + station * axis.direction[1],
                    p[2],
                ])?,
                vec![*axis_id],
                "Alinhamento",
            )
        }
        SnapRequest::Intersection {
            first_axis_id,
            second_axis_id,
            level_id,
            offset,
        } => {
            let point = LayoutPoint::GridIntersection {
                first_axis_id: *first_axis_id,
                second_axis_id: *second_axis_id,
                level_id: *level_id,
                offset: offset.clone(),
            };
            (
                resolve_point(project, &point)?,
                point_sources(&point),
                "Interseção",
            )
        }
        SnapRequest::Distance {
            from,
            towards,
            distance,
        } => {
            let a = resolve_point(project, from)?;
            let b = resolve_point(project, towards)?;
            let length = a.distance(b);
            if length <= crate::MIN_MEMBER_LENGTH_M {
                return Err(DomainError::DegenerateMember);
            }
            let distance = parse_length(distance)?;
            let point = Point3::try_from(std::array::from_fn(|i| {
                a[i] + (b[i] - a[i]) / length * distance
            }))?;
            let mut sources = point_sources(from);
            sources.extend(point_sources(towards));
            sources.sort();
            sources.dedup();
            (point, sources, "Distância")
        }
    };
    Ok(SnapProposal {
        project_id: project.id,
        revision: project.revision,
        point_m,
        sources,
        creates_structural_connection: false,
        explanation: format!(
            "{kind} geométrico. A proposta não cria vínculo estrutural, hidráulico ou elétrico."
        ),
    })
}

pub fn apply(
    project: &mut Project,
    command: LayoutEdit,
    actor: &str,
    revision: u64,
) -> Result<(), DomainError> {
    validate_name(actor)?;
    let mut candidate = project.clone();
    let old = state(&candidate)?;
    let mut layout = old.clone();
    match command {
        LayoutEdit::PutLevel {
            id,
            name,
            elevation,
        } => {
            validate_name(&name)?;
            let value = Level {
                id,
                name,
                elevation_m: parse_length(&elevation)?,
                original_expression: elevation,
            };
            if let Some(existing) = layout.levels.iter_mut().find(|x| x.id == id) {
                *existing = value;
            } else {
                record_identity(&mut candidate, id, actor, revision, false)?;
                layout.levels.push(value);
            }
        }
        LayoutEdit::PutAxis { id, input } => {
            if let Some(existing) = layout.axes.iter_mut().find(|x| x.id == id) {
                existing.input = input;
            } else {
                record_identity(&mut candidate, id, actor, revision, false)?;
                layout.axes.push(Axis { id, input });
            }
        }
        LayoutEdit::PutFrame {
            id,
            name,
            origin,
            angle_degrees,
        } => {
            validate_name(&name)?;
            angle(angle_degrees)?;
            let value = PlanFrame {
                id,
                name,
                origin_m: origin.to_si()?,
                angle_degrees,
            };
            if let Some(existing) = layout.frames.iter_mut().find(|x| x.id == id) {
                *existing = value;
            } else {
                record_identity(&mut candidate, id, actor, revision, false)?;
                layout.frames.push(value);
            }
        }
        LayoutEdit::BindObject {
            object_id,
            level_id,
            offset,
        } => {
            if crate::assemblies::is_child(&candidate, object_id)? {
                return Err(invalid(
                    "Vincule o conjunto completo ao nível, não um filho isolado.",
                ));
            }
            if layout.bindings.iter().any(|b| b.object_id == object_id) {
                return Err(invalid(
                    "A peça já está vinculada a um nível. Desvincule antes de escolher outro.",
                ));
            }
            let object = candidate
                .objects
                .iter()
                .find(|o| o.id == object_id)
                .ok_or(DomainError::UnknownReference(object_id))?;
            if object.extensions.contains_key(crate::framing::FRAMING_KEY) {
                return Err(invalid(
                    "Peça paramétrica usa níveis nos pontos de origem. Edite seus pontos em vez de criar vínculo adicional.",
                ));
            }
            let elevation = layout
                .levels
                .iter()
                .find(|l| l.id == level_id)
                .ok_or(DomainError::UnknownReference(level_id))?
                .elevation_m;
            let offset_m = parse_length(&offset)?;
            let dz = elevation + offset_m - object.start_m[2];
            translate_bound(&mut candidate, object_id, dz)?;
            layout.bindings.push(LevelBinding {
                object_id,
                level_id,
                offset_m,
            });
        }
        LayoutEdit::UnbindObject { object_id } => {
            let before = layout.bindings.len();
            layout.bindings.retain(|b| b.object_id != object_id);
            if before == layout.bindings.len() {
                return Err(invalid("Peça sem vínculo de nível."));
            }
        }
    }
    save(&mut candidate, &layout)?;
    for binding in &layout.bindings {
        if let Some(previous) = old
            .bindings
            .iter()
            .find(|b| b.object_id == binding.object_id)
        {
            let old_z = old
                .levels
                .iter()
                .find(|l| l.id == previous.level_id)
                .ok_or(DomainError::UnknownReference(previous.level_id))?
                .elevation_m
                + previous.offset_m;
            let new_z = layout
                .levels
                .iter()
                .find(|l| l.id == binding.level_id)
                .ok_or(DomainError::UnknownReference(binding.level_id))?
                .elevation_m
                + binding.offset_m;
            if old_z != new_z {
                translate_bound(&mut candidate, binding.object_id, new_z - old_z)?;
            }
        }
    }
    crate::framing::regenerate_layout_dependents(&mut candidate)?;
    validate_project(&candidate)?;
    *project = candidate;
    Ok(())
}
fn translate_bound(project: &mut Project, id: Uuid, dz: f64) -> Result<(), DomainError> {
    let object = project
        .objects
        .iter()
        .find(|o| o.id == id)
        .ok_or(DomainError::UnknownReference(id))?;
    if object.extensions.contains_key(crate::surfaces::SURFACE_KEY) {
        return crate::surfaces::translate_object(project, id, [0.0, 0.0, dz]);
    }
    if object
        .extensions
        .contains_key(crate::assemblies::ASSEMBLY_KEY)
    {
        return crate::assemblies::translate_object(project, id, [0.0, 0.0, dz]);
    }
    if project
        .model
        .links
        .iter()
        .any(|link| link.physical_id == id)
        || project.ports.iter().any(|port| port.object_id == id)
    {
        return Err(invalid(
            "Peça genérica com vínculos exige edição coordenada explícita; use uma peça paramétrica.",
        ));
    }
    let object = project
        .objects
        .iter_mut()
        .find(|o| o.id == id)
        .ok_or(DomainError::UnknownReference(id))?;
    object.start_m =
        Point3::try_from([object.start_m[0], object.start_m[1], object.start_m[2] + dz])?;
    object.end_m = Point3::try_from([object.end_m[0], object.end_m[1], object.end_m[2] + dz])?;
    Ok(())
}
pub fn register_ids(project: &Project, ids: &mut BTreeSet<Uuid>) -> Result<(), DomainError> {
    let state = state(project)?;
    for id in state
        .levels
        .iter()
        .map(|x| x.id)
        .chain(state.axes.iter().map(|x| x.id))
        .chain(state.frames.iter().map(|x| x.id))
    {
        crate::insert_id(ids, id)?;
    }
    Ok(())
}
pub fn active_values(project: &Project) -> Result<BTreeMap<Uuid, serde_json::Value>, DomainError> {
    let layout = state(project)?;
    let mut values = BTreeMap::new();
    for level in &layout.levels {
        let bindings: Vec<_> = layout
            .bindings
            .iter()
            .filter(|b| b.level_id == level.id)
            .collect();
        values.insert(
            level.id,
            serde_json::to_value((level, bindings)).map_err(|e| invalid(e.to_string()))?,
        );
    }
    for axis in &layout.axes {
        values.insert(
            axis.id,
            serde_json::to_value(axis).map_err(|e| invalid(e.to_string()))?,
        );
    }
    for frame in &layout.frames {
        values.insert(
            frame.id,
            serde_json::to_value(frame).map_err(|e| invalid(e.to_string()))?,
        );
    }
    Ok(values)
}
pub fn validate_project(project: &Project) -> Result<(), DomainError> {
    let layout = state(project)?;
    if layout.levels.len() + layout.axes.len() + layout.frames.len() + layout.bindings.len() > LIMIT
    {
        return Err(DomainError::ModelLimit);
    }
    let mut ids = BTreeSet::new();
    register_ids(project, &mut ids)?;
    for level in &layout.levels {
        validate_name(&level.name)?;
        if parse_length(&level.original_expression)? != level.elevation_m {
            return Err(invalid(
                "Cota do nível não corresponde à expressão registrada.",
            ));
        }
    }
    for frame in &layout.frames {
        validate_name(&frame.name)?;
        angle(frame.angle_degrees)?;
    }
    for axis in &layout.axes {
        resolve_axis_in(&layout, axis.id)?;
    }
    let mut bound = BTreeSet::new();
    for binding in &layout.bindings {
        if !bound.insert(binding.object_id) {
            return Err(invalid("Peça vinculada a mais de um nível."));
        }
        let object = project
            .objects
            .iter()
            .find(|o| o.id == binding.object_id)
            .ok_or(DomainError::UnknownReference(binding.object_id))?;
        if object.extensions.contains_key(crate::framing::FRAMING_KEY)
            || crate::assemblies::is_child(project, binding.object_id)?
        {
            return Err(invalid(
                "Vínculo de nível não pode apontar a filho de conjunto ou peça com pontos paramétricos próprios.",
            ));
        }
        let level = layout
            .levels
            .iter()
            .find(|l| l.id == binding.level_id)
            .ok_or(DomainError::UnknownReference(binding.level_id))?;
        let expected = level.elevation_m + binding.offset_m;
        if !binding.offset_m.is_finite() || (object.start_m[2] - expected).abs() > 1.0e-8 {
            return Err(invalid(
                "Geometria da peça diverge do nível associado. Edite o nível ou desvincule a peça.",
            ));
        }
    }
    Ok(())
}
