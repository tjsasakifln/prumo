//! PRU-024: explicit one-level multipoint constraints and exact linear
//! coordinate elimination. These are algebraic mechanisms, not a diaphragm
//! qualification or automatic structural connectivity.
use crate::{AnalysisError, ensure, sparse};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_MPC: usize = 128;
pub const MAX_MASTERS: usize = 16;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Master {
    pub dof: usize,
    pub coefficient: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Constraint {
    pub slave_dof: usize,
    pub masters: Vec<Master>,
    pub offset_m: f64,
    pub justification: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Reduction {
    pub condensed: sparse::System,
    pub master_dofs: Vec<usize>,
    pub slave_dofs: Vec<usize>,
    pub original_dimension: usize,
    pub transformation: Vec<Vec<(usize, f64)>>,
    pub offsets_m: Vec<f64>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Recovery {
    pub full_displacements_m: Vec<f64>,
    /// Generalized equilibrium residual in full original DOFs (including MPC forces).
    pub full_residual_n: Vec<f64>,
    /// Work of original stiffness on recovered displacements.
    pub strain_energy_j: f64,
    pub engineering_release: bool,
}

pub fn reduce(system:&sparse::System, constraints:&[Constraint])
    -> Result<Reduction,AnalysisError>
{
    ensure((1..=sparse::MAX_DOF).contains(&system.dimension) &&
        system.forces_n.len()==system.dimension,
        "Sistema original inválido.")?;
    if constraints.len()>MAX_MPC {
        return Err(AnalysisError::ResourceLimit("excesso de restrições multiponto".into()));
    }
    let mut by_slave=BTreeMap::new();
    for constraint in constraints {
        ensure(constraint.slave_dof<system.dimension && constraint.offset_m.is_finite()
            && !constraint.justification.trim().is_empty()
            && (1..=MAX_MASTERS).contains(&constraint.masters.len()),
            "Vínculo multiponto com escravo, deslocamento ou justificativa inválidos.")?;
        ensure(by_slave.insert(constraint.slave_dof,constraint).is_none(),
            "Dependência duplicada do mesmo grau de liberdade.")?;
    }
    let slaves: BTreeSet<_>=by_slave.keys().copied().collect();
    for relation in constraints {
        let mut seen=BTreeSet::new();
        for master in &relation.masters {
            ensure(master.dof<system.dimension && master.dof!=relation.slave_dof
                && master.coefficient.is_finite() && master.coefficient!=0.0
                && seen.insert(master.dof) && !slaves.contains(&master.dof),
                "Dependência redundante, cíclica, não finita ou com mestre repetido.")?;
        }
    }
    let independent: Vec<usize>=(0..system.dimension)
        .filter(|d|!slaves.contains(d)).collect();
    ensure(!independent.is_empty(),"Sem graus de liberdade independentes.")?;
    let indexes: BTreeMap<usize,usize>=independent.iter().enumerate().map(|(i,&d)|(d,i)).collect();
    let mut transformation=vec![Vec::new();system.dimension];
    let mut offsets=vec![0.0;system.dimension];
    for dof in 0..system.dimension {
        if let Some(relation)=by_slave.get(&dof) {
            offsets[dof]=relation.offset_m;
            transformation[dof]=relation.masters.iter().map(|m|
                (indexes[&m.dof],m.coefficient)).collect();
        } else {
            transformation[dof]=vec![(indexes[&dof],1.0)];
        }
    }
    // Reject contradictory prescribed displacements on dependent DOFs.
    let mut fixed=BTreeSet::new();
    let mut reduced_fixed=Vec::new();
    for p in &system.prescribed {
        ensure(p.dof<system.dimension && p.displacement_m.is_finite()
            && fixed.insert(p.dof), "Apoios prescritos inválidos ou duplicados.")?;
        ensure(!slaves.contains(&p.dof),
            "Um grau escravo não pode receber deslocamento prescrito independente.")?;
        reduced_fixed.push(sparse::Prescribed {
            dof:indexes[&p.dof], displacement_m:p.displacement_m,
        });
    }
    ensure(system.terms.len()<=sparse::MAX_TERMS, "Excesso de termos na matriz de origem.")?;
    let mut reduced: BTreeMap<(usize,usize),f64>=BTreeMap::new();
    let mut reduced_forces=vec![0.0;independent.len()];
    for (original, &force) in system.forces_n.iter().enumerate() {
        ensure(force.is_finite(), "Força aplicada não finita.")?;
        for &(column, weight) in &transformation[original] {
            reduced_forces[column]+=weight*force;
        }
    }
    // K' = T^T K T, F' = T^T(F-Ku0).
    for term in &system.terms {
        ensure(term.row<system.dimension && term.column<system.dimension
            && term.stiffness_n_per_m.is_finite(),
            "Rigidez original inválida.")?;
        let contribution=term.stiffness_n_per_m;
        for &(i,wi) in &transformation[term.row] {
            reduced_forces[i]-=wi*contribution*offsets[term.column];
            for &(j,wj) in &transformation[term.column] {
                *reduced.entry((i,j)).or_default() += contribution*wi*wj;
            }
        }
    }
    ensure(reduced_forces.iter().all(|x|x.is_finite())
        && reduced.values().all(|x|x.is_finite()),
        "Transformação resultou em números não finitos.")?;
    if reduced.len()>sparse::MAX_TERMS {
        return Err(AnalysisError::ResourceLimit("transformação com rigidez excessivamente densa".into()));
    }
    Ok(Reduction {
        condensed:sparse::System {
            dimension:independent.len(),
            terms:reduced.into_iter().map(|((row,column),stiffness_n_per_m)|
                sparse::Term{row,column,stiffness_n_per_m}).collect(),
            forces_n:reduced_forces,prescribed:reduced_fixed,
        },
        master_dofs:independent,
        slave_dofs:slaves.into_iter().collect(),
        original_dimension:system.dimension,
        transformation,
        offsets_m:offsets,
        warnings:vec!["A matriz reduzida pressupõe vínculos lineares exatos. Não prova a validade de um diafragma rígido.".into()],
    })
}
pub fn recover(system:&sparse::System,reduction:&Reduction,
    solution:&sparse::Solution)->Result<Recovery,AnalysisError>
{
    ensure(reduction.original_dimension==system.dimension &&
        solution.displacements_m.len()==reduction.master_dofs.len() &&
        reduction.transformation.len()==system.dimension,
        "Dimensões incompatíveis para recuperação.")?;
    let full:Vec<f64>=reduction.transformation.iter().enumerate()
        .map(|(i,entries)|reduction.offsets_m[i]+
            entries.iter().map(|&(j,c)|c*solution.displacements_m[j]).sum::<f64>())
        .collect();
    let mut residual=system.forces_n.iter().map(|v|-v).collect::<Vec<_>>();
    let mut twice_energy=0.0;
    for term in &system.terms {
        let value=term.stiffness_n_per_m*full[term.column];
        residual[term.row]+=value;
        twice_energy+=full[term.row]*value;
    }
    ensure(full.iter().all(|x|x.is_finite())
        && residual.iter().all(|x|x.is_finite())
        && twice_energy.is_finite() && twice_energy>=-1e-9,
        "Recuperação não finita ou energia negativa.")?;
    Ok(Recovery {
        full_displacements_m:full,
        full_residual_n:residual,
        strain_energy_j:twice_energy*0.5,
        engineering_release:false,
    })
}
