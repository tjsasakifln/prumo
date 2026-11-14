#![forbid(unsafe_code)]

//! Native transactional persistence and bounded, offline `.prumo` interchange.
//!
//! The SQLite database is authoritative. JSON entries in the container are
//! auditable projections, checked against that database before a project is used.

use prumo_domain::{DomainError, Project};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

mod package;
mod store;

pub use package::{decode_package, encode_package, encode_validated_package};
pub use store::{SQLiteWorkingStore, restore_database};

pub const CONTAINER_VERSION: u32 = 1;
pub const DATABASE_VERSION: u32 = 1;
pub const MAX_PACKAGE_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_DATABASE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_EXPANDED_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 4096;
pub const MAX_COMPRESSION_RATIO: u64 = 200;
pub const MAX_MANIFEST_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_COMMAND_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_REVISIONS: usize = 2048;
pub const MAX_VALIDATION_BYTES: u64 = 512 * 1024 * 1024;

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Falha de leitura ou gravação: {0}")]
    Io(#[from] std::io::Error),
    #[error("Banco inválido ou indisponível: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Pacote ZIP inválido: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Documento JSON inválido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Projeto inválido: {0}")]
    Domain(#[from] DomainError),
    #[error("Versão incompatível de {component}: {found}; esta instalação suporta {supported}.")]
    UnsupportedVersion {
        component: String,
        found: u64,
        supported: u32,
    },
    #[error("Versão incompatível de {component}: {value}; esta instalação suporta {supported}.")]
    UnsupportedVersionValue {
        component: String,
        value: String,
        supported: u32,
    },
    #[error("Representação incompatível: {0}")]
    UnsupportedRepresentation(String),
    #[error("Limite de armazenamento excedido: {0}")]
    Limit(String),
    #[error("Integridade do armazenamento inválida: {0}")]
    Integrity(String),
    #[error("Revisão concorrente: esperada {expected:?}, encontrada {actual:?}.")]
    RevisionConflict {
        expected: Option<u64>,
        actual: Option<u64>,
    },
    #[error("Banco aberto somente para leitura.")]
    ReadOnly,
    #[error("O banco ainda não contém um projeto.")]
    Empty,
    #[error("Operação interrompida pelo host: {0}")]
    Interrupted(String),
}

/// Host-authenticated command metadata. The store derives base revision and
/// before/after hashes itself. `None` is reserved for a first imported baseline.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommitRecord {
    pub command_id: Uuid,
    pub actor: String,
    pub operation: String,
    pub reason: Option<String>,
    pub timestamp_unix_ms: u64,
    pub command: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitPhase {
    ObjectsInserted,
    RevisionInserted,
    HeadUpdated,
    BeforeCommit,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageOrigin {
    pub application: String,
    pub version: String,
    pub platform: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpaqueEntry {
    pub media_type: String,
    pub version: u32,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Database,
    ProjectProjection,
    ContentObject,
    CommandLog,
    Extension,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestEntry {
    pub sha256: String,
    pub size: u64,
    pub kind: EntryKind,
    pub media_type: String,
    pub version: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    pub format_version: u32,
    pub database_version: u32,
    pub model_version: u32,
    pub project_id: Uuid,
    pub revision: u64,
    pub created_unix_ms: u64,
    pub origin: PackageOrigin,
    pub project_sha256: String,
    pub head_sha256: String,
    #[serde(deserialize_with = "package::deserialize_entries")]
    pub entries: BTreeMap<String, ManifestEntry>,
}

#[derive(Debug)]
pub struct DecodedPackage {
    pub project: Project,
    pub manifest: PackageManifest,
    pub database_bytes: Vec<u8>,
    pub extensions: BTreeMap<String, OpaqueEntry>,
}

/// One fully validated, consistent image of a working database. The optional
/// project and the bytes always describe the same SQLite snapshot.
#[derive(Debug)]
pub struct WorkingDatabaseSnapshot {
    pub project: Option<Project>,
    pub database_bytes: Vec<u8>,
}

/// Bytes built by the native encoder from one fully validated database image.
/// This is an ownership boundary, not an engineering approval or a signature.
/// Received bytes must always go through `decode_package`.
///
/// ```compile_fail
/// let forged = prumo_storage::EncodedPackage { bytes: vec![1, 2, 3] };
/// ```
pub struct EncodedPackage {
    bytes: Vec<u8>,
}

impl EncodedPackage {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl std::fmt::Debug for EncodedPackage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EncodedPackage")
            .field("size", &self.bytes.len())
            .finish()
    }
}

impl From<prumo_domain::canonical::JsonRepresentationError> for StorageError {
    fn from(error: prumo_domain::canonical::JsonRepresentationError) -> Self {
        match error {
            prumo_domain::canonical::JsonRepresentationError::ReservedNamespace => Self::UnsupportedRepresentation(
                "A chave JSON usa um namespace interno reservado; o conteúdo não foi convertido.".into(),
            ),
            prumo_domain::canonical::JsonRepresentationError::InvalidJson(error) => Self::Json(error),
        }
    }
}

pub(crate) fn check_version(component: &str, found: impl Into<u64>, supported: u32) -> Result<()> {
    let found = found.into();
    if found != u64::from(supported) {
        return Err(StorageError::UnsupportedVersion {
            component: component.into(),
            found,
            supported,
        });
    }
    Ok(())
}

pub(crate) fn check_hash(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(StorageError::Integrity("SHA-256 não canônico.".into()));
    }
    Ok(())
}

pub(crate) fn check_text(value: &str, maximum: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        return Err(StorageError::Integrity("Metadado textual inválido.".into()));
    }
    Ok(())
}
