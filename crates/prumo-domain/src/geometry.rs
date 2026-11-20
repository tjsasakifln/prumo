//! Geometry in engineering coordinates. Contact never creates a connection.
use crate::canonical::{digest, model_snapshot};
use crate::model::{ModelEdit, PointInput, apply_model_edit};
use crate::{Discipline, DomainError, LengthUnit, Project, validate_name, validate_point};
use geo::algorithm::kernels::{Kernel, Orientation, RobustKernel};
use geo::algorithm::line_intersection::{LineIntersection, line_intersection};
use geo::{Area, Coord, Line, LineString, Polygon, Validation};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

pub const TOLERANCES_KEY: &str = "prumo.geometry.tolerances.v1";
pub const MAX_REPAIRS: usize = 128;
pub const MAX_AUDIT_OBJECTS: usize = 2048;
const MAX_FINDINGS: usize = 10_000;

fn invalid(message: impl Into<String>) -> DomainError {
    DomainError::InvalidModel(message.into())
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TolerancePolicy {
    #[serde(deserialize_with = "finite_f64")]
    pub selection_m: f64,
    #[serde(deserialize_with = "finite_f64")]
    pub topology_m: f64,
    #[serde(deserialize_with = "finite_f64")]
    pub mesh_m: f64,
    #[serde(deserialize_with = "finite_f64")]
    pub analysis_m: f64,
}

fn number_f64<E: serde::de::Error>(number: serde_json::Number) -> Result<f64, E> {
    number
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            E::custom("A coordenada ou tolerância deve ser finita e representável em f64.")
        })
}

// Tagged commands buffer arbitrary-precision JSON numbers as Number values.
// Decode that representation rather than rejecting valid exponent notation.
fn finite_f64<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    number_f64(serde_json::Number::deserialize(deserializer)?)
}

