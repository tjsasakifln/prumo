//! Typed batch preparation and audited history, usable without drag gestures.
use crate::UiAction;
use prumo_domain::transactions::{
    TransactionHistory, TransactionHistoryRequest, TransactionPreview, TransactionRequest,
};
use prumo_domain::{Command, Project};

#[derive(Default)]
pub struct TransactionView {
    pub enabled: bool,
    pub pending: Option<TransactionRequest>,
    pub preview: Option<TransactionPreview>,
    pub history: Option<TransactionHistory>,
}

impl TransactionView {
    pub fn should_stage(&self, command: &Command) -> bool {
        self.enabled
            && !matches!(
                command,
                Command::Create { .. } | Command::InspectModel { .. }
            )
    }

    pub fn stage(&mut self, project: &Project, command: Command) -> Result<(), String> {
        if self.pending.as_ref().is_some_and(|request| {
            request.project_id != project.id || request.expected_revision != project.revision
        }) {
            return Err("O lote pertence a outra revisão. Descarte o lote e confira as alterações antes de preparar novamente.".into());
        }
        let mut operations = self
            .pending
            .as_ref()
            .map(|request| request.operations.clone())
            .unwrap_or_default();
        if operations.len() >= prumo_domain::transactions::MAX_TRANSACTION_OPERATIONS {
            return Err(
                "O lote já contém 128 operações. Confira e aplique antes de continuar.".into(),
            );
        }
        operations.push(command);
        self.pending = Some(TransactionRequest::new(project, operations));
        self.preview = None;
        Ok(())
    }

    pub fn clear_pending(&mut self) {
        self.pending = None;
        self.preview = None;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.collapsing("Alterações em lote e histórico", |ui| {
            ui.checkbox(&mut self.enabled, "Preparar alterações em lote antes de salvar");
            if self.enabled {
                ui.label("As próximas alterações de modelagem entram no lote. Confira a prévia e aplique tudo de uma vez.");
            }
            let mut discard = false;
            if let Some(request) = &self.pending {
                let current = request.project_id == project.id && request.expected_revision == project.revision;
                ui.label(format!("{} operações preparadas na revisão {}", request.operations.len(), request.expected_revision));
                if !current { ui.colored_label(egui::Color32::YELLOW, "O projeto mudou. Descarte este lote e confira a nova revisão antes de continuar."); }
                for (index, command) in request.operations.iter().enumerate() {
                    ui.label(format!("{}. {}", index + 1, command_label(command)));
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(current, egui::Button::new("Conferir prévia do lote")).clicked() {
                        actions.push(UiAction::PreviewTransaction(Box::new(request.clone())));
                    }
                    discard = ui.button("Descartar lote").clicked();
                });
            }
            if discard { self.clear_pending(); }
            if let Some(preview) = &self.preview {
                let current = preview.project_id == project.id && preview.base_revision == project.revision;
                ui.separator();
                ui.strong("Prévia conferível");
                let created = preview.effects.iter().filter(|effect| effect.before_sha256.is_none()).count();
                let removed = preview.effects.iter().filter(|effect| effect.after_sha256.is_none()).count();
                ui.label(format!("{} fontes criadas, {} alteradas, {} removidas; {} resultados afetados.", created, preview.effects.len() - created - removed, removed, preview.invalidated_result_ids.len()));
                for effect in &preview.effects {
                    ui.label(format!("{:?}: {}", effect.source.kind, effect.source.id));
                }
                ui.small(format!("Plano: {}", preview.plan_sha256));
                if ui.add_enabled(current, egui::Button::new("Aplicar lote conferido")).clicked() {
                    actions.push(UiAction::ApplyTransaction(preview.plan_sha256.clone()));
                }
            }
            ui.separator();
            if ui.button("Atualizar histórico de alterações").clicked() { actions.push(UiAction::RefreshTransactions); }
            if let Some(history) = &self.history {
                let current = history.project_id == project.id && history.revision == project.revision;
                ui.horizontal(|ui| {
                    if ui.add_enabled(current && history.undo_command_id.is_some(), egui::Button::new("Desfazer última alteração")).clicked()
                        && let Some(id) = history.undo_command_id {
                        actions.push(UiAction::UndoTransaction(Box::new(TransactionHistoryRequest::new(project, id))));
                    }
                    if ui.add_enabled(current && history.redo_command_id.is_some(), egui::Button::new("Refazer alteração")).clicked()
                        && let Some(id) = history.redo_command_id {
                        actions.push(UiAction::RedoTransaction(Box::new(TransactionHistoryRequest::new(project, id))));
                    }
                });
                ui.small("Desfazer e refazer criam novas revisões e conservam autoria, relações e unidades. Resultados afetados exigem nova inspeção.");
                for receipt in history.receipts.iter().rev().take(8) {
                    ui.label(format!("Revisão {} | {} | {:?} | {} fontes afetadas", receipt.committed_revision, receipt.actor, receipt.kind, receipt.effects.len()));
                }
            }
        });
    }
}

