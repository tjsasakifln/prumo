//! Saint-Venant warping with quadratic triangles, sparse PCG and fixed convergence policy.
use super::{SectionInput, invalid, polygon, polygon_properties};
use crate::DomainError;
use geo::{Coord, TriangulateEarcut};
use serde::Serialize;
use std::collections::BTreeMap;

const MAX_REFINEMENTS: u32 = 6;
const MAX_TRIANGLES: usize = 50_000;
const MAX_NODES: usize = 26_000;
const CONVERGENCE_RELATIVE: f64 = 0.002;
const RESIDUAL_RELATIVE: f64 = 1e-10;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TorsionState {
    Analytical,
    Converged,
    NotConverged,
}
#[derive(Clone, Debug, Serialize)]
pub struct TorsionResult {
    pub j_m4: Option<f64>,
    pub estimate_m4: f64,
    pub state: TorsionState,
    pub method: String,
    pub relative_change: Option<f64>,
    pub refinements: u32,
    pub degrees_of_freedom: usize,
}
impl TorsionResult {
    pub(super) fn analytical(j: f64, method: &str) -> Self {
        Self {
            j_m4: Some(j),
            estimate_m4: j,
            state: TorsionState::Analytical,
            method: method.into(),
            relative_change: None,
            refinements: 0,
            degrees_of_freedom: 0,
        }
    }
}

pub(super) fn rectangle_torsion(width: f64, height: f64) -> f64 {
    let a = width.max(height);
    let b = width.min(height);
    let pi = std::f64::consts::PI;
    let sum = (0..200)
        .map(|i| {
            let n = (2 * i + 1) as f64;
            (n * pi * a / (2.0 * b)).tanh() / n.powi(5)
        })
        .sum::<f64>();
    a * b.powi(3) / 3.0 * (1.0 - 192.0 * b * sum / (pi.powi(5) * a))
}

#[derive(Clone)]
struct Mesh {
    nodes: Vec<[f64; 2]>,
    triangles: Vec<[usize; 3]>,
}

impl Mesh {
    fn initial(input: &SectionInput) -> Result<(Self, f64, f64), DomainError> {
        let properties = polygon_properties(input).map_err(|error| invalid(&error.to_string()))?;
        let (polygon, origin) = polygon(input).map_err(|error| invalid(&error.to_string()))?;
        let center = [
            properties.centroid_m[0] - origin[0],
            properties.centroid_m[1] - origin[1],
        ];
        let scale = polygon
            .exterior()
            .0
            .iter()
            .map(|point| (point.x - center[0]).abs().max((point.y - center[1]).abs()))
            .fold(0.0, f64::max);
        if !scale.is_finite() || scale <= 0.0 {
            return Err(invalid("Escala de malha de torção inválida."));
        }
        let mut mesh = Self {
            nodes: Vec::new(),
            triangles: Vec::new(),
        };
        let mut ids = BTreeMap::new();
        for triangle in polygon.earcut_triangles() {
            let mut corners = [0; 3];
            for (index, Coord { x, y }) in triangle.to_array().into_iter().enumerate() {
                let point = [(x - center[0]) / scale, (y - center[1]) / scale];
                let key = (
                    if point[0] == 0.0 {
                        0
                    } else {
                        point[0].to_bits()
                    },
                    if point[1] == 0.0 {
                        0
                    } else {
                        point[1].to_bits()
                    },
                );
                let id = *ids.entry(key).or_insert_with(|| {
                    mesh.nodes.push(point);
                    mesh.nodes.len() - 1
                });
                corners[index] = id;
            }
            if area(&mesh.nodes, corners) < 0.0 {
                corners.swap(1, 2);
            }
            if area(&mesh.nodes, corners) <= 0.0 {
                return Err(invalid("Triangulação degenerada na seção."));
            }
            mesh.triangles.push(corners);
        }
        let triangulated_area = mesh
            .triangles
            .iter()
            .map(|triangle| area(&mesh.nodes, *triangle))
            .sum::<f64>()
            * scale
            * scale;
        if mesh.triangles.is_empty()
            || ((triangulated_area - properties.area_m2) / properties.area_m2).abs() > 1e-9
        {
            return Err(invalid(
                "A triangulação não conserva a área da seção e de seus vazios.",
            ));
        }
        let polar = (properties.ixx_m4 + properties.iyy_m4) / scale.powi(4);
        // Resolve long, thin walls before global convergence refinement. Earcut is
        // a topological triangulation, so its unrefined diagonals can span a whole
        // wall. Longest-edge bisection plus legal Delaunay flips changes only the
        // mesh, preserving all polygon/hole boundaries and the physical operator.
        let perimeter = std::iter::once(polygon.exterior())
            .chain(polygon.interiors())
            .flat_map(|ring| ring.0.windows(2))
            .map(|edge| (edge[1].x - edge[0].x).hypot(edge[1].y - edge[0].y))
            .sum::<f64>();
        mesh.prepare(8.0 * properties.area_m2 / (perimeter * scale));
        Ok((mesh, scale, polar))
    }

