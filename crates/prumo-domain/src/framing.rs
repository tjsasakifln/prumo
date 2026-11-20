//! Parametric structural lines with explicit analytical topology.
//!
//! This module models geometry and member behavior; it does not solve a structure.
use crate::{
    Discipline, DomainError, ModelState, PhysicalObject, Project,
    catalogue::{self, CatalogKind, CatalogPayload},
    layout::{self, LayoutPoint},
    model::{
        AnalyticalMember, AnalyticalNode, PhysicalAnalyticalLink, PhysicalRole, record_identity,
    },
    units::{Point3, PointInput},
    validate_name,
};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const FRAMING_KEY: &str = "prumo.framing.v1";
pub const MAX_CURVE_SEGMENTS: u16 = 256;
fn invalid(message: impl Into<String>) -> DomainError {
    DomainError::InvalidModel(message.into())
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberKind {
    Column,
    Beam,
    Bar,
    TrussBar,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberBehavior {
    Frame,
    AxialOnly,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionAlignment {
    Centroid,
    SectionOrigin,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionVariation {
    Constant,
    Stepped,
    Linear,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SectionStation {
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub fraction: f64,
    pub section_id: Uuid,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MemberPath {
    Straight {
        start: LayoutPoint,
        end: LayoutPoint,
    },
    CircularArc {
        start: LayoutPoint,
        through: LayoutPoint,
        end: LayoutPoint,
        segments: u16,
    },
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FramingInput {
    pub name: String,
    pub kind: MemberKind,
    pub path: MemberPath,
    pub material_id: Uuid,
    pub sections: Vec<SectionStation>,
    pub variation: SectionVariation,
    pub alignment: SectionAlignment,
    /// Translation of the physical centroid relative to the analytical line, in engineering axes.
    pub eccentricity: PointInput,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub roll_degrees: f64,
    /// Existing end nodes are shared only when the author names them explicitly.
    pub connections: [Option<Uuid>; 2],
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum FramingEdit {
    PutMember {
        id: Uuid,
        input: Box<FramingInput>,
    },
    CreateTruss {
        id: Uuid,
        name: String,
        points: Vec<LayoutPoint>,
        edges: Vec<[usize; 2]>,
        material_id: Uuid,
        section_id: Uuid,
    },
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FramingRecord {
    pub input: FramingInput,
    pub behavior: MemberBehavior,
    pub node_ids: Vec<Uuid>,
    pub member_ids: Vec<Uuid>,
    pub link_ids: Vec<Uuid>,
    pub owned_node_ids: Vec<Uuid>,
    pub analytical_points_m: Vec<Point3>,
    pub physical_points_m: Vec<Point3>,
    /// Start indices of disconnected centroid traces at stepped section changes.
    /// No transverse segment or structural connection is implied by a jump.
    pub physical_breaks: Vec<usize>,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub analytical_path_length_m: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub physical_length_m: f64,
    pub physical_length_is_exact: bool,
    /// Composite Simpson convergence estimate; zero for an analytic expression.
    /// This is not a certified integration bound or an engineering tolerance.
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub physical_length_error_estimate_m: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub physical_chord_length_m: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub analytical_chord_length_m: f64,
    /// Maximum sagitta of the analytical circular-arc discretization.
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub maximum_chord_error_m: f64,
    /// Bound on physical trace deviation from its displayed chords, excluding
    /// explicit jumps between sections. The bound uses the second derivative.
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub maximum_physical_chord_error_m: f64,
    pub truss_id: Option<Uuid>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FramingInspection {
    pub project_id: Uuid,
    pub revision: u64,
    pub object_id: Uuid,
    pub record: FramingRecord,
    pub material_name: String,
    pub sections: Vec<ResolvedSection>,
    pub sources: Vec<crate::dependencies::SourceKey>,
    pub bending_stiffness_permitted: bool,
    pub engineering_release: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedSection {
    pub section_id: Uuid,
    pub revision: u64,
    pub sha256: String,
    pub name: String,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub fraction: f64,
    #[serde(deserialize_with = "crate::numeric::scalar")]
    pub area_m2: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedGeometry {
    pub analytical_points_m: Vec<Point3>,
    pub analytical_local_y: Vec<[f64; 3]>,
    pub physical_points_m: Vec<Point3>,
    pub physical_breaks: Vec<usize>,
    pub analytical_path_length_m: f64,
    pub physical_length_m: f64,
    pub physical_length_is_exact: bool,
    pub physical_length_error_estimate_m: f64,
    pub physical_chord_length_m: f64,
    pub analytical_chord_length_m: f64,
    pub maximum_chord_error_m: f64,
    pub maximum_physical_chord_error_m: f64,
}

pub fn metadata(object: &PhysicalObject) -> Result<Option<FramingRecord>, DomainError> {
    object
        .extensions
        .get(FRAMING_KEY)
        .map(|value| serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string())))
        .transpose()
}
pub fn object_length_m(object: &PhysicalObject) -> Result<Option<f64>, DomainError> {
    Ok(metadata(object)?.map(|record| record.physical_length_m))
}
/// Engineering consumers must use this contract before assembling a member.
/// Legacy generic members retain their original frame contract; parametric truss
/// members expressly prohibit flexural stiffness.
pub fn analytical_behavior(
    project: &Project,
    member_id: Uuid,
) -> Result<MemberBehavior, DomainError> {
    if !project
        .model
        .analytical_members
        .iter()
        .any(|member| member.id == member_id)
    {
        return Err(DomainError::UnknownReference(member_id));
    }
    for object in &project.objects {
        if let Some(record) = metadata(object)?
            && record.member_ids.contains(&member_id)
        {
            return Ok(record.behavior);
        }
    }
    Ok(MemberBehavior::Frame)
}
// Reconstruction permits the rounding envelope of eight binary64 ULPs at the
// value's magnitude (minimum scale 1). Persisted coordinates remain exact and
// are compared to each other without this envelope. This is not a topology or
// engineering tolerance and does not merge nodes or change stored values.
fn reconstructed_equal(a: f64, b: f64) -> bool {
    a.is_finite()
        && b.is_finite()
        && (a - b).abs() <= 8.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0)
}
fn points_reconstructed_equal(a: &[Point3], b: &[Point3]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| (0..3).all(|i| reconstructed_equal(a[i], b[i])))
}
fn derived_id(id: Uuid, kind: &str, index: usize) -> Uuid {
    let mut digest = Sha256::new();
    digest.update(id.as_bytes());
    digest.update(kind.as_bytes());
    digest.update((index as u64).to_be_bytes());
    let hash = digest.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}
fn vector(p: Point3) -> Vector3<f64> {
    Vector3::from(p.as_array())
}
fn point(v: Vector3<f64>) -> Result<Point3, DomainError> {
    Point3::try_from([v.x, v.y, v.z])
}
fn axes(a: Point3, b: Point3, roll: f64) -> Result<(Vector3<f64>, Vector3<f64>), DomainError> {
    let direction = vector(b) - vector(a);
    let length = direction.norm();
    if length <= crate::MIN_MEMBER_LENGTH_M {
        return Err(DomainError::DegenerateMember);
    }
    if !roll.is_finite() || roll.abs() > 360.0 {
        return Err(invalid("Rotação da seção exige graus entre -360 e 360."));
    }
    let x = direction / length;
    let helper = if x.z.abs() < 0.9 {
        Vector3::z()
    } else {
        Vector3::y()
    };
    let y = helper.cross(&x).normalize();
    let z = x.cross(&y);
    let (s, c) = roll.to_radians().sin_cos();
    let y = y * c + z * s;
    Ok((y, x.cross(&y)))
}
#[derive(Clone, Debug)]
struct ArcGeometry {
    center: Vector3<f64>,
    radius: f64,
    normal: Vector3<f64>,
    e1: Vector3<f64>,
    e2: Vector3<f64>,
    sweep: f64,
}
#[derive(Clone, Debug)]
struct ResolvedPath {
    start: Point3,
    end: Point3,
    arc: Option<ArcGeometry>,
    segments: usize,
}
impl ResolvedPath {
    fn at(&self, fraction: f64) -> Result<Point3, DomainError> {
        if fraction == 0.0 {
            return Ok(self.start);
        }
        if fraction == 1.0 {
            return Ok(self.end);
        }
        if let Some(arc) = &self.arc {
            let (s, c) = (arc.sweep * fraction).sin_cos();
            point(arc.center + (arc.e1 * c + arc.e2 * s) * arc.radius)
        } else {
            point(vector(self.start) + (vector(self.end) - vector(self.start)) * fraction)
        }
    }
    fn local_axes(
        &self,
        fraction: f64,
        roll: f64,
    ) -> Result<(Vector3<f64>, Vector3<f64>), DomainError> {
        if let Some(arc) = &self.arc {
            // The local y axis points towards the circle center, while z is
            // normal to the arc plane. Both follow the true tangent at this
            // station, including an inclined arc and a reversed traversal.
            let (s, c) = (arc.sweep * fraction).sin_cos();
            let x = (-arc.e1 * s + arc.e2 * c) * arc.sweep.signum();
            let z = arc.normal * arc.sweep.signum();
            let y = z.cross(&x);
            let (s, c) = roll.to_radians().sin_cos();
            let rotated_y = y * c + z * s;
            Ok((rotated_y, x.cross(&rotated_y)))
        } else {
            axes(self.start, self.end, roll)
        }
    }
    fn length(&self) -> f64 {
        self.arc.as_ref().map_or_else(
            || self.start.distance(self.end),
            |arc| arc.radius * arc.sweep.abs(),
        )
    }
    fn chord_error(&self) -> f64 {
        self.arc.as_ref().map_or(0.0, |arc| {
            2.0 * arc.radius * (arc.sweep / (4.0 * self.segments as f64)).sin().powi(2)
        })
    }
}
fn resolve_path(project: &Project, path: &MemberPath) -> Result<ResolvedPath, DomainError> {
    let (start, end) = match path {
        MemberPath::Straight { start, end } | MemberPath::CircularArc { start, end, .. } => (
            layout::resolve_point(project, start)?,
            layout::resolve_point(project, end)?,
        ),
    };
    if start.distance(end) <= crate::MIN_MEMBER_LENGTH_M {
        return Err(DomainError::DegenerateMember);
    }
    let MemberPath::CircularArc {
        through, segments, ..
    } = path
    else {
        return Ok(ResolvedPath {
            start,
            end,
            arc: None,
            segments: 1,
        });
    };
    if !(2..=MAX_CURVE_SEGMENTS).contains(segments) {
        return Err(invalid("Arco exige de 2 a 256 segmentos explícitos."));
    }
    let through = layout::resolve_point(project, through)?;
    let u = vector(through) - vector(start);
    let v = vector(end) - vector(start);
    let cross = u.cross(&v);
    let scale = u.norm().max(v.norm());
    if scale <= crate::MIN_MEMBER_LENGTH_M || cross.norm() <= scale * scale * 1e-10 {
        return Err(invalid(
            "Três pontos distintos e não colineares são necessários para a viga curva.",
        ));
    }
    // Circumcenter relative to the start avoids cancellation in survey coordinates.
    let center_relative = (v.cross(&cross) * u.norm_squared() + cross.cross(&u) * v.norm_squared())
        / (2.0 * cross.norm_squared());
    let center = vector(start) + center_relative;
    let radius = center_relative.norm();
    if !radius.is_finite() || radius > crate::MAX_COORDINATE_M {
        return Err(DomainError::InvalidQuantity);
    }
    let normal = cross.normalize();
    let e1 = -center_relative / radius;
    let e2 = normal.cross(&e1);
    let angle_of = |p: Point3| {
        let r = vector(p) - center;
        let value = r.dot(&e2).atan2(r.dot(&e1));
        if value < 0.0 {
            value + std::f64::consts::TAU
        } else {
            value
        }
    };
    let mut sweep = angle_of(end);
    if sweep < angle_of(through) {
        sweep -= std::f64::consts::TAU;
    }
    if sweep.abs() <= 1e-10 {
        return Err(invalid("Arco degenerado."));
    }
    Ok(ResolvedPath {
        start,
        end,
        arc: Some(ArcGeometry {
            center,
            radius,
            normal,
            e1,
            e2,
            sweep,
        }),
        segments: usize::from(*segments),
    })
}
#[derive(Clone, Copy)]
struct SectionInterval {
    from: f64,
    to: f64,
    start: [f64; 2],
    end: [f64; 2],
}
impl SectionInterval {
    fn centroid(self, fraction: f64) -> [f64; 2] {
        if fraction == self.from {
            return self.start;
        }
        if fraction == self.to {
            return self.end;
        }
        let t = (fraction - self.from) / (self.to - self.from);
        [
            self.start[0] + (self.end[0] - self.start[0]) * t,
            self.start[1] + (self.end[1] - self.start[1]) * t,
        ]
    }
}
fn section_intervals(
    project: &Project,
    input: &FramingInput,
) -> Result<Vec<SectionInterval>, DomainError> {
    if input.alignment == SectionAlignment::Centroid {
        return Ok(vec![SectionInterval {
            from: 0.0,
            to: 1.0,
            start: [0.0; 2],
            end: [0.0; 2],
        }]);
    }
    let catalogue = catalogue::project_catalogue(project)?;
    let centroids = input
        .sections
        .iter()
        .map(|station| {
            let CatalogPayload::Section { shape } = &catalogue.latest(station.section_id)?.payload
            else {
                return Err(invalid("Catálogo não é seção."));
            };
            Ok(shape.geometry()?.centroid_m)
        })
        .collect::<Result<Vec<_>, DomainError>>()?;
    if input.variation == SectionVariation::Constant {
        return Ok(vec![SectionInterval {
            from: 0.0,
            to: 1.0,
            start: centroids[0],
            end: centroids[0],
        }]);
    }
    let mut intervals = Vec::new();
    for (index, station) in input.sections.iter().enumerate() {
        let to = input
            .sections
            .get(index + 1)
            .map_or(1.0, |next| next.fraction);
        if station.fraction == to {
            continue;
        }
        intervals.push(SectionInterval {
            from: station.fraction,
            to,
            start: centroids[index],
            end: if input.variation == SectionVariation::Linear {
                centroids[index + 1]
            } else {
                centroids[index]
            },
        });
    }
    Ok(intervals)
}
fn physical_point(
    path: &ResolvedPath,
    interval: SectionInterval,
    t: f64,
    eccentricity: Vector3<f64>,
    roll: f64,
) -> Result<Point3, DomainError> {
    let centroid = interval.centroid(t);
    let (y, z) = path.local_axes(t, roll)?;
    point(vector(path.at(t)?) + eccentricity + y * centroid[0] + z * centroid[1])
}
/// Integrate the speed of a circular trace whose radial and out-of-plane
/// offsets vary linearly within one station interval. The returned error is a
/// convergence estimate, not a rigorous error bound.
fn varying_arc_length(
    radius0: f64,
    radius1: f64,
    rise: f64,
    angle: f64,
) -> Result<(f64, f64), DomainError> {
    let dr = radius1 - radius0;
    let speed = |t: f64| dr.hypot(rise).hypot(angle * (radius0 + dr * t));
    let simpson = |count: usize| {
        let mut sum = speed(0.0) + speed(1.0);
        for i in 1..count {
            sum += speed(i as f64 / count as f64) * if i % 2 == 0 { 2.0 } else { 4.0 };
        }
        sum / (3.0 * count as f64)
    };
    let mut count = 16;
    let mut previous = simpson(count);
    loop {
        count *= 2;
        let current = simpson(count);
        let estimate = (current - previous).abs() / 15.0;
        if !current.is_finite() || !estimate.is_finite() {
            return Err(DomainError::InvalidQuantity);
        }
        if estimate <= 1e-11 * current.abs().max(1.0) {
            return Ok((current, estimate));
        }
        if count >= 4096 {
            return Err(invalid(
                "Comprimento físico da curva não convergiu dentro do limite numérico. Revise as estações e a curvatura.",
            ));
        }
        previous = current;
    }
}
fn physical_interval_metrics(
    path: &ResolvedPath,
    interval: SectionInterval,
    roll: f64,
    samples: &[f64],
) -> Result<(f64, bool, f64, f64), DomainError> {
    let (y, z) = path.local_axes(interval.from, roll)?;
    let Some(arc) = &path.arc else {
        let direction = (vector(path.end) - vector(path.start)) * (interval.to - interval.from)
            + y * (interval.end[0] - interval.start[0])
            + z * (interval.end[1] - interval.start[1]);
        return Ok((direction.norm(), true, 0.0, 0.0));
    };
    let (s, c) = roll.to_radians().sin_cos();
    let radius0 = arc.radius - (interval.start[0] * c - interval.start[1] * s);
    let radius1 = arc.radius - (interval.end[0] * c - interval.end[1] * s);
    if radius0.min(radius1) <= crate::MIN_MEMBER_LENGTH_M {
        return Err(invalid(
            "Centroide da seção atinge ou atravessa o centro de curvatura. O traçado físico seria nulo ou invertido.",
        ));
    }
    let rise =
        (interval.end[0] - interval.start[0]) * s + (interval.end[1] - interval.start[1]) * c;
    let angle = arc.sweep * (interval.to - interval.from);
    let (length, exact, estimate) = if radius0 == radius1 {
        ((radius0 * angle).hypot(rise), true, 0.0)
    } else {
        let (length, error) = varying_arc_length(radius0, radius1, rise, angle)?;
        (length, false, error)
    };
    let max_step = samples.windows(2).map(|w| w[1] - w[0]).fold(0.0, f64::max);
    let error = if radius0 == radius1 {
        2.0 * radius0 * (arc.sweep * max_step / 4.0).sin().powi(2)
    } else {
        // Normalize this station interval to u in [0,1], avoiding division of
        // dr by a tiny global station span. For x(u)=r(u)e_r(theta(u))+q(u)n,
        // q''=r''=0 and ||x''|| <= theta'^2 max(r)+2|theta' r'|.
        // The vector interpolation remainder is bounded by M*du^2/8.
        let second =
            angle.powi(2) * radius0.max(radius1) + 2.0 * angle.abs() * (radius1 - radius0).abs();
        second * (max_step / (interval.to - interval.from)).powi(2) / 8.0
    };
    Ok((length, exact, estimate, error))
}
fn analytical_y(
    path: &ResolvedPath,
    roll: f64,
    points: &[Point3],
    index: usize,
) -> Result<[f64; 3], DomainError> {
    let (y, _) = path.local_axes((index as f64 + 0.5) / path.segments as f64, roll)?;
    let chord = vector(points[index + 1]) - vector(points[index]);
    if chord.norm() <= crate::MIN_MEMBER_LENGTH_M {
        return Err(DomainError::DegenerateMember);
    }
    let x = chord.normalize();
    // The analytical member is the stored chord, whose rounded coordinates can
    // differ slightly from the mathematical tangent at survey-scale origins.
    let y = (y - x * y.dot(&x)).normalize();
    Ok([y.x, y.y, y.z])
}
pub fn resolve_geometry(
    project: &Project,
    input: &FramingInput,
) -> Result<ResolvedGeometry, DomainError> {
    validate_input(project, input)?;
    let path = resolve_path(project, &input.path)?;
    let fractions = (0..=path.segments)
        .map(|i| i as f64 / path.segments as f64)
        .collect::<Vec<_>>();
    let analytical_points_m = fractions
        .iter()
        .map(|t| path.at(*t))
        .collect::<Result<Vec<_>, _>>()?;
    let analytical_local_y = (0..path.segments)
        .map(|index| analytical_y(&path, input.roll_degrees, &analytical_points_m, index))
        .collect::<Result<Vec<_>, DomainError>>()?;
    let eccentricity = vector(input.eccentricity.to_si()?);
    let mut physical_points_m = Vec::new();
    let mut physical_breaks = Vec::new();
    let mut physical_length_m = 0.0;
    let mut physical_length_is_exact = true;
    let mut physical_length_error_estimate_m = 0.0;
    let mut physical_chord_length_m = 0.0;
    let mut maximum_physical_chord_error_m = 0.0_f64;
    for interval in section_intervals(project, input)? {
        let mut samples = vec![interval.from];
        samples.extend(
            fractions
                .iter()
                .copied()
                .filter(|t| *t > interval.from && *t < interval.to),
        );
        samples.push(interval.to);
        let points = samples
            .iter()
            .map(|t| physical_point(&path, interval, *t, eccentricity, input.roll_degrees))
            .collect::<Result<Vec<_>, _>>()?;
        physical_chord_length_m += points.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>();
        let (length, exact, estimate, error) =
            physical_interval_metrics(&path, interval, input.roll_degrees, &samples)?;
        physical_length_m += length;
        physical_length_is_exact &= exact;
        physical_length_error_estimate_m += estimate;
        maximum_physical_chord_error_m = maximum_physical_chord_error_m.max(error);
        if let Some(previous) = physical_points_m.last() {
            if *previous == points[0] {
                physical_points_m.extend(points.into_iter().skip(1));
            } else {
                physical_breaks.push(physical_points_m.len());
                physical_points_m.extend(points);
            }
        } else {
            physical_points_m.extend(points);
        }
    }
    if !physical_length_m.is_finite() || physical_length_m <= crate::MIN_MEMBER_LENGTH_M {
        return Err(DomainError::DegenerateMember);
    }
    let analytical_chord_length_m = analytical_points_m
        .windows(2)
        .map(|w| w[0].distance(w[1]))
        .sum();
    Ok(ResolvedGeometry {
        analytical_points_m,
        analytical_local_y,
        physical_points_m,
        physical_breaks,
        analytical_path_length_m: path.length(),
        physical_length_m,
        physical_length_is_exact,
        physical_length_error_estimate_m,
        physical_chord_length_m,
        analytical_chord_length_m,
        maximum_chord_error_m: path.chord_error(),
        maximum_physical_chord_error_m,
    })
}
fn validate_input(project: &Project, input: &FramingInput) -> Result<(), DomainError> {
    validate_name(&input.name)?;
    if !project
        .model
        .materials
        .iter()
        .any(|m| m.id == input.material_id)
    {
        return Err(DomainError::UnknownReference(input.material_id));
    }
    input.eccentricity.to_si()?;
    if !input.roll_degrees.is_finite() || input.roll_degrees.abs() > 360.0 {
        return Err(invalid("Rotação da seção fora do domínio."));
    }
    if input.sections.is_empty() || input.sections.len() > 64 {
        return Err(invalid("Peça exige de 1 a 64 estações de seção."));
    }
    let catalogue = catalogue::project_catalogue(project)?;
    let mut previous = -1.0;
    for station in &input.sections {
        if !station.fraction.is_finite()
            || !(0.0..=1.0).contains(&station.fraction)
            || station.fraction <= previous
        {
            return Err(invalid(
                "Estações de seção devem crescer entre 0 e 1, sem duplicidade.",
            ));
        }
        previous = station.fraction;
        if !matches!(
            catalogue.latest(station.section_id)?.payload,
            CatalogPayload::Section { .. }
        ) {
            return Err(invalid(
                "A seção deve referenciar uma entrada de seção do catálogo.",
            ));
        }
    }
    if input.sections[0].fraction != 0.0 {
        return Err(invalid("Primeira estação de seção deve estar na origem 0."));
    }
    match input.variation {
        SectionVariation::Constant if input.sections.len() != 1 => {
            return Err(invalid("Seção constante usa uma estação na origem."));
        }
        SectionVariation::Linear
            if input.sections.len() < 2
                || input.sections.last().is_none_or(|s| s.fraction != 1.0) =>
        {
            return Err(invalid("Variação linear exige seções nas posições 0 e 1."));
        }
        SectionVariation::Stepped if input.sections.last().is_some_and(|s| s.fraction == 1.0) => {
            return Err(invalid(
                "Cada trecho de seção escalonada exige comprimento positivo. A última transição deve ocorrer antes de 1.",
            ));
        }
        _ => {}
    }
    if input.kind == MemberKind::TrussBar && !matches!(input.path, MemberPath::Straight { .. }) {
        return Err(invalid(
            "Barra de treliça deve ser reta e trabalhar somente axialmente.",
        ));
    }
    Ok(())
}
pub fn apply(
    project: &mut Project,
    command: FramingEdit,
    actor: &str,
    revision: u64,
) -> Result<(), DomainError> {
    validate_name(actor)?;
    let mut candidate = project.clone();
    match command {
        FramingEdit::PutMember { id, input } => {
            put_member(&mut candidate, id, *input, actor, revision, None)?;
            // A point may name a node generated by this member without creating
            // a structural connection. Rebuild those explicit geometric
            // dependencies in the same atomic command as the source edit.
            regenerate_layout_dependents(&mut candidate)?;
        }
        FramingEdit::CreateTruss {
            id,
            name,
            points,
            edges,
            material_id,
            section_id,
        } => {
            validate_name(&name)?;
            if id.is_nil()
                || id == candidate.id
                || candidate.model.identities.contains_key(&id)
                || points.len() < 2
                || points.len() > 512
                || edges.is_empty()
                || edges.len() > 1024
            {
                return Err(invalid(
                    "Treliça exige identidade própria, 2 a 512 pontos e 1 a 1024 barras.",
                ));
            }
            if candidate
                .objects
                .iter()
                .any(|o| metadata(o).is_ok_and(|r| r.is_some_and(|r| r.truss_id == Some(id))))
            {
                return Err(invalid(
                    "Treliça já criada. Edite suas barras pelo identificador.",
                ));
            }
            let resolved = points
                .iter()
                .map(|p| layout::resolve_point(&candidate, p))
                .collect::<Result<Vec<_>, _>>()?;
            let mut used = BTreeSet::<usize>::new();
            let mut unique = BTreeSet::new();
            for edge in &edges {
                if edge[0] >= points.len()
                    || edge[1] >= points.len()
                    || edge[0] == edge[1]
                    || !unique.insert((edge[0].min(edge[1]), edge[0].max(edge[1])))
                {
                    return Err(invalid(
                        "Conexão de treliça inexistente, nula ou duplicada.",
                    ));
                }
                if resolved[edge[0]].distance(resolved[edge[1]]) <= crate::MIN_MEMBER_LENGTH_M {
                    return Err(DomainError::DegenerateMember);
                }
                used.extend(edge);
            }
            if used.len() != points.len() {
                return Err(invalid("Treliça contém pontos órfãos."));
            }
            // Connectivity must be explicit and connected; coincidence never joins another object.
            let mut reached = BTreeSet::from([0usize]);
            loop {
                let before = reached.len();
                for edge in &edges {
                    if reached.contains(&edge[0]) || reached.contains(&edge[1]) {
                        reached.extend(edge);
                    }
                }
                if reached.len() == before {
                    break;
                }
            }
            if reached.len() != points.len() {
                return Err(invalid(
                    "Treliça desconectada. Separe os conjuntos explicitamente.",
                ));
            }
            let nodes = resolved
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let node_id = derived_id(id, "truss-node", i);
                    record_identity(&mut candidate, node_id, actor, revision, false)?;
                    candidate.model.analytical_nodes.push(AnalyticalNode {
                        id: node_id,
                        name: format!("N{} {}", i + 1, name),
                        position_m: *p,
                    });
                    Ok(node_id)
                })
                .collect::<Result<Vec<_>, DomainError>>()?;
            for (index, edge) in edges.iter().enumerate() {
                let member_id = derived_id(id, "truss-piece", index);
                let input = FramingInput {
                    name: format!("{} / barra {}", name, index + 1),
                    kind: MemberKind::TrussBar,
                    path: MemberPath::Straight {
                        start: points[edge[0]].clone(),
                        end: points[edge[1]].clone(),
                    },
                    material_id,
                    sections: vec![SectionStation {
                        fraction: 0.0,
                        section_id,
                    }],
                    variation: SectionVariation::Constant,
                    alignment: SectionAlignment::Centroid,
                    eccentricity: PointInput {
                        values: [0.0; 3],
                        unit: crate::LengthUnit::Metre,
                    },
                    roll_degrees: 0.0,
                    connections: [Some(nodes[edge[0]]), Some(nodes[edge[1]])],
                };
                put_member(&mut candidate, member_id, input, actor, revision, Some(id))?;
            }
        }
    }
    validate_project(&candidate)?;
    *project = candidate;
    Ok(())
}
fn put_member(
    project: &mut Project,
    id: Uuid,
    input: FramingInput,
    actor: &str,
    revision: u64,
    truss_id: Option<Uuid>,
) -> Result<(), DomainError> {
    if id.is_nil() {
        return Err(DomainError::DuplicateIdentity(id));
    }
    let geometry = resolve_geometry(project, &input)?;
    let old = project
        .objects
        .iter()
        .find(|o| o.id == id)
        .map(metadata)
        .transpose()?
        .flatten();
    if project.objects.iter().any(|o| o.id == id) && old.is_none() {
        return Err(invalid("Identidade pertence a peça de outro tipo."));
    }
    if let Some(old) = &old {
        if old.node_ids.len() != geometry.analytical_points_m.len() {
            return Err(invalid(
                "Alterar a discretização de uma peça existente exige nova peça; preserve os vínculos explícitos.",
            ));
        }
        if old.input.connections != input.connections {
            return Err(invalid(
                "A alteração de nós compartilhados exige manter a conexão atual; crie outra peça para refazer a topologia.",
            ));
        }
    }
    let parametric_members = if old.is_some() {
        project
            .objects
            .iter()
            .filter_map(|object| metadata(object).transpose())
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(|record| record.member_ids)
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    let count = geometry.analytical_points_m.len();
    let mut node_ids = Vec::with_capacity(count);
    let mut owned_node_ids = Vec::new();
    for (index, position) in geometry.analytical_points_m.iter().enumerate() {
        let explicit = if index == 0 {
            input.connections[0]
        } else if index + 1 == count {
            input.connections[1]
        } else {
            None
        };
        if let Some(node_id) = explicit {
            let node = project
                .model
                .analytical_nodes
                .iter()
                .find(|n| n.id == node_id)
                .ok_or(DomainError::UnknownReference(node_id))?;
            if node.position_m.distance(*position) > 1e-8 {
                return Err(invalid(
                    "Nó compartilhado não coincide com o ponto declarado; não há encaixe estrutural automático.",
                ));
            }
            node_ids.push(node_id);
        } else {
            let node_id = derived_id(id, "node", index);
            node_ids.push(node_id);
            owned_node_ids.push(node_id);
            if let Some(node) = project
                .model
                .analytical_nodes
                .iter_mut()
                .find(|n| n.id == node_id)
            {
                if old
                    .as_ref()
                    .is_none_or(|old| !old.owned_node_ids.contains(&node_id))
                {
                    return Err(DomainError::DuplicateIdentity(node_id));
                }
                if node.position_m != *position
                    && project.model.analytical_members.iter().any(|m| {
                        (m.start_node_id == node_id || m.end_node_id == node_id)
                            && old
                                .as_ref()
                                .is_none_or(|old| !old.member_ids.contains(&m.id))
                            && !parametric_members.contains(&m.id)
                    })
                {
                    return Err(DomainError::SharedTopology);
                }
                node.position_m = *position;
            } else {
                record_identity(project, node_id, actor, revision, false)?;
                project.model.analytical_nodes.push(AnalyticalNode {
                    id: node_id,
                    name: format!("{} / nó {}", input.name, index + 1),
                    position_m: *position,
                });
            }
        }
    }
    let mut member_ids = Vec::with_capacity(count - 1);
    let mut link_ids = Vec::with_capacity(count - 1);
    for index in 0..count - 1 {
        let member_id = derived_id(id, "member", index);
        let link_id = derived_id(id, "link", index);
        if project.model.analytical_members.iter().any(|m| {
            m.id != member_id
                && ((m.start_node_id == node_ids[index] && m.end_node_id == node_ids[index + 1])
                    || (m.start_node_id == node_ids[index + 1] && m.end_node_id == node_ids[index]))
        }) {
            return Err(invalid("Ligação analítica duplicada entre os mesmos nós."));
        }
        let member = AnalyticalMember {
            id: member_id,
            name: format!("{} / trecho {}", input.name, index + 1),
            start_node_id: node_ids[index],
            end_node_id: node_ids[index + 1],
            local_y: geometry.analytical_local_y[index],
        };
        if let Some(existing) = project
            .model
            .analytical_members
            .iter_mut()
            .find(|m| m.id == member_id)
        {
            if old.is_none() {
                return Err(DomainError::DuplicateIdentity(member_id));
            }
            *existing = member;
        } else {
            record_identity(project, member_id, actor, revision, false)?;
            project.model.analytical_members.push(member);
        }
        if !project.model.links.iter().any(|l| l.id == link_id) {
            record_identity(project, link_id, actor, revision, false)?;
            project.model.links.push(PhysicalAnalyticalLink {
                id: link_id,
                physical_id: id,
                analytical_id: member_id,
            });
        }
        member_ids.push(member_id);
        link_ids.push(link_id);
    }
    let record = FramingRecord {
        behavior: if input.kind == MemberKind::TrussBar {
            MemberBehavior::AxialOnly
        } else {
            MemberBehavior::Frame
        },
        input: input.clone(),
        node_ids,
        member_ids,
        link_ids,
        owned_node_ids,
        analytical_points_m: geometry.analytical_points_m,
        physical_points_m: geometry.physical_points_m,
        physical_breaks: geometry.physical_breaks,
        analytical_path_length_m: geometry.analytical_path_length_m,
        physical_length_m: geometry.physical_length_m,
        physical_length_is_exact: geometry.physical_length_is_exact,
        physical_length_error_estimate_m: geometry.physical_length_error_estimate_m,
        physical_chord_length_m: geometry.physical_chord_length_m,
        analytical_chord_length_m: geometry.analytical_chord_length_m,
        maximum_chord_error_m: geometry.maximum_chord_error_m,
        maximum_physical_chord_error_m: geometry.maximum_physical_chord_error_m,
        truss_id: truss_id.or_else(|| old.as_ref().and_then(|o| o.truss_id)),
    };
    let start_m = record.physical_points_m[0];
    let end_m = *record
        .physical_points_m
        .last()
        .ok_or(DomainError::DegenerateMember)?;
    if let Some(object) = project.objects.iter_mut().find(|o| o.id == id) {
        object.name = input.name;
        object.start_m = start_m;
        object.end_m = end_m;
        object.material_id = Some(input.material_id);
        object.extensions.insert(
            FRAMING_KEY.into(),
            serde_json::to_value(&record).map_err(|e| invalid(e.to_string()))?,
        );
    } else {
        record_identity(project, id, actor, revision, true)?;
        let mut extensions = BTreeMap::new();
        extensions.insert(
            FRAMING_KEY.into(),
            serde_json::to_value(&record).map_err(|e| invalid(e.to_string()))?,
        );
        project.objects.push(PhysicalObject {
            id,
            name: input.name,
            discipline: Discipline::Structure,
            start_m,
            end_m,
            state: ModelState::Modelled,
            role: PhysicalRole::AnalyticalSource,
            material_id: Some(input.material_id),
            environment_id: None,
            extensions,
        });
    }
    catalogue::assign_entry(project, id, input.sections[0].section_id)?;
    Ok(())
}

pub fn inspect(project: &Project, id: Uuid) -> Result<FramingInspection, DomainError> {
    let object = project
        .objects
        .iter()
        .find(|o| o.id == id)
        .ok_or(DomainError::UnknownReference(id))?;
    let record =
        metadata(object)?.ok_or_else(|| invalid("A peça não possui parâmetros estruturais."))?;
    let catalogue = catalogue::project_catalogue(project)?;
    let mut sections = Vec::new();
    for station in &record.input.sections {
        let entry = catalogue.latest(station.section_id)?;
        let CatalogPayload::Section { shape } = &entry.payload else {
            return Err(invalid("Catálogo não é seção."));
        };
        sections.push(ResolvedSection {
            section_id: entry.id,
            revision: entry.revision,
            sha256: entry.sha256()?,
            name: entry.name.clone(),
            fraction: station.fraction,
            area_m2: shape.geometry()?.area_m2,
        });
    }
    let material = project
        .model
        .materials
        .iter()
        .find(|m| m.id == record.input.material_id)
        .ok_or(DomainError::UnknownReference(record.input.material_id))?;
    let sources = object_sources(project, id)?;
    Ok(FramingInspection {
        project_id: project.id,
        revision: project.revision,
        object_id: id,
        bending_stiffness_permitted: record.behavior == MemberBehavior::Frame,
        record,
        material_name: material.name.clone(),
        sections,
        sources,
        engineering_release: false,
    })
}
pub fn display_polylines(project: &Project, id: Uuid) -> Result<Vec<Vec<Point3>>, DomainError> {
    let object = project
        .objects
        .iter()
        .find(|o| o.id == id)
        .ok_or(DomainError::UnknownReference(id))?;
    Ok(metadata(object)?.map_or_else(Vec::new, |record| {
        let mut lines = physical_polylines(&record.physical_points_m, &record.physical_breaks);
        lines.push(record.analytical_points_m);
        lines
    }))
}
/// Split displayed physical traces at explicit section jumps. Break indices
/// come from validated records or resolved geometry; invalid ranges are ignored
/// defensively here and are rejected by project validation.
pub fn physical_polylines(points: &[Point3], breaks: &[usize]) -> Vec<Vec<Point3>> {
    let mut result = Vec::new();
    let mut from = 0;
    for to in breaks.iter().copied().chain(std::iter::once(points.len())) {
        if to > from && to <= points.len() {
            result.push(points[from..to].to_vec());
            from = to;
        }
    }
    result
}
pub fn source_ids(record: &FramingRecord) -> Vec<Uuid> {
    let mut ids = vec![record.input.material_id];
    ids.extend(record.input.sections.iter().map(|s| s.section_id));
    match &record.input.path {
        MemberPath::Straight { start, end } => {
            ids.extend(layout::point_sources(start));
            ids.extend(layout::point_sources(end));
        }
        MemberPath::CircularArc {
            start,
            through,
            end,
            ..
        } => {
            ids.extend(layout::point_sources(start));
            ids.extend(layout::point_sources(through));
            ids.extend(layout::point_sources(end));
        }
    }
    ids.sort();
    ids.dedup();
    ids
}
pub fn object_sources(
    project: &Project,
    id: Uuid,
) -> Result<Vec<crate::dependencies::SourceKey>, DomainError> {
    use crate::dependencies::{SourceKey, SourceKind};
    let object = project
        .objects
        .iter()
        .find(|object| object.id == id)
        .ok_or(DomainError::UnknownReference(id))?;
    let record = metadata(object)?.ok_or_else(|| invalid("Peça sem parâmetros estruturais."))?;
    let mut sources = BTreeSet::from([
        SourceKey {
            kind: SourceKind::Physical,
            id,
        },
        SourceKey {
            kind: SourceKind::PhysicalGeometry,
            id,
        },
        SourceKey {
            kind: SourceKind::Material,
            id: record.input.material_id,
        },
    ]);
    sources.extend(record.input.sections.iter().map(|section| SourceKey {
        kind: SourceKind::CatalogEntry,
        id: section.section_id,
    }));
    sources.extend(record.node_ids.iter().map(|id| SourceKey {
        kind: SourceKind::AnalyticalNode,
        id: *id,
    }));
    sources.extend(record.member_ids.iter().map(|id| SourceKey {
        kind: SourceKind::AnalyticalMember,
        id: *id,
    }));
    sources.extend(record.link_ids.iter().map(|id| SourceKey {
        kind: SourceKind::Link,
        id: *id,
    }));
    let layout = layout::state(project)?;
    let layout_ids: BTreeSet<_> = layout
        .levels
        .iter()
        .map(|x| x.id)
        .chain(layout.axes.iter().map(|x| x.id))
        .chain(layout.frames.iter().map(|x| x.id))
        .collect();
    for id in source_ids(&record) {
        if layout_ids.contains(&id) {
            sources.insert(SourceKey {
                kind: SourceKind::Layout,
                id,
            });
            if let Some(axis) = layout.axes.iter().find(|axis| axis.id == id)
                && let Some(frame_id) = axis.input.frame_id
            {
                sources.insert(SourceKey {
                    kind: SourceKind::Layout,
                    id: frame_id,
                });
            }
        } else if project
            .model
            .analytical_nodes
            .iter()
            .any(|node| node.id == id)
        {
            sources.insert(SourceKey {
                kind: SourceKind::AnalyticalNode,
                id,
            });
        }
    }
    Ok(sources.into_iter().collect())
}
/// Rebuild all construction points atomically. Explicit shared nodes move only if
/// every incident parametric member agrees on their new location.
pub fn regenerate_layout_dependents(project: &mut Project) -> Result<(), DomainError> {
    let mut candidate = project.clone();
    let mut pending = project
        .objects
        .iter()
        .filter_map(|o| metadata(o).transpose().map(|r| r.map(|r| (o.id, r))))
        .collect::<Result<Vec<_>, _>>()?;
    let ownership: BTreeMap<_, _> = pending
        .iter()
        .flat_map(|(id, record)| record.owned_node_ids.iter().map(move |node| (*node, *id)))
        .collect();
    let mut done = BTreeSet::new();
    let mut desired = BTreeMap::<Uuid, Point3>::new();
    let mut resolved = Vec::new();
    while !pending.is_empty() {
        let ready=pending.iter().position(|(_,record)|{
            let points:Vec<_>=match &record.input.path{MemberPath::Straight{start,end}=>vec![start,end],MemberPath::CircularArc{start,through,end,..}=>vec![start,through,end]};
            points.iter().all(|p|match p{LayoutPoint::Node{node_id}=>ownership.get(node_id).is_none_or(|owner|done.contains(owner)),_=>true})
        }).ok_or_else(||invalid("Referências de pontos formam ciclo entre peças. Use eixo, nível ou coordenada de origem independente."))?;
        let (id, record) = pending.remove(ready);
        let geometry = resolve_geometry(&candidate, &record.input)?;
        if geometry.analytical_points_m.len() != record.node_ids.len() {
            return Err(invalid("Quantidade de nós diverge da discretização."));
        }
        for (node_id, position) in record.node_ids.iter().zip(&geometry.analytical_points_m) {
            if let Some(existing) = desired.get(node_id) {
                if existing.distance(*position) > 1e-8 {
                    return Err(DomainError::SharedTopology);
                }
            } else {
                desired.insert(*node_id, *position);
            }
            let node = candidate
                .model
                .analytical_nodes
                .iter_mut()
                .find(|n| n.id == *node_id)
                .ok_or(DomainError::UnknownReference(*node_id))?;
            node.position_m = *position;
        }
        done.insert(id);
        resolved.push((id, record, geometry));
    }
    let all_members: BTreeSet<_> = resolved
        .iter()
        .flat_map(|(_, record, _)| record.member_ids.iter().copied())
        .collect();
    for (node_id, position) in &desired {
        let node = project
            .model
            .analytical_nodes
            .iter()
            .find(|n| n.id == *node_id)
            .ok_or(DomainError::UnknownReference(*node_id))?;
        if node.position_m != *position
            && project.model.analytical_members.iter().any(|m| {
                (m.start_node_id == *node_id || m.end_node_id == *node_id)
                    && !all_members.contains(&m.id)
            })
        {
            return Err(DomainError::SharedTopology);
        }
    }
    for (id, mut record, geometry) in resolved {
        for (index, member_id) in record.member_ids.iter().enumerate() {
            let member = candidate
                .model
                .analytical_members
                .iter_mut()
                .find(|m| m.id == *member_id)
                .ok_or(DomainError::UnknownReference(*member_id))?;
            member.local_y = geometry.analytical_local_y[index];
        }
        record.analytical_points_m = geometry.analytical_points_m;
        record.physical_points_m = geometry.physical_points_m;
        record.physical_breaks = geometry.physical_breaks;
        record.analytical_path_length_m = geometry.analytical_path_length_m;
        record.physical_length_m = geometry.physical_length_m;
        record.physical_length_is_exact = geometry.physical_length_is_exact;
        record.physical_length_error_estimate_m = geometry.physical_length_error_estimate_m;
        record.physical_chord_length_m = geometry.physical_chord_length_m;
        record.analytical_chord_length_m = geometry.analytical_chord_length_m;
        record.maximum_chord_error_m = geometry.maximum_chord_error_m;
        record.maximum_physical_chord_error_m = geometry.maximum_physical_chord_error_m;
        let object = candidate
            .objects
            .iter_mut()
            .find(|o| o.id == id)
            .ok_or(DomainError::UnknownReference(id))?;
        object.start_m = record.physical_points_m[0];
        object.end_m = *record
            .physical_points_m
            .last()
            .ok_or(DomainError::DegenerateMember)?;
        object.extensions.insert(
            FRAMING_KEY.into(),
            serde_json::to_value(record).map_err(|e| invalid(e.to_string()))?,
        );
    }
    validate_project(&candidate)?;
    *project = candidate;
    Ok(())
}
pub fn validate_project(project: &Project) -> Result<(), DomainError> {
    let mut represented = BTreeSet::new();
    for object in &project.objects {
        let Some(record) = metadata(object)? else {
            continue;
        };
        if object.discipline != Discipline::Structure
            || object.role != PhysicalRole::AnalyticalSource
            || object.material_id != Some(record.input.material_id)
            || object.name != record.input.name
        {
            return Err(invalid(
                "Atributos da peça divergem de seus parâmetros estruturais.",
            ));
        }
        let geometry = resolve_geometry(project, &record.input)?;
        let path = resolve_path(project, &record.input.path)?;
        let count = geometry.analytical_points_m.len();
        if !points_reconstructed_equal(&record.analytical_points_m, &geometry.analytical_points_m)
            || !points_reconstructed_equal(&record.physical_points_m, &geometry.physical_points_m)
            || record.physical_breaks != geometry.physical_breaks
            || !reconstructed_equal(
                record.analytical_path_length_m,
                geometry.analytical_path_length_m,
            )
            || !reconstructed_equal(record.physical_length_m, geometry.physical_length_m)
            || record.physical_length_is_exact != geometry.physical_length_is_exact
            || !reconstructed_equal(
                record.physical_length_error_estimate_m,
                geometry.physical_length_error_estimate_m,
            )
            || !reconstructed_equal(
                record.physical_chord_length_m,
                geometry.physical_chord_length_m,
            )
            || !reconstructed_equal(
                record.analytical_chord_length_m,
                geometry.analytical_chord_length_m,
            )
            || !reconstructed_equal(record.maximum_chord_error_m, geometry.maximum_chord_error_m)
            || !reconstructed_equal(
                record.maximum_physical_chord_error_m,
                geometry.maximum_physical_chord_error_m,
            )
            || record.node_ids.len() != count
            || record.member_ids.len() + 1 != count
            || record.link_ids.len() != record.member_ids.len()
        {
            return Err(invalid(
                "Geometria paramétrica ou analítica desatualizada. Edite pelos parâmetros da peça.",
            ));
        }
        if object.start_m != record.physical_points_m[0]
            || object.end_m
                != *record
                    .physical_points_m
                    .last()
                    .ok_or(DomainError::DegenerateMember)?
        {
            return Err(invalid("Extremos físicos divergem da curva paramétrica."));
        }
        if record.behavior
            != if record.input.kind == MemberKind::TrussBar {
                MemberBehavior::AxialOnly
            } else {
                MemberBehavior::Frame
            }
        {
            return Err(invalid(
                "Comportamento analítico incompatível; treliça não admite rigidez à flexão.",
            ));
        }
        if catalogue::assignments(object)?.get(&CatalogKind::Section)
            != Some(&record.input.sections[0].section_id)
        {
            return Err(invalid(
                "Atribuição de seção diverge das estações paramétricas.",
            ));
        }
        let expected_owned: Vec<_> = (0..count)
            .filter(|i| {
                !((*i == 0 && record.input.connections[0].is_some())
                    || (*i + 1 == count && record.input.connections[1].is_some()))
            })
            .map(|i| derived_id(object.id, "node", i))
            .collect();
        if record.owned_node_ids != expected_owned {
            return Err(invalid("Propriedade dos nós paramétricos inválida."));
        }
        for (index, node_id) in record.node_ids.iter().enumerate() {
            let expected = if index == 0 {
                record.input.connections[0]
            } else if index + 1 == count {
                record.input.connections[1]
            } else {
                None
            }
            .unwrap_or_else(|| derived_id(object.id, "node", index));
            if *node_id != expected {
                return Err(invalid("Identidade de nó paramétrico alterada."));
            }
            let node = project
                .model
                .analytical_nodes
                .iter()
                .find(|n| n.id == *node_id)
                .ok_or(DomainError::UnknownReference(*node_id))?;
            if node.position_m != record.analytical_points_m[index] {
                return Err(invalid("Nó analítico diverge da posição paramétrica."));
            }
        }
        for (index, member_id) in record.member_ids.iter().enumerate() {
            if *member_id != derived_id(object.id, "member", index)
                || record.link_ids[index] != derived_id(object.id, "link", index)
                || !represented.insert(*member_id)
            {
                return Err(invalid("Identidade ou vínculo analítico duplicado."));
            }
            let member = project
                .model
                .analytical_members
                .iter()
                .find(|m| m.id == *member_id)
                .ok_or(DomainError::UnknownReference(*member_id))?;
            if member.start_node_id != record.node_ids[index]
                || member.end_node_id != record.node_ids[index + 1]
                || Vector3::from(member.local_y).metric_distance(&Vector3::from(analytical_y(
                    &path,
                    record.input.roll_degrees,
                    &record.analytical_points_m,
                    index,
                )?)) > 1e-12
            {
                return Err(invalid("Barra analítica diverge dos parâmetros."));
            }
            if !project.model.links.iter().any(|l| {
                l.id == record.link_ids[index]
                    && l.physical_id == object.id
                    && l.analytical_id == *member_id
            }) {
                return Err(invalid("Vínculo físico-analítico paramétrico ausente."));
            }
        }
        let links: Vec<_> = project
            .model
            .links
            .iter()
            .filter(|l| l.physical_id == object.id)
            .collect();
        if links.len() != record.member_ids.len() {
            return Err(invalid("Peça paramétrica contém vínculos extras."));
        }
    }
    Ok(())
}
