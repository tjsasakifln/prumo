//! Experimental structural-analysis kernels (PRU-021..025).
//! All outputs are technical diagnostics, never a released design calculation.
#![forbid(unsafe_code)]

pub mod constraints;
pub mod diaphragm;
pub mod frame;
pub mod loads;
pub mod membrane;
pub mod mesh;
pub mod sparse;
pub mod truss;

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum AnalysisError {
    #[error("Entrada inválida: {0}")]
    Invalid(String),
    #[error("Limite de recursos da análise ultrapassado: {0}")]
    ResourceLimit(String),
    #[error("Matriz singular, mecanismo ou perda de positividade no grau de liberdade {0}; não será adicionada rigidez artificial.")]
    Mechanism(usize),
    #[error("Sistema mal condicionado para o perfil experimental (razão de pivôs {0:e}).")]
    Conditioning(f64),
    #[error("Resíduo relativo {0:e} acima da tolerância declarada.")]
    Residual(f64),
    #[error("Análise cancelada antes de produzir resultado.")]
    Cancelled,
}

pub(crate) fn ensure(condition: bool, reason: &str) -> Result<(), AnalysisError> {
    if condition { Ok(()) } else { Err(AnalysisError::Invalid(reason.into())) }
}


use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum TrialRequest {
    Linear { system: sparse::System },
    Truss { model: truss::Truss },
    Frame2d { model: frame::Frame2D },
    MeshInspect { mesh: mesh::Mesh },
    Membrane { model: membrane::Membrane },
    MeshRefine { mesh: mesh::Mesh },
    MeshConvergence { points: Vec<mesh::ConvergencePoint>, relative_tolerance: f64 },
    Multipoint { system: sparse::System, constraints: Vec<constraints::Constraint> },
    RigidDiaphragm { diaphragm: diaphragm::Diaphragm },
    LoadLedger { input: loads::AppliedLoads },
    LoadEquilibrium { input: loads::AppliedLoads, reactions: Vec<loads::Reaction>, tolerance: f64 },
}

pub fn trial(request:&TrialRequest,cancelled:Option<&AtomicBool>)->Result<Value,AnalysisError>{
    let serializable: Value = match request {
        TrialRequest::Linear{system} =>
            serde_json::to_value(sparse::solve(system,cancelled)?),
        TrialRequest::Truss{model} =>
            serde_json::to_value(truss::solve(model)?),
        TrialRequest::Frame2d{model} =>
            serde_json::to_value(frame::solve(model,cancelled)?),
        TrialRequest::MeshInspect{mesh} =>
            serde_json::to_value(mesh::inspect(mesh)?),
        TrialRequest::Membrane{model} =>
            serde_json::to_value(membrane::solve(model,cancelled)?),
        TrialRequest::MeshRefine{mesh} =>
            serde_json::to_value(mesh::refine(mesh)?),
        TrialRequest::MeshConvergence{points,relative_tolerance} =>
            serde_json::to_value(mesh::convergence(points,*relative_tolerance)?),
        TrialRequest::RigidDiaphragm{diaphragm} =>
            serde_json::to_value(diaphragm::propose(diaphragm)?),
        TrialRequest::Multipoint{system,constraints} => {
            let condensed=constraints::reduce(system,constraints)?;
            let solved=sparse::solve(&condensed.condensed,cancelled)?;
            let recovered=constraints::recover(system,&condensed,&solved)?;
            Ok(json!({"condensed":condensed,"solution":solved,"recovery":recovered}))
        }
        TrialRequest::LoadLedger{input} =>
            serde_json::to_value(loads::ledger(input)?),
        TrialRequest::LoadEquilibrium{input,reactions,tolerance} => {
            let ledger=loads::ledger(input)?;
            let balance=loads::check_equilibrium(&ledger,reactions,*tolerance)?;
            Ok(json!({"ledger":ledger,"equilibrium":balance}))
        }
    }.map_err(|e|AnalysisError::Invalid(format!("Serialização da análise: {e}")))?;
    Ok(json!({"result":serializable,"engineering_release":false,
        "scope":"ensaio_experimental_nao_calculo_do_projeto"}))
}
