//! Read operations for the same geometry and verification used by all clients.
use crate::{AppError, ProjectService, access::AccessScope, invalid_input};
use prumo_domain::catalogue::{
    self, CatalogEntry, CatalogPayload, MaterialState, PhysicalQuantity, PhysicalValue,
    ProjectCatalogue, SectionAnalysis,
};
use prumo_domain::components::ExperimentBasis;
use prumo_domain::geometry::{
    self, FaceInput, GeometryFilter, GeometryRepair, GeometryRepairPreview, GeometryReport,
    ProjectFaceInspection,
};
use prumo_verification::{PreparedProtocol, RunContext, SuiteReport};
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Serialize)]
pub struct CatalogueView {
    pub project_id: Uuid,
    pub revision: u64,
    pub snapshot_sha256: String,
    pub catalogue: ProjectCatalogue,
}

#[derive(Serialize)]
pub struct CatalogueSectionReport {
    pub project_id: Uuid,
    pub revision: u64,
    pub snapshot_sha256: String,
    pub entry: CatalogEntry,
    pub entry_sha256: String,
    pub analysis: SectionAnalysis,
}

#[derive(Serialize)]
pub struct CatalogueMaterialReport {
    pub project_id: Uuid,
    pub revision: u64,
    pub snapshot_sha256: String,
    pub entry: CatalogEntry,
    pub entry_sha256: String,
    pub conditions: BTreeMap<PhysicalQuantity, PhysicalValue>,
    pub state: MaterialState,
    pub engineering_release: bool,
}

