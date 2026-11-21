//! Guided preparation before structural calculation. Uses exactly the domain/MCP edits.
use std::collections::BTreeSet;

use egui::{Color32, RichText};
use prumo_domain::model::{ModelEdit, PointInput};
use prumo_domain::precalculation::{
    self, Freedom, MemberEnd, MemberRelease, NodeSupport, PrecalculationEdit, Severity,
};
use prumo_domain::transactions::TransactionRequest;
use prumo_domain::{Command, LengthUnit, Project};
use uuid::Uuid;

use crate::UiAction;

pub struct PrecalculationPanel {
    node: Option<Uuid>,
    support_modes: [u8; 6],
    support_stiffness: [f64; 6],
    support_note: String,
    support_rotation_deg: f64,
    member: Option<Uuid>,
    release_end: MemberEnd,
    release_modes: [u8; 6],
    release_stiffness: [f64; 6],
    release_note: String,
    selected: BTreeSet<Uuid>,
    batch_material: Option<Uuid>,
    translation_m: [f64; 3],
    linked: bool,
    confirm_batch: bool,
    confirm_promotion: bool,
    tab: u8,
}

impl Default for PrecalculationPanel {
    fn default() -> Self {
        Self {
            node: None,
            support_modes: [1, 1, 1, 0, 0, 0],
            support_stiffness: [1000.0; 6],
            support_note: "Hipótese declarada pelo projetista".into(),
            support_rotation_deg: 0.0,
            member: None,
            release_end: MemberEnd::Start,
            release_modes: [1, 1, 1, 0, 0, 0],
            release_stiffness: [1000.0; 6],
            release_note: "Liberação explicitamente definida".into(),
            selected: BTreeSet::new(),
            batch_material: None,
            translation_m: [0.0, 0.0, 3.0],
            linked: false,
            confirm_batch: false,
            confirm_promotion: false,
            tab: 0,
        }
    }
}

