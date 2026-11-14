#![forbid(unsafe_code)]

//! Project-scoped command service shared by desktop, CLI, and MCP.

pub use prumo_domain::Command;
use prumo_domain::{
    ChangeRecord, Discipline, DomainError, Environment, LengthUnit, ModelState,
    PROJECT_SCHEMA_VERSION, PhysicalObject, Project, validate_name,
};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
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
                analytical_id: Uuid::new_v4(),
                name,
                discipline: Discipline::Structure,
                start_m: project.units.to_metres(start).map_err(invalid_input)?,
                end_m: project.units.to_metres(end).map_err(invalid_input)?,
                state: ModelState::Modelled,
                extensions: BTreeMap::new(),
            };
            project.objects.push(object);
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
            project.environments.push(Environment {
                id: Uuid::new_v4(),
                name,
                extensions: BTreeMap::new(),
            });
            Ok(())
        })
    }

    fn mutate(
        &self,
        expected_revision: u64,
        operation: &str,
        mutation: impl FnOnce(&mut Project) -> Result<(), AppError>,
    ) -> Result<Project, AppError> {
        self.require_write()?;
        let _lock = self.lock()?;
        let mut project = self.read_snapshot()?;
        if project.revision != expected_revision {
            return Err(AppError::Conflict {
                expected: expected_revision,
                actual: project.revision,
            });
        }
        mutation(&mut project)?;
        project.revision = project
            .revision
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidInput("Limite de revisões atingido.".into()))?;
        project
            .history
            .push(self.change(project.revision, operation)?);
        project.validate().map_err(invalid_input)?;
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
            && version != u64::from(PROJECT_SCHEMA_VERSION)
        {
            return Err(AppError::UnsupportedSchema { found: version });
        }
        let project: Project = serde_json::from_value(value)?;
        project
            .validate()
            .map_err(|error| AppError::InvalidProject(error.to_string()))?;
        Ok(project)
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

#[derive(Debug, Error)]
pub enum AppError {
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
