#![forbid(unsafe_code)]

//! Project-scoped command service shared by desktop, CLI, and MCP.

pub use prumo_domain::Command;
pub mod access;
mod authoring;
pub use authoring::WebViewExport;
mod modeling;
mod storage;
mod transactions;
pub use access::{AccessScope, ProjectGrant, ScopedModelView};
#[cfg(test)]
mod storage_fault_tests;
use prumo_domain::canonical::ModelSnapshot;
use prumo_domain::components::{ComponentDescriptor, ExperimentBasis, QualificationReport};
use prumo_domain::dependencies::DependencyGraph;
use prumo_domain::model::{ModelEdit, SemanticModel};
use prumo_domain::{
    ChangeRecord, DomainError, LengthUnit, PROJECT_SCHEMA_VERSION, Project, validate_name,
};
use prumo_registry::{
    CoverageAudit, FinalResultContext, GateReport, NormReferenceInput, NormativePackage,
    NormativePackagePreview, NormativeState, RegistryCatalog, RegistryError, audit_coverage,
    bundled_catalog, compare_packages, evaluate_final_result, reason,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, atomic::AtomicBool};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

pub const MAX_PROJECT_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Debug)]
pub struct ProjectService {
    path: PathBuf,
    lock_path: PathBuf,
    access: AccessMode,
    actor: String,
    grant: ProjectGrant,
    project_binding: Arc<OnceLock<Uuid>>,
    write_basis: Option<(Uuid, String)>,
    view_state: Arc<Mutex<prumo_domain::view::ViewState>>,
    #[cfg(test)]
    fault: Option<storage::SavePhase>,
}

impl ProjectService {
    pub fn model_snapshot(&self) -> Result<ModelSnapshot, AppError> {
        prumo_domain::canonical::model_snapshot(&self.read()?).map_err(invalid_input)
    }

    pub fn dependency_report(&self) -> Result<DependencyGraph, AppError> {
        Ok(self.read()?.dependencies)
    }

    pub fn edit_model(&self, expected_revision: u64, edit: ModelEdit) -> Result<Project, AppError> {
        self.execute_single(Command::EditModel {
            expected_revision,
            edit,
        })
    }

    pub fn inspect_model(&self, expected_revision: u64) -> Result<Project, AppError> {
        self.execute_single(Command::InspectModel { expected_revision })
    }

    /// Read-only inventory. This does not change capability release states.
    pub fn component_inventory(&self) -> Vec<ComponentDescriptor> {
        prumo_components::inventory()
    }

    pub fn qualify_components(&self) -> Result<QualificationReport, AppError> {
        self.qualify_components_with_cancel(&AtomicBool::new(false))
    }

    /// Run the fixed reference suite, recording the selected project's identity.
    /// Project contents never become solver inputs and no result is persisted.
    pub fn qualify_components_with_cancel(
        &self,
        cancel: &AtomicBool,
    ) -> Result<QualificationReport, AppError> {
        let project = self.read()?;
        let content = serde_json::to_vec(&project)?;
        let basis = ExperimentBasis {
            project_id: project.id,
            revision: project.revision,
            content_sha256: Sha256::digest(&content)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        };
        let request = prumo_components::process::WorkerRequest::new(Some(basis));
        let worker = prumo_components::process::installed_worker()?;
        Ok(prumo_components::process::run_worker(
            &worker,
            &request,
            cancel,
            prumo_components::process::DEFAULT_TIMEOUT,
        )?)
    }

    /// Selects one project on the trusted host. This path is never a command parameter.
    pub fn open(
        path: impl Into<PathBuf>,
        access: AccessMode,
        actor: impl Into<String>,
    ) -> Result<Self, AppError> {
        Self::open_scoped(path, ProjectGrant::for_access(access), actor)
    }

