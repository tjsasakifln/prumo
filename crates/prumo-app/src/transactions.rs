//! One transaction boundary for immediate commands, reviewed batches and history.
use super::{AppError, Command, ProjectService, catalog_for, invalid_input, reject_non_regular};
use crate::storage::LoadedFile;
use prumo_domain::canonical::{hash_bytes, model_snapshot, source_index_unchecked};
use prumo_domain::dependencies::{SourceChange, SourceKind};
use prumo_domain::model::{PhysicalRole, PointInput};
use prumo_domain::transactions::{
    MAX_TRANSACTION_OPERATIONS, TransactionHistory, TransactionHistoryRequest, TransactionKind,
    TransactionPreview, TransactionReceipt, TransactionRequest, TransactionResult,
};
use prumo_domain::{Discipline, Environment, ModelState, PhysicalObject, Project, validate_name};
use prumo_registry::{NormativePackage, compare_packages, register_reference};
use prumo_storage::{CommitRecord, JournalEntry, database_journal, database_revision};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use uuid::Uuid;

const PLAN_VERSION: u32 = 1;
const MAX_PLAN_BYTES: u64 = super::MAX_PROJECT_BYTES + prumo_storage::MAX_COMMAND_BYTES;
const MAX_PENDING_PLANS: usize = 32;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PreparedPlan {
    version: u32,
    actor: String,
    before_primary_sha256: String,
    request: TransactionRequest,
    request_sha256: String,
    after: Project,
    effects: Vec<SourceChange>,
    invalidated_result_ids: Vec<Uuid>,
    before_snapshot_sha256: String,
    after_snapshot_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredTransaction {
    transaction_version: u32,
    receipt: TransactionReceipt,
    request: Value,
}

impl ProjectService {
    /// Immediate single commands use the exact same preparation/validation path.
    pub(super) fn execute_single(&self, command: Command) -> Result<Project, AppError> {
        self.require_write()?;
        let expected_revision = command_revision(&command)?;
        let _lock = self.lock()?;
        let original = self.load_editor_basis()?;
        let request = TransactionRequest {
            project_id: original.project.id,
            expected_revision,
            idempotency_key: Uuid::new_v4(),
            preconditions: Vec::new(),
            operations: vec![command],
        };
        let plan = self.prepare(&original, request)?;
        let hash = plan_hash(&plan)?;
        Ok(self.commit_plan(&original, plan, hash)?.project)
    }

    /// A keyed request can be retried after a disconnect or after reopening.
    pub fn execute_transaction(
        &self,
        request: TransactionRequest,
    ) -> Result<TransactionResult, AppError> {
        self.require_write()?;
        self.grant.check_all_disciplines()?;
        validate_request(&request)?;
        let _lock = self.lock()?;
        let original = self.load_primary()?;
        if let Some(receipt) = self.replayed_request(&original, &request)? {
            return Ok(TransactionResult {
                project: original.project,
                receipt,
                replayed: true,
            });
        }
        self.check_write_basis(&original)?;
        let plan = self.prepare(&original, request)?;
        let hash = plan_hash(&plan)?;
        self.commit_plan(&original, plan, hash)
    }

    /// Persists only a bounded host-owned plan, never changes the model/revision.
    pub fn preview_transaction(
        &self,
        request: TransactionRequest,
    ) -> Result<TransactionPreview, AppError> {
        self.require_write()?;
        self.grant.check_all_disciplines()?;
        validate_request(&request)?;
        let _lock = self.lock()?;
        let original = self.load_primary()?;
        if let Some(receipt) = self.replayed_request(&original, &request)? {
            return Ok(preview_from_receipt(&receipt, request.operations.len()));
        }
        self.check_write_basis(&original)?;
        let request_sha256 = request_hash(&request)?;
        let root = self.prepared_root(true)?;
        let mut pending = 0;
        for entry in fs::read_dir(&root)? {
            let path = entry?.path();
            reject_non_regular(&path)?;
            let existing = read_plan(&path)?;
            if existing.before_primary_sha256 != original.sha256() {
                fs::remove_file(path)?;
                continue;
            }
            pending += 1;
            if existing.request.idempotency_key == request.idempotency_key {
                if existing.request_sha256 != request_sha256 || existing.actor != self.actor {
                    return Err(AppError::IdempotencyConflict);
                }
                return preview(&existing, plan_hash(&existing)?);
            }
        }
        if pending >= MAX_PENDING_PLANS {
            return Err(AppError::InvalidInput("Há 32 prévias pendentes nesta revisão. Aplique uma delas antes de preparar outras.".into()));
        }
        let plan = self.prepare(&original, request)?;
        let hash = plan_hash(&plan)?;
        let bytes = serde_json::to_vec(&plan)?;
        if bytes.len() as u64 > MAX_PLAN_BYTES {
            return Err(AppError::InvalidInput(
                "O plano excede o limite de tamanho.".into(),
            ));
        }
        self.write_prepared(&root.join(format!("{hash}.json")), &bytes)?;
        preview(&plan, hash)
    }

    /// Applies the exact prepared IDs and effects. No caller-supplied after-state.
    pub fn apply_transaction(&self, plan_sha256: &str) -> Result<TransactionResult, AppError> {
        self.require_write()?;
        self.grant.check_all_disciplines()?;
        validate_hash(plan_sha256)?;
        let _lock = self.lock()?;
        let original = self.load_primary()?;
        let entries = journal(&original)?;
        for entry in &entries {
            if let Some(stored) = stored_transaction(entry)?
                && stored.receipt.plan_sha256 == plan_sha256
            {
                if stored.receipt.actor != self.actor {
                    return Err(AppError::PermissionDenied);
                }
                return Ok(TransactionResult {
                    project: original.project,
                    receipt: stored.receipt,
                    replayed: true,
                });
            }
        }
        self.check_write_basis(&original)?;
        let path = self
            .prepared_root(false)?
            .join(format!("{plan_sha256}.json"));
        let plan = read_plan(&path)?;
        if plan_hash(&plan)? != plan_sha256 || plan.version != PLAN_VERSION {
            return Err(AppError::PlanConflict);
        }
        if plan.actor != self.actor {
            return Err(AppError::PermissionDenied);
        }
        if let Some(receipt) = self.replayed_request(&original, &plan.request)? {
            if receipt.plan_sha256 != plan_sha256 {
                return Err(AppError::IdempotencyConflict);
            }
            return Ok(TransactionResult {
                project: original.project,
                receipt,
                replayed: true,
            });
        }
        // Revision checks precede content conflicts so concurrent users get an actionable error.
        check_basis(
            &original.project,
            plan.request.project_id,
            plan.request.expected_revision,
        )?;
        if plan.before_primary_sha256 != original.sha256() {
            return Err(AppError::PlanConflict);
        }
        self.check_plan_authorization(&original.project, &plan)?;
        self.commit_plan(&original, plan, plan_sha256.into())
    }

    pub fn transaction_history(&self) -> Result<TransactionHistory, AppError> {
        let original = self.load_primary()?;
        self.grant.check_full_read(&original.project)?;
        history(&original)
    }

    pub fn undo_transaction(
        &self,
        request: TransactionHistoryRequest,
    ) -> Result<TransactionResult, AppError> {
        self.restore_transaction(request, TransactionKind::Undo)
    }

    pub fn redo_transaction(
        &self,
        request: TransactionHistoryRequest,
    ) -> Result<TransactionResult, AppError> {
        self.restore_transaction(request, TransactionKind::Redo)
    }

    fn replayed_request(
        &self,
        original: &LoadedFile,
        request: &TransactionRequest,
    ) -> Result<Option<TransactionReceipt>, AppError> {
        if request.project_id != original.project.id {
            return Err(AppError::PermissionDenied);
        }
        replay(
            &journal(original)?,
            request.idempotency_key,
            &request_hash(request)?,
            &self.actor,
        )
    }

    fn prepare(
        &self,
        original: &LoadedFile,
        request: TransactionRequest,
    ) -> Result<PreparedPlan, AppError> {
        validate_request(&request)?;
        check_basis(
            &original.project,
            request.project_id,
            request.expected_revision,
        )?;
        let before = source_index_unchecked(&original.project).map_err(invalid_input)?;
        for condition in &request.preconditions {
            if before.get(&condition.source) != condition.sha256.as_ref() {
                return Err(AppError::PreconditionFailed);
            }
        }
        let mut after = original.project.clone();
        for command in &request.operations {
            self.grant.check_command(&after, command)?;
            apply_uncommitted(&mut after, command.clone(), &self.actor)?;
        }
        self.grant.check_delta(&original.project, &after)?;
        let operation = if request.operations.len() == 1 {
            operation_name(&request.operations[0])?
        } else {
            "transaction.batch".into()
        };
        let (effects, invalidated_result_ids) =
            self.finalize(&original.project, &mut after, &operation)?;
        // Commit stamps its real time. Review is over effects and immutable identities.
        after
            .history
            .last_mut()
            .expect("finalized revision")
            .timestamp_unix_ms = 0;
        let before_snapshot_sha256 = model_snapshot(&original.project)
            .map_err(invalid_input)?
            .sha256;
        let after_snapshot_sha256 = model_snapshot(&after).map_err(invalid_input)?.sha256;
        Ok(PreparedPlan {
            version: PLAN_VERSION,
            actor: self.actor.clone(),
            before_primary_sha256: original.sha256(),
            request_sha256: request_hash(&request)?,
            request,
            after,
            effects,
            invalidated_result_ids,
            before_snapshot_sha256,
            after_snapshot_sha256,
        })
    }

    fn check_plan_authorization(
        &self,
        before: &Project,
        plan: &PreparedPlan,
    ) -> Result<(), AppError> {
        // Full read/model grant is required for serialized plans and global receipts.
        self.grant.check_full_read(before)?;
        self.grant.check_delta(before, &plan.after)?;
        if request_hash(&plan.request)? != plan.request_sha256 {
            return Err(AppError::PlanConflict);
        }
        for condition in &plan.request.preconditions {
            if source_index_unchecked(before)
                .map_err(invalid_input)?
                .get(&condition.source)
                != condition.sha256.as_ref()
            {
                return Err(AppError::PreconditionFailed);
            }
        }
        Ok(())
    }

    fn commit_plan(
        &self,
        original: &LoadedFile,
        mut plan: PreparedPlan,
        plan_sha256: String,
    ) -> Result<TransactionResult, AppError> {
        self.grant.check_delta(&original.project, &plan.after)?;
        check_basis(
            &original.project,
            plan.request.project_id,
            plan.request.expected_revision,
        )?;
        let stamp = self.change(
            plan.after.revision,
            &plan
                .after
                .history
                .last()
                .ok_or(AppError::PlanConflict)?
                .operation,
        )?;
        *plan
            .after
            .history
            .last_mut()
            .ok_or(AppError::PlanConflict)? = stamp;
        if model_snapshot(&plan.after).map_err(invalid_input)?.sha256 != plan.after_snapshot_sha256
        {
            return Err(AppError::PlanConflict);
        }
        let receipt = TransactionReceipt {
            command_id: plan.request.idempotency_key,
            project_id: original.project.id,
            base_revision: original.project.revision,
            committed_revision: plan.after.revision,
            actor: self.actor.clone(),
            kind: TransactionKind::Apply,
            target_command_id: None,
            request_sha256: plan.request_sha256,
            plan_sha256,
            before_snapshot_sha256: plan.before_snapshot_sha256,
            after_snapshot_sha256: plan.after_snapshot_sha256,
            effects: plan.effects,
            invalidated_result_ids: plan.invalidated_result_ids,
        };
        self.save_receipt(
            original,
            &plan.after,
            &receipt,
            serde_json::to_value(plan.request)?,
        )?;
        Ok(TransactionResult {
            project: plan.after,
            receipt,
            replayed: false,
        })
    }

    fn finalize(
        &self,
        before_project: &Project,
        after_project: &mut Project,
        operation: &str,
    ) -> Result<(Vec<SourceChange>, Vec<Uuid>), AppError> {
        let before = source_index_unchecked(before_project).map_err(invalid_input)?;
        let after = source_index_unchecked(after_project).map_err(invalid_input)?;
        let revision = before_project
            .revision
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidInput("Limite de revisões atingido.".into()))?;
        let previous_events = after_project.dependencies.invalidations.len();
        after_project
            .dependencies
            .invalidate_changes(&before, &after, revision, &self.actor, operation)
            .map_err(invalid_input)?;
        let invalidated = after_project
            .dependencies
            .invalidations
            .iter()
            .skip(previous_events)
            .flat_map(|event| event.result_ids.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let effects = source_changes(&before, &after);
        if after_project.normative.active.is_some()
            && effects
                .iter()
                .any(|effect| effect.source.kind != SourceKind::ProjectMetadata)
        {
            after_project.normative.recalculation_required = true;
        }
        after_project.revision = revision;
        after_project.history = before_project.history.clone();
        after_project
            .history
            .push(self.change(revision, operation)?);
        after_project.validate().map_err(invalid_input)?;
        Ok((effects, invalidated))
    }

    fn save_receipt(
        &self,
        original: &LoadedFile,
        project: &Project,
        receipt: &TransactionReceipt,
        request: Value,
    ) -> Result<(), AppError> {
        let change = project.history.last().ok_or(AppError::PlanConflict)?;
        let record = CommitRecord {
            command_id: receipt.command_id,
            actor: self.actor.clone(),
            operation: change.operation.clone(),
            reason: None,
            timestamp_unix_ms: change.timestamp_unix_ms,
            command: Some(serde_json::to_value(StoredTransaction {
                transaction_version: PLAN_VERSION,
                receipt: receipt.clone(),
                request,
            })?),
        };
        self.save_changed_record_locked(original, project, record)
    }

    fn restore_transaction(
        &self,
        request: TransactionHistoryRequest,
        kind: TransactionKind,
    ) -> Result<TransactionResult, AppError> {
        self.require_write()?;
        self.grant.check_all_disciplines()?;
        if request.idempotency_key.is_nil()
            || request.target_command_id.is_nil()
            || request.project_id.is_nil()
        {
            return Err(AppError::InvalidInput(
                "Identidade de comando inválida.".into(),
            ));
        }
        let _lock = self.lock()?;
        let original = self.load_primary()?;
        if request.project_id != original.project.id {
            return Err(AppError::PermissionDenied);
        }
        let entries = journal(&original)?;
        let request_value = json!({"kind": kind, "request": request});
        let request_sha256 = hash_bytes(&serde_json::to_vec(&request_value)?);
        if let Some(receipt) = replay(
            &entries,
            request.idempotency_key,
            &request_sha256,
            &self.actor,
        )? {
            return Ok(TransactionResult {
                project: original.project,
                receipt,
                replayed: true,
            });
        }
        self.check_write_basis(&original)?;
        check_basis(
            &original.project,
            request.project_id,
            request.expected_revision,
        )?;
        let state = history(&original)?;
        let available = if kind == TransactionKind::Undo {
            state.undo_command_id
        } else {
            state.redo_command_id
        };
        if available != Some(request.target_command_id) {
            return Err(AppError::HistoryConflict);
        }
        let target = state
            .receipts
            .iter()
            .find(|receipt| {
                receipt.command_id == request.target_command_id
                    && receipt.kind == TransactionKind::Apply
            })
            .ok_or(AppError::HistoryConflict)?;
        let revision = if kind == TransactionKind::Undo {
            target.base_revision
        } else {
            target.committed_revision
        };
        let package = original.package.as_ref().ok_or(AppError::HistoryConflict)?;
        let mut after = database_revision(&package.database_bytes, revision)?;
        // Results stay auditable and cannot become current again merely by undo.
        after.dependencies = original.project.dependencies.clone();
        if after.normative.active.is_some() {
            after.normative.recalculation_required |=
                original.project.normative.recalculation_required;
        }
        after.revision = original.project.revision;
        after.history = original.project.history.clone();
        self.grant.check_delta(&original.project, &after)?;
        let operation = if kind == TransactionKind::Undo {
            "transaction.undo"
        } else {
            "transaction.redo"
        };
        let (effects, invalidated_result_ids) =
            self.finalize(&original.project, &mut after, operation)?;
        let before_snapshot_sha256 = model_snapshot(&original.project)
            .map_err(invalid_input)?
            .sha256;
        let after_snapshot_sha256 = model_snapshot(&after).map_err(invalid_input)?.sha256;
        let plan_sha256 = hash_bytes(&serde_json::to_vec(
            &json!({"request": request_value, "before": before_snapshot_sha256, "after": after_snapshot_sha256, "actor": self.actor}),
        )?);
        let receipt = TransactionReceipt {
            command_id: request.idempotency_key,
            project_id: original.project.id,
            base_revision: original.project.revision,
            committed_revision: after.revision,
            actor: self.actor.clone(),
            kind,
            target_command_id: Some(request.target_command_id),
            request_sha256,
            plan_sha256,
            before_snapshot_sha256,
            after_snapshot_sha256,
            effects,
            invalidated_result_ids,
        };
        self.save_receipt(&original, &after, &receipt, request_value)?;
        Ok(TransactionResult {
            project: after,
            receipt,
            replayed: false,
        })
    }
}

fn journal(original: &LoadedFile) -> Result<Vec<JournalEntry>, AppError> {
    original.package.as_ref().map_or_else(
        || Ok(Vec::new()),
        |package| Ok(database_journal(&package.database_bytes)?),
    )
}

fn stored_transaction(entry: &JournalEntry) -> Result<Option<StoredTransaction>, AppError> {
    let Some(value) = entry
        .record
        .command
        .as_ref()
        .filter(|value| value.get("transaction_version").is_some())
    else {
        return Ok(None);
    };
    let stored: StoredTransaction = serde_json::from_value(value.clone())?;
    if stored.transaction_version != PLAN_VERSION
        || stored.receipt.command_id != entry.record.command_id
        || stored.receipt.actor != entry.record.actor
        || stored.receipt.committed_revision != entry.revision
        || Some(stored.receipt.base_revision) != entry.base_revision
    {
        return Err(AppError::InvalidProject(
            "Recibo transacional diverge do histórico armazenado.".into(),
        ));
    }
    Ok(Some(stored))
}

fn replay(
    entries: &[JournalEntry],
    id: Uuid,
    request_sha256: &str,
    actor: &str,
) -> Result<Option<TransactionReceipt>, AppError> {
    let Some(entry) = entries.iter().find(|entry| entry.record.command_id == id) else {
        return Ok(None);
    };
    let stored = stored_transaction(entry)?.ok_or(AppError::IdempotencyConflict)?;
    if stored.receipt.request_sha256 != request_sha256 || stored.receipt.actor != actor {
        return Err(AppError::IdempotencyConflict);
    }
    Ok(Some(stored.receipt))
}

fn history(original: &LoadedFile) -> Result<TransactionHistory, AppError> {
    let mut receipts = Vec::new();
    let mut undo = Vec::new();
    let mut redo = Vec::new();
    for entry in journal(original)? {
        let Some(stored) = stored_transaction(&entry)? else {
            // Recovery/import/unknown mutation is a history boundary, never guessed.
            undo.clear();
            redo.clear();
            continue;
        };
        let receipt = stored.receipt;
        match receipt.kind {
            TransactionKind::Apply if !receipt.effects.is_empty() => {
                undo.push(receipt.command_id);
                redo.clear();
            }
            TransactionKind::Apply => {} // Read-derived inspection results are not model edits.
            TransactionKind::Undo => {
                let target = receipt.target_command_id.ok_or(AppError::HistoryConflict)?;
                if undo.pop() != Some(target) {
                    return Err(AppError::HistoryConflict);
                }
                redo.push(target);
            }
            TransactionKind::Redo => {
                let target = receipt.target_command_id.ok_or(AppError::HistoryConflict)?;
                if redo.pop() != Some(target) {
                    return Err(AppError::HistoryConflict);
                }
                undo.push(target);
            }
        }
        receipts.push(receipt);
    }
    Ok(TransactionHistory {
        project_id: original.project.id,
        revision: original.project.revision,
        receipts,
        undo_command_id: undo.last().copied(),
        redo_command_id: redo.last().copied(),
    })
}

fn read_plan(path: &std::path::Path) -> Result<PreparedPlan, AppError> {
    reject_non_regular(path)?;
    let file = File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::PlanConflict
        } else {
            error.into()
        }
    })?;
    if file.metadata()?.len() > MAX_PLAN_BYTES {
        return Err(AppError::PlanConflict);
    }
    let mut bytes = Vec::new();
    file.take(MAX_PLAN_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PLAN_BYTES {
        return Err(AppError::PlanConflict);
    }
    prumo_domain::canonical::validate_json_representation(&bytes)
        .map_err(|error| AppError::InvalidInput(error.to_string()))?;
    Ok(serde_json::from_value(serde_json::from_slice(&bytes)?)?)
}

