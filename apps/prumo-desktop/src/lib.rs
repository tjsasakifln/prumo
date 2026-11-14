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
            UiAction::Open(path) => self.select(PathBuf::from(path), true),
            UiAction::Refresh => {
                if let Some(service) = &self.service {
                    match service.read() {
                        Ok(project) => {
                            self.view.project_name = project.name.clone();
                            self.project = Some(project);
                            self.view.report("Projeto reaberto do disco.", false);
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
                    match service.execute(command) {
                        Ok(project) => {
                            self.view.report(
                                format!("Alteração salva. Revisão {}.", project.revision),
                                false,
                            );
                            self.project = Some(project);
                        }
                        Err(error) => self.view.report(error.to_string(), true),
                    }
                }
            }
        }
    }
}

impl eframe::App for Desktop {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ui, |ui| self.render(ui));
    }
}
