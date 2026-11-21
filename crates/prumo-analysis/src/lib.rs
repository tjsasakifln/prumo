//! Experimental structural-analysis kernels (PRU-021..025).
//! All outputs are technical diagnostics, never a released design calculation.
#![forbid(unsafe_code)]

pub mod constraints;
pub mod loads;
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
