#![forbid(unsafe_code)]

//! Project-scoped command service shared by desktop, CLI, and MCP.

pub use prumo_domain::Command;
use prumo_domain::canonical::{ModelSnapshot, source_index_unchecked};
use prumo_domain::components::{ComponentDescriptor, ExperimentBasis, QualificationReport};
use prumo_domain::dependencies::{DependencyGraph, SourceKind};
use prumo_domain::model::{ModelEdit, PhysicalRole, PointInput, SemanticModel};
use prumo_domain::{
    ChangeRecord, Discipline, DomainError, Environment, LengthUnit, ModelState,
    PROJECT_SCHEMA_VERSION, PhysicalObject, Project, validate_name,
};
use prumo_registry::{
    CoverageAudit, FinalResultContext, GateReport, NormReferenceInput, NormativePackage,
    NormativePackagePreview, NormativeState, RegistryCatalog, RegistryError, audit_coverage,
    bundled_catalog, compare_packages, evaluate_final_result, reason, register_reference,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
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
}

impl ProjectService {
    pub fn model_snapshot(&self) -> Result<ModelSnapshot, AppError> {
        prumo_domain::canonical::model_snapshot(&self.read()?).map_err(invalid_input)
    }

    pub fn dependency_report(&self) -> Result<DependencyGraph, AppError> {
        Ok(self.read()?.dependencies)
    }

