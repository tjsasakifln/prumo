//! Read-only model inspection and session navigation used by both UI and MCP.
//! View changes never publish a technical revision or revive stale evidence.
use crate::{AppError, ProjectService, invalid_input};
use prumo_domain::view::{ViewCommand, ViewReport};
use serde::Serialize;
use std::io::Write;

#[derive(Clone, Debug, Serialize)]
pub struct WebViewExport {
    pub project_id: uuid::Uuid,
    pub revision: u64,
    pub file_name: String,
    pub content_sha256: String,
    pub snapshot_sha256: String,
}

fn report(
    project: &prumo_domain::Project,
    value: impl Serialize,
) -> Result<serde_json::Value, AppError> {
    let mut value = serde_json::to_value(value)?;
    let snapshot = prumo_domain::canonical::model_snapshot(project).map_err(invalid_input)?;
    value["project_id"] = serde_json::json!(project.id);
    value["revision"] = serde_json::json!(project.revision);
    value["snapshot_sha256"] = serde_json::json!(snapshot.sha256);
    Ok(value)
}

impl ProjectService {
    /// A host may bind a view to the exact file shown in its editor. Check that
    /// basis on the very same immutable read used below, so a concurrent change
    /// cannot be substituted between a preliminary check and the actual query.
    fn visible_project(&self) -> Result<prumo_domain::Project, AppError> {
        self.grant.check_scope(crate::AccessScope::Read)?;
        let file = self.load_primary()?;
        self.grant.check_full_read(&file.project)?;
        self.check_write_basis(&file)?;
        Ok(file.project)
    }

    /// The trusted host chooses the project directory. Neither UI nor MCP can
    /// choose an output path or replace an existing unrelated file.
    pub fn export_web_view(&self) -> Result<WebViewExport, AppError> {
        self.grant().check_scope(crate::AccessScope::Model)?;
        let project = self.visible_project()?;
        let bytes = serde_json::to_vec_pretty(&project)?;
        if bytes.len() > 32 * 1024 * 1024 {
            return Err(AppError::InvalidInput(
                "A cópia excede o limite de 32 MiB do visualizador web. O projeto foi preservado."
                    .into(),
            ));
        }
        let content_sha256 = prumo_domain::canonical::hash_bytes(&bytes);
        let snapshot = prumo_domain::canonical::model_snapshot(&project).map_err(invalid_input)?;
        let stem: String = self
            .path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .take(40)
            .collect();
        let file_name = format!(
            "{stem}-rev-{}-{}.view.json",
            project.revision,
            &content_sha256[..16]
        );
        let output = self.path.with_file_name(&file_name);
        crate::reject_non_regular(&output)?;
        if output.exists() {
            if std::fs::read(&output)? != bytes {
                return Err(AppError::InvalidInput(
                    "Já existe outro conteúdo com o nome da exportação. O arquivo foi preservado."
                        .into(),
                ));
            }
        } else {
            let parent = output.parent().ok_or(AppError::NotFound)?;
            let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
            temporary.write_all(&bytes)?;
            temporary.as_file().sync_all()?;
            temporary
                .persist_noclobber(&output)
                .map_err(|error| error.error)?;
        }
        Ok(WebViewExport {
            project_id: project.id,
            revision: project.revision,
            file_name,
            content_sha256,
            snapshot_sha256: snapshot.sha256,
        })
    }

    pub fn framing_inspection(&self, object_id: uuid::Uuid) -> Result<serde_json::Value, AppError> {
        let project = self.read()?;
        report(
            &project,
            prumo_domain::framing::inspect(&project, object_id).map_err(invalid_input)?,
        )
    }

    pub fn surface_inspection(&self, object_id: uuid::Uuid) -> Result<serde_json::Value, AppError> {
        let project = self.read()?;
        report(
            &project,
            prumo_domain::surfaces::inspect(&project, object_id).map_err(invalid_input)?,
        )
    }

    pub fn surface_preview(
        &self,
        spec: &prumo_domain::surfaces::SurfaceSpec,
    ) -> Result<serde_json::Value, AppError> {
        let project = self.read()?;
        report(
            &project,
            prumo_domain::surfaces::preview(&project, spec).map_err(invalid_input)?,
        )
    }

    pub fn assembly_inspection(
        &self,
        object_id: uuid::Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let project = self.read()?;
        report(
            &project,
            prumo_domain::assemblies::inspect(&project, object_id).map_err(invalid_input)?,
        )
    }

    pub fn assembly_preview(
        &self,
        spec: &prumo_domain::assemblies::AssemblySpec,
    ) -> Result<serde_json::Value, AppError> {
        let project = self.read()?;
        report(
            &project,
            prumo_domain::assemblies::preview(&project, spec).map_err(invalid_input)?,
        )
    }

    pub fn view_report(&self) -> Result<ViewReport, AppError> {
        let project = self.visible_project()?;
        let mut state = self.view_state.lock().map_err(|_| {
            AppError::InvalidInput(
                "A sessão de navegação não está disponível. Reabra o projeto.".into(),
            )
        })?;
        state.synchronize(&project);
        state.report(&project).map_err(invalid_input)
    }

    pub fn view_command(&self, command: ViewCommand) -> Result<ViewReport, AppError> {
        let project = self.visible_project()?;
        self.view_state
            .lock()
            .map_err(|_| {
                AppError::InvalidInput(
                    "A sessão de navegação não está disponível. Reabra o projeto.".into(),
                )
            })?
            .apply(&project, command)
            .map_err(invalid_input)
    }

    pub fn layout_view(&self) -> Result<serde_json::Value, AppError> {
        let project = self.read()?;
        let layout = prumo_domain::layout::state(&project).map_err(invalid_input)?;
        let snapshot = prumo_domain::canonical::model_snapshot(&project).map_err(invalid_input)?;
        Ok(
            serde_json::json!({"project_id":project.id,"revision":project.revision,
            "snapshot_sha256":snapshot.sha256,"layout":layout,"length_unit":"m"}),
        )
    }

    pub fn layout_measure(&self, expression: &str) -> Result<serde_json::Value, AppError> {
        self.read()?;
        let value = prumo_domain::layout::parse_length(expression).map_err(invalid_input)?;
        Ok(serde_json::json!({"expression":expression,"value_m":value,"length_unit":"m"}))
    }

    pub fn layout_snap(
        &self,
        request: &prumo_domain::layout::SnapRequest,
    ) -> Result<prumo_domain::layout::SnapProposal, AppError> {
        prumo_domain::layout::propose_snap(&self.read()?, request).map_err(invalid_input)
    }
}