fn request_hash(request: &TransactionRequest) -> Result<String, AppError> {
    Ok(hash_bytes(&serde_json::to_vec(request)?))
}
fn plan_hash(plan: &PreparedPlan) -> Result<String, AppError> {
    Ok(hash_bytes(&serde_json::to_vec(plan)?))
}

fn preview(plan: &PreparedPlan, hash: String) -> Result<TransactionPreview, AppError> {
    if plan.version != PLAN_VERSION {
        return Err(AppError::PlanConflict);
    }
    Ok(TransactionPreview {
        project_id: plan.request.project_id,
        base_revision: plan.request.expected_revision,
        idempotency_key: plan.request.idempotency_key,
        actor: plan.actor.clone(),
        plan_sha256: hash,
        request_sha256: plan.request_sha256.clone(),
        before_snapshot_sha256: plan.before_snapshot_sha256.clone(),
        after_snapshot_sha256: plan.after_snapshot_sha256.clone(),
        operation_count: plan.request.operations.len(),
        effects: plan.effects.clone(),
        invalidated_result_ids: plan.invalidated_result_ids.clone(),
    })
}

fn preview_from_receipt(
    receipt: &TransactionReceipt,
    operation_count: usize,
) -> TransactionPreview {
    TransactionPreview {
        project_id: receipt.project_id,
        base_revision: receipt.base_revision,
        idempotency_key: receipt.command_id,
        actor: receipt.actor.clone(),
        plan_sha256: receipt.plan_sha256.clone(),
        request_sha256: receipt.request_sha256.clone(),
        before_snapshot_sha256: receipt.before_snapshot_sha256.clone(),
        after_snapshot_sha256: receipt.after_snapshot_sha256.clone(),
        operation_count,
        effects: receipt.effects.clone(),
        invalidated_result_ids: receipt.invalidated_result_ids.clone(),
    }
}

