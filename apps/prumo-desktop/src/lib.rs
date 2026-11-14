#![forbid(unsafe_code)]
use prumo_app::{AccessMode, ProjectService};
use prumo_domain::Project;
use prumo_ui::{ProjectView, UiAction};
use std::path::PathBuf;

pub struct Desktop {
    pub view: ProjectView,
    pub project: Option<Project>,
    service: Option<ProjectService>,
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
        };
        app.select(path, false);
        app
    }

    fn select(&mut self, path: PathBuf, report_missing: bool) {
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
        for action in self.view.show(ui, self.project.as_ref()) {
            self.dispatch(action);
        }
    }

    pub fn dispatch(&mut self, action: UiAction) {
        match action {
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
}

impl eframe::App for Desktop {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ui, |ui| self.render(ui));
    }
}
