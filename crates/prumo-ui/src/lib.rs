//! Reusable Rust UI. It emits domain commands and never owns project persistence.
#![forbid(unsafe_code)]

use egui::{Color32, RichText, Stroke, Vec2};
use prumo_domain::{Command, LengthUnit, Project};
mod registry;
pub use registry::RegistryView;
mod components;
pub use components::ComponentView;
mod model;
mod numeric;
pub use model::{ModelTab, SemanticView};
mod storage;
pub use storage::StorageView;
mod transactions;
pub use transactions::TransactionView;
mod geometry;
pub use geometry::GeometryView;
mod catalogue;
pub use catalogue::CatalogueView;

#[derive(Debug)]
pub enum UiAction {
    RefreshTransactions,
    PreviewTransaction(Box<prumo_domain::transactions::TransactionRequest>),
    ApplyTransaction(String),
    UndoTransaction(Box<prumo_domain::transactions::TransactionHistoryRequest>),
    RedoTransaction(Box<prumo_domain::transactions::TransactionHistoryRequest>),
    InspectGeometry(prumo_domain::geometry::GeometryFilter),
    PreviewGeometry(Vec<prumo_domain::geometry::GeometryRepair>),
    InspectFace(prumo_domain::geometry::FaceInput),
    RunVerification,
    AnalyseCatalogueSection(uuid::Uuid),
    InspectCatalogueMaterial(
        uuid::Uuid,
        std::collections::BTreeMap<
            prumo_domain::catalogue::PhysicalQuantity,
            prumo_domain::catalogue::PhysicalValue,
        >,
    ),
    Open(String),
    Refresh,
    RefreshRegistry,
    RefreshModel,
    RefreshStorage,
    AssessCapability(String),
    RunQualification,
    CancelQualification,
    Command(Box<Command>),
    Storage(Box<prumo_domain::storage::StorageAction>),
}

impl UiAction {
    pub fn command(command: Command) -> Self {
        Self::Command(Box::new(command))
    }
}

pub struct ProjectView {
    pub transactions: TransactionView,
    pub geometry: GeometryView,
    pub catalogue: CatalogueView,
    pub storage: StorageView,
    pub model: SemanticView,
    pub components: ComponentView,
    pub registry: RegistryView,
    pub path: String,
    pub project_name: String,
    pub member_name: String,
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub units: LengthUnit,
    pub environment_name: String,
    pub notice: String,
    pub is_error: bool,
    pub isometric: bool,
    pub selected_path: String,
}

impl Default for ProjectView {
    fn default() -> Self {
        Self {
            transactions: TransactionView::default(),
            geometry: GeometryView::default(),
            catalogue: CatalogueView::default(),
            storage: StorageView::default(),
            model: SemanticView::default(),
            components: ComponentView::default(),
            registry: RegistryView::default(),
            path: "meu-projeto.prumo".into(),
            project_name: "Meu primeiro projeto".into(),
            member_name: "Viga 01".into(),
            start: [0.0, 0.0, 0.0],
            end: [4.0, 0.0, 0.0],
            units: LengthUnit::Metre,
            environment_name: "Ambiente 01".into(),
            notice: "Escolha o arquivo e crie ou abra um projeto.".into(),
            is_error: false,
            isometric: true,
            selected_path: String::new(),
        }
    }
}