fn validate_hash(hash: &str) -> Result<(), AppError> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Err(AppError::InvalidInput("Hash SHA-256 inválido.".into()))
    } else {
        Ok(())
    }
}

fn check_basis(project: &Project, id: Uuid, revision: u64) -> Result<(), AppError> {
    if project.id != id {
        return Err(AppError::PermissionDenied);
    }
    if project.revision != revision {
        return Err(AppError::Conflict {
            expected: revision,
            actual: project.revision,
        });
    }
    Ok(())
}

fn validate_request(request: &TransactionRequest) -> Result<(), AppError> {
    if request.project_id.is_nil()
        || request.idempotency_key.is_nil()
        || request.expected_revision == 0
        || request.operations.is_empty()
        || request.operations.len() > MAX_TRANSACTION_OPERATIONS
        || request.preconditions.len() > MAX_TRANSACTION_OPERATIONS * 16
    {
        return Err(AppError::InvalidInput(
            "Pedido deve identificar projeto, revisão e comando, com 1 a 128 operações.".into(),
        ));
    }
    let mut sources = BTreeSet::new();
    for condition in &request.preconditions {
        if condition.source.id.is_nil() || !sources.insert(condition.source) {
            return Err(AppError::InvalidInput(
                "Pré-condição nula ou repetida.".into(),
            ));
        }
        if let Some(hash) = &condition.sha256 {
            validate_hash(hash)?;
        }
    }
    for command in &request.operations {
        if command_revision(command)? != request.expected_revision {
            return Err(AppError::InvalidInput(
                "Todas as operações do lote devem usar a mesma revisão base.".into(),
            ));
        }
        if request.operations.len() > 1 && matches!(command, Command::InspectModel { .. }) {
            return Err(AppError::InvalidInput(
                "Execute a inspeção de resultados depois de aplicar o lote de modelagem.".into(),
            ));
        }
    }
    if serde_json::to_vec(request)?.len() as u64 > prumo_storage::MAX_COMMAND_BYTES / 2 {
        return Err(AppError::InvalidInput(
            "O pedido transacional excede o limite de tamanho.".into(),
        ));
    }
    Ok(())
}