    pub fn edit_model(&self, expected_revision: u64, edit: ModelEdit) -> Result<Project, AppError> {
        self.require_write()?;
        let tag = serde_json::to_value(&edit)?
            .get("edit")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::InvalidInput("Edição sem operação declarada.".into()))?
            .to_owned();
        self.mutate(expected_revision, &format!("model.{tag}"), |project| {
            prumo_domain::model::apply_model_edit(project, edit, &self.actor)
                .map_err(invalid_input)?;
            Ok(())
        })
    }

    pub fn inspect_model(&self, expected_revision: u64) -> Result<Project, AppError> {
        self.mutate(expected_revision, "inspect_model", |project| {
            prumo_domain::dependencies::inspect_model(project).map_err(invalid_input)
        })
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
        })
    }

    pub fn project_path(&self) -> &Path {
        &self.path
    }

    pub const fn access_mode(&self) -> AccessMode {
        self.access
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
        self.save_locked(&project)?;
        Ok(project)
    }

    pub fn read(&self) -> Result<Project, AppError> {
        // Writers publish a complete immutable file with an atomic replacement.
        // Opening that file gives one complete snapshot, including during a save.
        // A query must also work when the project directory is not writable.
        self.read_snapshot()
    }

    /// Coordinates use the project's declared display unit; storage is always in metres.
    pub fn add_member(
        &self,
        expected_revision: u64,
        name: impl Into<String>,
        start: [f64; 3],
        end: [f64; 3],
    ) -> Result<Project, AppError> {
        self.require_write()?;
        let name = name.into();
        validate_name(&name).map_err(invalid_input)?;
        self.mutate(expected_revision, "add_member", |project| {
            let object = PhysicalObject {
                id: Uuid::new_v4(),
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
                return Err(invalid_input(DomainError::DegenerateMember));
            }
            let physical_id = object.id;
            project.objects.push(object);
            prumo_domain::model::initialize_legacy_member(
                project,
                physical_id,
                None,
                &self.actor,
                project
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| AppError::InvalidInput("Limite de revisões atingido.".into()))?,
            )
            .map_err(invalid_input)?;
            Ok(())
        })
    }

    pub fn rename_project(
        &self,
        expected_revision: u64,
        name: impl Into<String>,
    ) -> Result<Project, AppError> {
        self.require_write()?;
        let name = name.into();
        validate_name(&name).map_err(invalid_input)?;
        self.mutate(expected_revision, "rename_project", |project| {
            project.name = name;
            Ok(())
        })
    }

    pub fn add_environment(
        &self,
        expected_revision: u64,
        name: impl Into<String>,
    ) -> Result<Project, AppError> {
        self.require_write()?;
        let name = name.into();
        validate_name(&name).map_err(invalid_input)?;
        self.mutate(expected_revision, "add_environment", |project| {
            let id = Uuid::new_v4();
            project.environments.push(Environment {
                id,
                name,
                extensions: BTreeMap::new(),
            });
            prumo_domain::model::record_identity(
                project,
                id,
                &self.actor,
                project
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| AppError::InvalidInput("Limite de revisões atingido.".into()))?,
                true,
            )
            .map_err(invalid_input)?;
            Ok(())
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
        self.require_write()?;
        self.mutate(expected_revision, "register_norm_reference", |project| {
            let catalog = catalog_for(project)?;
            let next_revision = project
                .revision
                .checked_add(1)
                .ok_or_else(|| AppError::InvalidInput("Limite de revisões atingido.".into()))?;
            let version = format!("project:{}:revision:{next_revision}", project.id);
            let (updated, change) =
                register_reference(&catalog, reference, version, next_revision)?;
            project.normative.catalog = Some(updated);
            project.normative.registry_changes.push(change);
            if project.normative.active.is_some() {
                project.normative.recalculation_required = true;
            }
            Ok(())
        })
    }

    pub fn pin_normative_package(
        &self,
        expected_revision: u64,
        package_hash: &str,
    ) -> Result<Project, AppError> {
        self.require_write()?;
        self.mutate(expected_revision, "pin_normative_package", |project| {
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
            Ok(())
        })
    }

    pub fn migrate_normative_package(
        &self,
        expected_revision: u64,
        from_package_hash: &str,
        to_package_hash: &str,
        confirm: bool,
    ) -> Result<Project, AppError> {
        self.require_write()?;
        if !confirm {
            return Err(AppError::ConfirmationRequired);
        }
        self.mutate(expected_revision, "migrate_normative_package", |project| {
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
            let next_revision = project
                .revision
                .checked_add(1)
                .ok_or_else(|| AppError::InvalidInput("Limite de revisões atingido.".into()))?;
            let impact = compare_packages(original, &candidate, next_revision)?;
            project.normative.previous.push(original.clone());
            project.normative.migrations.push(impact);
            project.normative.active = Some(candidate);
            project.normative.recalculation_required = true;
            Ok(())
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

    fn mutate(
        &self,
        expected_revision: u64,
        operation: &str,
        mutation: impl FnOnce(&mut Project) -> Result<(), AppError>,
    ) -> Result<Project, AppError> {
        self.require_write()?;
        let _lock = self.lock()?;
        let (mut project, legacy_bytes) = self.read_snapshot_with_origin()?;
        if project.revision != expected_revision {
            return Err(AppError::Conflict {
                expected: expected_revision,
                actual: project.revision,
            });
        }
        let before = source_index_unchecked(&project).map_err(invalid_input)?;
        let next_revision = project
            .revision
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidInput("Limite de revisões atingido.".into()))?;
        mutation(&mut project)?;
        let after = source_index_unchecked(&project).map_err(invalid_input)?;
        project
            .dependencies
            .invalidate_changes(&before, &after, next_revision, &self.actor, operation)
            .map_err(invalid_input)?;
        if project.normative.active.is_some()
            && before.keys().chain(after.keys()).any(|key| {
                key.kind != SourceKind::ProjectMetadata && before.get(key) != after.get(key)
            })
        {
            project.normative.recalculation_required = true;
        }
        project.revision = next_revision;
        project
            .history
            .push(self.change(project.revision, operation)?);
        project.validate().map_err(invalid_input)?;
        if let Some(bytes) = legacy_bytes {
            self.preserve_legacy(&bytes)?;
        }
        self.save_locked(&project)?;
        Ok(project)
    }

    fn require_write(&self) -> Result<(), AppError> {
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

    fn read_snapshot(&self) -> Result<Project, AppError> {
        self.read_snapshot_with_origin().map(|(project, _)| project)
    }

    fn read_snapshot_with_origin(&self) -> Result<(Project, Option<Vec<u8>>), AppError> {
        reject_non_regular(&self.path)?;
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(AppError::NotFound);
            }
            Err(error) => return Err(error.into()),
        };
        if file.metadata()?.len() > MAX_PROJECT_BYTES {
            return Err(AppError::InvalidProject(
                "Arquivo maior que o limite de 16 MiB.".into(),
            ));
        }
        let mut bytes = Vec::new();
        file.take(MAX_PROJECT_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_PROJECT_BYTES {
            return Err(AppError::InvalidProject(
                "Arquivo maior que o limite de 16 MiB.".into(),
            ));
        }
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        if let Some(version) = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            && version != 1
            && version != u64::from(PROJECT_SCHEMA_VERSION)
        {
            return Err(AppError::UnsupportedSchema { found: version });
        }
        let legacy = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            == Some(1);
        let project = prumo_domain::model::decode_project(value)
            .map_err(|error| AppError::InvalidProject(error.to_string()))?;
        Ok((project, legacy.then_some(bytes)))
    }

    fn preserve_legacy(&self, bytes: &[u8]) -> Result<(), AppError> {
        let destination = self.path.with_file_name(format!(
            "prumo-v1-{}.backup",
            prumo_domain::canonical::hash_bytes(bytes)
        ));
        reject_non_regular(&destination)?;
        let parent = destination
            .parent()
            .ok_or_else(|| AppError::InvalidInput("Pasta do projeto ausente.".into()))?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(bytes)?;
        temporary.as_file().sync_all()?;
        match temporary.persist_noclobber(&destination) {
            Ok(_) => {}
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                reject_non_regular(&destination)?;
                let file = File::open(&destination)?;
                if file.metadata()?.len() != bytes.len() as u64 {
                    return Err(AppError::InvalidProject(
                        "O backup de migração existente diverge do original.".into(),
                    ));
                }
                let mut existing = Vec::new();
                file.take(bytes.len() as u64 + 1)
                    .read_to_end(&mut existing)?;
                if existing != bytes {
                    return Err(AppError::InvalidProject(
                        "O backup de migração existente diverge do original.".into(),
                    ));
                }
            }
            Err(error) => return Err(error.error.into()),
        }
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        Ok(())
    }

    fn save_locked(&self, project: &Project) -> Result<(), AppError> {
        project.validate().map_err(invalid_input)?;
        reject_non_regular(&self.path)?;
        let bytes = serde_json::to_vec_pretty(project)?;
        if bytes.len() as u64 > MAX_PROJECT_BYTES {
            return Err(AppError::InvalidInput(
                "O projeto excede o limite de 16 MiB desta versão.".into(),
            ));
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| AppError::InvalidInput("Pasta do projeto ausente.".into()))?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        temporary
            .persist(&self.path)
            .map_err(|error| AppError::Io(error.error))?;
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        Ok(())
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
    #[error("A sessão permite apenas consultar o projeto.")]
    PermissionDenied,
    #[error("Já existe um projeto no arquivo selecionado. Abra-o ou selecione outro arquivo.")]
    AlreadyExists,
    #[error("O projeto selecionado ainda não existe.")]
    NotFound,
    #[error("A revisão mudou de {expected} para {actual}. Atualize o projeto antes de alterar.")]
    Conflict { expected: u64, actual: u64 },
    #[error("{0}")]
    InvalidInput(String),
    #[error("A versão {found} do projeto não é suportada. O arquivo foi preservado.")]
    UnsupportedSchema { found: u64 },
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
            Self::Component(_) => "component_qualification_failed",
            Self::PackageConflict => "package_conflict",
            Self::PackageAlreadyPinned => "package_already_pinned",
            Self::PackageNotPinned => "package_not_pinned",
            Self::ConfirmationRequired => "confirmation_required",
            Self::Registry(_) => "invalid_normative_registry",
            Self::PermissionDenied => "permission_denied",
            Self::AlreadyExists => "project_exists",
            Self::NotFound => "project_not_found",
            Self::Conflict { .. } => "revision_conflict",
            Self::InvalidInput(_) => "invalid_input",
            Self::UnsupportedSchema { .. } => "unsupported_schema",
            Self::InvalidProject(_) | Self::Serialization(_) => "invalid_project",
            Self::Io(_) => "io_error",
        }
    }
}
