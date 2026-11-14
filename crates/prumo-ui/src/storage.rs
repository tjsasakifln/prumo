//! Storage diagnosis remains available even when the project cannot be decoded.
use crate::UiAction;
use egui::{Color32, Ui};
use prumo_domain::storage::{
    StorageAction, StorageCandidate, StorageFormat, StorageState, StorageStatus,
};

#[derive(Default)]
pub struct StorageView {
    pub status: Option<StorageStatus>,
    pub selected_candidate: String,
    pub confirm_migration: bool,
    pub confirm_recovery: bool,
    pub expanded: bool,
}

impl StorageView {
    pub fn clear_confirmations(&mut self) {
        self.confirm_migration = false;
        self.confirm_recovery = false;
    }

    pub fn accept(&mut self, status: StorageStatus) {
        self.clear_confirmations();
        if !status
            .candidates
            .iter()
            .any(|candidate| candidate.id == self.selected_candidate && candidate.recoverable)
        {
            self.selected_candidate.clear();
        }
        if matches!(
            status.state,
            StorageState::Legacy
                | StorageState::RecoveryAvailable
                | StorageState::Invalid
                | StorageState::Unsupported
        ) {
            self.expanded = true;
        }
        self.status = Some(status);
    }

    pub fn can_create(&self) -> bool {
        self.status.as_ref().is_some_and(|status| {
            status.state == StorageState::Missing && status.candidates.is_empty()
        })
    }

