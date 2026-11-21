//! PRU-023: mesh geometry, orientation, quality and convergence protocols.
//! This module intentionally does not claim a membrane/plate/shell FEM formulation.
use crate::{AnalysisError, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_VERTICES: usize = 100_000;
pub const MAX_TRIANGLES: usize = 200_000;
pub const MAX_ASPECT_RATIO: f64 = 100.0;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mesh {
    pub points_m: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
    pub thickness_m: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct TriangleQuality {
    pub triangle: usize,
    pub area_m2: f64,
    pub aspect_ratio: f64,
    pub normal: [f64; 3],
}
#[derive(Clone, Debug, Serialize)]
pub struct Inspection {
    pub elements: Vec<TriangleQuality>,
    pub total_area_m2: f64,
    pub number_of_boundary_edges: usize,
    pub analysis_available: bool,
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn inspect(mesh: &Mesh) -> Result<Inspection, AnalysisError> {
    if mesh.points_m.len() > MAX_VERTICES || mesh.triangles.len() > MAX_TRIANGLES {
        return Err(AnalysisError::ResourceLimit(
            "malha acima do limite de segurança".into(),
        ));
    }
    ensure(
        !mesh.triangles.is_empty() && mesh.points_m.len() >= 3,
        "Malha vazia ou sem triângulos.",
    )?;
    ensure(
        mesh.thickness_m.is_finite() && mesh.thickness_m > 0.0 && mesh.thickness_m <= 100.0,
        "Espessura de placa inválida.",
    )?;
    ensure(
        mesh.points_m
            .iter()
            .flatten()
            .all(|v| v.is_finite() && v.abs() <= 1e8),
        "Vértice não finito ou fora do domínio.",
    )?;
    let mut indices = BTreeSet::new();
    let mut edges = BTreeMap::<(usize, usize), usize>::new();
    let mut values = Vec::new();
    let mut normal0: Option<[f64; 3]> = None;
    for (index, &triangle) in mesh.triangles.iter().enumerate() {
        ensure(
            triangle.iter().all(|&i| i < mesh.points_m.len())
                && triangle.iter().copied().collect::<BTreeSet<_>>().len() == 3,
            "Triângulo com vértice inválido, repetido ou fora do intervalo.",
        )?;
        let mut sorted = triangle;
        sorted.sort_unstable();
        ensure(indices.insert(sorted), "Triângulo duplicado.")?;
        let [i, j, k] = triangle;
        for (a, b) in [(i, j), (j, k), (k, i)] {
            let key = (a.min(b), a.max(b));
            *edges.entry(key).or_default() += 1;
        }
        let p = mesh.points_m[i];
        let q = mesh.points_m[j];
        let r = mesh.points_m[k];
        let u: [f64; 3] = std::array::from_fn(|m| q[m] - p[m]);
        let v: [f64; 3] = std::array::from_fn(|m| r[m] - p[m]);
        let w: [f64; 3] = std::array::from_fn(|m| r[m] - q[m]);
        let n = cross(u, v);
        let magnitude = n.iter().map(|v| v * v).sum::<f64>().sqrt();
        ensure(
            magnitude.is_finite() && magnitude > 1e-14,
            "Triângulo degenerado ou de área próxima de zero.",
        )?;
        let unit = n.map(|v| v / magnitude);
        if let Some(basis) = normal0 {
            let dot: f64 = (0..3).map(|m| basis[m] * unit[m]).sum();
            ensure(
                dot > 0.0,
                "Elemento invertido ou malha não orientada consistentemente.",
            )?;
        } else {
            normal0 = Some(unit);
        }
        let lengths = [u, v, w].map(|side| side.iter().map(|v| v * v).sum::<f64>().sqrt());
        let longest = lengths.iter().copied().fold(0.0, f64::max);
        let altitude = magnitude / longest;
        let aspect = longest / altitude;
        ensure(
            aspect.is_finite() && aspect <= MAX_ASPECT_RATIO,
            "Triângulo excessivamente alongado, sujeito a erro numérico.",
        )?;
        values.push(TriangleQuality {
            triangle: index,
            area_m2: magnitude / 2.0,
            aspect_ratio: aspect,
            normal: unit,
        });
    }
    ensure(
        edges.values().all(|&count| count <= 2),
        "Aresta pertencente a mais de dois triângulos.",
    )?;
    Ok(Inspection {
        total_area_m2: values.iter().map(|x| x.area_m2).sum(),
        elements: values,
        number_of_boundary_edges: edges.values().filter(|&&c| c == 1).count(),
        analysis_available: false,
    })
}
/// Splits every triangle into four without changing the boundary geometry.
/// A hole only remains a hole when no triangle covers its interior.
pub fn refine(mesh: &Mesh) -> Result<Mesh, AnalysisError> {
    inspect(mesh)?;
    if mesh.triangles.len() > MAX_TRIANGLES / 4
        || mesh.points_m.len() + 3 * mesh.triangles.len() > MAX_VERTICES
    {
        return Err(AnalysisError::ResourceLimit(
            "refinamento excede o limite".into(),
        ));
    }
    let mut out = Mesh {
        points_m: mesh.points_m.clone(),
        triangles: Vec::with_capacity(mesh.triangles.len() * 4),
        thickness_m: mesh.thickness_m,
    };
    let mut midpoints = BTreeMap::new();
    let mut midpoint = |a: usize, b: usize, points: &mut Vec<[f64; 3]>| -> usize {
        let key = (a.min(b), a.max(b));
        *midpoints.entry(key).or_insert_with(|| {
            let original = points[a];
            let other = points[b];
            let id = points.len();
            points.push(std::array::from_fn(|d| (original[d] + other[d]) * 0.5));
            id
        })
    };
    for &[a, b, c] in &mesh.triangles {
        let ab = midpoint(a, b, &mut out.points_m);
        let bc = midpoint(b, c, &mut out.points_m);
        let ca = midpoint(c, a, &mut out.points_m);
        out.triangles
            .extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
    }
    inspect(&out)?;
    Ok(out)
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConvergencePoint {
    pub elements: usize,
    pub value: f64,
    pub unit: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct ConvergenceReport {
    pub last_value: f64,
    pub absolute_difference: f64,
    pub relative_difference: f64,
    pub converged: bool,
    pub engineering_release: bool,
}
/// A convergence protocol alone is not a mesh-independent structural solution.
pub fn convergence(
    series: &[ConvergencePoint],
    relative_tolerance: f64,
) -> Result<ConvergenceReport, AnalysisError> {
    ensure(
        series.len() >= 3
            && series.len() <= 32
            && relative_tolerance.is_finite()
            && relative_tolerance > 0.0
            && relative_tolerance <= 0.1,
        "Requer pelo menos três malhas e tolerância explícita.",
    )?;
    let unit = &series[0].unit;
    ensure(
        !unit.is_empty()
            && series
                .iter()
                .all(|p| p.unit == *unit && p.value.is_finite() && p.elements > 0),
        "Grandezas, unidades ou valores de convergência inválidos.",
    )?;
    ensure(
        series.windows(2).all(|p| p[1].elements > p[0].elements),
        "A série deve ser estritamente refinada.",
    )?;
    let last = series[series.len() - 1].value;
    let previous = series[series.len() - 2].value;
    let old = series[series.len() - 3].value;
    let difference = (last - previous).abs();
    let previous_difference = (previous - old).abs();
    let relative = difference / last.abs().max(previous.abs()).max(1e-12);
    Ok(ConvergenceReport {
        last_value: last,
        absolute_difference: difference,
        relative_difference: relative,
        converged: relative <= relative_tolerance && difference <= previous_difference,
        engineering_release: false,
    })
}