impl ProjectService {
    /// Read-only, size-bounded experiment on explicitly supplied input. No
    /// force, stiffness or load is silently extracted from the open BIM model.
    pub fn analysis_trial(
        &self,
        input: serde_json::Value,
        cancelled: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<serde_json::Value, AppError> {
        let project = self.read()?;
        let bytes = serde_json::to_vec(&input)?;
        if bytes.len() > 64 * 1024 {
            return Err(AppError::InvalidInput(
                "O ensaio excede o limite de 64 KiB de entrada.".into(),
            ));
        }
        let request: prumo_analysis::TrialRequest = serde_json::from_value(input)
            .map_err(|error| AppError::InvalidInput(format!(
                "Contrato do ensaio inválido: {error}"
            )))?;
        let snapshot = prumo_domain::canonical::model_snapshot(&project).map_err(invalid_input)?;
        let evidence = prumo_analysis::trial(&request, cancelled)
            .map_err(|error| AppError::InvalidInput(error.to_string()))?;
        Ok(serde_json::json!({
            "project_id":project.id,"revision":project.revision,
            "snapshot_sha256":snapshot.sha256,
            "input_sha256":prumo_domain::canonical::hash_bytes(&bytes),
            "kernel":"prumo-analysis-0.1",
            "purpose":"ensaios de matrizes, barras, malhas, vínculos e balanço",
            "engineering_release":false,
            "result":evidence
        }))
    }

    pub fn precalculation_state(&self) -> Result<serde_json::Value, AppError> {
        let project = self.read()?;
        let snapshot = prumo_domain::canonical::model_snapshot(&project).map_err(invalid_input)?;
        Ok(serde_json::json!({"project_id":project.id,"revision":project.revision,
            "snapshot_sha256":snapshot.sha256,"state":prumo_domain::precalculation::state(&project).map_err(invalid_input)?}))
    }

    pub fn precalculation_review(&self) -> Result<serde_json::Value, AppError> {
        let project = self.read()?;
        let snapshot = prumo_domain::canonical::model_snapshot(&project).map_err(invalid_input)?;
        let report = prumo_domain::precalculation::review(&project).map_err(invalid_input)?;
        Ok(serde_json::json!({"snapshot_sha256":snapshot.sha256,"review":report}))
    }

    pub fn catalogue_view(&self) -> Result<CatalogueView, AppError> {
        let project = self.read()?;
        let snapshot = prumo_domain::canonical::model_snapshot(&project).map_err(invalid_input)?;
        Ok(CatalogueView {
            project_id: project.id,
            revision: project.revision,
            snapshot_sha256: snapshot.sha256,
            catalogue: catalogue::project_catalogue(&project).map_err(invalid_input)?,
        })
    }

    pub fn catalogue_section(&self, entry_id: Uuid) -> Result<CatalogueSectionReport, AppError> {
        self.grant().check_scope(AccessScope::Calculation)?;
        let view = self.catalogue_view()?;
        let entry = view
            .catalogue
            .latest(entry_id)
            .map_err(invalid_input)?
            .clone();
        let CatalogPayload::Section { shape } = &entry.payload else {
            return Err(AppError::InvalidInput(
                "A entrada selecionada não é uma seção.".into(),
            ));
        };
        let analysis = catalogue::analyse_section(shape).map_err(invalid_input)?;
        Ok(CatalogueSectionReport {
            project_id: view.project_id,
            revision: view.revision,
            snapshot_sha256: view.snapshot_sha256,
            entry_sha256: entry.sha256().map_err(invalid_input)?,
            entry,
            analysis,
        })
    }

    pub fn catalogue_material(
        &self,
        entry_id: Uuid,
        conditions: &BTreeMap<PhysicalQuantity, PhysicalValue>,
    ) -> Result<CatalogueMaterialReport, AppError> {
        let view = self.catalogue_view()?;
        let entry = view
            .catalogue
            .latest(entry_id)
            .map_err(invalid_input)?
            .clone();
        for (quantity, value) in conditions {
            value.require(*quantity).map_err(invalid_input)?;
        }
        let state = entry
            .material_state_for(conditions)
            .map_err(invalid_input)?
            .clone();
        Ok(CatalogueMaterialReport {
            project_id: view.project_id,
            revision: view.revision,
            snapshot_sha256: view.snapshot_sha256,
            entry_sha256: entry.sha256().map_err(invalid_input)?,
            entry,
            conditions: conditions.clone(),
            state,
            engineering_release: false,
        })
    }

    pub fn geometry_report(&self, filter: &GeometryFilter) -> Result<GeometryReport, AppError> {
        geometry::audit_project(&self.read()?, filter).map_err(invalid_input)
    }

    pub fn geometry_repair_preview(
        &self,
        repairs: &[GeometryRepair],
    ) -> Result<GeometryRepairPreview, AppError> {
        self.grant().check_scope(AccessScope::Model)?;
        geometry::preview_repairs(&self.read()?, repairs, &self.actor).map_err(invalid_input)
    }

    pub fn inspect_geometry_face(
        &self,
        face: &FaceInput,
    ) -> Result<ProjectFaceInspection, AppError> {
        let project = self.read()?;
        let tolerances = geometry::project_tolerances(&project).map_err(invalid_input)?;
        let snapshot = prumo_domain::canonical::model_snapshot(&project).map_err(invalid_input)?;
        Ok(ProjectFaceInspection {
            project_id: project.id,
            revision: project.revision,
            snapshot_sha256: snapshot.sha256,
            tolerances,
            face: face.clone(),
            inspection: geometry::inspect_face(face, tolerances).map_err(invalid_input)?,
        })
    }

    pub fn verification_protocols(&self) -> Result<Vec<PreparedProtocol>, AppError> {
        self.read()?;
        prumo_verification::reference_protocols().map_err(|e| AppError::InvalidInput(e.to_string()))
    }

    pub fn run_verification_suite(&self) -> Result<SuiteReport, AppError> {
        self.grant().check_scope(AccessScope::Calculation)?;
        let snapshot = self.model_snapshot()?;
        let context = RunContext::capture(Some(ExperimentBasis {
            project_id: snapshot.project_id,
            revision: snapshot.source_revision,
            content_sha256: snapshot.sha256,
        }))
        .map_err(|e| AppError::InvalidInput(e.to_string()))?;
        prumo_verification::run_reference_suite(context)
            .map_err(|e| AppError::InvalidInput(e.to_string()))
    }
}
