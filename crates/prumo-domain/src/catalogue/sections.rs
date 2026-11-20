//! A single geometric authority for catalogue sections and component qualification.
use super::{finite, invalid};
use crate::DomainError;
use geo::{Area, BooleanOps, Centroid, Coord, LineString, MultiPolygon, Polygon, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod torsion;
pub use torsion::{TorsionResult, TorsionState};

pub const MAX_VERTICES: usize = 512;
pub const MAX_COORDINATE_M: f64 = 1e9;
pub const MAX_SPAN_M: f64 = 1e6;
pub const MIN_EDGE_M: f64 = 1e-9;
pub const MIN_RELATIVE_AREA: f64 = 1e-12;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SectionInput {
    #[serde(deserialize_with = "points")]
    pub exterior_m: Vec<[f64; 2]>,
    #[serde(deserialize_with = "holes")]
    pub holes_m: Vec<Vec<[f64; 2]>>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SectionGeometry {
    pub area_m2: f64,
    pub centroid_m: [f64; 2],
    pub ixx_m4: f64,
    pub iyy_m4: f64,
    /// Integral (x-xc)(y-yc)dA, not its negated tensor entry.
    pub ixy_m4: f64,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SectionShape {
    Rectangle {
        #[serde(deserialize_with = "finite")]
        width_m: f64,
        #[serde(deserialize_with = "finite")]
        height_m: f64,
    },
    Circle {
        #[serde(deserialize_with = "finite")]
        diameter_m: f64,
    },
    ISection {
        #[serde(deserialize_with = "finite")]
        width_m: f64,
        #[serde(deserialize_with = "finite")]
        height_m: f64,
        #[serde(deserialize_with = "finite")]
        flange_m: f64,
        #[serde(deserialize_with = "finite")]
        web_m: f64,
    },
    Box {
        #[serde(deserialize_with = "finite")]
        width_m: f64,
        #[serde(deserialize_with = "finite")]
        height_m: f64,
        #[serde(deserialize_with = "finite")]
        wall_m: f64,
    },
    Polygon {
        section: SectionInput,
    },
    /// Explicit homogeneous geometric union of polygonal parts. Overlap is rejected.
    Compound {
        parts: Vec<SectionPart>,
    },
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SectionPart {
    pub shape: Box<SectionShape>,
    #[serde(deserialize_with = "pair")]
    pub translation_m: [f64; 2],
    #[serde(deserialize_with = "finite")]
    pub rotation_rad: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct SectionAnalysis {
    pub geometry: SectionGeometry,
    pub torsion: TorsionResult,
    /// The geometric J is qualified for this material/model hypothesis only.
    pub torsion_assumption: &'static str,
    pub engineering_release: bool,
}

fn pair<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[f64; 2], D::Error> {
    let values = <[serde_json::Number; 2]>::deserialize(d)?;
    let mut result = [0.0; 2];
    for (i, value) in values.into_iter().enumerate() {
        result[i] = value
            .as_f64()
            .filter(|x| x.is_finite())
            .ok_or_else(|| serde::de::Error::custom("Coordenada não finita."))?;
    }
    Ok(result)
}
fn points<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<[f64; 2]>, D::Error> {
    let values = Vec::<[serde_json::Number; 2]>::deserialize(d)?;
    values
        .into_iter()
        .map(|point| {
            let [x, y] = point;
            match (x.as_f64(), y.as_f64()) {
                (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Ok([x, y]),
                _ => Err(serde::de::Error::custom("Coordenada não finita.")),
            }
        })
        .collect()
}
fn holes<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Vec<[f64; 2]>>, D::Error> {
    let values = Vec::<Vec<[serde_json::Number; 2]>>::deserialize(d)?;
    values
        .into_iter()
        .map(|ring| {
            ring.into_iter()
                .map(|point| {
                    let [x, y] = point;
                    match (x.as_f64(), y.as_f64()) {
                        (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Ok([x, y]),
                        _ => Err(serde::de::Error::custom("Coordenada não finita.")),
                    }
                })
                .collect()
        })
        .collect()
}

#[derive(Debug, Error)]
pub enum SectionError {
    #[error("Este adaptador não qualifica seções com furos.")]
    HolesUnsupported,
    #[error("O polígono deve ter de 3 a 512 vértices, mais fechamento opcional.")]
    VertexCount,
    #[error("Coordenadas devem ser finitas e ter módulo até 1e9 m.")]
    InvalidCoordinate,
    #[error("A dimensão máxima da seção excede 1e6 m.")]
    SpanExceeded,
    #[error("Aresta menor que 1e-9 m ou vértice consecutivo repetido.")]
    ShortEdge,
    #[error("Polígono inválido: {0}.")]
    InvalidPolygon(String),
    #[error(
        "Seção degenerada, numericamente indefinida ou com área relativa menor ou igual a 1e-12."
    )]
    Degenerate,
}

fn ring(points: &[[f64; 2]], origin: [f64; 2]) -> Result<LineString<f64>, SectionError> {
    if points.len() < 3 || points.len() > MAX_VERTICES + 1 {
        return Err(SectionError::VertexCount);
    }
    let mut vertices = points.to_vec();
    if vertices.first() == vertices.last() {
        vertices.pop();
    }
    if vertices.len() < 3 || vertices.len() > MAX_VERTICES {
        return Err(SectionError::VertexCount);
    }
    for vertex in &mut vertices {
        if vertex
            .iter()
            .any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE_M)
        {
            return Err(SectionError::InvalidCoordinate);
        }
        vertex[0] -= origin[0];
        vertex[1] -= origin[1];
    }
    for i in 0..vertices.len() {
        let next = vertices[(i + 1) % vertices.len()];
        if (vertices[i][0] - next[0]).hypot(vertices[i][1] - next[1]) < MIN_EDGE_M {
            return Err(SectionError::ShortEdge);
        }
    }
    Ok(LineString::new(
        vertices
            .into_iter()
            .map(|p| Coord { x: p[0], y: p[1] })
            .collect(),
    ))
}

pub(crate) fn polygon(input: &SectionInput) -> Result<(Polygon<f64>, [f64; 2]), SectionError> {
    let origin = *input.exterior_m.first().ok_or(SectionError::VertexCount)?;
    if input.holes_m.len() > 64
        || input.holes_m.iter().map(Vec::len).sum::<usize>() + input.exterior_m.len()
            > MAX_VERTICES * 4
    {
        return Err(SectionError::VertexCount);
    }
    let exterior = ring(&input.exterior_m, origin)?;
    let interiors = input
        .holes_m
        .iter()
        .map(|points| ring(points, origin))
        .collect::<Result<Vec<_>, _>>()?;
    let polygon = Polygon::new(exterior, interiors);
    polygon
        .check_validation()
        .map_err(|error| SectionError::InvalidPolygon(error.to_string()))?;
    let (min, max) = polygon.exterior().0.iter().fold(
        ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]),
        |(mut min, mut max), p| {
            min[0] = min[0].min(p.x);
            min[1] = min[1].min(p.y);
            max[0] = max[0].max(p.x);
            max[1] = max[1].max(p.y);
            (min, max)
        },
    );
    let span = (max[0] - min[0]).max(max[1] - min[1]);
    if span > MAX_SPAN_M {
        return Err(SectionError::SpanExceeded);
    }
    if !polygon.unsigned_area().is_finite()
        || polygon.unsigned_area() <= MIN_RELATIVE_AREA * span * span
    {
        return Err(SectionError::Degenerate);
    }
    Ok((polygon, origin))
}

/// Green integrals about the local centroid preserve inertia at large coordinates.
/// The implementation previously qualified for simple polygons now also subtracts holes.
pub fn polygon_properties(input: &SectionInput) -> Result<SectionGeometry, SectionError> {
    let (polygon, origin) = polygon(input)?;
    let area = polygon.unsigned_area();
    let centroid = polygon.centroid().ok_or(SectionError::Degenerate)?;
    let mut moments = [0.0; 3];
    for (index, line) in std::iter::once(polygon.exterior())
        .chain(polygon.interiors())
        .enumerate()
    {
        let signed_area = line
            .0
            .windows(2)
            .map(|pair| pair[0].x * pair[1].y - pair[1].x * pair[0].y)
            .sum::<f64>()
            / 2.0;
        let orientation = signed_area.signum() * if index == 0 { 1.0 } else { -1.0 };
        for points in line.0.windows(2) {
            let x = points[0].x - centroid.x();
            let y = points[0].y - centroid.y();
            let nx = points[1].x - centroid.x();
            let ny = points[1].y - centroid.y();
            let cross = orientation * (x * ny - nx * y);
            moments[0] += cross * (y * y + y * ny + ny * ny) / 12.0;
            moments[1] += cross * (x * x + x * nx + nx * nx) / 12.0;
            moments[2] += cross * (2.0 * x * y + x * ny + nx * y + 2.0 * nx * ny) / 24.0;
        }
    }
    let result = SectionGeometry {
        area_m2: area,
        centroid_m: [origin[0] + centroid.x(), origin[1] + centroid.y()],
        ixx_m4: moments[0],
        iyy_m4: moments[1],
        ixy_m4: moments[2],
    };
    if moments.iter().any(|v| !v.is_finite())
        || result.centroid_m.iter().any(|v| !v.is_finite())
        || result.ixx_m4 <= 0.0
        || result.iyy_m4 <= 0.0
        || result.ixx_m4 * result.iyy_m4 <= result.ixy_m4.powi(2)
    {
        return Err(SectionError::Degenerate);
    }
    Ok(result)
}

fn dimension(value: f64) -> Result<(), DomainError> {
    if !value.is_finite() || !(1e-8..=MAX_SPAN_M).contains(&value) {
        Err(invalid(
            "Dimensão de seção deve estar entre 1e-8 m e 1e6 m.",
        ))
    } else {
        Ok(())
    }
}
fn rectangle(width: f64, height: f64) -> SectionInput {
    SectionInput {
        exterior_m: vec![[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]],
        holes_m: vec![],
    }
}

impl SectionShape {
    pub fn geometry(&self) -> Result<SectionGeometry, DomainError> {
        self.geometry_depth(0)
    }
    fn geometry_depth(&self, depth: usize) -> Result<SectionGeometry, DomainError> {
        if depth > 4 {
            return Err(invalid("Composição de seções excede quatro níveis."));
        }
        if let Self::Circle { diameter_m } = self {
            dimension(*diameter_m)?;
            let radius = diameter_m / 2.0;
            return Ok(SectionGeometry {
                area_m2: std::f64::consts::PI * radius.powi(2),
                centroid_m: [0.0, 0.0],
                ixx_m4: std::f64::consts::PI * radius.powi(4) / 4.0,
                iyy_m4: std::f64::consts::PI * radius.powi(4) / 4.0,
                ixy_m4: 0.0,
            });
        }
        let polygons = self.polygons(depth)?;
        aggregate(&polygons)
    }
    pub(crate) fn polygons(&self, depth: usize) -> Result<Vec<SectionInput>, DomainError> {
        if depth > 4 {
            return Err(invalid("Composição de seções excede quatro níveis."));
        }
        let single = match self {
            Self::Rectangle {
                width_m: w,
                height_m: h,
            } => {
                dimension(*w)?;
                dimension(*h)?;
                rectangle(*w, *h)
            }
            Self::ISection {
                width_m: w,
                height_m: h,
                flange_m: f,
                web_m: t,
            } => {
                for value in [*w, *h, *f, *t] {
                    dimension(value)?;
                }
                if 2.0 * f >= *h || t >= w {
                    return Err(invalid("Mesa ou alma incompatível com a seção I."));
                }
                let a = (w - t) / 2.0;
                let b = (w + t) / 2.0;
                SectionInput {
                    exterior_m: vec![
                        [0.0, 0.0],
                        [*w, 0.0],
                        [*w, *f],
                        [b, *f],
                        [b, h - f],
                        [*w, h - f],
                        [*w, *h],
                        [0.0, *h],
                        [0.0, h - f],
                        [a, h - f],
                        [a, *f],
                        [0.0, *f],
                    ],
                    holes_m: vec![],
                }
            }
            Self::Box {
                width_m: w,
                height_m: h,
                wall_m: t,
            } => {
                for value in [*w, *h, *t] {
                    dimension(value)?;
                }
                if 2.0 * t >= w.min(*h) {
                    return Err(invalid("Espessura de caixa fecha ou inverte o vazio."));
                }
                let mut result = rectangle(*w, *h);
                result
                    .holes_m
                    .push(vec![[*t, *t], [w - t, *t], [w - t, h - t], [*t, h - t]]);
                result
            }
            Self::Polygon { section } => section.clone(),
            Self::Circle { .. } => {
                return Err(invalid(
                    "A composição admite partes poligonais. Círculos isolados usam solução analítica exata.",
                ));
            }
            Self::Compound { parts } => {
                if parts.is_empty() || parts.len() > 32 {
                    return Err(invalid("Composição exige de 1 a 32 partes."));
                }
                let mut merged = MultiPolygon::<f64>(Vec::new());
                for part in parts {
                    if !part.rotation_rad.is_finite()
                        || part
                            .translation_m
                            .iter()
                            .any(|v| !v.is_finite() || v.abs() > MAX_COORDINATE_M)
                    {
                        return Err(invalid("Posição de parte inválida."));
                    }
                    let (sin, cos) = part.rotation_rad.sin_cos();
                    for mut input in part.shape.polygons(depth + 1)? {
                        for ring in
                            std::iter::once(&mut input.exterior_m).chain(input.holes_m.iter_mut())
                        {
                            for point in ring {
                                let [x, y] = *point;
                                *point = [
                                    cos * x - sin * y + part.translation_m[0],
                                    sin * x + cos * y + part.translation_m[1],
                                ];
                            }
                        }
                        let (local, origin) =
                            polygon(&input).map_err(|e| invalid(&e.to_string()))?;
                        let translated = translate_polygon(local, origin);
                        if !merged.0.is_empty()
                            && merged
                                .intersection(&MultiPolygon(vec![translated.clone()]))
                                .unsigned_area()
                                > 1e-12 * translated.unsigned_area()
                        {
                            return Err(invalid(
                                "Partes de seção sobrepostas; remova a duplicidade antes de unir.",
                            ));
                        }
                        merged = merged.union(&MultiPolygon(vec![translated]));
                    }
                }
                let result: Vec<_> = merged.0.into_iter().map(from_polygon).collect();
                for input in &result {
                    polygon_properties(input).map_err(|e| invalid(&e.to_string()))?;
                }
                return Ok(result);
            }
        };
        polygon_properties(&single).map_err(|e| invalid(&e.to_string()))?;
        Ok(vec![single])
    }
}

fn translate_polygon(polygon: Polygon<f64>, origin: [f64; 2]) -> Polygon<f64> {
    let convert = |line: &LineString<f64>| {
        LineString::new(
            line.0
                .iter()
                .map(|p| Coord {
                    x: p.x + origin[0],
                    y: p.y + origin[1],
                })
                .collect(),
        )
    };
    Polygon::new(
        convert(polygon.exterior()),
        polygon.interiors().iter().map(convert).collect(),
    )
}
fn from_polygon(polygon: Polygon<f64>) -> SectionInput {
    SectionInput {
        exterior_m: polygon.exterior().0.iter().map(|p| [p.x, p.y]).collect(),
        holes_m: polygon
            .interiors()
            .iter()
            .map(|line| line.0.iter().map(|p| [p.x, p.y]).collect())
            .collect(),
    }
}
fn aggregate(polygons: &[SectionInput]) -> Result<SectionGeometry, DomainError> {
    if polygons.is_empty() {
        return Err(invalid("Seção vazia."));
    }
    let items = polygons
        .iter()
        .map(polygon_properties)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| invalid(&e.to_string()))?;
    let origin = items[0].centroid_m;
    let area = items.iter().map(|p| p.area_m2).sum::<f64>();
    let local = [
        items
            .iter()
            .map(|p| p.area_m2 * (p.centroid_m[0] - origin[0]))
            .sum::<f64>()
            / area,
        items
            .iter()
            .map(|p| p.area_m2 * (p.centroid_m[1] - origin[1]))
            .sum::<f64>()
            / area,
    ];
    let centroid = [origin[0] + local[0], origin[1] + local[1]];
    let mut result = SectionGeometry {
        area_m2: area,
        centroid_m: centroid,
        ixx_m4: 0.0,
        iyy_m4: 0.0,
        ixy_m4: 0.0,
    };
    for item in items {
        let dx = (item.centroid_m[0] - origin[0]) - local[0];
        let dy = (item.centroid_m[1] - origin[1]) - local[1];
        result.ixx_m4 += item.ixx_m4 + item.area_m2 * dy * dy;
        result.iyy_m4 += item.iyy_m4 + item.area_m2 * dx * dx;
        result.ixy_m4 += item.ixy_m4 + item.area_m2 * dx * dy;
    }
    Ok(result)
}

pub fn analyse_section(shape: &SectionShape) -> Result<SectionAnalysis, DomainError> {
    let geometry = shape.geometry()?;
    let torsion = match shape {
        SectionShape::Circle { diameter_m } => TorsionResult::analytical(
            std::f64::consts::PI * diameter_m.powi(4) / 32.0,
            "Saint-Venant, círculo maciço: pi*d^4/32",
        ),
        SectionShape::Rectangle { width_m, height_m } => TorsionResult::analytical(
            torsion::rectangle_torsion(*width_m, *height_m),
            "Saint-Venant, retângulo: série de Fourier, 200 termos ímpares",
        ),
        _ => torsion::polygon_torsion(&shape.polygons(0)?)?,
    };
    Ok(SectionAnalysis {
        geometry,
        torsion,
        torsion_assumption: "homogeneous_isotropic_saint_venant",
        engineering_release: false,
    })
}