    pub fn open_scoped(
        path: impl Into<PathBuf>,
        grant: ProjectGrant,
        actor: impl Into<String>,
    ) -> Result<Self, AppError> {
        grant.validate()?;
        let access = if grant.allows(AccessScope::Model) {
            AccessMode::ReadWrite
        } else {
            AccessMode::ReadOnly
        };
        let path = path.into();
        let actor = actor.into();
        validate_name(&actor).map_err(invalid_input)?;
        if path.extension().and_then(|extension| extension.to_str()) != Some("prumo") {
            return Err(AppError::InvalidInput(
                "Selecione um arquivo com extensão .prumo.".into(),
            ));
        }
        let file_name = path.file_name().ok_or_else(|| {
            AppError::InvalidInput("Informe um nome de arquivo de projeto.".into())
        })?;
        let parent = path
            .parent()
            .filter(|value| !value.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        if access == AccessMode::ReadWrite {
            fs::create_dir_all(parent)?;
        }
        let parent = parent.canonicalize()?;
        let path = parent.join(file_name);
        reject_non_regular(&path)?;
        let mut lock_name = file_name.to_os_string();
        lock_name.push(".lock");
        let lock_path = parent.join(lock_name);
        reject_non_regular(&lock_path)?;
        Ok(Self {
            path,
            lock_path,
            access,
            actor,
            grant,
            project_binding: Arc::new(OnceLock::new()),
            write_basis: None,
            view_state: Arc::new(Mutex::new(prumo_domain::view::ViewState::default())),
            #[cfg(test)]
            fault: None,
        })
    }

    pub fn grant(&self) -> &ProjectGrant {
        &self.grant
    }

    pub fn read_scoped_model(&self) -> Result<ScopedModelView, AppError> {
        self.grant.check_scope(AccessScope::Read)?;
        self.grant.project_view(&self.load_primary()?.project)
    }

    fn check_bound_project(&self, project: &Project) -> Result<(), AppError> {
        self.grant.check_project(project)?;
        self.check_bound_id(project.id)
    }

    fn check_bound_id(&self, project_id: Uuid) -> Result<(), AppError> {
        if self.grant.project_id.is_some_and(|id| id != project_id) {
            return Err(AppError::PermissionDenied);
        }
        let bound = self.project_binding.get_or_init(|| project_id);
        if *bound == project_id {
            Ok(())
        } else {
            Err(AppError::PermissionDenied)
        }
    }

    pub fn project_path(&self) -> &Path {
        &self.path
    }

    pub const fn access_mode(&self) -> AccessMode {
        self.access
    }

    /// Bind an editor's displayed basis without trusting a fresh on-disk revision
    /// to represent the content that the person actually reviewed. The check is
    /// performed inside the same project lock as the eventual model mutation.
    /// Navigation and web export also check it against their immutable file read.
    pub fn with_write_basis(
        &self,
        project_id: Uuid,
        primary_sha256: String,
    ) -> Result<Self, AppError> {
        if project_id.is_nil()
            || primary_sha256.len() != 64
            || !primary_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(AppError::InvalidInput(
                "Base de edição inválida. Reabra o arquivo.".into(),
            ));
        }
        let mut scoped = self.clone();
        scoped.write_basis = Some((project_id, primary_sha256));
        Ok(scoped)
    }

    pub fn execute(&self, command: Command) -> Result<Project, AppError> {
        match command {
            Command::Create { name, units } => self.create(name, units),
            Command::AddMember {
                expected_revision,
                name,
                start,
                end,
            } => self.add_member(expected_revision, name, start, end),
            Command::RenameProject {
                expected_revision,
                name,
            } => self.rename_project(expected_revision, name),
            Command::AddEnvironment {
                expected_revision,
                name,
            } => self.add_environment(expected_revision, name),
            Command::RegisterNormReference {
                expected_revision,
                reference,
            } => self.register_norm_reference(expected_revision, reference),
            Command::PinNormativePackage {
                expected_revision,
                package_hash,
            } => self.pin_normative_package(expected_revision, &package_hash),
            Command::MigrateNormativePackage {
                expected_revision,
                from_package_hash,
                to_package_hash,
                confirm,
            } => self.migrate_normative_package(
                expected_revision,
                &from_package_hash,
                &to_package_hash,
                confirm,
            ),
            Command::EditModel {
                expected_revision,
                edit,
            } => self.edit_model(expected_revision, edit),
            Command::InspectModel { expected_revision } => self.inspect_model(expected_revision),
        }
    }