    pub fn show(&mut self, ui: &mut Ui, actions: &mut Vec<UiAction>) {
        let response=egui::CollapsingHeader::new("Arquivo e recuperação").open(Some(self.expanded)).show(ui,|ui| {
            if ui.button("Inspecionar arquivo").clicked(){actions.push(UiAction::RefreshStorage);}
            let Some(status)=&self.status else {
                ui.label("O diagnóstico não está disponível. Confira a mensagem exibida, selecione um arquivo local ou inspecione novamente.");
                return;
            };
            let (state,color)=match status.state {
                StorageState::Ready=>("Arquivo íntegro e pronto para uso",Color32::LIGHT_GREEN),
                StorageState::Legacy=>("Arquivo legado preservado",Color32::YELLOW),
                StorageState::RecoveryAvailable=>("Há conteúdo recuperável para conferir",Color32::YELLOW),
                StorageState::Missing=>("Arquivo ainda não criado",Color32::GRAY),
                StorageState::Invalid=>("Arquivo inválido ou corrompido",Color32::LIGHT_RED),
                StorageState::Unsupported=>("Versão ou formato incompatível",Color32::LIGHT_RED),
            };
            ui.colored_label(color,state);
            ui.label(format!("Formato: {}",match status.format {
                StorageFormat::NativePackage=>"Pacote Prumo",StorageFormat::LegacyJson=>"JSON legado",
                StorageFormat::Missing=>"Ausente",StorageFormat::Unknown=>"Não identificado",
            }));
            if let (Some(project_id),Some(revision))=(status.project_id,status.revision) {
                ui.label(format!("Projeto {project_id} | revisão {revision}"));
            }
            for diagnostic in &status.diagnostics {
                ui.label(&diagnostic.message).on_hover_text(&diagnostic.code);
            }
            if status.state==StorageState::Legacy {
                ui.label("Abrir este arquivo não o modifica. A primeira alteração válida migra para o pacote Prumo e preserva uma cópia original. Você também pode migrar agora.");
            }
            if let Some(package)=&status.package {
                ui.label(format!("Pacote v{} | modelo v{} | {} conteúdos verificados",package.format_version,package.model_version,package.entries.len()));
                ui.collapsing("Identidades do pacote",|ui| {
                    ui.label(format!("Origem: {} | banco v{}",package.origin,package.database_version));
                    ui.monospace(format!("Projeto: {}",package.project_sha256));
                    ui.monospace(format!("Revisão armazenada: {}",package.head_sha256));
                    for entry in &package.entries {
                        ui.label(format!("{} | {} bytes",entry.name,entry.size));ui.small(&entry.sha256);
                    }
                });
            }
            if status.can_migrate&&let Some(expected_revision)=status.revision {
                ui.checkbox(&mut self.confirm_migration,"Confirmo migrar o arquivo e manter a cópia original");
                if ui.add_enabled(self.confirm_migration,egui::Button::new("Migrar arquivo Prumo")).clicked() {
                    actions.push(UiAction::Storage(Box::new(StorageAction::Migrate {
                        expected_revision,inspection_sha256:status.inspection_sha256.clone(),confirm:self.confirm_migration,
                    })));
                }
            }
            if status.can_backup&&let Some(expected_revision)=status.revision
                && ui.button("Criar backup verificado").clicked() {
                actions.push(UiAction::Storage(Box::new(StorageAction::Backup {
                    expected_revision,inspection_sha256:status.inspection_sha256.clone(),
                })));
            }
            if !status.candidates.is_empty() {
                ui.separator();ui.strong("Cópias disponíveis");
                for candidate in &status.candidates {
                    ui.label(format!("{} | revisão {} | {}",candidate.label,candidate.revision,
                        if candidate.recoverable {"Disponível para conferir"}else{"Recuperação bloqueada neste contexto"}));
                }
                ui.add_enabled_ui(status.can_recover,|ui| {
                    let selected=status.candidates.iter().find(|candidate|candidate.id==self.selected_candidate);
                    let selected_text=selected.map(candidate_label).unwrap_or_else(||"Selecione uma cópia".into());
                    let caption=ui.label("Candidato de recuperação");
                    egui::ComboBox::from_id_salt("storage-candidate").selected_text(selected_text).width(450.0).show_ui(ui,|ui| {
                        for candidate in &status.candidates {
                            if ui.add_enabled(candidate.recoverable,egui::Button::selectable(self.selected_candidate==candidate.id,candidate_label(candidate))).clicked() {
                                self.selected_candidate=candidate.id.clone();self.confirm_recovery=false;
                            }
                        }
                    }).response.labelled_by(caption.id);
                    if let Some(candidate)=status.candidates.iter().find(|candidate|candidate.id==self.selected_candidate) {
                        ui.label(format!("Restaurar projeto {} na revisão {}",candidate.project_id,candidate.revision));
                        ui.monospace(format!("Candidato: {}",candidate.sha256));
                        ui.label("Confira a revisão escolhida. O serviço verifica novamente o arquivo e todas as fontes antes de recuperar.");
                        ui.checkbox(&mut self.confirm_recovery,"Confirmo restaurar este candidato e preservar o arquivo atual");
                        if ui.add_enabled(candidate.recoverable&&self.confirm_recovery,egui::Button::new("Recuperar candidato selecionado")).clicked() {
                            actions.push(UiAction::Storage(Box::new(StorageAction::Recover {
                                candidate_id:candidate.id.clone(),inspection_sha256:status.inspection_sha256.clone(),confirm:self.confirm_recovery,
                            })));
                        }
                    }
                });
            }
            ui.collapsing("Identidade do diagnóstico",|ui| {
                if let Some(hash)=&status.primary_sha256 {ui.monospace(format!("Arquivo atual: {hash}"));}
                ui.monospace(format!("Diagnóstico: {}",status.inspection_sha256));
                ui.small("A confirmação deixa de valer se o arquivo ou qualquer candidato mudar.");
            });
        });
        if response.header_response.clicked() {
            self.expanded = !self.expanded;
        }
    }
}

fn candidate_label(candidate: &StorageCandidate) -> String {
    format!(
        "{} | revisão {} | {}",
        candidate.label,
        candidate.revision,
        candidate.sha256.chars().take(8).collect::<String>()
    )
}