fn command_revision(command: &Command) -> Result<u64, AppError> {
    match command {
        Command::Create { .. } => Err(AppError::InvalidInput(
            "Crie o projeto antes de preparar um lote.".into(),
        )),
        Command::AddMember {
            expected_revision, ..
        }
        | Command::RenameProject {
            expected_revision, ..
        }
        | Command::AddEnvironment {
            expected_revision, ..
        }
        | Command::RegisterNormReference {
            expected_revision, ..
        }
        | Command::PinNormativePackage {
            expected_revision, ..
        }
        | Command::MigrateNormativePackage {
            expected_revision, ..
        }
        | Command::EditModel {
            expected_revision, ..
        }
        | Command::InspectModel { expected_revision } => Ok(*expected_revision),
    }
}

fn operation_name(command: &Command) -> Result<String, AppError> {
    if let Command::EditModel { edit, .. } = command {
        let value = serde_json::to_value(edit)?;
        Ok(format!(
            "model.{}",
            value
                .get("edit")
                .and_then(Value::as_str)
                .ok_or(AppError::PlanConflict)?
        ))
    } else {
        let value = serde_json::to_value(command)?;
        Ok(value
            .get("operation")
            .and_then(Value::as_str)
            .ok_or(AppError::PlanConflict)?
            .into())
    }
}