fn finite_point<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<[f64; 3], D::Error> {
    let numbers = <[serde_json::Number; 3]>::deserialize(deserializer)?;
    let mut result = [0.0; 3];
    for (index, number) in numbers.into_iter().enumerate() {
        result[index] = number_f64(number)?;
    }
    Ok(result)
}
impl Default for TolerancePolicy {
    fn default() -> Self {
        Self {
            selection_m: 0.01,
            topology_m: 1e-6,
            mesh_m: 1e-4,
            analysis_m: 1e-8,
        }
    }
}
impl TolerancePolicy {
    pub fn validate(&self) -> Result<(), DomainError> {
        for value in [
            self.selection_m,
            self.topology_m,
            self.mesh_m,
            self.analysis_m,
        ] {
            if !value.is_finite() || !(1e-12..=10.0).contains(&value) {
                return Err(invalid(
                    "Cada tolerância deve estar entre 1e-12 e 10 metros.",
                ));
            }
        }
        Ok(())
    }
}
pub fn project_tolerances(project: &Project) -> Result<TolerancePolicy, DomainError> {
    match project.extensions.get(TOLERANCES_KEY) {
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string())),
        None => Ok(TolerancePolicy::default()),
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryFilter {
    pub discipline: Option<Discipline>,
    pub network_id: Option<Uuid>,
    pub environment_id: Option<Uuid>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeometryFinding {
    ShortElement {
        object_id: Uuid,
        length_m: f64,
    },
    Duplicate {
        first_id: Uuid,
        second_id: Uuid,
    },
    NearContact {
        first_id: Uuid,
        second_id: Uuid,
        distance_m: f64,
        explicitly_connected: bool,
    },
    Crossing {
        first_id: Uuid,
        second_id: Uuid,
        position_m: [f64; 3],
    },
    Overlap {
        first_id: Uuid,
        second_id: Uuid,
        start_m: [f64; 3],
        end_m: [f64; 3],
    },
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryReport {
    pub project_id: Uuid,
    pub revision: u64,
    pub snapshot_sha256: String,
    pub tolerances: TolerancePolicy,
    pub filter: GeometryFilter,
    pub object_count: usize,
    pub findings: Vec<GeometryFinding>,
    pub topology_is_explicit: bool,
    pub engineering_calculation_available: bool,
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
}

/// Metric proximity to a finite segment, separate from exact intersection.
/// Coordinates are bounded by validate_point, so differences and determinants
/// fit binary64. Adaptive determinants use the original coordinates: subtracting
/// a remote origin before a cross product could erase the very gap being tested.
fn point_segment_distance(point: [f64; 3], segment: [[f64; 3]; 2]) -> f64 {
    let endpoint_distances = [distance(point, segment[0]), distance(point, segment[1])];
    let nearest = usize::from(endpoint_distances[1] < endpoint_distances[0]);
    let endpoint_distance = endpoint_distances[nearest];
    let origin = segment[nearest];
    let other = segment[1 - nearest];
    let length = distance(origin, other);
    if length == 0.0 {
        return endpoint_distance;
    }
    // The nearest endpoint minimizes cancellation in the projection test. This
    // projection only clamps to the finite segment, never classifies collinearity.
    let direction = std::array::from_fn::<_, 3, _>(|i| (other[i] - origin[i]) / length);
    let delta = std::array::from_fn::<_, 3, _>(|i| point[i] - origin[i]);
    let projection = direction[0].mul_add(
        delta[0],
        direction[1].mul_add(delta[1], direction[2] * delta[2]),
    );
    if projection <= 0.0 || projection >= length {
        return endpoint_distance;
    }
    let projected_area = |u: usize, v: usize| {
        let coord = |p: [f64; 3]| robust::Coord { x: p[u], y: p[v] };
        robust::orient2d(coord(segment[0]), coord(segment[1]), coord(point))
    };
    let perpendicular_distance = projected_area(1, 2)
        .hypot(projected_area(2, 0))
        .hypot(projected_area(0, 1))
        / length;
    perpendicular_distance.min(endpoint_distance)
}

fn endpoint_segment_gap(a: [[f64; 3]; 2], b: [[f64; 3]; 2]) -> f64 {
    [
        point_segment_distance(a[0], b),
        point_segment_distance(a[1], b),
        point_segment_distance(b[0], a),
        point_segment_distance(b[1], a),
    ]
    .into_iter()
    .fold(f64::INFINITY, f64::min)
}

fn connected(project: &Project, a: Uuid, b: Uuid) -> bool {
    project.connections.iter().any(|connection| {
        let from = project.ports.iter().find(|p| p.id == connection.from_port);
        let to = project.ports.iter().find(|p| p.id == connection.to_port);
        matches!((from, to), (Some(x), Some(y)) if (x.object_id == a && y.object_id == b) || (x.object_id == b && y.object_id == a))
    })
}

pub fn audit_project(
    project: &Project,
    filter: &GeometryFilter,
) -> Result<GeometryReport, DomainError> {
    let snapshot = model_snapshot(project)?;
    let tolerances = project_tolerances(project)?;
    tolerances.validate()?;
    if let Some(id) = filter.environment_id
        && !project.environments.iter().any(|e| e.id == id)
    {
        return Err(DomainError::UnknownReference(id));
    }
    let network_objects = match filter.network_id {
        Some(id) => {
            let network = project
                .networks
                .iter()
                .find(|n| n.id == id)
                .ok_or(DomainError::UnknownReference(id))?;
            Some(
                project
                    .ports
                    .iter()
                    .filter(|p| network.port_ids.contains(&p.id))
                    .map(|p| p.object_id)
                    .collect::<BTreeSet<_>>(),
            )
        }
        None => None,
    };
    let mut objects: Vec<_> = project
        .objects
        .iter()
        .filter(|o| {
            filter.discipline.is_none_or(|d| o.discipline == d)
                && filter
                    .environment_id
                    .is_none_or(|id| o.environment_id == Some(id))
                && network_objects
                    .as_ref()
                    .is_none_or(|ids| ids.contains(&o.id))
        })
        .collect();
    objects.sort_by_key(|o| o.id);
    if objects.len() > MAX_AUDIT_OBJECTS {
        return Err(invalid(
            "A inspeção admite até 2048 peças por seleção. Filtre por disciplina, rede ou ambiente.",
        ));
    }
    let mut findings = Vec::new();
    for (i, a) in objects.iter().enumerate() {
        if a.length_m() <= tolerances.analysis_m {
            findings.push(GeometryFinding::ShortElement {
                object_id: a.id,
                length_m: a.length_m(),
            });
        }
        let ap = [a.start_m.as_array(), a.end_m.as_array()];
        for b in &objects[i + 1..] {
            let bp = [b.start_m.as_array(), b.end_m.as_array()];
            let same = distance(ap[0], bp[0]) <= tolerances.topology_m
                && distance(ap[1], bp[1]) <= tolerances.topology_m;
            let reversed = distance(ap[0], bp[1]) <= tolerances.topology_m
                && distance(ap[1], bp[0]) <= tolerances.topology_m;
            if same || reversed {
                findings.push(GeometryFinding::Duplicate {
                    first_id: a.id,
                    second_id: b.id,
                });
            } else {
                match intersect_segments_3d(ap, bp)? {
                    SpatialIntersection::Point {
                        point_m,
                        proper: true,
                    } => findings.push(GeometryFinding::Crossing {
                        first_id: a.id,
                        second_id: b.id,
                        position_m: point_m,
                    }),
                    SpatialIntersection::Point { proper: false, .. } => {
                        findings.push(GeometryFinding::NearContact {
                            first_id: a.id,
                            second_id: b.id,
                            distance_m: 0.0,
                            explicitly_connected: connected(project, a.id, b.id),
                        });
                    }
                    SpatialIntersection::Overlap { start_m, end_m } => {
                        findings.push(GeometryFinding::Overlap {
                            first_id: a.id,
                            second_id: b.id,
                            start_m,
                            end_m,
                        })
                    }
                    SpatialIntersection::None => {
                        let gap = endpoint_segment_gap(ap, bp);
                        if gap <= tolerances.topology_m {
                            findings.push(GeometryFinding::NearContact {
                                first_id: a.id,
                                second_id: b.id,
                                distance_m: gap,
                                explicitly_connected: connected(project, a.id, b.id),
                            });
                        }
                    }
                }
            }
            if findings.len() > MAX_FINDINGS {
                return Err(invalid(
                    "A inspeção excedeu 10000 ocorrências. Reduza a seleção.",
                ));
            }
        }
    }
    Ok(GeometryReport {
        project_id: project.id,
        revision: project.revision,
        snapshot_sha256: snapshot.sha256,
        tolerances,
        filter: filter.clone(),
        object_count: objects.len(),
        findings,
        topology_is_explicit: true,
        engineering_calculation_available: false,
    })
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SegmentIntersection {
    None,
    Point { point: [f64; 2], proper: bool },
    Overlap { start: [f64; 2], end: [f64; 2] },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpatialIntersection {
    None,
    Point { point_m: [f64; 3], proper: bool },
    Overlap { start_m: [f64; 3], end_m: [f64; 3] },
}

pub fn intersect_segments_3d(
    a: [[f64; 3]; 2],
    b: [[f64; 3]; 2],
) -> Result<SpatialIntersection, DomainError> {
    for p in a.into_iter().chain(b) {
        validate_point(p)?;
    }
    if a[0] == a[1] || b[0] == b[1] {
        return Err(invalid("Segmento degenerado."));
    }
    let coord = |p: [f64; 3]| robust::Coord3D {
        x: p[0],
        y: p[1],
        z: p[2],
    };
    // Adaptive exact orientation distinguishes skew lines from coplanar contact.
    // A screen projection or an analysis tolerance never changes this predicate.
    if robust::orient3d(coord(a[0]), coord(a[1]), coord(b[0]), coord(b[1])) != 0.0 {
        return Ok(SpatialIntersection::None);
    }
    for axis in 0..3 {
        let u = (axis + 1) % 3;
        let v = (axis + 2) % 3;
        let project = |p: [f64; 3]| [p[u], p[v]];
        let aa = [project(a[0]), project(a[1])];
        let bb = [project(b[0]), project(b[1])];
        if aa[0] == aa[1] || bb[0] == bb[1] {
            continue;
        }
        if orientation_2d([aa[0], aa[1], bb[0]])? == 0
            && orientation_2d([aa[0], aa[1], bb[1]])? == 0
        {
            continue;
        }
        return Ok(match intersect_segments_2d(aa, bb)? {
            SegmentIntersection::Point { point, proper } => {
                let k = usize::from((aa[1][1] - aa[0][1]).abs() > (aa[1][0] - aa[0][0]).abs());
                let fraction = (point[k] - aa[0][k]) / (aa[1][k] - aa[0][k]);
                let mut point_m = [0.; 3];
                point_m[u] = point[0];
                point_m[v] = point[1];
                point_m[axis] = (a[1][axis] - a[0][axis]).mul_add(fraction, a[0][axis]);
                SpatialIntersection::Point { point_m, proper }
            }
            _ => SpatialIntersection::None,
        });
    }
    let axis = (0..3)
        .max_by(|x, y| {
            (a[1][*x] - a[0][*x])
                .abs()
                .total_cmp(&(a[1][*y] - a[0][*y]).abs())
        })
        .unwrap_or(0);
    let ordered = |p: [[f64; 3]; 2]| {
        if p[0][axis] <= p[1][axis] {
            p
        } else {
            [p[1], p[0]]
        }
    };
    let aa = ordered(a);
    let bb = ordered(b);
    let start = if aa[0][axis] >= bb[0][axis] {
        aa[0]
    } else {
        bb[0]
    };
    let end = if aa[1][axis] <= bb[1][axis] {
        aa[1]
    } else {
        bb[1]
    };
    if start[axis] > end[axis] {
        Ok(SpatialIntersection::None)
    } else if start[axis] == end[axis] {
        Ok(SpatialIntersection::Point {
            point_m: start,
            proper: false,
        })
    } else {
        Ok(SpatialIntersection::Overlap {
            start_m: start,
            end_m: end,
        })
    }
}
pub fn orientation_2d(points: [[f64; 2]; 3]) -> Result<i8, DomainError> {
    for p in points {
        validate_point([p[0], p[1], 0.0])?;
    }
    Ok(
        match RobustKernel::orient2d(points[0].into(), points[1].into(), points[2].into()) {
            Orientation::CounterClockwise => 1,
            Orientation::Clockwise => -1,
            Orientation::Collinear => 0,
        },
    )
}
pub fn intersect_segments_2d(
    a: [[f64; 2]; 2],
    b: [[f64; 2]; 2],
) -> Result<SegmentIntersection, DomainError> {
    for p in a.into_iter().chain(b) {
        validate_point([p[0], p[1], 0.0])?;
    }
    if a[0] == a[1] || b[0] == b[1] {
        return Err(invalid("Segmento degenerado."));
    }
    // Preserve original coordinates for adaptive predicates. A preliminary
    // translation could round away a small separation far from the chosen
    // origin. geo normalizes only the subsequent intersection calculation.
    Ok(
        match line_intersection(
            Line::new(Coord::from(a[0]), Coord::from(a[1])),
            Line::new(Coord::from(b[0]), Coord::from(b[1])),
        ) {
            None => SegmentIntersection::None,
            Some(LineIntersection::SinglePoint {
                intersection,
                is_proper,
            }) => SegmentIntersection::Point {
                point: [intersection.x, intersection.y],
                proper: is_proper,
            },
            Some(LineIntersection::Collinear { intersection }) => SegmentIntersection::Overlap {
                start: [intersection.start.x, intersection.start.y],
                end: [intersection.end.x, intersection.end.y],
            },
        },
    )
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaceInput {
    pub vertices_m: Vec<[f64; 3]>,
    pub expected_normal: [f64; 3],
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaceInspection {
    pub area_m2: f64,
    pub normal: [f64; 3],
    pub inverted: bool,
    pub valid: bool,
    pub problems: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectFaceInspection {
    pub project_id: Uuid,
    pub revision: u64,
    pub snapshot_sha256: String,
    pub tolerances: TolerancePolicy,
    pub face: FaceInput,
    #[serde(flatten)]
    pub inspection: FaceInspection,
}
pub fn inspect_face(
    input: &FaceInput,
    tolerances: TolerancePolicy,
) -> Result<FaceInspection, DomainError> {
    tolerances.validate()?;
    if !(3..=512).contains(&input.vertices_m.len()) {
        return Err(invalid(
            "Uma face deve ter de 3 a 512 vértices, sem repetir o fechamento.",
        ));
    }
    for p in &input.vertices_m {
        validate_point(*p)?;
    }
    let hint = nalgebra::Vector3::from(input.expected_normal);
    if !hint.iter().all(|x| x.is_finite()) || !hint.norm().is_finite() || hint.norm() < 1e-12 {
        return Err(invalid(
            "Informe uma normal de referência finita e não nula.",
        ));
    }
    let origin = nalgebra::Vector3::from(input.vertices_m[0]);
    let local: Vec<_> = input
        .vertices_m
        .iter()
        .map(|p| nalgebra::Vector3::from(*p) - origin)
        .collect();
    let mut normal = nalgebra::Vector3::zeros();
    let mut problems = Vec::new();
    for i in 0..local.len() {
        normal += local[i].cross(&local[(i + 1) % local.len()]);
        if (local[i] - local[(i + 1) % local.len()]).norm() <= tolerances.analysis_m {
            problems.push("Aresta degenerada ou vértice consecutivo duplicado.".into());
        }
        if local[..i].contains(&local[i]) {
            problems.push("Vértice duplicado.".into());
        }
    }
    let norm = normal.norm();
    if !norm.is_finite() || norm <= tolerances.analysis_m.powi(2) {
        problems.push("Face degenerada ou orientação indefinida.".into());
        return Ok(FaceInspection {
            area_m2: 0.0,
            normal: [0.0; 3],
            inverted: false,
            valid: false,
            problems,
        });
    }
    normal /= norm;
    if local
        .iter()
        .any(|p| p.dot(&normal).abs() > tolerances.mesh_m)
    {
        problems.push("Face não plana dentro da tolerância de malha.".into());
    }
    let axis = (0..3)
        .max_by(|a, b| normal[*a].abs().total_cmp(&normal[*b].abs()))
        .unwrap_or(2);
    let u = (axis + 1) % 3;
    let v = (axis + 2) % 3;
    let polygon = Polygon::new(
        LineString::new(local.iter().map(|p| Coord { x: p[u], y: p[v] }).collect()),
        vec![],
    );
    if polygon.check_validation().is_err() {
        problems.push("Face autointersectante ou topologicamente inválida.".into());
    }
    let inverted = normal.dot(&hint.normalize()) < 0.0;
    if inverted {
        problems.push("Face invertida em relação à normal informada.".into());
    }
    let area_m2 = polygon.unsigned_area() / normal[axis].abs();
    Ok(FaceInspection {
        area_m2,
        normal: normal.into(),
        inverted,
        valid: problems.is_empty(),
        problems,
    })
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Endpoint {
    Start,
    End,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "repair", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeometryRepair {
    SnapEndpoint {
        object_id: Uuid,
        endpoint: Endpoint,
        #[serde(deserialize_with = "finite_point")]
        target_m: [f64; 3],
        reason: String,
    },
    ConnectPorts {
        network_id: Uuid,
        from_port: Uuid,
        to_port: Uuid,
        reason: String,
    },
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryRepairPreview {
    pub project_id: Uuid,
    pub revision: u64,
    pub snapshot_sha256: String,
    pub plan_hash: String,
    pub repairs: Vec<GeometryRepair>,
    pub dimensions_changed: Vec<DimensionChange>,
    pub explicit_connections_added: usize,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionChange {
    pub object_id: Uuid,
    pub before_m: f64,
    pub after_m: f64,
}
fn execute_repairs(
    project: &mut Project,
    repairs: &[GeometryRepair],
    actor: &str,
) -> Result<(), DomainError> {
    if repairs.is_empty() || repairs.len() > MAX_REPAIRS {
        return Err(invalid("Informe entre 1 e 128 correções explícitas."));
    }
    let tolerance = project_tolerances(project)?.topology_m;
    for repair in repairs {
        let edit = match repair {
            GeometryRepair::SnapEndpoint {
                object_id,
                endpoint,
                target_m,
                reason,
            } => {
                validate_name(reason)?;
                validate_point(*target_m)?;
                let object = project
                    .objects
                    .iter()
                    .find(|o| o.id == *object_id)
                    .ok_or(DomainError::UnknownReference(*object_id))?;
                let mut start = object.start_m.as_array();
                let mut end = object.end_m.as_array();
                let previous = match endpoint {
                    Endpoint::Start => start,
                    Endpoint::End => end,
                };
                if distance(previous, *target_m) > tolerance {
                    return Err(invalid(
                        "A correção excede a tolerância topológica. Faça uma edição numérica explícita.",
                    ));
                }
                if previous == *target_m {
                    return Err(invalid("A correção não altera a posição."));
                }
                match endpoint {
                    Endpoint::Start => start = *target_m,
                    Endpoint::End => end = *target_m,
                }
                ModelEdit::SetPhysicalGeometry {
                    object_id: *object_id,
                    start: PointInput {
                        values: start,
                        unit: LengthUnit::Metre,
                    },
                    end: PointInput {
                        values: end,
                        unit: LengthUnit::Metre,
                    },
                }
            }
            GeometryRepair::ConnectPorts {
                network_id,
                from_port,
                to_port,
                reason,
            } => {
                validate_name(reason)?;
                let a = project
                    .ports
                    .iter()
                    .find(|p| p.id == *from_port)
                    .ok_or(DomainError::UnknownReference(*from_port))?;
                let b = project
                    .ports
                    .iter()
                    .find(|p| p.id == *to_port)
                    .ok_or(DomainError::UnknownReference(*to_port))?;
                if distance(a.position_m.as_array(), b.position_m.as_array()) > tolerance {
                    return Err(invalid(
                        "As portas não estão dentro da tolerância topológica.",
                    ));
                }
                ModelEdit::ConnectPorts {
                    network_id: *network_id,
                    from_port: *from_port,
                    to_port: *to_port,
                }
            }
        };
        apply_model_edit(project, edit, actor)?;
    }
    Ok(())
}
pub fn preview_repairs(
    project: &Project,
    repairs: &[GeometryRepair],
    actor: &str,
) -> Result<GeometryRepairPreview, DomainError> {
    let snapshot = model_snapshot(project)?;
    let mut candidate = project.clone();
    execute_repairs(&mut candidate, repairs, actor)?;
    let dimensions_changed = project
        .objects
        .iter()
        .filter_map(|old| {
            let new = candidate.objects.iter().find(|o| o.id == old.id)?;
            (old.start_m != new.start_m || old.end_m != new.end_m).then_some(DimensionChange {
                object_id: old.id,
                before_m: old.length_m(),
                after_m: new.length_m(),
            })
        })
        .collect();
    Ok(GeometryRepairPreview {
        project_id: project.id,
        revision: project.revision,
        snapshot_sha256: snapshot.sha256.clone(),
        plan_hash: digest(&(
            project.id,
            project.revision,
            snapshot.sha256,
            repairs,
            actor,
        ))?,
        repairs: repairs.to_vec(),
        dimensions_changed,
        explicit_connections_added: repairs
            .iter()
            .filter(|r| matches!(r, GeometryRepair::ConnectPorts { .. }))
            .count(),
    })
}
pub fn apply_repairs(
    project: &mut Project,
    repairs: &[GeometryRepair],
    plan_hash: &str,
    actor: &str,
) -> Result<(), DomainError> {
    if preview_repairs(project, repairs, actor)?.plan_hash != plan_hash {
        return Err(invalid(
            "A prévia geométrica mudou. Consulte novamente antes de aplicar.",
        ));
    }
    execute_repairs(project, repairs, actor)
}