fn command_label(command: &Command) -> String {
    match command {
        Command::Create { name, .. } => format!("Criar projeto: {name}"),
        Command::AddMember { name, .. } => format!("Adicionar peça: {name}"),
        Command::RenameProject { name, .. } => format!("Renomear projeto: {name}"),
        Command::AddEnvironment { name, .. } => format!("Adicionar ambiente: {name}"),
        Command::RegisterNormReference { reference, .. } => {
            format!("Registrar referência: {}", reference.designation)
        }
        Command::PinNormativePackage { .. } => "Fixar pacote normativo".into(),
        Command::MigrateNormativePackage { .. } => "Migrar pacote normativo".into(),
        Command::EditModel { edit, .. } => edit_label(edit),
        Command::InspectModel { .. } => "Inspecionar medidas do modelo".into(),
    }
}

fn edit_label(edit: &prumo_domain::model::ModelEdit) -> String {
    use prumo_domain::model::ModelEdit;
    match edit {
        ModelEdit::AddPhysical { name, .. } => format!("Adicionar elemento: {name}"),
        ModelEdit::SetPhysicalGeometry { object_id, .. } => {
            format!("Alterar geometria do elemento {object_id}")
        }
        ModelEdit::SetPhysicalRole { object_id, .. } => {
            format!("Alterar participação analítica do elemento {object_id}")
        }
        ModelEdit::SetPhysicalMaterial { object_id, .. } => {
            format!("Alterar material do elemento {object_id}")
        }
        ModelEdit::SetDisplayUnit { unit } => format!("Exibir medidas em {}", unit.symbol()),
        ModelEdit::AddAnalyticalNode { name, .. } => format!("Adicionar nó: {name}"),
        ModelEdit::AddAnalyticalMember { name, .. } => format!("Adicionar barra analítica: {name}"),
        ModelEdit::AddMaterial { material } | ModelEdit::SetMaterial { material, .. } => {
            format!("Registrar material: {}", material.name)
        }
        ModelEdit::AddNetwork { name, .. } => format!("Adicionar rede ou circuito: {name}"),
        ModelEdit::AddPort { object_id, .. } => format!("Adicionar porta do elemento {object_id}"),
        ModelEdit::AssignPort { .. } => "Vincular porta à rede".into(),
        ModelEdit::ConnectPorts { .. } => "Conectar portas da rede".into(),
        ModelEdit::DisconnectPorts { .. } => "Desconectar portas da rede".into(),
        ModelEdit::CopyPhysical { name, .. } => format!("Copiar elemento: {name}"),
        ModelEdit::MergePhysical { name, .. } => format!("Unir elementos: {name}"),
        ModelEdit::SplitPhysical { names, .. } => {
            format!("Dividir elemento em {} e {}", names[0], names[1])
        }
        ModelEdit::RigidMove { .. } => "Mover elementos preservando relações".into(),
        ModelEdit::ChangeEngineeringFrame { .. } => "Alterar referencial de engenharia".into(),
        ModelEdit::SetGeoreference { .. } => "Alterar georreferenciamento".into(),
        _ => "Atualizar propriedades, vínculos ou dados rastreáveis do modelo".into(),
    }
}
