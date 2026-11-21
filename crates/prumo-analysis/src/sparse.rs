//! Bounded sparse SPD structural algebra. This is a diagnostic kernel, not an
//! approved project solver. No artificial support or pivot regularization.
use crate::{AnalysisError, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAX_DOF: usize = 256;
pub const MAX_TERMS: usize = 65_536;
pub const RESIDUAL_LIMIT: f64 = 1e-10;
pub const PIVOT_RATIO_LIMIT: f64 = 1e10;
pub const METHOD: &str = "prumo-sparse-ldlt-rust-0.1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Term {
    pub row: usize,
    pub column: usize,
    pub stiffness_n_per_m: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Prescribed {
    pub dof: usize,
    pub displacement_m: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct System {
    pub dimension: usize,
    pub terms: Vec<Term>,
    pub forces_n: Vec<f64>,
    pub prescribed: Vec<Prescribed>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Solution {
    pub displacements_m: Vec<f64>,
    /// Global K * u - F. Values on prescribed DOFs are reactions.
    pub reactions_n: Vec<f64>,
    pub free_relative_residual: f64,
    /// Max/min LDL pivots. Not a spectral or infinity-norm condition number.
    pub pivot_ratio_indicator: f64,
    pub method: &'static str,
    pub residual_limit: f64,
    pub engineering_release: bool,
}

fn checked_assembly(system: &System) -> Result<BTreeMap<(usize, usize), f64>, AnalysisError> {
    ensure((1..=MAX_DOF).contains(&system.dimension), "Quantidade de graus de liberdade inválida.")?;
    ensure(system.forces_n.len() == system.dimension, "Vetor de forças não coincide com a matriz.")?;
    if system.terms.len() > MAX_TERMS {
        return Err(AnalysisError::ResourceLimit("mais de 65536 contribuições".into()));
    }
    ensure(system.forces_n.iter().all(|v| v.is_finite()), "Força não finita.")?;
    let mut map = BTreeMap::new();
    for term in &system.terms {
        ensure(term.row < system.dimension && term.column < system.dimension,
            "Índice de rigidez fora da dimensão informada.")?;
        ensure(term.stiffness_n_per_m.is_finite() && term.stiffness_n_per_m.abs() <= 1e20,
            "Rigidez ausente, não finita ou acima do limite.")?;
        let entry = map.entry((term.row, term.column)).or_insert(0.0_f64);
        *entry += term.stiffness_n_per_m;
        ensure(entry.is_finite(), "Soma de coeficientes excede a representação numérica.")?;
    }
    let norm = (0..system.dimension)
        .map(|i| map.range((i, 0)..=(i, system.dimension - 1))
            .map(|(_, v)| v.abs()).sum::<f64>())
        .fold(0.0, f64::max);
    ensure(norm.is_finite() && norm > 0.0, "Matriz nula ou inválida.")?;
    for (&(i, j), &a) in &map {
        let b = map.get(&(j, i)).copied().unwrap_or(0.0);
        ensure((a - b).abs() <= norm * 1e-12,
            "Rigidez assimétrica: confira os termos (i,j) e (j,i).")?;
    }
    Ok(map)
}

fn factor_solve(
    stiffness: &BTreeMap<(usize, usize), f64>,
    rhs: &[f64],
    cancelled: Option<&AtomicBool>,
) -> Result<(Vec<f64>, f64), AnalysisError> {
    let n = rhs.len();
    if n == 0 { return Ok((Vec::new(), 1.0)); }
    let mut rows: Vec<BTreeMap<usize, f64>> = vec![BTreeMap::new(); n];
    for (&(i, j), &value) in stiffness {
        if i >= j && value != 0.0 {
            rows[i].insert(j, value);
        }
    }
    let mut max_pivot: f64 = 0.0;
    let mut min_pivot: f64 = f64::INFINITY;
    let mut updates: usize = 0;
    for pivot in 0..n {
        if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(AnalysisError::Cancelled);
        }
        let diag = rows[pivot].get(&pivot).copied().unwrap_or(0.0);
        if !diag.is_finite() || diag <= 0.0 {
            return Err(AnalysisError::Mechanism(pivot));
        }
        max_pivot = max_pivot.max(diag);
        min_pivot = min_pivot.min(diag);
        if max_pivot / min_pivot > PIVOT_RATIO_LIMIT {
            return Err(AnalysisError::Conditioning(max_pivot / min_pivot));
        }
        let column: Vec<(usize, f64)> = ((pivot + 1)..n)
            .filter_map(|i| rows[i].get(&pivot).copied().filter(|v| *v != 0.0).map(|v| (i, v)))
            .collect();
        for &(i, value) in &column {
            rows[i].insert(pivot, value / diag);
        }
        for (at, &(i, a_ik)) in column.iter().enumerate() {
            for &(j, a_jk) in &column[at..] {
                updates += 1;
                if updates > MAX_TERMS * 4 {
                    return Err(AnalysisError::ResourceLimit("preenchimento de fatoração excessivo".into()));
                }
                let previous = rows[j].get(&i).copied().unwrap_or(0.0);
                let replacement = previous - a_ik * a_jk / diag;
                if !replacement.is_finite() {
                    return Err(AnalysisError::Conditioning(f64::INFINITY));
                }
                if replacement == 0.0 {
                    rows[j].remove(&i);
                } else {
                    rows[j].insert(i, replacement);
                }
            }
        }
    }
    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut value = rhs[i];
        for (&j, &lij) in rows[i].range(..i) {
            value -= lij * y[j];
        }
        y[i] = value;
    }
    let mut result = vec![0.0; n];
    for i in (0..n).rev() {
        let mut value = y[i] / rows[i][&i];
        for j in (i + 1)..n {
            if let Some(&lji) = rows[j].get(&i) {
                value -= lji * result[j];
            }
        }
        result[i] = value;
    }
    ensure(result.iter().all(|x| x.is_finite() && x.abs() <= 1e9),
        "Deslocamento não finito ou fora do perfil de ensaio.")?;
    Ok((result, max_pivot / min_pivot))
}

/// Solves a trial system in explicit translational SI units. Prescribed DOFs
/// are eliminated exactly; their reactions use the original full matrix.
pub fn solve(system: &System, cancelled: Option<&AtomicBool>) -> Result<Solution, AnalysisError> {
    let k = checked_assembly(system)?;
    let mut fixed = BTreeMap::new();
    for condition in &system.prescribed {
        ensure(condition.dof < system.dimension && condition.displacement_m.is_finite(),
            "Restrição com índice ou deslocamento inválido.")?;
        ensure(fixed.insert(condition.dof, condition.displacement_m).is_none(),
            "Mesmo grau de liberdade prescrito duas vezes.")?;
    }
    let free: Vec<_> = (0..system.dimension).filter(|i| !fixed.contains_key(i)).collect();
    let mut full = vec![0.0; system.dimension];
    for (&i, &u) in &fixed { full[i] = u; }
    let free_positions: BTreeMap<usize, usize> =
        free.iter().enumerate().map(|(j, &i)| (i, j)).collect();
    let mut right = free.iter().map(|&i| system.forces_n[i]).collect::<Vec<_>>();
    let mut reduced: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    for (&(i, j), &value) in &k {
        if let Some(&row) = free_positions.get(&i) {
            if let Some(&col) = free_positions.get(&j) {
                reduced.insert((row, col), value);
            } else {
                right[row] -= value * full[j];
            }
        }
    }
    let (unknowns, pivot_ratio_indicator) = factor_solve(&reduced, &right, cancelled)?;
    for (index, &i) in free.iter().enumerate() {
        full[i] = unknowns[index];
    }
    let mut reactions = system.forces_n.iter().map(|f| -f).collect::<Vec<_>>();
    for (&(i, j), &value) in &k { reactions[i] += value * full[j]; }
    let free_norm = free.iter().map(|&i| reactions[i].abs()).fold(0.0, f64::max);
    let max_load = system.forces_n.iter().map(|f| f.abs()).fold(0.0, f64::max);
    let max_u = full.iter().map(|u| u.abs()).fold(0.0, f64::max);
    let norm_k = (0..system.dimension)
        .map(|i| k.range((i, 0)..=(i, system.dimension - 1))
            .map(|(_, v)| v.abs()).sum::<f64>())
        .fold(0.0, f64::max);
    let denominator = norm_k * max_u + max_load;
    let relative = if denominator == 0.0 { free_norm } else { free_norm / denominator };
    if !relative.is_finite() || relative > RESIDUAL_LIMIT {
        return Err(AnalysisError::Residual(relative));
    }
    Ok(Solution {
        displacements_m: full, reactions_n: reactions,
        free_relative_residual: relative, pivot_ratio_indicator,
        method: METHOD, residual_limit: RESIDUAL_LIMIT, engineering_release: false,
    })
}