fn source_changes(
    before: &BTreeMap<prumo_domain::dependencies::SourceKey, String>,
    after: &BTreeMap<prumo_domain::dependencies::SourceKey, String>,
) -> Vec<SourceChange> {
    before
        .keys()
        .chain(after.keys())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|key| before.get(key) != after.get(key))
        .map(|source| SourceChange {
            source,
            before_sha256: before.get(&source).cloned(),
            after_sha256: after.get(&source).cloned(),
        })
        .collect()
}

fn apply_uncommitted(project: &mut Project, command: Command, actor: &str) -> Result<(), AppError> {
    let next_revision = project
        .revision
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidInput("Limite de revisões atingido.".into()))?;
    match command {
        Command::Create { .. } => {
            return Err(AppError::InvalidInput(
                "Criação não pertence a um lote existente.".into(),
            ));
        }
        Command::AddMember {
            name, start, end, ..
        } => {
            validate_name(&name).map_err(invalid_input)?;
            let id = Uuid::new_v4();
            let object = PhysicalObject {
                id,
                name,
                discipline: Discipline::Structure,
                start_m: PointInput {
                    values: start,
                    unit: project.units,
                }
                .to_si()
                .map_err(invalid_input)?,
                end_m: PointInput {
                    values: end,
                    unit: project.units,
                }
                .to_si()
                .map_err(invalid_input)?,
                state: ModelState::Modelled,
                role: PhysicalRole::AnalyticalSource,
                material_id: None,
                environment_id: None,
                extensions: BTreeMap::new(),
            };
            if object.length_m() <= prumo_domain::MIN_MEMBER_LENGTH_M {
                return Err(invalid_input(prumo_domain::DomainError::DegenerateMember));
            }
            project.objects.push(object);
            prumo_domain::model::initialize_legacy_member(project, id, None, actor, next_revision)
                .map_err(invalid_input)?;
        }
        Command::RenameProject { name, .. } => {
            validate_name(&name).map_err(invalid_input)?;
            project.name = name;
        }
        Command::AddEnvironment { name, .. } => {
            validate_name(&name).map_err(invalid_input)?;
            let id = Uuid::new_v4();
            project.environments.push(Environment {
                id,
                name,
                extensions: BTreeMap::new(),
            });
            prumo_domain::model::record_identity(project, id, actor, next_revision, true)
                .map_err(invalid_input)?;
        }
        Command::EditModel { edit, .. } => {
            prumo_domain::model::apply_model_edit(project, edit, actor).map_err(invalid_input)?;
        }
        Command::InspectModel { .. } => {
            prumo_domain::dependencies::inspect_model(project).map_err(invalid_input)?;
        }
        Command::RegisterNormReference { reference, .. } => {
            let catalog = catalog_for(project)?;
            let version = format!("project:{}:revision:{next_revision}", project.id);
            let (updated, change) =
                register_reference(&catalog, reference, version, next_revision)?;
            project.normative.catalog = Some(updated);
            project.normative.registry_changes.push(change);
            if project.normative.active.is_some() {
                project.normative.recalculation_required = true;
            }
        }
        Command::PinNormativePackage { package_hash, .. } => {
            if project.normative.active.is_some() {
                return Err(AppError::PackageAlreadyPinned);
            }
            let candidate = NormativePackage::from_catalog(&catalog_for(project)?)?;
            if candidate.hash != package_hash {
                return Err(AppError::PackageConflict);
            }
            project.normative.catalog = Some(candidate.catalog.clone());
            project.normative.active = Some(candidate);
            project.normative.recalculation_required = true;
        }
        Command::MigrateNormativePackage {
            from_package_hash,
            to_package_hash,
            confirm,
            ..
        } => {
            if !confirm {
                return Err(AppError::ConfirmationRequired);
            }
            let original = project
                .normative
                .active
                .as_ref()
                .ok_or(AppError::PackageNotPinned)?;
            let candidate = NormativePackage::from_catalog(&catalog_for(project)?)?;
            if original.hash != from_package_hash
                || candidate.hash != to_package_hash
                || original.hash == candidate.hash
            {
                return Err(AppError::PackageConflict);
            }
            let impact = compare_packages(original, &candidate, next_revision)?;
            project.normative.previous.push(original.clone());
            project.normative.migrations.push(impact);
            project.normative.active = Some(candidate);
            project.normative.recalculation_required = true;
        }
    }
    Ok(())
}