    pub fn create(&self, name: impl Into<String>, units: LengthUnit) -> Result<Project, AppError> {
        self.require_write()?;
        self.grant.check_all_disciplines()?;
        let name = name.into();
        validate_name(&name).map_err(invalid_input)?;
        let _lock = self.lock()?;
        if self.path.try_exists()? {
            return Err(AppError::AlreadyExists);
        }
        let project = Project {
            schema_version: PROJECT_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            name,
            revision: 1,
            units,
            objects: Vec::new(),
            environments: Vec::new(),
            networks: Vec::new(),
            ports: Vec::new(),
            connections: Vec::new(),
            history: vec![self.change(1, "create")?],
            normative: NormativeState::default(),
            model: SemanticModel::default(),
            dependencies: DependencyGraph::default(),
            extensions: BTreeMap::new(),
        };
        self.save_new_locked(
            &project,
            serde_json::json!({"operation":"create", "name":project.name, "units":project.units}),
        )?;
        Ok(project)
    }

    pub fn read(&self) -> Result<Project, AppError> {
        // Writers publish a complete immutable file with an atomic replacement.
        // Opening that file gives one complete snapshot, including during a save.
        // A query must also work when the project directory is not writable.
        self.grant.check_scope(AccessScope::Read)?;
        let project = self.load_primary()?.project;
        self.grant.check_full_read(&project)?;
        Ok(project)
    }

    /// A native editor must associate its model with the bytes from that exact
    /// read, rather than borrowing a hash from a later, independent diagnosis.
    pub fn read_with_basis(&self) -> Result<(Project, String), AppError> {
        self.grant.check_scope(AccessScope::Read)?;
        let file = self.load_primary()?;
        self.grant.check_full_read(&file.project)?;
        let hash = file.sha256();
        Ok((file.project, hash))
    }

    /// Coordinates use the project's declared display unit; storage is always in metres.
    pub fn add_member(
        &self,
        expected_revision: u64,
        name: impl Into<String>,
        start: [f64; 3],
        end: [f64; 3],
    ) -> Result<Project, AppError> {
        self.execute_single(Command::AddMember {
            expected_revision,
            name: name.into(),
            start,
            end,
        })
    }

    pub fn rename_project(
        &self,
        expected_revision: u64,
        name: impl Into<String>,
    ) -> Result<Project, AppError> {
        self.execute_single(Command::RenameProject {
            expected_revision,
            name: name.into(),
        })
    }

    pub fn add_environment(
        &self,
        expected_revision: u64,
        name: impl Into<String>,
    ) -> Result<Project, AppError> {
        self.execute_single(Command::AddEnvironment {
            expected_revision,
            name: name.into(),
        })
    }

    pub fn registry_catalog(&self) -> Result<RegistryCatalog, AppError> {
        match self.read() {
            Ok(project) => catalog_for(&project),
            Err(AppError::NotFound) => Ok(bundled_catalog()?),
            Err(error) => Err(error),
        }
    }

    pub fn coverage_audit(&self) -> Result<CoverageAudit, AppError> {
        Ok(audit_coverage(&self.registry_catalog()?))
    }

    pub fn available_normative_package(&self) -> Result<NormativePackage, AppError> {
        Ok(NormativePackage::from_catalog(&self.registry_catalog()?)?)
    }

