#![forbid(unsafe_code)]
use prumo_app::{AccessMode, ProjectService};
use prumo_domain::Project;
use prumo_domain::components::QualificationReport;
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
        let selected_path = path.to_string_lossy().into_owned();
        match ProjectService::open(path, AccessMode::ReadWrite, "interface-local") {
            Ok(service) => {
                match service.read() {
                    Ok(project) => {
                        self.view.project_name = project.name.clone();
                        self.view.report(
                            format!("Projeto aberto. Revisão {}.", project.revision),
                            false,
                        );
                        self.project = Some(project);
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
        for action in self.view.show(ui, self.project.as_ref()) {
            self.dispatch(action);
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
                    match service.read() {
                        Ok(project) => {
                            if self.project.as_ref().is_some_and(|previous| {
                                previous.id != project.id || previous.revision != project.revision
                            }) {
                                self.clear_qualification();
                            }
                            self.view.project_name = project.name.clone();
                            self.project = Some(project);
                            self.view.report("Projeto reaberto do disco.", false);
                            self.refresh_registry();
                        }
                        Err(error) => self.view.report(error.to_string(), true),
                    }
                }
            }
            UiAction::Command(command) => {
                if self.project.is_some() && self.view.path != self.view.selected_path {
                    self.view.report(
                        "Abra o arquivo selecionado antes de alterar o projeto.",
                        true,
                    );
                    return;
                }
                if self.project.is_none() {
                    self.select(PathBuf::from(&self.view.path), false);
                }
                if let Some(service) = &self.service {
                    match service.execute(*command) {
                        Ok(project) => {
                            self.clear_qualification();
                            self.view.report(
                                format!("Alteração salva. Revisão {}.", project.revision),
                                false,
                            );
                            self.project = Some(project);
                            self.refresh_registry();
                        }
                        Err(error) => self.view.report(error.to_string(), true),
                    }
                }
            }
        }
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
