//! PRU-023: constant-strain triangle (CST) plane-stress membrane.
//! This is not a bending plate, shell, drilling-rotation or punching solver.
use crate::{AnalysisError, ensure, mesh, sparse};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MembraneMaterial {
    pub elastic_modulus_pa: f64,
    pub poisson_ratio: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MembraneForce {
    pub vertex: usize,
    pub fx_n: f64,
    pub fy_n: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MembraneSupport {
    pub vertex: usize,
    /// 0 for X, 1 for Y
    pub axis: usize,
    pub displacement_m: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Membrane {
    pub mesh: mesh::Mesh,
    pub material: MembraneMaterial,
    pub forces: Vec<MembraneForce>,
    pub supports: Vec<MembraneSupport>,
}
#[derive(Clone, Debug, Serialize)]
pub struct TriangleStress {
    pub triangle: usize,
    pub strain: [f64; 3],
    /// [sigma_xx, sigma_yy, tau_xy], Pa, engineering shear gamma_xy.
    pub stress_pa: [f64; 3],
    pub energy_j: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct MembraneResult {
    pub solution: sparse::Solution,
    pub elements: Vec<TriangleStress>,
    pub total_strain_energy_j: f64,
    pub engineering_release: bool,
}
struct Element {
    dofs: [usize; 6],
    b: [[f64; 6]; 3],
    d: [[f64; 3]; 3],
    area: f64,
}
fn element(
    nodes: &[[f64; 3]],
    triangle: [usize; 3],
    thickness: f64,
    material: &MembraneMaterial,
) -> Result<(Element, [[f64; 6]; 6]), AnalysisError> {
    let [i, j, k] = triangle;
    let a = nodes[i];
    let b = nodes[j];
    let c = nodes[k];
    ensure(
        (a[2] - b[2]).abs() < 1e-8 && (a[2] - c[2]).abs() < 1e-8,
        "CST requer superfície no plano XY sem distorção fora do plano.",
    )?;
    let signed = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
    ensure(
        signed > 1e-12,
        "Triângulo invertido ou degenerado na projeção XY.",
    )?;
    let area = signed / 2.0;
    let bx = [b[1] - c[1], c[1] - a[1], a[1] - b[1]];
    let cx = [c[0] - b[0], a[0] - c[0], b[0] - a[0]];
    let mut strain = [[0.0; 6]; 3];
    for p in 0..3 {
        strain[0][2 * p] = bx[p] / signed;
        strain[1][2 * p + 1] = cx[p] / signed;
        strain[2][2 * p] = cx[p] / signed;
        strain[2][2 * p + 1] = bx[p] / signed;
    }
    let e = material.elastic_modulus_pa;
    let nu = material.poisson_ratio;
    let factor = e / (1.0 - nu * nu);
    let d = [
        [factor, nu * factor, 0.0],
        [nu * factor, factor, 0.0],
        [0.0, 0.0, (1.0 - nu) * factor / 2.0],
    ];
    let mut k6 = [[0.0; 6]; 6];
    for m in 0..6 {
        for n in 0..6 {
            k6[m][n] = thickness
                * area
                * (0..3)
                    .map(|r| {
                        (0..3)
                            .map(|s| strain[r][m] * d[r][s] * strain[s][n])
                            .sum::<f64>()
                    })
                    .sum::<f64>();
        }
    }
    ensure(
        k6.iter()
            .flatten()
            .all(|x| x.is_finite() && x.abs() <= 1e20),
        "Matriz CST não finita ou fora do domínio.",
    )?;
    Ok((
        Element {
            dofs: [2 * i, 2 * i + 1, 2 * j, 2 * j + 1, 2 * k, 2 * k + 1],
            b: strain,
            d,
            area,
        },
        k6,
    ))
}
pub fn solve(
    model: &Membrane,
    cancelled: Option<&AtomicBool>,
) -> Result<MembraneResult, AnalysisError> {
    let inspected = mesh::inspect(&model.mesh)?;
    ensure(
        model.mesh.points_m.len() * 2 <= sparse::MAX_DOF,
        "Malha de membrana excede 256 graus de liberdade.",
    )?;
    ensure(
        model.material.elastic_modulus_pa.is_finite()
            && model.material.elastic_modulus_pa > 0.0
            && model.material.elastic_modulus_pa <= 1e15
            && model.material.poisson_ratio.is_finite()
            && model.material.poisson_ratio > -0.99
            && model.material.poisson_ratio < 0.499,
        "Material plano isotrópico fora do domínio.",
    )?;
    let mut stiffness = Vec::new();
    let mut matrices = Vec::new();
    for triangle in &model.mesh.triangles {
        let (m, k) = element(
            &model.mesh.points_m,
            *triangle,
            model.mesh.thickness_m,
            &model.material,
        )?;
        for i in 0..6 {
            for j in 0..6 {
                stiffness.push(sparse::Term {
                    row: m.dofs[i],
                    column: m.dofs[j],
                    stiffness_n_per_m: k[i][j],
                });
            }
        }
        matrices.push(m);
    }
    ensure(
        inspected.elements.len() == matrices.len(),
        "Malha inválida.",
    )?;
    let mut forces = vec![0.0; model.mesh.points_m.len() * 2];
    for input in &model.forces {
        ensure(
            input.vertex < model.mesh.points_m.len()
                && input.fx_n.is_finite()
                && input.fy_n.is_finite(),
            "Carga fora do intervalo ou não finita.",
        )?;
        forces[2 * input.vertex] += input.fx_n;
        forces[2 * input.vertex + 1] += input.fy_n;
    }
    let mut fixed = BTreeSet::new();
    let mut prescribed = Vec::new();
    for p in &model.supports {
        ensure(
            p.vertex < model.mesh.points_m.len() && p.axis < 2 && p.displacement_m.is_finite(),
            "Apoio CST inválido.",
        )?;
        let dof = 2 * p.vertex + p.axis;
        ensure(
            fixed.insert(dof),
            "Deslocamento de membrana prescrito mais de uma vez.",
        )?;
        prescribed.push(sparse::Prescribed {
            dof,
            displacement_m: p.displacement_m,
        });
    }
    let solution = sparse::solve(
        &sparse::System {
            dimension: forces.len(),
            terms: stiffness,
            forces_n: forces,
            prescribed,
        },
        cancelled,
    )?;
    let mut elements = Vec::new();
    for (index, m) in matrices.iter().enumerate() {
        let displacement: [f64; 6] = std::array::from_fn(|i| solution.displacements_m[m.dofs[i]]);
        let strain =
            std::array::from_fn(|i| (0..6).map(|j| m.b[i][j] * displacement[j]).sum::<f64>());
        let stress: [f64; 3] =
            std::array::from_fn(|i| (0..3).map(|j| m.d[i][j] * strain[j]).sum::<f64>());
        let energy = model.mesh.thickness_m * m.area / 2.0
            * (0..3).map(|i| strain[i] * stress[i]).sum::<f64>();
        ensure(
            stress.iter().all(|v| v.is_finite()) && energy.is_finite() && energy >= -1e-6,
            "Recuperação de tensão ou energia CST inválida.",
        )?;
        elements.push(TriangleStress {
            triangle: index,
            strain,
            stress_pa: stress,
            energy_j: energy,
        });
    }
    let total = elements.iter().map(|e| e.energy_j).sum();
    Ok(MembraneResult {
        solution,
        elements,
        total_strain_energy_j: total,
        engineering_release: false,
    })
}