    /// One read supplies the revision, active hash, candidate and impact together.
    pub fn normative_package_preview(&self) -> Result<NormativePackagePreview, AppError> {
        let project = match self.read() {
            Ok(project) => Some(project),
            Err(AppError::NotFound) => None,
            Err(error) => return Err(error),
        };
        let catalog = match &project {
            Some(project) => catalog_for(project)?,
            None => bundled_catalog()?,
        };
        let candidate = NormativePackage::from_catalog(&catalog)?;
        let active = project
            .as_ref()
            .and_then(|project| project.normative.active.as_ref());
        let impact = match (project.as_ref(), active) {
            (Some(project), Some(active)) if active.hash != candidate.hash => {
                let revision = project
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| AppError::InvalidInput("Limite de revisões atingido.".into()))?;
                Some(compare_packages(active, &candidate, revision)?)
            }
            _ => None,
        };
        Ok(NormativePackagePreview {
            project_revision: project.as_ref().map(|project| project.revision),
            active_hash: active.map(|active| active.hash.clone()),
            candidate,
            impact,
        })
    }

    pub fn register_norm_reference(
        &self,
        expected_revision: u64,
        reference: NormReferenceInput,
    ) -> Result<Project, AppError> {
        self.execute_single(Command::RegisterNormReference {
            expected_revision,
            reference,
        })
    }

    pub fn pin_normative_package(
        &self,
        expected_revision: u64,
        package_hash: &str,
    ) -> Result<Project, AppError> {
        self.execute_single(Command::PinNormativePackage {
            expected_revision,
            package_hash: package_hash.into(),
        })
    }

    pub fn migrate_normative_package(
        &self,
        expected_revision: u64,
        from_package_hash: &str,
        to_package_hash: &str,
        confirm: bool,
    ) -> Result<Project, AppError> {
        self.execute_single(Command::MigrateNormativePackage {
            expected_revision,
            from_package_hash: from_package_hash.into(),
            to_package_hash: to_package_hash.into(),
            confirm,
        })
    }

    pub fn final_result_gate(&self, capability_id: &str) -> Result<GateReport, AppError> {
        let project = self.read()?;
        let active = project.normative.active.as_ref();
        let capability = active.and_then(|package| {
            package
                .catalog
                .capabilities
                .iter()
                .find(|capability| capability.id == capability_id)
        });
        let context = FinalResultContext {
            project_revision: project.revision,
            method: capability
                .map(|capability| capability.method.clone())
                .unwrap_or_default(),
            material: capability
                .map(|capability| capability.material.clone())
                .unwrap_or_default(),
            recalculation_required: project.normative.recalculation_required,
            // Only a future qualified calculation pipeline may populate these fields.
            result_revision: None,
            result_package_hash: None,
            calculation_evidence: None,
        };
        let mut gate = evaluate_final_result(active, capability_id, &context);
        if let Some(active) = active
            && NormativePackage::from_catalog(&catalog_for(&project)?)?.hash != active.hash
        {
            gate.reasons.push(reason("pending_normative_migration", "Há uma versão candidata do registro; a migração ainda precisa de escolha explícita."));
            gate.allowed = false;
        }
        Ok(gate)
    }

    fn require_write(&self) -> Result<(), AppError> {
        self.grant.check_scope(AccessScope::Read)?;
        self.grant.check_scope(AccessScope::Model)?;
        if self.access == AccessMode::ReadOnly {
            Err(AppError::PermissionDenied)
        } else {
            Ok(())
        }
    }

    fn change(&self, revision: u64, operation: &str) -> Result<ChangeRecord, AppError> {
        let timestamp_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                AppError::InvalidInput("O relógio do sistema está anterior a 1970.".into())
            })?
            .as_millis()
            .try_into()
            .map_err(|_| AppError::InvalidInput("Data fora do intervalo suportado.".into()))?;
        Ok(ChangeRecord {
            revision,
            actor: self.actor.clone(),
            operation: operation.into(),
            timestamp_unix_ms,
        })
    }

    fn lock(&self) -> Result<File, AppError> {
        reject_non_regular(&self.lock_path)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)?;
        file.lock()?;
        Ok(file)
    }
}

