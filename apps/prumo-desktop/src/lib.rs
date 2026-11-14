#![forbid(unsafe_code)]
use prumo_app::{AccessMode, ProjectService};
use prumo_domain::Project;
use prumo_domain::components::QualificationReport;
use prumo_domain::storage::{StorageAction, StorageState};
use prumo_ui::{ProjectView, UiAction};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};

struct QualificationJob {
    cancel: Arc<AtomicBool>,
    receiver: mpsc::Receiver<Result<QualificationReport, prumo_app::AppError>>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for QualificationJob {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub struct Desktop {
    pub view: ProjectView,
    pub project: Option<Project>,
    project_basis_sha256: Option<String>,
    service: Option<ProjectService>,
    qualification: Option<QualificationJob>,
}

impl Desktop {
    pub fn new(path: PathBuf) -> Self {
        let view = ProjectView {
            path: path.to_string_lossy().into_owned(),
            ..Default::default()
        };
        let mut app = Self {
            view,
            project: None,
            project_basis_sha256: None,
            service: None,
            qualification: None,
        };
        app.select(path, false);
        app
    }

    fn select(&mut self, path: PathBuf, report_missing: bool) {
        self.qualification = None;
        self.view.components = Default::default();
        self.view.registry = Default::default();
        self.view.model = Default::default();
        self.view.storage = Default::default();
        self.project_basis_sha256 = None;
        let selected_path = path.to_string_lossy().into_owned();
        self.view.path = selected_path.clone();
        self.view.selected_path.clear();
        match ProjectService::open(path, AccessMode::ReadWrite, "interface-local") {
            Ok(service) => {
                match service.read_with_basis() {
                    Ok((project, hash)) => {
                        self.view.project_name = project.name.clone();
                        self.view.report(
                            format!("Projeto aberto. Revisão {}.", project.revision),
                            false,
                        );
                        self.project = Some(project);
                        self.project_basis_sha256 = Some(hash);
                    }
                    Err(error) => {
                        self.project = None;
                        if report_missing || !matches!(error, prumo_app::AppError::NotFound) {
                            self.view.report(error.to_string(), true);
                        }
                    }
                }
                self.service = Some(service);
                self.view.components.inventory = self
                    .service
                    .as_ref()
                    .map(ProjectService::component_inventory)
                    .unwrap_or_default();
                self.view.selected_path = selected_path;
                self.refresh_storage();
                if self.project.is_some() {
                    self.refresh_registry();
                }
            }
            Err(error) => {
                self.service = None;
                self.project = None;
                self.view.report(error.to_string(), true);
            }
        }
    }

    pub fn render(&mut self, ui: &mut eframe::egui::Ui) {
        self.poll_qualification();
        if self.qualification.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        let actions = self.view.show(ui, self.project.as_ref());
        let changed = !actions.is_empty();
        for action in actions {
            self.dispatch(action);
        }
        if changed {
            ui.ctx().request_repaint();
        }
    }

    pub fn dispatch(&mut self, action: UiAction) {
        match action {
            UiAction::RunQualification => self.start_qualification(),
            UiAction::CancelQualification => {
                if let Some(job) = &self.qualification {
                    job.cancel.store(true, Ordering::Release);
                }
            }
            UiAction::RefreshRegistry => self.refresh_registry(),
            UiAction::RefreshModel => self.refresh_model(),
            UiAction::RefreshStorage => self.refresh_storage(),
            UiAction::Storage(action) => self.execute_storage(*action),
            UiAction::AssessCapability(id) => {
                if let Some(service) = &self.service {
                    match service.final_result_gate(&id) {
                        Ok(report) => self.view.registry.gate = Some(report),
                        Err(error) => self.view.report(error.to_string(), true),
                    }
                }
            }
            UiAction::Open(path) => self.select(PathBuf::from(path), true),
            UiAction::Refresh => {
                if let Some(service) = &self.service {
                    match service.read_with_basis() {
                        Ok((project, hash)) => {
                            if self
                                .project
                                .as_ref()
                                .is_some_and(|previous| previous != &project)
                            {
                                self.clear_qualification();
                            }
                            if self.project.as_ref().is_some_and(|previous| {
                                previous.id != project.id
                                    || (previous.revision == project.revision
                                        && previous != &project)
                            }) {
                                self.view.registry = Default::default();
                                self.view.model = Default::default();
                            }
                            self.view.project_name = project.name.clone();
                            self.project = Some(project);
                            self.project_basis_sha256 = Some(hash);
                            self.view.model.trace.snapshot = None;
                            self.view.report("Projeto reaberto do disco.", false);
                            self.refresh_registry();
                            self.refresh_storage();
                        }
                        Err(error) => {
                            self.clear_editor();
                            self.refresh_storage();
                            self.view.report(error.to_string(), true);
                        }
                    }
                }
            }
            UiAction::Command(command) => {
                let creating = matches!(&*command, prumo_app::Command::Create { .. });
                if !creating && self.project.is_none() {
                    self.view
                        .report("Abra o projeto antes de alterar o modelo.", true);
                    return;
                }
                if self.project.is_some() && self.view.path != self.view.selected_path {
                    self.view.report(
                        "Abra o arquivo selecionado antes de alterar o projeto.",
                        true,
                    );
                    return;
                }
                if creating && self.project.is_none() {
                    self.select(PathBuf::from(&self.view.path), false);
                }
                if let Some(service) = &self.service {
                    let service = if creating {
                        service.clone()
                    } else {
                        let Some((project, hash)) = self
                            .project
                            .as_ref()
                            .zip(self.project_basis_sha256.as_ref())
                        else {
                            self.view.report(
                                "Reabra o arquivo para conferir a base antes de editar.",
                                true,
                            );
                            return;
                        };
                        match service.with_write_basis(project.id, hash.clone()) {
                            Ok(service) => service,
                            Err(error) => {
                                self.view.report(error.to_string(), true);
                                return;
                            }
                        }
                    };
                    match service.execute(*command) {
                        Ok(project) => {
                            let revision = project.revision;
                            self.clear_qualification();
                            if !self.accept_saved_project(project) {
                                return;
                            }
                            self.view
                                .report(format!("Alteração salva. Revisão {revision}."), false);
                            self.view.model.trace.snapshot = None;
                            self.view.model.error = None;
                            self.refresh_registry();
                            self.refresh_storage();
                        }
                        Err(error) => {
                            self.refresh_storage();
                            self.view.report(error.to_string(), true);
                        }
                    }
                }
            }
        }
    }

    fn clear_editor(&mut self) {
        self.clear_qualification();
        self.project = None;
        self.project_basis_sha256 = None;
        self.view.registry = Default::default();
        self.view.model = Default::default();
    }

    fn accept_saved_project(&mut self, expected: Project) -> bool {
        let message = match self.service.as_ref().map(ProjectService::read_with_basis) {
            Some(Ok((project, hash))) if project == expected => {
                self.view.project_name = project.name.clone();
                self.project = Some(project);
                self.project_basis_sha256 = Some(hash);
                return true;
            }
            Some(Ok(_)) => "O arquivo mudou depois da operação. Reabra para conferir o projeto atual antes de editar.".into(),
            Some(Err(error)) => format!("A operação terminou, mas o arquivo não pôde ser conferido: {error}. Reabra antes de editar."),
            None => "Reabra o arquivo para conferir o resultado da operação antes de editar.".into(),
        };
        self.clear_editor();
        self.refresh_storage();
        self.view.report(message, true);
        false
    }

    fn refresh_registry(&mut self) {
        if let Some(service) = &self.service {
            match (
                service.normative_package_preview(),
                service.coverage_audit(),
            ) {
                (Ok(preview), Ok(audit)) => {
                    if self.project.as_ref().map(|project| project.revision)
                        != preview.project_revision
                    {
                        self.view.report("O projeto mudou em outra instância. Reabra para conferir a revisão atual.",true);
                        return;
                    }
                    self.view.registry.gate = None;
                    self.view.registry.catalog = Some(preview.candidate.catalog.clone());
                    self.view.registry.audit = Some(audit);
                    self.view.registry.candidate = Some(preview.candidate);
                }
                (Err(error), _) | (_, Err(error)) => self.view.report(error.to_string(), true),
            }
        }
    }

    fn refresh_storage(&mut self) {
        let Some(service) = &self.service else {
            return;
        };
        match service.storage_status() {
            Ok(status) => {
                let loaded_identity = self
                    .project
                    .as_ref()
                    .map(|project| (project.id, project.revision));
                let observed_identity = status.project_id.zip(status.revision);
                // The model and its basis came from the same read. A later
                // diagnosis cannot authorize new bytes for that earlier model.
                let changed_file = loaded_identity.is_some()
                    && (loaded_identity != observed_identity
                        || self.project_basis_sha256.as_ref() != status.primary_sha256.as_ref());
                if matches!(
                    status.state,
                    StorageState::Invalid | StorageState::Unsupported | StorageState::Missing
                ) || (status.state == StorageState::RecoveryAvailable
                    && status.project_id.is_none())
                    || changed_file
                {
                    self.clear_editor();
                }
                if changed_file {
                    self.view.report("O arquivo mudou desde a abertura. Reabra para conferir o projeto atual antes de editar.", true);
                }
                self.view.storage.accept(status);
            }
            Err(error) => {
                self.view.storage.clear_confirmations();
                self.view.storage.status = None;
                self.view.storage.expanded = true;
                self.view.report(error.to_string(), true);
            }
        }
    }

    fn execute_storage(&mut self, action: StorageAction) {
        if self.view.path != self.view.selected_path || self.view.selected_path.is_empty() {
            self.view.storage.clear_confirmations();
            self.view.report(
                "Abra o arquivo selecionado antes de alterar seu armazenamento.",
                true,
            );
            return;
        }
        let Some(service) = &self.service else {
            return;
        };
        match service.execute_storage(action) {
            Ok(result) => {
                if self.project.as_ref() != Some(&result.project) {
                    self.clear_qualification();
                    self.view.model = Default::default();
                }
                if !self.accept_saved_project(result.project) {
                    return;
                }
                self.view.report(result.message, false);
                self.refresh_registry();
                self.refresh_storage();
            }
            Err(error) => {
                self.refresh_storage();
                self.view.report(error.to_string(), true);
            }
        }
    }

    fn refresh_model(&mut self) {
        if let Some(service) = &self.service {
            match service.model_snapshot() {
                Ok(snapshot) => {
                    if self
                        .project
                        .as_ref()
                        .map(|project| (project.id, project.revision))
                        != Some((snapshot.project_id, snapshot.source_revision))
                    {
                        self.view.model.trace.snapshot = None;
                        self.view.report("O projeto mudou em outra instância. Reabra para consultar o snapshot atual.", true);
                        return;
                    }
                    self.view.model.trace.snapshot = Some(snapshot);
                    self.view.report(
                        "Snapshot e dependências conferidos na revisão exibida.",
                        false,
                    );
                }
                Err(error) => self.view.report(error.to_string(), true),
            }
        }
    }

    fn start_qualification(&mut self) {
        if self.qualification.is_some() {
            return;
        }
        if self.view.path != self.view.selected_path {
            self.view.report(
                "Abra o arquivo selecionado antes de iniciar a consulta.",
                true,
            );
            return;
        }
        let Some(service) = self.service.clone() else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = thread::spawn(move || {
            let result = service.qualify_components_with_cancel(&worker_cancel);
            let _ = sender.send(result);
        });
        self.view.components.report = None;
        self.view.components.running = true;
        self.qualification = Some(QualificationJob {
            cancel,
            receiver,
            thread: Some(thread),
        });
    }

    fn clear_qualification(&mut self) {
        self.qualification = None;
        self.view.components.running = false;
        self.view.components.report = None;
    }

    pub fn poll_qualification(&mut self) {
        let result = self
            .qualification
            .as_ref()
            .and_then(|job| match job.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err(prumo_app::AppError::InvalidInput(
                        "O processo de qualificação foi encerrado sem resposta.".into(),
                    )))
                }
            });
        if let Some(result) = result {
            self.qualification = None;
            self.view.components.running = false;
            match result {
                Ok(report) => {
                    if report
                        .basis
                        .as_ref()
                        .map(|basis| (basis.project_id, basis.revision))
                        != self
                            .project
                            .as_ref()
                            .map(|project| (project.id, project.revision))
                    {
                        self.view.components.report = None;
                        self.view.report("A revisão do projeto mudou durante os ensaios. Atualize o projeto e consulte novamente.",true);
                        return;
                    }
                    let passed = report.passed();
                    self.view.components.report = Some(report);
                    self.view.report(
                        if passed {
                            "Ensaios concluídos. Nenhum estado de engenharia foi alterado."
                        } else {
                            "A qualificação encontrou divergências. Confira os detalhes."
                        },
                        !passed,
                    );
                }
                Err(error) => self.view.report(error.to_string(), true),
            }
        }
    }
}

impl eframe::App for Desktop {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ui, |ui| self.render(ui));
    }
}