    fn prepare(&mut self, target: f64) {
        for _ in 0..2048 {
            let longest = self
                .triangles
                .iter()
                .flat_map(|&[a, b, c]| [(a, b), (b, c), (c, a)])
                .map(|(a, b)| {
                    let length = (self.nodes[a][0] - self.nodes[b][0]).powi(2)
                        + (self.nodes[a][1] - self.nodes[b][1]).powi(2);
                    (length, a, b)
                })
                .max_by(|a, b| a.0.total_cmp(&b.0));
            let Some((length, a, b)) = longest else {
                break;
            };
            if length <= target * target || self.nodes.len() >= 2048 {
                break;
            }
            let midpoint = self.nodes.len();
            self.nodes.push([
                (self.nodes[a][0] + self.nodes[b][0]) * 0.5,
                (self.nodes[a][1] + self.nodes[b][1]) * 0.5,
            ]);
            let mut replacement = Vec::new();
            for &triangle in &self.triangles {
                if triangle.contains(&a) && triangle.contains(&b) {
                    let c = *triangle
                        .iter()
                        .find(|&&index| index != a && index != b)
                        .expect("third corner");
                    for mut next in [[a, midpoint, c], [midpoint, b, c]] {
                        if area(&self.nodes, next) < 0.0 {
                            next.swap(1, 2);
                        }
                        replacement.push(next);
                    }
                } else {
                    replacement.push(triangle);
                }
            }
            self.triangles = replacement;
            self.delaunay_flips();
        }
    }

    fn delaunay_flips(&mut self) {
        for _ in 0..8 {
            let mut edges: BTreeMap<(usize, usize), Vec<(usize, usize)>> = BTreeMap::new();
            for (id, &[a, b, c]) in self.triangles.iter().enumerate() {
                for (a, b, c) in [(a, b, c), (b, c, a), (c, a, b)] {
                    edges.entry((a.min(b), a.max(b))).or_default().push((id, c));
                }
            }
            let mut changed = false;
            for ((mut a, mut b), adjacent) in edges {
                let [(first, c), (second, d)] = adjacent.as_slice() else {
                    continue;
                };
                if !self.triangles[*first].contains(&a)
                    || !self.triangles[*first].contains(&b)
                    || !self.triangles[*first].contains(c)
                    || !self.triangles[*second].contains(&a)
                    || !self.triangles[*second].contains(&b)
                    || !self.triangles[*second].contains(d)
                {
                    continue;
                }
                let (c, d) = (*c, *d);
                if area(&self.nodes, [c, d, a]) * area(&self.nodes, [c, d, b]) >= 0.0 {
                    continue;
                }
                if area(&self.nodes, [a, b, c]) < 0.0 {
                    std::mem::swap(&mut a, &mut b);
                }
                let coord = |index: usize| robust::Coord {
                    x: self.nodes[index][0],
                    y: self.nodes[index][1],
                };
                if robust::incircle(coord(a), coord(b), coord(c), coord(d)) <= 0.0 {
                    continue;
                }
                let mut one = [c, d, a];
                let mut two = [d, c, b];
                if area(&self.nodes, one) < 0.0 {
                    one.swap(1, 2);
                }
                if area(&self.nodes, two) < 0.0 {
                    two.swap(1, 2);
                }
                self.triangles[*first] = one;
                self.triangles[*second] = two;
                changed = true;
            }
            if !changed {
                break;
            }
        }
    }