impl ProjectView {
    pub fn report(&mut self, message: impl Into<String>, is_error: bool) {
        self.notice = message.into();
        self.is_error = is_error;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, project: Option<&Project>) -> Vec<UiAction> {
        let mut actions = Vec::new();
        let teal = Color32::from_rgb(33, 193, 164);
        ui.spacing_mut().item_spacing = Vec2::new(12.0, 10.0);
        ui.horizontal(|ui| {
            ui.heading(RichText::new("PRUMO").size(29.0).color(teal));
            ui.add_space(16.0);
            ui.label(RichText::new("Um modelo. Suas decisões.").size(17.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label("VERSÃO DE DESENVOLVIMENTO");
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            let label = ui.label("Arquivo local");
            if ui
                .add(egui::TextEdit::singleline(&mut self.path).desired_width(480.0))
                .labelled_by(label.id)
                .changed()
            {
                self.storage.clear_confirmations();
            }
            if ui.button("Abrir arquivo").clicked() {
                actions.push(UiAction::Open(self.path.clone()));
            }
            if ui
                .add_enabled(
                    !self.selected_path.is_empty(),
                    egui::Button::new("Reabrir / atualizar"),
                )
                .clicked()
            {
                actions.push(UiAction::Refresh);
            }
        });
        ui.label(
            RichText::new(
                "Modelagem BIM disponível. Cálculo e dimensionamento ainda não disponíveis.",
            )
            .color(Color32::from_rgb(244, 198, 105)),
        );
        let color = if self.is_error {
            Color32::from_rgb(255, 145, 145)
        } else {
            teal
        };
        ui.label(RichText::new(&self.notice).color(color));
        let selected = self.path == self.selected_path;
        if !selected {
            ui.colored_label(Color32::YELLOW, "O caminho foi alterado. Clique em Abrir arquivo para selecioná-lo antes de editar.");
            ui.label(format!("Projeto carregado: {}", self.selected_path));
        }
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_enabled_ui(selected, |ui| self.storage.show(ui,&mut actions));
            ui.separator();
            if let Some(project) = project {
                ui.add_enabled_ui(selected, |ui| self.transactions.show(ui,project,&mut actions));
                ui.add_enabled_ui(selected, |ui| self.geometry.show(ui,project,&mut actions));
                ui.add_enabled_ui(selected, |ui| self.catalogue.show(ui,project,&mut actions));
                ui.add_enabled_ui(selected, |ui| self.components.show(ui,&mut actions));
                ui.separator();
                ui.add_enabled_ui(selected, |ui| self.registry.show(ui, project, &mut actions));
                ui.separator();
                ui.add_enabled_ui(selected, |ui| self.project_controls(ui, project, &mut actions));
                ui.separator();
                ui.columns(2, |columns| {
                    columns[0].add_enabled_ui(selected, |ui| self.member_form(ui, project, &mut actions));
                    self.model_view(&mut columns[1], project);
                });
                ui.separator();
                ui.add_enabled_ui(selected, |ui| self.model.show(ui, project, &mut actions));
                ui.separator();
                ui.heading("Objetos do projeto");
                ui.label("Identidades físicas e analíticas são distintas e ligadas explicitamente. Nenhuma peça está dimensionada.");
                egui::Grid::new("objects").striped(true).min_col_width(130.0).show(ui, |ui| {
                    ui.strong("Peça"); ui.strong("Comprimento"); ui.strong("Estado"); ui.end_row();
                    for object in &project.objects {
                        let length = object.length_m();
                        let analytical: Vec<_> = project.model.links.iter().filter(|link|link.physical_id==object.id).map(|link|link.analytical_id.to_string()).collect();
                        ui.label(&object.name).on_hover_text(format!("Físico: {}\nAnalíticos: {}", object.id, if analytical.is_empty(){"Sem vínculo".into()}else{analytical.join(", ")}));
                        ui.label(format!("{length:.3} m"));
                        ui.label(if object.role == prumo_domain::model::PhysicalRole::VisualOnly {"Somente visual"} else {"Modelado, não calculado"}); ui.end_row();
                    }
                });
                if project.objects.is_empty() { ui.label("Adicione a primeira peça pelas coordenadas acima."); }
                ui.separator();
                ui.collapsing("Identidades e histórico", |ui| {
                    ui.label(format!("Projeto: {}", project.id));
                    ui.label(format!("Revisão: {} | Unidade: {:?}", project.revision, project.units));
                    for change in &project.history { ui.label(format!("{change:?}")); }
                });
            } else if self.storage.can_create() {
                ui.add_space(20.0);
                ui.heading("Comece pelo seu projeto");
                ui.label("Defina o nome e a unidade. As alterações são salvas a cada operação.");
                let label = ui.label("Nome do projeto");
                ui.add(egui::TextEdit::singleline(&mut self.project_name).desired_width(380.0)).labelled_by(label.id);
                units_selector(ui, &mut self.units);
                if ui.button("Criar projeto").clicked() {
                    actions.push(UiAction::command(Command::Create { name: self.project_name.clone(), units: self.units }));
                }
            } else if self.storage.status.as_ref().is_some_and(|status| matches!(status.state,prumo_domain::storage::StorageState::Ready|prumo_domain::storage::StorageState::Legacy)) {
                ui.heading("Reabra o projeto para continuar");
                ui.label("O diagnóstico corresponde ao arquivo atual. Use Reabrir / atualizar para carregar seu modelo antes de editar.");
            } else {
                ui.heading("O arquivo selecionado não pode ser aberto");
                ui.label("Confira o diagnóstico e as cópias disponíveis acima. Para iniciar outro projeto, selecione outro arquivo local.");
            }
        });
        actions
    }

    fn project_controls(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        actions: &mut Vec<UiAction>,
    ) {
        ui.horizontal(|ui| {
            ui.heading(&project.name);
            ui.label(format!("Revisão {}", project.revision));
            ui.label("Salvo neste computador");
        });
        ui.collapsing("Nome e ambientes", |ui| {
            ui.horizontal(|ui| {
                let label = ui.label("Nome do projeto");
                ui.text_edit_singleline(&mut self.project_name)
                    .labelled_by(label.id);
                if ui.button("Renomear projeto").clicked() {
                    actions.push(UiAction::command(Command::RenameProject {
                        expected_revision: project.revision,
                        name: self.project_name.clone(),
                    }));
                }
            });
            ui.horizontal(|ui| {
                let label = ui.label("Ambiente");
                ui.text_edit_singleline(&mut self.environment_name)
                    .labelled_by(label.id);
                if ui.button("Adicionar ambiente").clicked() {
                    actions.push(UiAction::command(Command::AddEnvironment {
                        expected_revision: project.revision,
                        name: self.environment_name.clone(),
                    }));
                }
            });
            for environment in &project.environments {
                ui.label(&environment.name);
            }
        });
    }

