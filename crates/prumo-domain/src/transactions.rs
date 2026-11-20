//! Project-scoped transactions shared by the desktop and MCP adapters.
//!
//! Actor, grants and filesystem paths are deliberately absent from requests.
use crate::{
    Command, Project,
    dependencies::{SourceChange, SourceKey},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_TRANSACTION_OPERATIONS: usize = 128;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePrecondition {
    pub source: SourceKey,
    /// `None` explicitly requires that this source does not exist.
    #[serde(deserialize_with = "required_sha256")]
    pub sha256: Option<String>,
}

fn required_sha256<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionRequest {
    pub project_id: Uuid,
    pub expected_revision: u64,
    pub idempotency_key: Uuid,
    #[serde(default)]
    pub preconditions: Vec<SourcePrecondition>,
    pub operations: Vec<Command>,
}

impl TransactionRequest {
    pub fn new(project: &Project, operations: Vec<Command>) -> Self {
        Self {
            project_id: project.id,
            expected_revision: project.revision,
            idempotency_key: Uuid::new_v4(),
            preconditions: Vec::new(),
            operations,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionHistoryRequest {
    pub project_id: Uuid,
    pub expected_revision: u64,
    pub idempotency_key: Uuid,
    pub target_command_id: Uuid,
}

impl TransactionHistoryRequest {
    pub fn new(project: &Project, target_command_id: Uuid) -> Self {
        Self {
            project_id: project.id,
            expected_revision: project.revision,
            idempotency_key: Uuid::new_v4(),
            target_command_id,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionKind {
    Apply,
    Undo,
    Redo,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionPreview {
    pub project_id: Uuid,
    pub base_revision: u64,
    pub idempotency_key: Uuid,
    pub actor: String,
    pub plan_sha256: String,
    pub request_sha256: String,
    pub before_snapshot_sha256: String,
    pub after_snapshot_sha256: String,
    pub operation_count: usize,
    pub effects: Vec<SourceChange>,
    pub invalidated_result_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionReceipt {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub base_revision: u64,
    pub committed_revision: u64,
    pub actor: String,
    pub kind: TransactionKind,
    pub target_command_id: Option<Uuid>,
    pub request_sha256: String,
    pub plan_sha256: String,
    pub before_snapshot_sha256: String,
    pub after_snapshot_sha256: String,
    pub effects: Vec<SourceChange>,
    pub invalidated_result_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionResult {
    /// Always the current state. A replay never replaces later work in the UI.
    pub project: Project,
    /// The immutable receipt still identifies the original committed revision.
    pub receipt: TransactionReceipt,
    pub replayed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionHistory {
    pub project_id: Uuid,
    pub revision: u64,
    pub receipts: Vec<TransactionReceipt>,
    pub undo_command_id: Option<Uuid>,
    pub redo_command_id: Option<Uuid>,
}