    fn refined(&self) -> Self {
        let mut nodes = self.nodes.clone();
        let mut midpoints = BTreeMap::new();
        let mut triangles = Vec::with_capacity(self.triangles.len() * 4);
        for &[a, b, c] in &self.triangles {
            let mut midpoint = |a: usize, b: usize| {
                *midpoints.entry((a.min(b), a.max(b))).or_insert_with(|| {
                    nodes.push([
                        (self.nodes[a][0] + self.nodes[b][0]) * 0.5,
                        (self.nodes[a][1] + self.nodes[b][1]) * 0.5,
                    ]);
                    nodes.len() - 1
                })
            };
            let ab = midpoint(a, b);
            let bc = midpoint(b, c);
            let ca = midpoint(c, a);
            triangles.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
        }
        Self { nodes, triangles }
    }

    fn warping_energy(&self) -> Result<(f64, usize), DomainError> {
        // Quadratic interpolation is exact for the bilinear warping terms of thin walls.
        // The three-point rule integrates K and f (degree <=2) exactly on each triangle.
        let mut nodes = self.nodes.clone();
        let mut midpoints = BTreeMap::new();
        let mut elements = Vec::with_capacity(self.triangles.len());
        for &[a, b, c] in &self.triangles {
            let mut midpoint = |a: usize, b: usize| {
                *midpoints.entry((a.min(b), a.max(b))).or_insert_with(|| {
                    nodes.push([
                        (self.nodes[a][0] + self.nodes[b][0]) * 0.5,
                        (self.nodes[a][1] + self.nodes[b][1]) * 0.5,
                    ]);
                    nodes.len() - 1
                })
            };
            let ab = midpoint(a, b);
            let bc = midpoint(b, c);
            let ca = midpoint(c, a);
            elements.push([a, b, c, ab, bc, ca]);
        }
        let n = nodes.len();
        if n > MAX_NODES {
            return Err(invalid(
                "Malha quadrática de torção excede o limite de nós.",
            ));
        }
        let mut matrix = vec![BTreeMap::<usize, f64>::new(); n];
        let mut load = vec![0.0; n];
        for &element in &elements {
            let triangle = [element[0], element[1], element[2]];
            let points = triangle.map(|i| nodes[i]);
            let a = area(&nodes, triangle);
            if a <= 0.0 {
                return Err(invalid("Elemento de torção com área não positiva."));
            }
            let linear = [
                [
                    (points[1][1] - points[2][1]) / (2.0 * a),
                    (points[2][0] - points[1][0]) / (2.0 * a),
                ],
                [
                    (points[2][1] - points[0][1]) / (2.0 * a),
                    (points[0][0] - points[2][0]) / (2.0 * a),
                ],
                [
                    (points[0][1] - points[1][1]) / (2.0 * a),
                    (points[1][0] - points[0][0]) / (2.0 * a),
                ],
            ];
            for barycentric in [
                [2.0 / 3.0, 1.0 / 6.0, 1.0 / 6.0],
                [1.0 / 6.0, 2.0 / 3.0, 1.0 / 6.0],
                [1.0 / 6.0, 1.0 / 6.0, 2.0 / 3.0],
            ] {
                let mut gradients = [[0.0; 2]; 6];
                for i in 0..3 {
                    for j in 0..2 {
                        gradients[i][j] = (4.0 * barycentric[i] - 1.0) * linear[i][j];
                    }
                }
                for (edge, (i, j)) in [(0, 1), (1, 2), (2, 0)].into_iter().enumerate() {
                    for axis in 0..2 {
                        gradients[3 + edge][axis] = 4.0
                            * (barycentric[i] * linear[j][axis] + barycentric[j] * linear[i][axis]);
                    }
                }
                let point = [
                    (0..3).map(|i| barycentric[i] * points[i][0]).sum::<f64>(),
                    (0..3).map(|i| barycentric[i] * points[i][1]).sum::<f64>(),
                ];
                let weight = a / 3.0;
                for (i, &row) in element.iter().enumerate() {
                    load[row] += weight * (point[1] * gradients[i][0] - point[0] * gradients[i][1]);
                    for (j, &column) in element.iter().enumerate() {
                        *matrix[row].entry(column).or_default() += weight
                            * (gradients[i][0] * gradients[j][0]
                                + gradients[i][1] * gradients[j][1]);
                    }
                }
            }
        }
        // One scalar gauge fixes only the arbitrary constant of warping; no physical
        // support, rotational stiffness or connection is added to the section.
        matrix[0].clear();
        matrix[0].insert(0, 1.0);
        load[0] = 0.0;
        for row in matrix.iter_mut().skip(1) {
            row.remove(&0);
        }
        let matrix: Vec<Vec<(usize, f64)>> = matrix
            .into_iter()
            .map(|row| row.into_iter().collect())
            .collect();
        let omega = pcg(&matrix, &load)?;
        let energy = load
            .iter()
            .zip(omega)
            .map(|(force, displacement)| force * displacement)
            .sum::<f64>();
        if !energy.is_finite() || energy < -1e-12 {
            return Err(invalid("Energia de empenamento não finita ou negativa."));
        }
        Ok((energy.max(0.0), n - 1))
    }
}

