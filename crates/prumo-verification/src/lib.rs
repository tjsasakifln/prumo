//! Preregistered verification protocols and reproducible evidence.
//! The shipped mechanical cases are bounded fixtures, never project calculations.
#![forbid(unsafe_code)]

mod protocol;
mod provenance;
mod references;
mod runner;

pub use protocol::*;
pub use provenance::*;
pub use references::{reference_protocols, run_reference_suite};
pub use runner::*;

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("Protocolo de verificação inválido: {0}")]
    InvalidProtocol(String),
    #[error("A revisão não corresponde ao protocolo pré-registrado: {0}")]
    InvalidReview(String),
    #[error("Manifesto de execução incompleto: {0}")]
    InvalidProvenance(String),
    #[error("Bateria de verificação inválida: {0}")]
    InvalidSuite(String),
    #[error("Não foi possível registrar a evidência: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Não foi possível identificar o artefato executado: {0}")]
    Io(#[from] std::io::Error),
}

pub(crate) fn hash<T: Serialize>(value: &T) -> Result<String, VerificationError> {
    // Struct field order is fixed by the schema; all keyed collections are BTreeMaps/Sets.
    Ok(prumo_domain::canonical::hash_bytes(&serde_json::to_vec(
        value,
    )?))
}

pub(crate) fn text_ok(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 16_384 && !value.contains('\0')
}

pub(crate) fn sha_ok(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
