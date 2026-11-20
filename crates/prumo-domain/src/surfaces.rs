//! Editable planar surfaces, material partitions and candidate analytical regions.
//!
//! No finite-element stiffness or design result is produced here. Openings are
//! removed before triangulation and every volume component is counted once.
use crate::geometry::{FaceInput, inspect_face, project_tolerances};
use crate::model::{PhysicalAnalyticalLink, PhysicalRole, record_identity};
use crate::units::Point3;
use crate::{Discipline, DomainError, ModelState, PhysicalObject, Project, validate_name};
use geo::{
    Area, BooleanOps, BoundingRect, Contains, Coord, Intersects, LineString, MultiPolygon, Point,
    Polygon, TriangulateEarcut, Validation,
};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const SURFACE_KEY: &str = "prumo.surface.v1";
const MAX_VERTICES: usize = 512;
const MAX_PARTITIONS: usize = 256;
const AREA_TOLERANCE: f64 = 1.0e-8;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalPoint {
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub x_m: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub y_m: f64,
}

impl LocalPoint {
    pub const fn new(x_m: f64, y_m: f64) -> Self {
        Self { x_m, y_m }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceFrame {
    pub origin_m: Point3,
    #[serde(deserialize_with = "crate::numeric::vector3")]
    pub u: [f64; 3],
    #[serde(deserialize_with = "crate::numeric::vector3")]
    pub v: [f64; 3],
}

impl Default for SurfaceFrame {
    fn default() -> Self {
        Self {
            origin_m: Point3::ORIGIN,
            u: [1., 0., 0.],
            v: [0., 1., 0.],
        }
    }
}

impl SurfaceFrame {
    pub fn normal(&self) -> [f64; 3] {
        Vector3::from(self.u).cross(&Vector3::from(self.v)).into()
    }
    pub fn validate(&self) -> Result<(), DomainError> {
        crate::validate_point(self.origin_m.as_array())?;
        let u = Vector3::from(self.u);
        let v = Vector3::from(self.v);
        if u.iter().chain(v.iter()).any(|x| !x.is_finite())
            || (u.norm_squared() - 1.).abs() > 1e-12
            || (v.norm_squared() - 1.).abs() > 1e-12
            || u.dot(&v).abs() > 1e-12
        {
            return Err(invalid(
                "Os eixos locais da superfície devem ser unitários e perpendiculares.",
            ));
        }
        Ok(())
    }
    pub fn point(&self, local: LocalPoint, offset_m: f64) -> Result<Point3, DomainError> {
        point3(
            Vector3::from(self.origin_m.as_array())
                + Vector3::from(self.u) * local.x_m
                + Vector3::from(self.v) * local.y_m
                + Vector3::from(self.normal()) * offset_m,
        )
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceOpening {
    pub name: String,
    pub outline: Vec<LocalPoint>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThicknessRegion {
    pub name: String,
    pub outline: Vec<LocalPoint>,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub thickness_m: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub offset_m: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SurfaceKind {
    SolidSlab {},
    Panel {},
    Cantilever {},
    Ribbed {
        #[serde(deserialize_with = "crate::numeric::scalar")]
        rib_width_m: f64,
        #[serde(deserialize_with = "crate::numeric::scalar")]
        spacing_x_m: f64,
        #[serde(deserialize_with = "optional_scalar")]
        spacing_y_m: Option<f64>,
        #[serde(deserialize_with = "crate::numeric::scalar")]
        extra_depth_m: f64,
    },
    Grillage {
        #[serde(deserialize_with = "crate::numeric::scalar")]
        rib_width_m: f64,
        #[serde(deserialize_with = "crate::numeric::scalar")]
        spacing_x_m: f64,
        #[serde(deserialize_with = "crate::numeric::scalar")]
        spacing_y_m: f64,
        #[serde(deserialize_with = "crate::numeric::scalar")]
        depth_m: f64,
    },
}

fn optional_scalar<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<f64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(transparent)]
    struct Number(#[serde(deserialize_with = "crate::numeric::scalar")] f64);
    Ok(Option::<Number>::deserialize(deserializer)?.map(|x| x.0))
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSupport {
    pub object_id: Uuid,
    pub point: LocalPoint,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EdgeBehavior {
    Free {},
    CandidateSupport { object_id: Uuid },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeCondition {
    pub edge: usize,
    pub behavior: EdgeBehavior,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundarySegment {
    pub edge: usize,
    pub start_m: Point3,
    pub end_m: Point3,
    pub behavior: EdgeBehavior,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSpec {
    pub name: String,
    pub role: PhysicalRole,
    pub material_id: Option<Uuid>,
    pub environment_id: Option<Uuid>,
    pub frame: SurfaceFrame,
    pub kind: SurfaceKind,
    pub outline: Vec<LocalPoint>,
    pub openings: Vec<SurfaceOpening>,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub thickness_m: f64,
    pub regions: Vec<ThicknessRegion>,
    pub supports: Vec<SurfaceSupport>,
    /// An omitted edge is free. A supported edge always names its candidate support.
    #[serde(default)]
    pub edge_conditions: Vec<EdgeCondition>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum SurfaceEdit {
    Create {
        surface: SurfaceSpec,
    },
    Update {
        object_id: Uuid,
        surface: SurfaceSpec,
    },
    SetOpenings {
        object_id: Uuid,
        openings: Vec<SurfaceOpening>,
    },
    SetSupports {
        object_id: Uuid,
        supports: Vec<SurfaceSupport>,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfacePatch {
    pub component: String,
    /// Disjoint material footprint for this layer; openings are already removed.
    pub triangles_m: Vec<[[f64; 3]; 3]>,
    pub area_m2: f64,
    pub thickness_m: f64,
    pub offset_m: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportContact {
    pub object_id: Uuid,
    pub point_m: Point3,
    pub material_present: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceRegion {
    pub id: Uuid,
    pub physical_id: Uuid,
    pub normal: [f64; 3],
    pub local_u: [f64; 3],
    pub local_v: [f64; 3],
    pub patches: Vec<SurfacePatch>,
    pub supports: Vec<SupportContact>,
    pub boundaries: Vec<BoundarySegment>,
    /// Always false until a later qualified solver consumes these regions.
    pub engineering_release: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceRecord {
    pub physical_id: Uuid,
    pub surface: SurfaceSpec,
    pub region: Option<SurfaceRegion>,
    pub link_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct QuantityComponent {
    pub component: String,
    pub area_m2: f64,
    pub thickness_m: f64,
    pub volume_m3: f64,
    pub mass_kg: Option<f64>,
    pub self_weight_n: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SurfaceQuantities {
    pub components: Vec<QuantityComponent>,
    pub total_volume_m3: f64,
    pub total_mass_kg: Option<f64>,
    pub density_kg_m3: Option<f64>,
    pub gravity_m_s2: f64,
    pub material_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SurfacePreview {
    pub patches: Vec<SurfacePatch>,
    pub supports: Vec<SupportContact>,
    pub boundaries: Vec<BoundarySegment>,
    pub quantities: SurfaceQuantities,
    pub net_area_m2: f64,
    pub opening_area_m2: f64,
    pub mesh_rebuild_required: bool,
    pub punching_review_required: bool,
    pub warnings: Vec<String>,
    pub engineering_release: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SurfaceInspection {
    pub object_id: Uuid,
    pub region_id: Option<Uuid>,
    pub surface: SurfaceSpec,
    pub preview: SurfacePreview,
    pub sources: Vec<crate::dependencies::SourceKey>,
}

fn invalid(text: &str) -> DomainError {
    DomainError::InvalidModel(text.into())
}
fn encoded<T: Serialize>(value: &T) -> Result<Value, DomainError> {
    serde_json::to_value(value).map_err(|e| invalid(&e.to_string()))
}
fn positive(value: f64, what: &str) -> Result<(), DomainError> {
    if !value.is_finite() || value <= 0. || value > 1.0e6 {
        Err(invalid(what))
    } else {
        Ok(())
    }
}

pub fn metadata(object: &PhysicalObject) -> Result<Option<SurfaceRecord>, DomainError> {
    object
        .extensions
        .get(SURFACE_KEY)
        .map(|value| {
            serde_json::from_value(value.clone())
                .map_err(|e| invalid(&format!("Registro de superfície inválido: {e}")))
        })
        .transpose()
}

fn polygon(points: &[LocalPoint]) -> Result<Polygon<f64>, DomainError> {
    if !(3..=MAX_VERTICES).contains(&points.len()) {
        return Err(invalid(
            "O contorno deve conter de 3 a 512 vértices, sem repetir o fechamento.",
        ));
    }
    for point in points {
        if !point.x_m.is_finite()
            || !point.y_m.is_finite()
            || point.x_m.abs() > crate::MAX_COORDINATE_M
            || point.y_m.abs() > crate::MAX_COORDINATE_M
        {
            return Err(invalid("Coordenada local inválida."));
        }
    }
    if points
        .iter()
        .enumerate()
        .any(|(i, p)| points[..i].contains(p))
    {
        return Err(invalid("O contorno possui vértices duplicados."));
    }
    let poly = Polygon::new(
        LineString::new(
            points
                .iter()
                .map(|p| Coord { x: p.x_m, y: p.y_m })
                .collect(),
        ),
        vec![],
    );
    if poly.check_validation().is_err() || poly.unsigned_area() <= 1e-12 {
        return Err(invalid("Contorno degenerado ou autointersectante."));
    }
    Ok(poly)
}

fn net_polygon(spec: &SurfaceSpec) -> Result<(Polygon<f64>, Polygon<f64>), DomainError> {
    let outer = polygon(&spec.outline)?;
    if spec.openings.len() > MAX_PARTITIONS
        || spec.regions.len() > MAX_PARTITIONS
        || spec.supports.len() > MAX_PARTITIONS
        || spec.edge_conditions.len() > spec.outline.len()
        || spec.outline.len()
            + spec.openings.iter().map(|o| o.outline.len()).sum::<usize>()
            + spec.regions.iter().map(|r| r.outline.len()).sum::<usize>()
            > 4096
    {
        return Err(crate::DomainError::ModelLimit);
    }
    let mut holes = Vec::new();
    let mut names = BTreeSet::new();
    for opening in &spec.openings {
        validate_name(&opening.name)?;
        if !names.insert(&opening.name) {
            return Err(invalid("As aberturas devem ter nomes distintos."));
        }
        let hole = polygon(&opening.outline)?;
        if !outer.contains(&hole) || outer.exterior().intersects(hole.exterior()) {
            return Err(invalid(
                "Uma abertura deve ficar dentro do contorno. Para recortes na borda, edite o contorno externo.",
            ));
        }
        if holes
            .iter()
            .any(|ring: &LineString<f64>| ring.intersects(hole.exterior()))
        {
            return Err(invalid("Aberturas não podem se tocar ou se cruzar."));
        }
        holes.push(hole.exterior().clone());
    }
    let net = Polygon::new(outer.exterior().clone(), holes);
    if net.check_validation().is_err() || net.unsigned_area() <= 1e-12 {
        return Err(invalid(
            "Aberturas sobrepostas, tocantes ou sem área resistente remanescente.",
        ));
    }
    Ok((outer, net))
}

fn append_patch(
    patches: &mut Vec<SurfacePatch>,
    geometry: &MultiPolygon<f64>,
    spec: &SurfaceSpec,
    component: String,
    thickness_m: f64,
    offset_m: f64,
) -> Result<(), DomainError> {
    let area = geometry.unsigned_area();
    if area <= 1e-12 {
        return Ok(());
    }
    let mut triangles = Vec::new();
    let mut triangulated = 0.;
    for poly in &geometry.0 {
        for tri in poly.earcut_triangles() {
            let local = tri.to_array();
            let signed = (local[1].x - local[0].x) * (local[2].y - local[0].y)
                - (local[1].y - local[0].y) * (local[2].x - local[0].x);
            if signed.abs() <= 1e-20 {
                return Err(invalid("A triangulação gerou região degenerada."));
            }
            triangulated += signed.abs() * 0.5;
            let mut points = [
                spec.frame
                    .point(LocalPoint::new(local[0].x, local[0].y), offset_m)?
                    .as_array(),
                spec.frame
                    .point(LocalPoint::new(local[1].x, local[1].y), offset_m)?
                    .as_array(),
                spec.frame
                    .point(LocalPoint::new(local[2].x, local[2].y), offset_m)?
                    .as_array(),
            ];
            if signed < 0. {
                points.swap(1, 2);
            }
            triangles.push(points);
        }
    }
    if (area - triangulated).abs() > AREA_TOLERANCE * area.max(1.) {
        return Err(invalid(
            "A triangulação não conservou a área material. A região não foi aceita.",
        ));
    }
    patches.push(SurfacePatch {
        component,
        triangles_m: triangles,
        area_m2: area,
        thickness_m,
        offset_m,
    });
    Ok(())
}

fn rectangular(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon<f64> {
    Polygon::new(
        LineString::new(vec![
            Coord { x: x0, y: y0 },
            Coord { x: x1, y: y0 },
            Coord { x: x1, y: y1 },
            Coord { x: x0, y: y1 },
        ]),
        vec![],
    )
}

struct RibLayout {
    width: f64,
    spacing_x: f64,
    spacing_y: Option<f64>,
    depth: f64,
    offset: f64,
}

fn append_ribs(
    patches: &mut Vec<SurfacePatch>,
    spec: &SurfaceSpec,
    net: &Polygon<f64>,
    layout: RibLayout,
) -> Result<(), DomainError> {
    let RibLayout {
        width,
        spacing_x: sx,
        spacing_y: sy,
        depth,
        offset,
    } = layout;
    positive(width, "Largura de nervura inválida.")?;
    positive(sx, "Espaçamento de nervura inválido.")?;
    positive(depth, "Altura de nervura inválida.")?;
    if width >= sx {
        return Err(invalid(
            "A largura de nervura deve ser menor que o espaçamento.",
        ));
    }
    if let Some(sy) = sy {
        positive(sy, "Espaçamento de nervura inválido.")?;
        if width >= sy {
            return Err(invalid(
                "A largura de nervura deve ser menor que o espaçamento.",
            ));
        }
    }
    let bounds = net
        .bounding_rect()
        .ok_or_else(|| invalid("Sem limite de superfície."))?;
    let mut claimed = MultiPolygon::<f64>::new(vec![]);
    for (axis, spacing) in [(0, Some(sx)), (1, sy)] {
        let Some(spacing) = spacing else { continue };
        let (min, max) = if axis == 0 {
            (bounds.min().x, bounds.max().x)
        } else {
            (bounds.min().y, bounds.max().y)
        };
        let intervals = ((max - min) / spacing).floor();
        if !intervals.is_finite() || intervals < 0. || intervals >= MAX_PARTITIONS as f64 {
            return Err(invalid(
                "Mais de 256 nervuras por direção. Divida a superfície.",
            ));
        }
        let count = intervals as usize + 1;
        for index in 0..count {
            let location = min + index as f64 * spacing;
            let band = if axis == 0 {
                rectangular(
                    location - width * 0.5,
                    bounds.min().y,
                    location + width * 0.5,
                    bounds.max().y,
                )
            } else {
                rectangular(
                    bounds.min().x,
                    location - width * 0.5,
                    bounds.max().x,
                    location + width * 0.5,
                )
            };
            let within = net.intersection(&band);
            let exclusive = within.difference(&claimed);
            let label = format!(
                "Nervura {}{}: material sem cruzamentos já contados",
                if axis == 0 { "X" } else { "Y" },
                index + 1
            );
            let mut ordinary = exclusive.clone();
            for region in &spec.regions {
                let region_polygon = polygon(&region.outline)?;
                let portion = exclusive.intersection(&region_polygon);
                if portion.unsigned_area() <= 1e-12 {
                    continue;
                }
                ordinary = ordinary.difference(&region_polygon);
                let bottom = offset - depth * 0.5;
                let top = offset + depth * 0.5;
                let region_bottom = region.offset_m - region.thickness_m * 0.5;
                let region_top = region.offset_m + region.thickness_m * 0.5;
                // Subtract only the overlap in normal coordinates. This avoids
                // counting a drop panel and the rib volume in the same solid.
                if region_top <= bottom || region_bottom >= top {
                    append_patch(
                        patches,
                        &portion,
                        spec,
                        format!("{label}, junto a {}", region.name),
                        depth,
                        offset,
                    )?;
                } else {
                    for (a, b) in [
                        (bottom, top.min(region_bottom)),
                        (bottom.max(region_top), top),
                    ] {
                        if b - a > 1e-12 {
                            append_patch(
                                patches,
                                &portion,
                                spec,
                                format!("{label}, fora do volume de {}", region.name),
                                b - a,
                                0.5 * (a + b),
                            )?;
                        }
                    }
                }
            }
            append_patch(patches, &ordinary, spec, label, depth, offset)?;
            claimed = claimed.union(&within);
        }
    }
    Ok(())
}

pub fn preview(project: &Project, surface: &SurfaceSpec) -> Result<SurfacePreview, DomainError> {
    validate_name(&surface.name)?;
    surface.frame.validate()?;
    if let Some(id) = surface.environment_id
        && !project.environments.iter().any(|e| e.id == id)
    {
        return Err(DomainError::UnknownReference(id));
    }
    let density = match surface.material_id {
        Some(id) => Some(
            project
                .model
                .materials
                .iter()
                .find(|m| m.id == id)
                .ok_or(DomainError::UnknownReference(id))?
                .density
                .si(),
        ),
        None => None,
    };
    if surface.role == PhysicalRole::AnalyticalSource && density.is_none() {
        return Err(invalid(
            "Selecione o material antes de promover a superfície estrutural.",
        ));
    }
    let (outer, net) = net_polygon(surface)?;
    // The plane is checked independently through the qualified geometric adapter.
    let face = FaceInput {
        vertices_m: surface
            .outline
            .iter()
            .map(|p| surface.frame.point(*p, 0.).map(Point3::as_array))
            .collect::<Result<_, _>>()?,
        expected_normal: surface.frame.normal(),
    };
    let inspection = inspect_face(&face, project_tolerances(project)?)?;
    // Normalize contour winding for topology, but reject an inverted declared face.
    if !inspection.valid {
        return Err(invalid(&format!(
            "Face inválida: {}",
            inspection.problems.join(" ")
        )));
    }
    let mut patches = Vec::new();
    let mut remaining = MultiPolygon::new(vec![net.clone()]);
    let mut used = MultiPolygon::new(vec![]);
    let mut region_names = BTreeSet::new();
    for region in &surface.regions {
        validate_name(&region.name)?;
        positive(region.thickness_m, "Espessura de região inválida.")?;
        if !region.offset_m.is_finite() || region.offset_m.abs() > 1e6 {
            return Err(invalid("Deslocamento normal de região inválido."));
        }
        if !region_names.insert(&region.name) {
            return Err(invalid("Regiões de espessura devem ter nomes distintos."));
        }
        let poly = polygon(&region.outline)?;
        let covers_outline = region.outline == surface.outline;
        if !covers_outline && poly.difference(&outer).unsigned_area() > AREA_TOLERANCE {
            return Err(invalid("Região de espessura fora do contorno físico."));
        }
        // An identical contour is already the complete net surface, including
        // its holes. Avoid a redundant boolean round trip, whose internal
        // coordinate conversion can perturb an irrational flight length.
        let clipped = if covers_outline {
            MultiPolygon::new(vec![net.clone()])
        } else {
            poly.intersection(&net)
        };
        if clipped.unsigned_area() <= 1e-12
            || (!used.0.is_empty() && clipped.intersection(&used).unsigned_area() > AREA_TOLERANCE)
        {
            return Err(invalid(
                "Regiões de espessura sobrepostas ou inteiramente removidas por abertura.",
            ));
        }
        append_patch(
            &mut patches,
            &clipped,
            surface,
            region.name.clone(),
            region.thickness_m,
            region.offset_m,
        )?;
        if covers_outline {
            remaining = MultiPolygon::new(vec![]);
            used = clipped;
        } else {
            remaining = remaining.difference(&clipped);
            used = if used.0.is_empty() {
                clipped
            } else {
                used.union(&clipped)
            };
        }
    }
    match surface.kind {
        SurfaceKind::Grillage {
            rib_width_m,
            spacing_x_m,
            spacing_y_m,
            depth_m,
        } => {
            if surface.thickness_m != 0. {
                return Err(invalid(
                    "Grelha sem capa exige espessura nominal da capa zero.",
                ));
            }
            append_ribs(
                &mut patches,
                surface,
                &net,
                RibLayout {
                    width: rib_width_m,
                    spacing_x: spacing_x_m,
                    spacing_y: Some(spacing_y_m),
                    depth: depth_m,
                    offset: 0.,
                },
            )?;
        }
        _ => {
            positive(surface.thickness_m, "Espessura da superfície inválida.")?;
            append_patch(
                &mut patches,
                &remaining,
                surface,
                "Capa ou laje: área líquida sem regiões substituídas".into(),
                surface.thickness_m,
                0.,
            )?;
            if let SurfaceKind::Ribbed {
                rib_width_m,
                spacing_x_m,
                spacing_y_m,
                extra_depth_m,
            } = surface.kind
            {
                append_ribs(
                    &mut patches,
                    surface,
                    &net,
                    RibLayout {
                        width: rib_width_m,
                        spacing_x: spacing_x_m,
                        spacing_y: spacing_y_m,
                        depth: extra_depth_m,
                        offset: -0.5 * (surface.thickness_m + extra_depth_m),
                    },
                )?;
            }
        }
    }
    let mut supports = Vec::new();
    let mut support_pairs = BTreeSet::new();
    let mut warnings = Vec::new();
    for support in &surface.supports {
        let target = project
            .objects
            .iter()
            .find(|o| o.id == support.object_id)
            .ok_or(DomainError::UnknownReference(support.object_id))?;
        if target.discipline != Discipline::Structure
            || target.role != PhysicalRole::AnalyticalSource
        {
            return Err(invalid(
                "Apoio candidato precisa referenciar uma peça estrutural promovida.",
            ));
        }
        if !support_pairs.insert((
            support.object_id,
            support.point.x_m.to_bits(),
            support.point.y_m.to_bits(),
        )) {
            return Err(invalid("Apoio candidato repetido."));
        }
        let p = Point::new(support.point.x_m, support.point.y_m);
        if !outer.intersects(&p) {
            return Err(invalid(
                "Ponto declarado de apoio fora do contorno externo.",
            ));
        }
        let material_present =
            net.intersects(&p) && material_at(&patches, &surface.frame, support.point);
        if !material_present {
            warnings.push(format!("O apoio {} está em área sem material, por abertura ou vazio entre nervuras. Não há ligação resistente liberada.",target.name));
        }
        supports.push(SupportContact {
            object_id: support.object_id,
            point_m: surface.frame.point(support.point, 0.)?,
            material_present,
        });
    }
    let mut boundaries = Vec::new();
    let mut edge_ids = BTreeSet::new();
    for condition in &surface.edge_conditions {
        if condition.edge >= surface.outline.len() || !edge_ids.insert(condition.edge) {
            return Err(invalid("Borda repetida ou inexistente."));
        }
        if let EdgeBehavior::CandidateSupport { object_id } = condition.behavior {
            let object = project
                .objects
                .iter()
                .find(|o| o.id == object_id)
                .ok_or(DomainError::UnknownReference(object_id))?;
            if object.discipline != Discipline::Structure
                || object.role != PhysicalRole::AnalyticalSource
            {
                return Err(invalid(
                    "Borda apoiada deve indicar uma peça estrutural promovida.",
                ));
            }
        }
    }
    for (edge, p) in surface.outline.iter().enumerate() {
        boundaries.push(BoundarySegment {
            edge,
            start_m: surface.frame.point(*p, 0.)?,
            end_m: surface
                .frame
                .point(surface.outline[(edge + 1) % surface.outline.len()], 0.)?,
            behavior: surface
                .edge_conditions
                .iter()
                .find(|c| c.edge == edge)
                .map_or(EdgeBehavior::Free {}, |c| c.behavior.clone()),
        });
    }
    let gravity = 9.80665;
    let components: Vec<_> = patches
        .iter()
        .map(|p| {
            let volume = p.area_m2 * p.thickness_m;
            QuantityComponent {
                component: p.component.clone(),
                area_m2: p.area_m2,
                thickness_m: p.thickness_m,
                volume_m3: volume,
                mass_kg: density.map(|d| volume * d),
                self_weight_n: density.map(|d| volume * d * gravity),
            }
        })
        .collect();
    let total_volume = components.iter().map(|c| c.volume_m3).sum::<f64>();
    if !total_volume.is_finite() || total_volume <= 0. || total_volume > 1e18 {
        return Err(invalid("Volume da superfície fora do domínio."));
    }
    Ok(SurfacePreview {
        patches,
        supports,
        boundaries,
        quantities: SurfaceQuantities {
            components,
            total_volume_m3: total_volume,
            total_mass_kg: density.map(|d| d * total_volume),
            density_kg_m3: density,
            gravity_m_s2: gravity,
            material_id: surface.material_id,
        },
        net_area_m2: net.unsigned_area(),
        opening_area_m2: outer.unsigned_area() - net.unsigned_area(),
        mesh_rebuild_required: true,
        punching_review_required: !surface.supports.is_empty(),
        warnings,
        engineering_release: false,
    })
}

fn region(
    id: Uuid,
    physical_id: Uuid,
    spec: &SurfaceSpec,
    preview: SurfacePreview,
) -> SurfaceRegion {
    SurfaceRegion {
        id,
        physical_id,
        normal: spec.frame.normal(),
        local_u: spec.frame.u,
        local_v: spec.frame.v,
        patches: preview.patches,
        supports: preview.supports,
        boundaries: preview.boundaries,
        engineering_release: false,
    }
}

pub fn inspect(project: &Project, object_id: Uuid) -> Result<SurfaceInspection, DomainError> {
    let object = project
        .objects
        .iter()
        .find(|o| o.id == object_id)
        .ok_or(DomainError::UnknownReference(object_id))?;
    let record =
        metadata(object)?.ok_or_else(|| invalid("A peça não é uma superfície paramétrica."))?;
    Ok(SurfaceInspection {
        object_id,
        region_id: record.region.as_ref().map(|r| r.id),
        preview: preview(project, &record.surface)?,
        surface: record.surface,
        sources: object_sources(project, object_id)?,
    })
}

/// Exact source keys consumers must retain when producing results or documents.
pub fn object_sources(
    project: &Project,
    object_id: Uuid,
) -> Result<Vec<crate::dependencies::SourceKey>, DomainError> {
    use crate::dependencies::{SourceKey, SourceKind};
    let object = project
        .objects
        .iter()
        .find(|o| o.id == object_id)
        .ok_or(DomainError::UnknownReference(object_id))?;
    let record =
        metadata(object)?.ok_or_else(|| invalid("A peça não é uma superfície paramétrica."))?;
    let mut sources = BTreeSet::new();
    append_object_sources(project, object_id, &mut sources)?;
    if let Some(region) = &record.region {
        sources.insert(SourceKey {
            kind: SourceKind::AnalyticalRegion,
            id: region.id,
        });
    }
    if let Some(id) = record.link_id {
        sources.insert(SourceKey {
            kind: SourceKind::Link,
            id,
        });
    }
    for id in record.surface.supports.iter().map(|s| s.object_id).chain(
        record
            .surface
            .edge_conditions
            .iter()
            .filter_map(|e| match e.behavior {
                EdgeBehavior::CandidateSupport { object_id } => Some(object_id),
                _ => None,
            }),
    ) {
        append_object_sources(project, id, &mut sources)?;
    }
    sources.insert(SourceKey {
        kind: SourceKind::Coordinates,
        id: project.id,
    });
    if project
        .extensions
        .contains_key(crate::geometry::TOLERANCES_KEY)
    {
        sources.insert(SourceKey {
            kind: SourceKind::GeometrySettings,
            id: project.id,
        });
    }
    Ok(sources.into_iter().collect())
}

pub(crate) fn append_object_sources(
    project: &Project,
    id: Uuid,
    sources: &mut BTreeSet<crate::dependencies::SourceKey>,
) -> Result<(), DomainError> {
    use crate::dependencies::{SourceKey, SourceKind};
    let object = project
        .objects
        .iter()
        .find(|o| o.id == id)
        .ok_or(DomainError::UnknownReference(id))?;
    sources.insert(SourceKey {
        kind: SourceKind::Physical,
        id,
    });
    sources.insert(SourceKey {
        kind: SourceKind::PhysicalGeometry,
        id,
    });
    if let Some(id) = object.material_id {
        sources.insert(SourceKey {
            kind: SourceKind::Material,
            id,
        });
    }
    if crate::framing::metadata(object)?.is_some() {
        sources.extend(crate::framing::object_sources(project, id)?);
    }
    Ok(())
}

fn reference_endpoints(spec: &SurfaceSpec) -> Result<(Point3, Point3), DomainError> {
    let first = spec.frame.point(spec.outline[0], 0.)?;
    let farthest = spec
        .outline
        .iter()
        .map(|p| spec.frame.point(*p, 0.))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max_by(|a, b| first.distance(*a).total_cmp(&first.distance(*b)))
        .ok_or_else(|| invalid("Contorno vazio."))?;
    Ok((first, farthest))
}

pub(crate) fn retire_identity(
    project: &mut Project,
    id: Uuid,
    revision: u64,
) -> Result<(), DomainError> {
    let identity = project
        .model
        .identities
        .get_mut(&id)
        .ok_or(DomainError::UnknownReference(id))?;
    if identity.created_revision == Some(revision) {
        project.model.identities.remove(&id);
    } else {
        identity.retired_revision = Some(revision);
    }
    Ok(())
}

pub(crate) fn put_surface(
    project: &mut Project,
    id: Option<Uuid>,
    spec: SurfaceSpec,
    actor: &str,
    revision: u64,
) -> Result<Uuid, DomainError> {
    let prepared = preview(project, &spec)?;
    let (start, end) = reference_endpoints(&spec)?;
    let physical_id = id.unwrap_or_else(Uuid::new_v4);
    if spec.supports.iter().any(|s|s.object_id==physical_id) || spec.edge_conditions.iter().any(|c|matches!(c.behavior,EdgeBehavior::CandidateSupport{object_id} if object_id==physical_id)) { return Err(invalid("Uma superfície não pode servir de apoio a si mesma.")); }
    let previous = if let Some(id) = id {
        let object = project
            .objects
            .iter()
            .find(|o| o.id == id)
            .ok_or(DomainError::UnknownReference(id))?;
        Some(metadata(object)?.ok_or_else(|| invalid("A peça não é uma superfície paramétrica."))?)
    } else {
        None
    };
    let old_region = previous
        .as_ref()
        .and_then(|p| p.region.as_ref())
        .map(|r| r.id);
    let old_link = previous.as_ref().and_then(|p| p.link_id);
    let (analytical, link_id) = if spec.role == PhysicalRole::AnalyticalSource {
        let region_id = old_region.unwrap_or_else(Uuid::new_v4);
        let link_id = old_link.unwrap_or_else(Uuid::new_v4);
        if old_region.is_none() {
            record_identity(project, region_id, actor, revision, false)?;
        }
        if old_link.is_none() {
            record_identity(project, link_id, actor, revision, false)?;
            project.model.links.push(PhysicalAnalyticalLink {
                id: link_id,
                physical_id,
                analytical_id: region_id,
            });
        }
        (
            Some(region(region_id, physical_id, &spec, prepared)),
            Some(link_id),
        )
    } else {
        if let Some(id) = old_link {
            project.model.links.retain(|l| l.id != id);
            retire_identity(project, id, revision)?;
        }
        if let Some(id) = old_region {
            retire_identity(project, id, revision)?;
        }
        (None, None)
    };
    let record = SurfaceRecord {
        physical_id,
        surface: spec.clone(),
        region: analytical,
        link_id,
    };
    if let Some(object) = project.objects.iter_mut().find(|o| o.id == physical_id) {
        object.name = spec.name;
        object.start_m = start;
        object.end_m = end;
        object.material_id = spec.material_id;
        object.environment_id = spec.environment_id;
        object.role = spec.role;
        object
            .extensions
            .insert(SURFACE_KEY.into(), encoded(&record)?);
    } else {
        record_identity(project, physical_id, actor, revision, true)?;
        project.objects.push(PhysicalObject {
            id: physical_id,
            name: spec.name,
            discipline: Discipline::Structure,
            start_m: start,
            end_m: end,
            state: ModelState::Modelled,
            role: spec.role,
            material_id: spec.material_id,
            environment_id: spec.environment_id,
            extensions: BTreeMap::from([(SURFACE_KEY.into(), encoded(&record)?)]),
        });
    }
    Ok(physical_id)
}

pub fn apply(
    project: &mut Project,
    command: SurfaceEdit,
    actor: &str,
    revision: u64,
) -> Result<(), DomainError> {
    let mut candidate = project.clone();
    match command {
        SurfaceEdit::Create { surface } => {
            put_surface(&mut candidate, None, surface, actor, revision)?;
        }
        SurfaceEdit::Update { object_id, surface } => {
            ensure_independent(&candidate, object_id)?;
            put_surface(&mut candidate, Some(object_id), surface, actor, revision)?;
        }
        SurfaceEdit::SetOpenings {
            object_id,
            openings,
        } => {
            ensure_independent(&candidate, object_id)?;
            let mut surface = inspect(&candidate, object_id)?.surface;
            surface.openings = openings;
            put_surface(&mut candidate, Some(object_id), surface, actor, revision)?;
        }
        SurfaceEdit::SetSupports {
            object_id,
            supports,
        } => {
            ensure_independent(&candidate, object_id)?;
            let mut surface = inspect(&candidate, object_id)?.surface;
            surface.supports = supports;
            put_surface(&mut candidate, Some(object_id), surface, actor, revision)?;
        }
    }
    validate_project(&candidate)?;
    *project = candidate;
    Ok(())
}

fn ensure_independent(project: &Project, id: Uuid) -> Result<(), DomainError> {
    if crate::assemblies::is_child(project, id)? {
        Err(invalid(
            "Edite esta superfície no conjunto de parede, escada ou rampa para preservar a continuidade.",
        ))
    } else {
        Ok(())
    }
}

pub fn analytical_region_ids(project: &Project) -> Result<BTreeSet<Uuid>, DomainError> {
    let mut ids = BTreeSet::new();
    for object in &project.objects {
        if let Some(record) = metadata(object)?
            && let Some(region) = record.region
            && !ids.insert(region.id)
        {
            return Err(DomainError::DuplicateIdentity(region.id));
        }
    }
    Ok(ids)
}

pub fn register_ids(project: &Project, ids: &mut BTreeSet<Uuid>) -> Result<(), DomainError> {
    for id in analytical_region_ids(project)? {
        if id.is_nil() || !ids.insert(id) {
            return Err(DomainError::DuplicateIdentity(id));
        }
    }
    Ok(())
}

pub fn active_values(project: &Project) -> Result<BTreeMap<Uuid, Value>, DomainError> {
    let mut values = BTreeMap::new();
    for object in &project.objects {
        if let Some(record) = metadata(object)?
            && let Some(region) = record.region
        {
            values.insert(region.id, encoded(&region)?);
        }
    }
    Ok(values)
}

pub fn validate_project(project: &Project) -> Result<(), DomainError> {
    for object in &project.objects {
        let Some(record) = metadata(object)? else {
            continue;
        };
        let spec = &record.surface;
        let expected = preview(project, spec)?;
        let (start, end) = reference_endpoints(spec)?;
        if record.physical_id != object.id
            || object.name != spec.name
            || object.role != spec.role
            || object.discipline != Discipline::Structure
            || object.material_id != spec.material_id
            || object.environment_id != spec.environment_id
            || object.start_m != start
            || object.end_m != end
        {
            return Err(invalid(
                "Geometria ou identidade de superfície diverge dos parâmetros. Use o editor paramétrico.",
            ));
        }
        match (&record.region, record.link_id, spec.role) {
            (Some(actual), Some(link_id), PhysicalRole::AnalyticalSource) => {
                if *actual != region(actual.id, object.id, spec, expected)
                    || project
                        .model
                        .links
                        .iter()
                        .filter(|l| l.physical_id == object.id)
                        .count()
                        != 1
                    || !project.model.links.iter().any(|l| {
                        l.id == link_id
                            && l.physical_id == object.id
                            && l.analytical_id == actual.id
                    })
                {
                    return Err(invalid(
                        "A região analítica ou seu vínculo não acompanha a superfície física.",
                    ));
                }
            }
            (None, None, PhysicalRole::VisualOnly) => {
                if project
                    .model
                    .links
                    .iter()
                    .any(|l| l.physical_id == object.id)
                {
                    return Err(invalid(
                        "Referência arquitetônica possui vínculo analítico indevido.",
                    ));
                }
            }
            _ => return Err(invalid("Estado analítico da superfície incoerente.")),
        }
    }
    Ok(())
}

pub fn display_polylines(
    project: &Project,
    object_id: Uuid,
) -> Result<Vec<Vec<Point3>>, DomainError> {
    let Some(object) = project.objects.iter().find(|o| o.id == object_id) else {
        return Ok(vec![]);
    };
    let Some(record) = metadata(object)? else {
        return Ok(vec![]);
    };
    let s = record.surface;
    let mut lines = Vec::new();
    for outline in std::iter::once(&s.outline).chain(s.openings.iter().map(|o| &o.outline)) {
        let mut line = outline
            .iter()
            .map(|p| s.frame.point(*p, 0.))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(first) = line.first().copied() {
            line.push(first);
        }
        lines.push(line);
    }
    for region in &s.regions {
        let mut line = region
            .outline
            .iter()
            .map(|p| s.frame.point(*p, region.offset_m))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(first) = line.first().copied() {
            line.push(first);
        }
        lines.push(line);
    }
    if matches!(
        s.kind,
        SurfaceKind::Ribbed { .. } | SurfaceKind::Grillage { .. }
    ) {
        for patch in preview(project, &s)?.patches {
            if patch.component.starts_with("Nervura") {
                for triangle in patch.triangles_m {
                    let mut line = triangle
                        .into_iter()
                        .map(Point3::try_from)
                        .collect::<Result<Vec<_>, _>>()?;
                    line.push(line[0]);
                    lines.push(line);
                }
            }
        }
    }
    Ok(lines)
}

pub fn translate_object(
    project: &mut Project,
    object_id: Uuid,
    delta: [f64; 3],
) -> Result<(), DomainError> {
    let object = project
        .objects
        .iter()
        .find(|o| o.id == object_id)
        .ok_or(DomainError::UnknownReference(object_id))?;
    let Some(record) = metadata(object)? else {
        return Err(invalid("A peça não é uma superfície paramétrica."));
    };
    let mut surface = record.surface;
    surface.frame.origin_m =
        point3(Vector3::from(surface.frame.origin_m.as_array()) + Vector3::from(delta))?;
    let revision = project
        .revision
        .checked_add(1)
        .ok_or(DomainError::InvalidHistory)?;
    put_surface(
        project,
        Some(object_id),
        surface,
        "deslocamento-parametrico",
        revision,
    )?;
    Ok(())
}

pub(crate) fn remove_surface(
    project: &mut Project,
    object_id: Uuid,
    revision: u64,
) -> Result<(), DomainError> {
    let object = project
        .objects
        .iter()
        .find(|o| o.id == object_id)
        .ok_or(DomainError::UnknownReference(object_id))?;
    let record =
        metadata(object)?.ok_or_else(|| invalid("A peça não é uma superfície paramétrica."))?;
    if project.ports.iter().any(|p| p.object_id == object_id) {
        return Err(invalid(
            "A superfície possui portas externas. Remova as ligações antes de retirar a peça.",
        ));
    }
    if let Some(id) = record.link_id {
        project.model.links.retain(|l| l.id != id);
        retire_identity(project, id, revision)?;
    }
    if let Some(r) = record.region {
        retire_identity(project, r.id, revision)?;
    }
    project.objects.retain(|o| o.id != object_id);
    retire_identity(project, object_id, revision)
}

fn point3(value: Vector3<f64>) -> Result<Point3, DomainError> {
    Point3::try_from([value.x, value.y, value.z])
}

fn material_at(patches: &[SurfacePatch], frame: &SurfaceFrame, p: LocalPoint) -> bool {
    let origin = Vector3::from(frame.origin_m.as_array());
    let u = Vector3::from(frame.u);
    let v = Vector3::from(frame.v);
    patches
        .iter()
        .flat_map(|patch| &patch.triangles_m)
        .any(|triangle| {
            let t = triangle.map(|point| {
                let d = Vector3::from(point) - origin;
                [d.dot(&u), d.dot(&v)]
            });
            (0..3).all(|i| {
                let a = t[i];
                let b = t[(i + 1) % 3];
                (b[0] - a[0]) * (p.y_m - a[1]) - (b[1] - a[1]) * (p.x_m - a[0]) >= -1e-10
            })
        })
}