fn area(nodes: &[[f64; 2]], triangle: [usize; 3]) -> f64 {
    let [a, b, c] = triangle.map(|i| nodes[i]);
    ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])) / 2.0
}
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn multiply(matrix: &[Vec<(usize, f64)>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| row.iter().map(|(j, value)| value * vector[*j]).sum())
        .collect()
}

/// Incomplete LDLᵀ factorization, used solely as a solver preconditioner.
/// A diagonal shift may be used inside this auxiliary factorization if a dropped
/// fill entry would cause a nonpositive pivot. The physical K, load and residual
/// remain unchanged, and the converged solution is checked against that original K.
struct Preconditioner {
    lower: Vec<Vec<(usize, f64)>>,
    diagonal: Vec<f64>,
}
impl Preconditioner {
    fn new(matrix: &[Vec<(usize, f64)>]) -> Result<Self, DomainError> {
        let diagonal: Vec<_> = matrix
            .iter()
            .enumerate()
            .map(|(i, row)| row.iter().find(|(j, _)| *j == i).map_or(0.0, |(_, v)| *v))
            .collect();
        if diagonal.iter().any(|d| !d.is_finite() || *d <= 0.0) {
            return Err(invalid("Matriz de empenamento sem diagonal positiva."));
        }
        for shift in [0.0, 0.001, 0.01, 0.1, 1.0] {
            let mut lower: Vec<Vec<(usize, f64)>> = Vec::with_capacity(matrix.len());
            let mut pivots: Vec<f64> = Vec::with_capacity(matrix.len());
            let mut valid = true;
            for (i, row) in matrix.iter().enumerate() {
                let mut current: Vec<(usize, f64)> = Vec::new();
                for &(j, value) in row.iter().take_while(|(j, _)| *j < i) {
                    let mut inner = 0.0;
                    let mut a = 0;
                    let mut b = 0;
                    while a < current.len() && b < lower[j].len() {
                        let (ka, va) = current[a];
                        let (kb, vb) = lower[j][b];
                        match ka.cmp(&kb) {
                            std::cmp::Ordering::Less => a += 1,
                            std::cmp::Ordering::Greater => b += 1,
                            std::cmp::Ordering::Equal => {
                                inner += va * pivots[ka] * vb;
                                a += 1;
                                b += 1;
                            }
                        }
                    }
                    current.push((j, (value - inner) / pivots[j]));
                }
                let pivot = diagonal[i] * (1.0 + shift)
                    - current.iter().map(|(j, v)| v * v * pivots[*j]).sum::<f64>();
                if !pivot.is_finite() || pivot <= diagonal[i] * 1e-12 {
                    valid = false;
                    break;
                }
                lower.push(current);
                pivots.push(pivot);
            }
            if valid {
                return Ok(Self {
                    lower,
                    diagonal: pivots,
                });
            }
        }
        // Positive Jacobi is a safe, potentially slower fallback, never artificial
        // stiffness in the finite-element operator.
        Ok(Self {
            lower: vec![Vec::new(); matrix.len()],
            diagonal,
        })
    }
    fn apply(&self, residual: &[f64]) -> Vec<f64> {
        let mut result = residual.to_vec();
        for (i, row) in self.lower.iter().enumerate() {
            result[i] -= row.iter().map(|(j, v)| v * result[*j]).sum::<f64>();
        }
        for (value, diagonal) in result.iter_mut().zip(&self.diagonal) {
            *value /= diagonal;
        }
        for (i, row) in self.lower.iter().enumerate().rev() {
            let value = result[i];
            for &(j, factor) in row {
                result[j] -= factor * value;
            }
        }
        result
    }
}
fn pcg(matrix: &[Vec<(usize, f64)>], load: &[f64]) -> Result<Vec<f64>, DomainError> {
    let n = load.len();
    let mut solution = vec![0.0; n];
    let mut residual = load.to_vec();
    let scale = dot(load, load).sqrt();
    if scale <= 1e-15 {
        return Ok(solution);
    }
    let preconditioner = Preconditioner::new(matrix)?;
    let mut z = preconditioner.apply(&residual);
    let mut direction = z.clone();
    let mut rz = dot(&residual, &z);
    for _ in 0..(n * 4 + 100).min(12_000) {
        let product = multiply(matrix, &direction);
        let denominator = dot(&direction, &product);
        if !denominator.is_finite() || denominator <= 0.0 {
            return Err(invalid("Solução de empenamento perdeu positividade."));
        }
        let alpha = rz / denominator;
        for i in 0..n {
            solution[i] += alpha * direction[i];
            residual[i] -= alpha * product[i];
        }
        if dot(&residual, &residual).sqrt() <= RESIDUAL_RELATIVE * scale {
            let actual = multiply(matrix, &solution);
            let norm = actual
                .iter()
                .zip(load)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            if norm <= 4.0 * RESIDUAL_RELATIVE * scale && solution.iter().all(|x| x.is_finite()) {
                return Ok(solution);
            }
        }
        z = preconditioner.apply(&residual);
        let next = dot(&residual, &z);
        if !next.is_finite() || rz <= 0.0 {
            return Err(invalid("Resíduo de empenamento inválido."));
        }
        let beta = next / rz;
        for i in 0..n {
            direction[i] = z[i] + beta * direction[i];
        }
        rz = next;
    }
    Err(invalid(
        "Empenamento não convergiu no limite de iterações; nenhuma torção foi aprovada.",
    ))
}

