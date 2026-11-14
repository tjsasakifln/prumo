use crate::UiAction;
use egui::{Color32, RichText};
use prumo_domain::components::{ComponentDescriptor, QualificationReport};

#[derive(Default)]
pub struct ComponentView {
    pub inventory: Vec<ComponentDescriptor>,
    pub report: Option<QualificationReport>,
    pub running: bool,
}

impl ComponentView {
    pub fn show(&mut self, ui: &mut egui::Ui, actions: &mut Vec<UiAction>) {
        ui.collapsing("Verificação de componentes",|ui| {
            ui.label("Estes ensaios comparam componentes com referências independentes em domínios limitados. Eles não calculam os dados do projeto nem liberam uma capacidade de engenharia.");
            if self.running {
                ui.horizontal(|ui| {
                    ui.spinner();ui.label("Ensaios em execução em processo separado.");
                    if ui.button("Cancelar ensaios").clicked(){actions.push(UiAction::CancelQualification);}
                });
            } else if ui.button("Executar ensaios de componentes").clicked() {
                actions.push(UiAction::RunQualification);
            }
            ui.collapsing(format!("{} integrações disponíveis",self.inventory.len()),|ui| {
                for component in &self.inventory {
                    ui.collapsing(format!("{} | {}",component.name,component.id),|ui| {
                        ui.label(format!("Versão: {} | Licença: {}",component.version,component.license));
                        ui.label(&component.scope);
                        for limitation in &component.limitations {ui.label(limitation);}
                    });
                }
            });
            if let Some(report)=&self.report {
                let passed=report.passed();
                ui.colored_label(if passed{Color32::LIGHT_GREEN}else{Color32::LIGHT_RED},if passed{"Todos os ensaios passaram no domínio descrito."}else{"Há ensaios com falha. Consulte os detalhes abaixo."});
                ui.label(format!("Plataforma: {} | Bateria: {} | Duração: {:.3} ms",report.platform,report.suite_id,report.elapsed_ns as f64/1.0e6));
                ui.label(format!("Executável: {} bytes | SHA-256: {}",report.executable_bytes,report.executable_sha256));
                if let Some(basis)=&report.basis {
                    ui.label(format!("Consulta iniciada no projeto {}, revisão {}. Os ensaios usam casos de referência próprios.",basis.project_id,basis.revision));
                }
                for experiment in &report.experiments {
                    ui.collapsing(format!("{}: {}",experiment.component.id,if experiment.passed(){"conferido"}else{"com falha"}),|ui| {
                        ui.label(RichText::new(&experiment.component.scope).strong());
                        ui.label(format!("{} repetições | {:.3} ms",experiment.iterations,experiment.elapsed_ns as f64/1.0e6));
                        for check in &experiment.checks {
                            ui.label(format!("{}: {}",check.name,if check.passed{"passou"}else{"falhou"}));
                            ui.small(&check.detail);
                            if let (Some(error),Some(tolerance))=(check.absolute_error,check.tolerance) {
                                ui.label(format!("Erro absoluto: {error:.3e}; tolerância: {tolerance:.3e} {}",check.unit.as_deref().unwrap_or("")));
                            }
                        }
                        for limitation in &experiment.component.limitations {ui.label(limitation);}
                    });
                }
            }
        });
    }
}
