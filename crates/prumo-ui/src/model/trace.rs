use super::*;
use prumo_domain::canonical::ModelSnapshot;
use prumo_domain::dependencies::{Dependency, InspectionPayload, ResultState};

#[derive(Default)]
pub struct TraceView {
    pub snapshot: Option<ModelSnapshot>,
}

impl TraceView {
    pub(super) fn show(&mut self, ui: &mut Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.strong("Medidas geométricas e dependências");
        ui.label("Comprimentos e contagens são inspeções do modelo. Nenhuma peça recebe dimensionamento ou aprovação de engenharia.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Atualizar medidas do modelo").clicked() {
                actions.push(UiAction::command(Command::InspectModel {
                    expected_revision: project.revision,
                }));
            }
            if ui.button("Conferir snapshot e dependências").clicked() {
                actions.push(UiAction::RefreshModel);
            }
        });
        if let Some(snapshot) = &self.snapshot {
            ui.label(format!(
                "Snapshot v{} | projeto {} | revisão {}",
                snapshot.version, snapshot.project_id, snapshot.source_revision
            ));
            ui.monospace(format!("SHA-256: {}", snapshot.sha256));
            ui.label(format!(
                "{} fontes identificadas | {} bytes de entradas canônicas",
                snapshot.sources.len(),
                snapshot.canonical_json.len()
            ));
            ui.collapsing("Fontes do snapshot", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .id_salt("snapshot-sources")
                    .show(ui, |ui| {
                        for source in &snapshot.sources {
                            ui.label(format!("{:?}: {}", source.source.kind, source.source.id));
                            ui.small(&source.sha256);
                        }
                    });
            });
            ui.collapsing("Conteúdo canônico para conferência", |ui| {
                egui::ScrollArea::both()
                    .max_height(200.0)
                    .id_salt("canonical-content")
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&snapshot.canonical_json).monospace(),
                            )
                            .wrap(),
                        );
                    });
            });
        }
        ui.separator();
        let graph = &project.dependencies;
        let current = graph
            .results
            .iter()
            .filter(|result| result.state == ResultState::Current)
            .count();
        let stale = graph.results.len() - current;
        ui.label(format!(
            "{current} medidas atuais | {stale} medidas obsoletas"
        ));
        if graph.results.is_empty() {
            ui.label("Ainda não há medidas salvas. Use Atualizar medidas do modelo.");
        }
        for result in graph.results.iter().rev() {
            let (state, color) = match result.state {
                ResultState::Current => ("Atual", Color32::LIGHT_GREEN),
                ResultState::Stale => ("Obsoleta", Color32::YELLOW),
            };
            ui.collapsing(egui::RichText::new(format!("{state}: {} | revisão {}",result.name,result.created_revision)).color(color),|ui| {
                match &result.payload {
                    InspectionPayload::PhysicalLength {object_id,length_m}=>{ui.label(format!("Comprimento geométrico: {length_m:.6} m | peça {object_id}"));}
                    InspectionPayload::ModelSummary {physical_objects,visual_objects,analytical_members,materials,actions,networks,connections,total_length_m}=>{
                        ui.label(format!("{physical_objects} peças, {visual_objects} somente visuais, {analytical_members} barras, {materials} materiais e {actions} ações."));
                        ui.label(format!("{networks} redes, {connections} conexões explícitas. Comprimento total: {total_length_m:.6} m."));
                    }
                    InspectionPayload::ExternalEvidence {artifact_sha256,media_type}=>{ui.label(format!("Referência externa: {media_type}, SHA-256 {artifact_sha256}"));}
                }
                ui.small(format!("Produtor: {} | entrada da revisão {} | resultado {}",result.producer,result.input_revision,result.id));
                ui.small(format!("Snapshot de entrada: {}",result.input_snapshot_sha256));
                if let Some(revision)=result.invalidated_revision {ui.colored_label(Color32::YELLOW,format!("Obsoleta desde a revisão {revision}. Atualize as medidas após conferir a alteração."));}
                for dependency in &result.dependencies {
                    match dependency {
                        Dependency::Source {source,sha256}=>{ui.small(format!("Fonte {:?} {} | {}",source.kind,source.id,sha256));}
                        Dependency::Result {result_id,payload_sha256}=>{ui.small(format!("Depende da medida {result_id} | {payload_sha256}"));}
                    }
                }
            });
        }
        ui.collapsing("Causas de invalidação", |ui| {
            for event in graph.invalidations.iter().rev() {
                ui.label(format!(
                    "Revisão {} | {} | {}",
                    event.revision, event.actor, event.reason
                ));
                ui.small(format!(
                    "{} resultados afetados: {:?}",
                    event.result_ids.len(),
                    event.result_ids
                ));
                for change in &event.changes {
                    ui.small(format!(
                        "{:?}: {} | anterior {} | atual {}",
                        change.source.kind,
                        change.source.id,
                        change.before_sha256.as_deref().unwrap_or("ausente"),
                        change.after_sha256.as_deref().unwrap_or("removido")
                    ));
                }
            }
        });
    }
}