pub(super) fn polygon_torsion(inputs: &[SectionInput]) -> Result<TorsionResult, DomainError> {
    let mut states = inputs
        .iter()
        .map(Mesh::initial)
        .collect::<Result<Vec<_>, _>>()?;
    let mut previous: Option<f64> = None;
    let mut relative = None;
    let mut estimate = 0.0;
    let mut refinements = 0;
    let mut stable_levels = 0;
    let mut degrees = 0;
    for level in 0..=MAX_REFINEMENTS {
        let mut total = 0.0;
        degrees = 0;
        for (mesh, scale, polar) in &states {
            let (energy, dofs) = mesh.warping_energy()?;
            let j = (polar - energy) * scale.powi(4);
            if !j.is_finite() || j <= 0.0 {
                return Err(invalid(
                    "Constante de torção não positiva; não se usa momento polar como substituto.",
                ));
            }
            total += j;
            degrees += dofs;
        }
        estimate = total;
        refinements = level;
        if let Some(old) = previous {
            let change: f64 = (old - total).abs() / total;
            relative = Some(change);
            if change <= CONVERGENCE_RELATIVE {
                stable_levels += 1;
            } else {
                stable_levels = 0;
            }
            if level >= 3 && stable_levels >= 2 {
                return Ok(TorsionResult{j_m4:Some(total),estimate_m4:total,state:TorsionState::Converged,method:"Saint-Venant por empenamento triangular quadrático; duas variações sucessivas <= 0,2%; resíduo relativo <= 4e-10; não é momento polar".into(),relative_change:relative,refinements:level,degrees_of_freedom:degrees});
            }
        }
        previous = Some(total);
        if level == MAX_REFINEMENTS
            || states
                .iter()
                .map(|(mesh, _, _)| mesh.triangles.len() * 4)
                .sum::<usize>()
                > MAX_TRIANGLES
        {
            break;
        }
        let next: Vec<_> = states
            .iter()
            .map(|(mesh, scale, polar)| (mesh.refined(), *scale, *polar))
            .collect();
        if next
            .iter()
            .map(|(mesh, _, _)| mesh.refined().nodes.len())
            .sum::<usize>()
            > MAX_NODES
        {
            break;
        }
        states = next;
    }
    Ok(TorsionResult{j_m4:None,estimate_m4:estimate,state:TorsionState::NotConverged,method:"Empenamento triangular atingiu limite de recursos/refinamento. Estimativa preservada; sem constante de torção utilizável.".into(),relative_change:relative,refinements,degrees_of_freedom:degrees})
}
