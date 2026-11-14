//! Storage commands and read models shared by native UI and MCP.
//! Filesystem paths, SQLite connections and archive readers stay in the host.

use crate::Project;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageFormat {
    NativePackage,
    LegacyJson,
    Missing,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageState {
    Ready,
    Legacy,
    RecoveryAvailable,
    Missing,
    Invalid,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateKind {
    Autosave,
    Backup,
    WorkingDatabase,
    Original,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StorageDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StorageCandidate {
    pub id: String,
    pub kind: CandidateKind,
    pub label: String,
    pub project_id: Uuid,
    pub revision: u64,
    pub sha256: String,
    pub recoverable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageEntrySummary {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageSummary {
    pub format_version: u32,
    pub database_version: u32,
    pub model_version: u32,
    pub created_unix_ms: u64,
    pub origin: String,
    pub project_sha256: String,
    pub head_sha256: String,
    pub entries: Vec<PackageEntrySummary>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StorageStatus {
    pub state: StorageState,
    pub format: StorageFormat,
    pub project_id: Option<Uuid>,
    pub revision: Option<u64>,
    pub primary_sha256: Option<String>,
    /// Binds the primary file (including absence) and every recovery source.
    pub inspection_sha256: String,
    pub package: Option<PackageSummary>,
    pub candidates: Vec<StorageCandidate>,
    pub diagnostics: Vec<StorageDiagnostic>,
    pub can_migrate: bool,
    pub can_backup: bool,
    pub can_recover: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum StorageAction {
    Migrate {
        expected_revision: u64,
        inspection_sha256: String,
        confirm: bool,
    },
    Backup {
        expected_revision: u64,
        inspection_sha256: String,
    },
    Recover {
        candidate_id: String,
        inspection_sha256: String,
        confirm: bool,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StorageActionResult {
    pub project: Project,
    pub status: StorageStatus,
    pub changed: bool,
    pub message: String,
}
