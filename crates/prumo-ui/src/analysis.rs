//! Read-only trial panel; submits the same typed request as the MCP tool.
use crate::UiAction;
use egui::{Color32, RichText};
use prumo_domain::Project;
use serde_json::{Value, json};

pub struct AnalysisPanel {
    pub input: String,
    pub report: Option<Value>,
}
impl Default for AnalysisPanel {
    fn default() -> Self {
        Self {
            input: serde_json::to_string_pretty(&json!({
                "kind":"linear",
                "system":{
                    "dimension":2,
                    "terms":[
                        {"row":0,"column":0,"stiffness_n_per_m":4.0},
                        {"row":0,"column":1,"stiffness_n_per_m":1.0},
                        {"row":1,"column":0,"stiffness_n_per_m":1.0},
                        {"row":1,"column":1,"stiffness_n_per_m":3.0}
                    ],
                    "forces_n":[1.0,2.0],
                    "prescribed":[]
                }
            }))
            .unwrap_or_default(),
            report: None,
        }
    }
}
impl AnalysisPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.collapsing("Ensaios numéricos experimentais",|ui|{
            ui.colored_label(Color32::YELLOW,
                "Não é cálculo do projeto. O ensaio não libera estabilidade, dimensionamento, responsabilidade técnica ou emissão.");
            ui.label("Entre com um sistema de referência em unidades SI. O mesmo contrato é usado pela ferramenta MCP prumo.analysis.trial.");
            ui.horizontal_wrapped(|ui|{
                if ui.button("Exemplo: matriz 2 × 2").clicked() {
                    *self=Self::default();
                }
                if ui.button("Exemplo: malha triangular").clicked(){
                    self.input=serde_json::to_string_pretty(&json!({
                        "kind":"mesh_inspect",
                        "mesh":{"points_m":[[0,0,0],[2,0,0],[0,2,0]],
                            "triangles":[[0,1,2]],"thickness_m":0.15}
                    })).unwrap_or_default();
                    self.report=None;
                }
                if ui.button("Exemplo: convergência").clicked(){
                    self.input=serde_json::to_string_pretty(&json!({
                        "kind":"mesh_convergence",
                        "points":[
                            {"elements":10,"value":1.0,"unit":"m"},
                            {"elements":40,"value":0.91,"unit":"m"},
                            {"elements":160,"value":0.905,"unit":"m"}],
                        "relative_tolerance":0.01
                    })).unwrap_or_default();
                    self.report=None;
                }
            });
            let label=ui.label("Entrada JSON em unidades SI");
            if ui.add(egui::TextEdit::multiline(&mut self.input)
                .desired_rows(10).desired_width(f32::INFINITY))
                .labelled_by(label.id).changed()
            {
                self.report=None;
            }
            let parsed=serde_json::from_str::<Value>(&self.input);
            if let Err(error)=&parsed {
                ui.colored_label(Color32::LIGHT_RED,format!("JSON inválido: {error}"));
            }
            if self.input.len()>64*1024{
                ui.colored_label(Color32::LIGHT_RED,
                    "A entrada excede 64 KiB. Divida o ensaio em modelos menores.");
            }
            if ui.add_enabled(parsed.is_ok() && self.input.len()<=64*1024,
                egui::Button::new("Executar ensaio")).clicked()
                && let Ok(value)=parsed
            {
                self.report=None;
                actions.push(UiAction::AnalysisTrial(value));
            }
            if let Some(report)=&self.report {
                let rev=report["revision"].as_u64();
                let current=report["project_id"].as_str()==Some(project.id.to_string().as_str())
                    && rev==Some(project.revision);
                if !current {
                    ui.colored_label(Color32::YELLOW,
                        "Resultado desatualizado. Execute novamente na revisão aberta.");
                }
                ui.label(RichText::new(format!(
                    "Revisão {} · entrada {} · modelo {}",
                    report["revision"],report["input_sha256"],report["snapshot_sha256"]))
                    .small());
                if let Ok(mut formatted)=serde_json::to_string_pretty(report) {
                    ui.add(egui::TextEdit::multiline(&mut formatted)
                        .desired_rows(12).desired_width(f32::INFINITY)
                        .interactive(false));
                }
            }
        });
    }
}
