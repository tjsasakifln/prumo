//! PRU-022: axial bar/truss trial in global 3D coordinates.
//! Euler-Bernoulli, Timoshenko, torsion, releases, offsets, cables and
//! construction stages are NOT implemented by this profile.
use crate::{AnalysisError, ensure, sparse};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: Uuid,
    pub position_m: [f64; 3],
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AxialBar {
    pub id: Uuid,
    pub start_id: Uuid,
    pub end_id: Uuid,
    pub area_m2: f64,
    pub elastic_modulus_pa: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodalForce {
    pub node_id: Uuid,
    pub force_n: [f64; 3],
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Support {
    pub node_id: Uuid,
    pub fixed: [bool; 3],
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Truss {
    pub nodes: Vec<Node>,
    pub bars: Vec<AxialBar>,
    pub forces: Vec<NodalForce>,
    pub supports: Vec<Support>,
}
#[derive(Clone, Debug, Serialize)]
pub struct AxialResult {
    pub bar_id: Uuid,
    /// Positive is tension, negative compression.
    pub axial_force_n: f64,
    pub extension_m: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct Result {
    pub analysis: sparse::Solution,
    pub axial: Vec<AxialResult>,
    pub nodes: Vec<Uuid>,
    pub force_balance_n: [f64; 3],
    pub engineering_release: bool,
}
/// Purposefully refuses unstable transverse modes. No stabilization is added.
pub fn solve(truss: &Truss) -> std::result::Result<Result, AnalysisError> {
    ensure(
        !truss.nodes.is_empty()
            && !truss.bars.is_empty()
            && truss.nodes.len() <= sparse::MAX_DOF / 3
            && truss.bars.len() <= 1024,
        "Treliça vazia ou acima do perfil de ensaio.",
    )?;
    let mut ids = BTreeMap::new();
    for (index, node) in truss.nodes.iter().enumerate() {
        ensure(
            !node.id.is_nil() && ids.insert(node.id, index).is_none(),
            "Nó repetido ou nulo.",
        )?;
        ensure(
            node.position_m
                .iter()
                .all(|x| x.is_finite() && x.abs() <= 1e8),
            "Coordenada fora dos limites.",
        )?;
    }
    let mut terms = Vec::new();
    let mut unique_bars = BTreeSet::new();
    for bar in &truss.bars {
        ensure(
            !bar.id.is_nil() && unique_bars.insert(bar.id),
            "Identidade da barra duplicada.",
        )?;
        ensure(
            bar.start_id != bar.end_id
                && bar.area_m2.is_finite()
                && bar.elastic_modulus_pa.is_finite()
                && bar.area_m2 > 0.0
                && bar.elastic_modulus_pa > 0.0
                && bar.area_m2 <= 1e5
                && bar.elastic_modulus_pa <= 1e15,
            "Área, material ou extremidades inválidos.",
        )?;
        let a = *ids
            .get(&bar.start_id)
            .ok_or_else(|| AnalysisError::Invalid("Nó inicial ausente.".into()))?;
        let b = *ids
            .get(&bar.end_id)
            .ok_or_else(|| AnalysisError::Invalid("Nó final ausente.".into()))?;
        let start = truss.nodes[a].position_m;
        let end = truss.nodes[b].position_m;
        let delta: [f64; 3] = std::array::from_fn(|i| end[i] - start[i]);
        let length = delta.iter().map(|x| x * x).sum::<f64>().sqrt();
        ensure(length.is_finite() && length >= 1e-8, "Barra degenerada.")?;
        let direction = delta.map(|v| v / length);
        let factor = bar.elastic_modulus_pa * bar.area_m2 / length;
        ensure(
            factor.is_finite() && factor <= 1e20,
            "Rigidez da barra excessiva.",
        )?;
        for i in 0..6 {
            for j in 0..6 {
                let ni = direction[i % 3] * if i < 3 { -1.0 } else { 1.0 };
                let nj = direction[j % 3] * if j < 3 { -1.0 } else { 1.0 };
                terms.push(sparse::Term {
                    row: if i < 3 { 3 * a + i } else { 3 * b + i - 3 },
                    column: if j < 3 { 3 * a + j } else { 3 * b + j - 3 },
                    stiffness_n_per_m: factor * ni * nj,
                });
            }
        }
    }
    let mut forces_n = vec![0.0; truss.nodes.len() * 3];
    for force in &truss.forces {
        let i = *ids
            .get(&force.node_id)
            .ok_or_else(|| AnalysisError::Invalid("Carga em nó inexistente.".into()))?;
        ensure(
            force.force_n.iter().all(|v| v.is_finite()),
            "Força não finita.",
        )?;
        for d in 0..3 {
            forces_n[3 * i + d] += force.force_n[d];
        }
    }
    let mut prescribed = Vec::new();
    let mut fixed = BTreeSet::new();
    for support in &truss.supports {
        let i = *ids
            .get(&support.node_id)
            .ok_or_else(|| AnalysisError::Invalid("Apoio em nó inexistente.".into()))?;
        for d in 0..3 {
            if support.fixed[d] {
                let dof = 3 * i + d;
                ensure(
                    fixed.insert(dof),
                    "Apoio duplicado no mesmo grau de liberdade.",
                )?;
                prescribed.push(sparse::Prescribed {
                    dof,
                    displacement_m: 0.0,
                });
            }
        }
    }
    let analysis = sparse::solve(
        &sparse::System {
            dimension: truss.nodes.len() * 3,
            terms,
            forces_n: forces_n.clone(),
            prescribed,
        },
        None,
    )?;
    let mut axial = Vec::new();
    for bar in &truss.bars {
        let a = ids[&bar.start_id];
        let b = ids[&bar.end_id];
        let delta: [f64; 3] =
            std::array::from_fn(|j| truss.nodes[b].position_m[j] - truss.nodes[a].position_m[j]);
        let length = delta.iter().map(|v| v * v).sum::<f64>().sqrt();
        let extension_m = (0..3)
            .map(|j| {
                delta[j] / length
                    * (analysis.displacements_m[3 * b + j] - analysis.displacements_m[3 * a + j])
            })
            .sum::<f64>();
        axial.push(AxialResult {
            bar_id: bar.id,
            extension_m,
            axial_force_n: bar.elastic_modulus_pa * bar.area_m2 / length * extension_m,
        });
    }
    let balance = std::array::from_fn(|axis| {
        (0..truss.nodes.len())
            .map(|index| forces_n[3 * index + axis] + analysis.reactions_n[3 * index + axis])
            .sum::<f64>()
    });
    Ok(Result {
        analysis,
        axial,
        nodes: truss.nodes.iter().map(|node| node.id).collect(),
        force_balance_n: balance,
        engineering_release: false,
    })
}