fn reject_non_regular(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => Err(AppError::InvalidInput(
            "O projeto e seu bloqueio devem ser arquivos regulares, sem links simbólicos.".into(),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn invalid_input(error: DomainError) -> AppError {
    AppError::InvalidInput(error.to_string())
}

fn catalog_for(project: &Project) -> Result<RegistryCatalog, AppError> {
    match &project.normative.catalog {
        Some(catalog) => Ok(catalog.clone()),
        None => Ok(bundled_catalog()?),
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Storage(#[from] prumo_storage::StorageError),
    #[error("O diagnóstico do arquivo mudou. Atualize a consulta antes de confirmar.")]
    StorageConflict,
    #[error(
        "Há dados de trabalho pendentes ou incompatíveis. Confira o diagnóstico e escolha a revisão a recuperar."
    )]
    RecoveryRequired,
    #[error("{0}")]
    Component(#[from] prumo_components::process::ProcessError),
    #[error("O pacote normativo mudou. Atualize a consulta e confira a versão antes de confirmar.")]
    PackageConflict,
    #[error("O projeto já possui um pacote fixado. Use a migração explícita para alterá-lo.")]
    PackageAlreadyPinned,
    #[error("O projeto ainda não possui um pacote normativo fixado.")]
    PackageNotPinned,
    #[error("Confirme explicitamente a migração e a necessidade de recálculo.")]
    ConfirmationRequired,
    #[error("{0}")]
    Registry(#[from] RegistryError),
    #[error("A sessão não tem autorização para esta operação, projeto ou disciplina.")]
    PermissionDenied,
    #[error("A capacidade ainda não está disponível: {0}")]
    CapabilityUnavailable(String),
    #[error("Já existe um projeto no arquivo selecionado. Abra-o ou selecione outro arquivo.")]
    AlreadyExists,
    #[error("O projeto selecionado ainda não existe.")]
    NotFound,
    #[error("A revisão mudou de {expected} para {actual}. Atualize o projeto antes de alterar.")]
    Conflict { expected: u64, actual: u64 },
    #[error("A chave de idempotência já identifica outro pedido ou ator.")]
    IdempotencyConflict,
    #[error("O plano de alteração não corresponde à prévia autorizada. Gere uma nova prévia.")]
    PlanConflict,
    #[error("A pré-condição do elemento mudou. Atualize o modelo antes de alterar.")]
    PreconditionFailed,
    #[error(
        "O histórico mudou; somente a última operação disponível pode ser desfeita ou refeita."
    )]
    HistoryConflict,
    #[error("{0}")]
    InvalidInput(String),
    #[error("A versão {found} do projeto não é suportada. O arquivo foi preservado.")]
    UnsupportedSchema { found: u64 },
    #[error("A versão declarada do projeto ({value}) não é suportada. O arquivo foi preservado.")]
    UnsupportedSchemaValue { value: String },
    #[error("Formato JSON incompatível: {0}")]
    UnsupportedRepresentation(String),
    #[error("Projeto inválido: {0}")]
    InvalidProject(String),
    #[error("Não foi possível acessar o arquivo de projeto: {0}")]
    Io(#[from] std::io::Error),
    #[error("O arquivo não contém um projeto válido: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl AppError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Storage(
                prumo_storage::StorageError::UnsupportedVersion { .. }
                | prumo_storage::StorageError::UnsupportedVersionValue { .. }
                | prumo_storage::StorageError::UnsupportedRepresentation(_),
            )
            | Self::UnsupportedRepresentation(_) => "unsupported_storage",
            Self::Storage(_) => "invalid_storage",
            Self::StorageConflict => "storage_conflict",
            Self::RecoveryRequired => "recovery_required",
            Self::Component(_) => "component_qualification_failed",
            Self::PackageConflict => "package_conflict",
            Self::PackageAlreadyPinned => "package_already_pinned",
            Self::PackageNotPinned => "package_not_pinned",
            Self::ConfirmationRequired => "confirmation_required",
            Self::Registry(_) => "invalid_normative_registry",
            Self::PermissionDenied => "permission_denied",
            Self::CapabilityUnavailable(_) => "unsupported_capability",
            Self::AlreadyExists => "project_exists",
            Self::NotFound => "project_not_found",
            Self::Conflict { .. } => "revision_conflict",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::PlanConflict => "plan_conflict",
            Self::PreconditionFailed => "precondition_failed",
            Self::HistoryConflict => "history_conflict",
            Self::InvalidInput(_) => "invalid_input",
            Self::UnsupportedSchema { .. } | Self::UnsupportedSchemaValue { .. } => {
                "unsupported_schema"
            }
            Self::InvalidProject(_) | Self::Serialization(_) => "invalid_project",
            Self::Io(_) => "io_error",
        }
    }
}