    fn member_form(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.heading("Adicionar peça");
        ui.label("Barra física com eixo analítico associado");
        ui.horizontal(|ui| {
            let label = ui.label("Nome da peça");
            ui.text_edit_singleline(&mut self.member_name)
                .labelled_by(label.id);
        });
        ui.label(format!("Coordenadas em {}", unit_label(project.units)));
        numeric::begin(ui);
        numeric::context(ui, format!("{}:quick-member", project.id));
        coordinates(ui, "Início", &mut self.start);
        coordinates(ui, "Fim", &mut self.end);
        let invalid = numeric::errors(ui);
        if !invalid.is_empty() {
            ui.colored_label(
                Color32::LIGHT_RED,
                format!("Corrija os campos numéricos: {}.", invalid.join(", ")),
            );
        }
        if ui
            .add_enabled(
                invalid.is_empty(),
                egui::Button::new("Adicionar peça e salvar"),
            )
            .clicked()
        {
            actions.push(UiAction::command(Command::AddMember {
                expected_revision: project.revision,
                name: self.member_name.clone(),
                start: self.start,
                end: self.end,
            }));
        }
        ui.label("As coordenadas podem ser preenchidas pelo teclado. Um cruzamento no desenho não cria conexão de engenharia.");
    }

    fn model_view(&mut self, ui: &mut egui::Ui, project: &Project) {
        ui.horizontal(|ui| {
            ui.heading("Modelo");
            ui.checkbox(&mut self.isometric, "Vista isométrica");
        });
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().max(120.0), 225.0),
            egui::Sense::hover(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, Color32::from_rgb(18, 29, 39));
        let project_point = |p: [f64; 3]| -> [f64; 2] {
            if self.isometric {
                [(p[0] - p[1]) * 0.8660254038, (p[0] + p[1]) * 0.5 - p[2]]
            } else {
                [p[0], -p[1]]
            }
        };
        let points: Vec<_> = project
            .objects
            .iter()
            .flat_map(|o| {
                [
                    project_point(o.start_m.as_array()),
                    project_point(o.end_m.as_array()),
                ]
            })
            .collect();
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for p in &points {
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        let width = (hi[0] - lo[0]).max(1.0);
        let height = (hi[1] - lo[1]).max(1.0);
        let scale =
            ((rect.width() - 48.0) as f64 / width).min((rect.height() - 48.0) as f64 / height);
        let to_screen = |p: [f64; 2]| {
            rect.center()
                + Vec2::new(
                    ((p[0] - (lo[0] + hi[0]) * 0.5) * scale) as f32,
                    ((p[1] - (lo[1] + hi[1]) * 0.5) * scale) as f32,
                )
        };
        for pair in points.chunks(2) {
            let a = to_screen(pair[0]);
            let b = to_screen(pair[1]);
            painter.line_segment([a, b], Stroke::new(4.0, Color32::from_rgb(33, 193, 164)));
            painter.circle_filled(a, 4.0, Color32::WHITE);
            painter.circle_filled(b, 4.0, Color32::WHITE);
        }
        if points.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Seu modelo aparecerá aqui",
                egui::FontId::proportional(16.0),
                Color32::GRAY,
            );
        }
        ui.label("Vista de modelagem. As linhas representam os eixos das peças.");
    }
}

fn unit_label(unit: LengthUnit) -> &'static str {
    match unit {
        LengthUnit::Metre => "metros",
        LengthUnit::Centimetre => "centímetros",
        LengthUnit::Millimetre => "milímetros",
    }
}

fn units_selector(ui: &mut egui::Ui, unit: &mut LengthUnit) {
    ui.horizontal(|ui| {
        ui.label("Unidade de entrada");
        for candidate in [
            LengthUnit::Metre,
            LengthUnit::Centimetre,
            LengthUnit::Millimetre,
        ] {
            ui.selectable_value(unit, candidate, unit_label(candidate));
        }
    });
}

fn coordinates(ui: &mut egui::Ui, label: &str, point: &mut [f64; 3]) {
    ui.columns(3, |columns| {
        for (index, (axis, value)) in ["X", "Y", "Z"].into_iter().zip(point).enumerate() {
            numeric::field(&mut columns[index], &format!("{label} {axis}"), value);
        }
    });
}