impl PrecalculationPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.collapsing("Vínculos, referências, lotes e revisão pré-cálculo", |ui| {
            ui.label("As peças e hipóteses são modeladas, não dimensionadas. O projeto não recebe estabilizações artificiais.");
            ui.horizontal_wrapped(|ui| {
                for (tab, label) in ["Vínculos", "Referências", "Edição em lote", "Conferência"].iter().enumerate() {
                    ui.selectable_value(&mut self.tab, tab as u8, *label);
                }
            });
            ui.separator();
            match self.tab {
                0 => self.supports(ui, project, actions),
                1 => self.references(ui, project, actions),
                2 => self.batch(ui, project, actions),
                _ => self.review(ui, project),
            }
        });
    }

    fn edit(actions: &mut Vec<UiAction>, project: &Project, command: PrecalculationEdit) {
        actions.push(UiAction::command(Command::EditModel {
            expected_revision: project.revision,
            edit: ModelEdit::EditPrecalculation { command },
        }));
    }

    fn preview_edit(actions: &mut Vec<UiAction>, project: &Project, command: PrecalculationEdit) {
        let operation = Command::EditModel {
            expected_revision: project.revision,
            edit: ModelEdit::EditPrecalculation { command },
        };
        actions.push(UiAction::PreviewTransaction(Box::new(
            TransactionRequest::new(project, vec![operation]),
        )));
    }

    fn freedoms(modes: [u8; 6], stiffness: [f64; 6]) -> [Freedom; 6] {
        std::array::from_fn(|i| match modes[i] {
            1 => Freedom::Fixed {},
            2 => Freedom::Spring {
                stiffness_si: stiffness[i],
            },
            3 => Freedom::CompressionOnly {
                stiffness_si: stiffness[i],
            },
            _ => Freedom::Free {},
        })
    }

    fn degree_inputs(
        ui: &mut egui::Ui,
        modes: &mut [u8; 6],
        stiffness: &mut [f64; 6],
        prefix: &str,
    ) {
        for (i, label) in ["Ux", "Uy", "Uz", "Rx", "Ry", "Rz"].iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(*label);
                egui::ComboBox::from_id_salt(format!("{prefix}-{label}"))
                    .selected_text(["Livre", "Fixo", "Mola", "Unilateral"][modes[i] as usize])
                    .show_ui(ui, |ui| {
                        for (choice, name) in
                            ["Livre", "Fixo", "Mola", "Unilateral"].iter().enumerate()
                        {
                            ui.selectable_value(&mut modes[i], choice as u8, *name);
                        }
                    });
                if modes[i] >= 2 {
                    ui.label(if i < 3 { "N/m" } else { "N.m/rad" });
                    ui.add(egui::DragValue::new(&mut stiffness[i]).speed(10.0));
                }
            });
        }
    }

    fn supports(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        let Ok(saved) = precalculation::state(project) else {
            ui.colored_label(Color32::RED, "Os vínculos do arquivo são inválidos.");
            return;
        };
        ui.heading("Apoios nodais");
        ui.label(
            "Os seis graus de liberdade e a orientação são explícitos. Os valores de mola são SI.",
        );
        egui::ComboBox::from_label("Nó para apoio")
            .selected_text(
                self.node
                    .and_then(|id| project.model.analytical_nodes.iter().find(|n| n.id == id))
                    .map_or("Selecione o nó", |n| n.name.as_str()),
            )
            .show_ui(ui, |ui| {
                for node in &project.model.analytical_nodes {
                    if ui
                        .selectable_value(&mut self.node, Some(node.id), &node.name)
                        .changed()
                    {
                        if let Some(existing) = saved.supports.iter().find(|s| s.node_id == node.id)
                        {
                            for (i, value) in existing.freedoms.iter().enumerate() {
                                let (mode, stiffness) = match value {
                                    Freedom::Free {} => (0, 0.0),
                                    Freedom::Fixed {} => (1, 0.0),
                                    Freedom::Spring { stiffness_si } => (2, *stiffness_si),
                                    Freedom::CompressionOnly { stiffness_si } => (3, *stiffness_si),
                                };
                                self.support_modes[i] = mode;
                                if mode >= 2 {
                                    self.support_stiffness[i] = stiffness;
                                }
                            }
                            self.support_rotation_deg = existing.rotation_to_engineering[1][0]
                                .atan2(existing.rotation_to_engineering[0][0])
                                .to_degrees();
                            self.support_note = existing.justification.clone();
                        }
                    }
                }
            });
        ui.horizontal(|ui| {
            ui.label("Rotação em torno de Z global (graus)");
            ui.add(egui::DragValue::new(&mut self.support_rotation_deg).speed(1.0));
        });
        Self::degree_inputs(
            ui,
            &mut self.support_modes,
            &mut self.support_stiffness,
            "support",
        );
        ui.label("Justificativa da hipótese");
        ui.text_edit_singleline(&mut self.support_note);
        if ui
            .add_enabled(self.node.is_some(), egui::Button::new("Gravar apoio"))
            .clicked()
        {
            let node_id = self.node.expect("enabled");
            let id = saved
                .supports
                .iter()
                .find(|s| s.node_id == node_id)
                .map_or_else(Uuid::new_v4, |s| s.id);
            let (sine, cosine) = self.support_rotation_deg.to_radians().sin_cos();
            Self::edit(
                actions,
                project,
                PrecalculationEdit::PutSupport {
                    support: NodeSupport {
                        id,
                        node_id,
                        rotation_to_engineering: [
                            [cosine, -sine, 0.0],
                            [sine, cosine, 0.0],
                            [0.0, 0.0, 1.0],
                        ],
                        freedoms: Self::freedoms(self.support_modes, self.support_stiffness),
                        justification: self.support_note.clone(),
                    },
                },
            );
        }
        for support in &saved.supports {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Nó {}: {} restrições",
                    support.node_id,
                    support
                        .freedoms
                        .iter()
                        .filter(|f| !matches!(f, Freedom::Free {}))
                        .count()
                ));
                if ui.small_button("Excluir").clicked() {
                    Self::edit(
                        actions,
                        project,
                        PrecalculationEdit::RemoveSupport { id: support.id },
                    );
                }
            });
        }
        ui.separator();
        ui.heading("Liberações de extremidade");
        egui::ComboBox::from_label("Barra")
            .selected_text(
                self.member
                    .and_then(|id| project.model.analytical_members.iter().find(|m| m.id == id))
                    .map_or("Selecione uma barra", |m| m.name.as_str()),
            )
            .show_ui(ui, |ui| {
                for member in &project.model.analytical_members {
                    ui.selectable_value(&mut self.member, Some(member.id), &member.name);
                }
            });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.release_end, MemberEnd::Start, "Início");
            ui.selectable_value(&mut self.release_end, MemberEnd::End, "Fim");
        });
        Self::degree_inputs(
            ui,
            &mut self.release_modes,
            &mut self.release_stiffness,
            "release",
        );
        ui.label("Justificativa");
        ui.text_edit_singleline(&mut self.release_note);
        if ui
            .add_enabled(self.member.is_some(), egui::Button::new("Gravar liberação"))
            .clicked()
        {
            let member_id = self.member.expect("enabled");
            let id = saved
                .releases
                .iter()
                .find(|r| r.member_id == member_id && r.end == self.release_end)
                .map_or_else(Uuid::new_v4, |r| r.id);
            Self::edit(
                actions,
                project,
                PrecalculationEdit::PutRelease {
                    release: MemberRelease {
                        id,
                        member_id,
                        end: self.release_end,
                        freedoms: Self::freedoms(self.release_modes, self.release_stiffness),
                        justification: self.release_note.clone(),
                    },
                },
            );
        }
        for release in &saved.releases {
            ui.horizontal(|ui| {
                ui.label(format!("{}: {:?}", release.member_id, release.end));
                if ui.small_button("Excluir").clicked() {
                    Self::edit(
                        actions,
                        project,
                        PrecalculationEdit::RemoveRelease { id: release.id },
                    );
                }
            });
        }
    }

    fn references(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        let Ok(saved) = precalculation::state(project) else {
            return;
        };
        ui.heading("Referências de desenho e IFC");
        ui.checkbox(&mut self.confirm_promotion,
            "Conferi a geometria, as duas cotas, o posicionamento e a finalidade do elemento selecionado");
        ui.label("O cadastro informa SHA e geometria candidata fornecidos pelo usuário. O Prumo ainda não interpreta PDF, DXF ou IFC completo; confira duas medidas antes de promover.");
        for drawing in &saved.drawings {
            let discrepancy = drawing
                .calibration
                .factors()
                .map(|(_, _, d)| d)
                .unwrap_or(f64::INFINITY);
            ui.collapsing(
                format!(
                    "{} | {:?} | diferença {:.2}%",
                    drawing.label,
                    drawing.format,
                    discrepancy * 100.0
                ),
                |ui| {
                    ui.label(format!("SHA: {}", drawing.sha256));
                    for trace in &drawing.traces {
                        ui.horizontal(|ui| {
                            ui.label(format!("Camada {} | traço {}", trace.layer, trace.id));
                            ui.label(format!(
                                "Origem {:?} até {:?} nas unidades da referência",
                                trace.a, trace.b
                            ));
                            if ui
                                .add_enabled(
                                    self.confirm_promotion && discrepancy <= 0.01 && trace.visible,
                                    egui::Button::new("Promover com dimensões conferidas"),
                                )
                                .clicked()
                            {
                                Self::preview_edit(
                                    actions,
                                    project,
                                    PrecalculationEdit::PromoteDrawing {
                                        reference_id: drawing.id,
                                        trace_id: trace.id,
                                        name: format!("Traço {}", trace.id),
                                        material_id: None,
                                        confirm_dimensions: true,
                                    },
                                );
                                self.confirm_promotion = false;
                            }
                        });
                    }
                },
            );
        }
        for source in &saved.ifc {
            ui.collapsing(
                format!(
                    "IFC {:?}: {} | {} candidatos",
                    source.schema,
                    source.label,
                    source.candidates.len()
                ),
                |ui| {
                    ui.label(format!("SHA da origem: {}", source.sha256));
                    ui.label("Geometria proposta, sem validação de conexão analítica.");
                    for candidate in &source.candidates {
                        ui.horizontal(|ui| {
                            ui.label(format!("{} | {}", candidate.name, candidate.guid));
                            if ui
                                .add_enabled(
                                    self.confirm_promotion,
                                    egui::Button::new("Promover após conferência"),
                                )
                                .clicked()
                            {
                                Self::preview_edit(
                                    actions,
                                    project,
                                    PrecalculationEdit::PromoteIfc {
                                        reference_id: source.id,
                                        guid: candidate.guid.clone(),
                                        material_id: None,
                                        confirm_hypotheses: true,
                                    },
                                );
                                self.confirm_promotion = false;
                            }
                        });
                    }
                },
            );
        }
        if saved.drawings.is_empty() && saved.ifc.is_empty() {
            ui.label("Nenhuma referência registrada. Operações de registro disponíveis pelo MCP com SHA e geometria candidata explícitos.");
        }
    }

    fn batch(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.heading("Selecionar peças para edição");
        for obj in &project.objects {
            let mut checked = self.selected.contains(&obj.id);
            if ui.checkbox(&mut checked, &obj.name).changed() {
                if checked {
                    self.selected.insert(obj.id);
                } else {
                    self.selected.remove(&obj.id);
                }
            }
        }
        ui.label(format!(
            "{} selecionadas. Cópias de geradores complexos requerem o comando próprio.",
            self.selected.len()
        ));
        egui::ComboBox::from_label("Aplicar material")
            .selected_text(
                self.batch_material
                    .and_then(|id| project.model.materials.iter().find(|m| m.id == id))
                    .map_or("Sem material", |m| m.name.as_str()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.batch_material, None, "Sem material");
                for m in &project.model.materials {
                    ui.selectable_value(&mut self.batch_material, Some(m.id), &m.name);
                }
            });
        ui.label("Deslocamento da cópia (m)");
        ui.horizontal(|ui| {
            for i in 0..3 {
                ui.add(egui::DragValue::new(&mut self.translation_m[i]).speed(0.1));
            }
        });
        ui.checkbox(
            &mut self.linked,
            "Registrar vínculo paramétrico com a origem",
        );
        ui.checkbox(
            &mut self.confirm_batch,
            "Conferi as peças e a alteração proposta",
        );
        let enabled = self.confirm_batch && !self.selected.is_empty() && self.selected.len() <= 500;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(enabled, egui::Button::new("Conferir alteração de material"))
                .clicked()
            {
                Self::preview_edit(
                    actions,
                    project,
                    PrecalculationEdit::SetMaterialBatch {
                        object_ids: self.selected.iter().copied().collect(),
                        material_id: self.batch_material,
                    },
                );
                self.confirm_batch = false;
            }
            if ui
                .add_enabled(enabled, egui::Button::new("Conferir cópias"))
                .clicked()
            {
                Self::preview_edit(
                    actions,
                    project,
                    PrecalculationEdit::CopyBatch {
                        object_ids: self.selected.iter().copied().collect(),
                        translation: PointInput {
                            values: self.translation_m,
                            unit: LengthUnit::Metre,
                        },
                        linked: self.linked,
                    },
                );
                self.confirm_batch = false;
            }
        });
        ui.label("A prévia indica cada objeto afetado e eventuais resultados invalidados. Confira e aplique no painel de transações, no início da página. Desfazer permanece disponível no histórico.");
    }

    fn review(&mut self, ui: &mut egui::Ui, project: &Project) {
        ui.heading("Conferência orientada antes do cálculo");
        let Ok(report) = precalculation::review(project) else {
            ui.colored_label(Color32::RED, "Não foi possível inspecionar o modelo.");
            return;
        };
        ui.label(format!(
            "Projeto {} | revisão {}",
            report.project_id, report.revision
        ));
        for stage in &report.stages {
            ui.separator();
            ui.strong(stage.to_uppercase());
            for finding in report.findings.iter().filter(|f| &f.stage == stage) {
                let color = match finding.severity {
                    Severity::Blocker => Color32::from_rgb(255, 127, 127),
                    Severity::Warning => Color32::YELLOW,
                    Severity::Info => Color32::LIGHT_GRAY,
                };
                ui.label(RichText::new(&finding.explanation).color(color));
                if let Some(id) = finding.entity_id {
                    ui.label(format!("Elemento: {}", id));
                }
                ui.label(format!("Impacto: {}", finding.consequence));
                ui.label(format!("Correção: {}", finding.suggested_action));
            }
        }
        ui.separator();
        ui.label(if report.ready_for_engineering_calculation {
            "Modelo sem impedimentos nesta conferência parcial. Cálculo ainda indisponível."
        } else {
            "Revisão bloqueada: há dados ou condições de contorno a corrigir."
        });
        ui.colored_label(
            Color32::YELLOW,
            "Esta conferência não aprova esforços, estabilidade, resistência ou projeto executivo.",
        );
    }
}
