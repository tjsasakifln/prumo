//! Normative metadata and capability views share the domain command boundary.
use crate::UiAction;
use egui::{Color32, RichText};
use prumo_domain::registry::{
    CapabilityState, CoverageAudit, GateReport, NormReferenceInput, NormativePackage,
    RegistryCatalog, compare_packages,
};
use prumo_domain::{Command, Project};

#[derive(Default)]
pub struct RegistryView {
    pub catalog: Option<RegistryCatalog>,
    pub audit: Option<CoverageAudit>,
    pub candidate: Option<NormativePackage>,
    pub gate: Option<GateReport>,
    pub reference: Option<NormReferenceInput>,
    pub selected_norm: String,
    pub filter: String,
    pub confirm_migration: bool,
    pub amendments_text: String,
    pub errata_text: String,
}

impl RegistryView {
    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.collapsing("Normas e capacidades", |ui| {
            ui.label("As referências públicas abaixo ainda não constituem regras de cálculo verificadas. A liberação exige texto autorizado, interpretação, domínio e evidências de revisão.");
            if ui.button("Atualizar catálogo e conferir cobertura").clicked() { actions.push(UiAction::RefreshRegistry); }
            if let Some(audit)=&self.audit {
                ui.label(format!("{} de {} famílias | {} capacidades | {} referências normativas",audit.present_families,audit.expected_families,audit.capabilities,audit.norms));
                if audit.errors.is_empty() { ui.label("Vínculos de cobertura conferidos."); }
                for finding in &audit.errors { ui.colored_label(Color32::LIGHT_RED,format!("{}: {}",finding.subject,finding.message)); }
                ui.collapsing(format!("{} pendências técnicas registradas",audit.pending.len()), |ui| {
                    egui::ScrollArea::vertical().max_height(180.0).id_salt("norm-pending").show(ui,|ui| {
                        for finding in &audit.pending { ui.label(format!("{}: {}",finding.subject,finding.message)); }
                    });
                });
            }
            ui.separator();
            self.package_view(ui,project,actions);
            ui.separator();
            self.reference_form(ui,project,actions);
            ui.separator();
            self.capability_view(ui,actions);
        });
    }

    fn package_view(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.strong("Pacote do projeto");
        if let Some(active) = &project.normative.active {
            ui.label(format!("Versão fixada: {}", active.catalog.version));
            ui.monospace(format!("Identidade: {}", active.hash));
            ui.label(format!(
                "{} pacotes anteriores preservados",
                project.normative.previous.len()
            ));
        } else {
            ui.label("Nenhum pacote foi fixado neste projeto.");
        }
        if project.normative.recalculation_required {
            ui.colored_label(
                Color32::YELLOW,
                "A mudança normativa exige recálculo e revisão das capacidades afetadas.",
            );
        }
        let Some(candidate) = &self.candidate else {
            return;
        };
        match &project.normative.active {
            None => {
                ui.label(format!(
                    "Catálogo disponível: {}. A fixação preserva essa versão e suas pendências.",
                    candidate.catalog.version
                ));
                if ui.button("Fixar pacote de referências").clicked() {
                    actions.push(UiAction::command(Command::PinNormativePackage {
                        expected_revision: project.revision,
                        package_hash: candidate.hash.clone(),
                    }));
                }
            }
            Some(active) if active.hash != candidate.hash => {
                ui.label(format!(
                    "Nova versão disponível: {}",
                    candidate.catalog.version
                ));
                ui.monospace(format!("Nova identidade: {}", candidate.hash));
                match compare_packages(active, candidate, project.revision + 1) {
                    Ok(impact) => {
                        ui.strong(format!(
                            "Impacto previsto: {} normas alteradas; {} capacidades afetadas.",
                            impact.changed_norm_ids.len(),
                            impact.affected_capability_ids.len()
                        ));
                        egui::ScrollArea::vertical().max_height(180.0).id_salt("migration-preview").show(ui,|ui| {
                            for id in &impact.changed_norm_ids {
                                let before=active.catalog.norms.iter().find(|norm|&norm.id==id);
                                let after=candidate.catalog.norms.iter().find(|norm|&norm.id==id);
                                ui.label(format!("Referência alterada: {id}"));
                                for (label,norm) in [("Anterior",before),("Nova",after)] {
                                    match norm {
                                        Some(norm)=>{ui.label(format!("{label}: {} | parte {} | edição {} | emendas: {} | erratas: {}",norm.designation,norm.part.as_deref().unwrap_or("não informada"),norm.edition.as_deref().unwrap_or("não informada"),norm.amendments.join("; "),norm.errata.join("; ")));ui.label(format!("{label}, domínios: {}",norm.domains.join(", ")));ui.label(format!("{label}, fontes: {}",norm.sources.join("; ")));}
                                        None=>{ui.label(format!("{label}: referência ausente"));}
                                    }
                                }
                            }
                            ui.label(format!("Capacidades que exigirão revisão: {}",if impact.affected_capability_ids.is_empty(){"nenhuma vinculada".into()}else{impact.affected_capability_ids.join(", ")}));
                        });
                    }
                    Err(error) => {
                        ui.colored_label(Color32::LIGHT_RED, error.to_string());
                        return;
                    }
                }
                ui.checkbox(
                    &mut self.confirm_migration,
                    "Confirmo a mudança do pacote e a necessidade de recálculo",
                );
                if ui
                    .add_enabled(
                        self.confirm_migration,
                        egui::Button::new("Migrar pacote do projeto"),
                    )
                    .clicked()
                {
                    actions.push(UiAction::command(Command::MigrateNormativePackage {
                        expected_revision: project.revision,
                        from_package_hash: active.hash.clone(),
                        to_package_hash: candidate.hash.clone(),
                        confirm: true,
                    }));
                    self.confirm_migration = false;
                }
            }
            Some(_) => {
                ui.label("O catálogo disponível corresponde ao pacote fixado.");
            }
        }
        for impact in project.normative.migrations.iter().rev().take(3) {
            ui.label(format!(
                "Revisão {}: {} normas alteradas; {} capacidades afetadas.",
                impact.revision,
                impact.changed_norm_ids.len(),
                impact.affected_capability_ids.len()
            ));
            ui.collapsing(
                format!("Ver impacto da revisão {}", impact.revision),
                |ui| {
                    for id in &impact.changed_norm_ids {
                        ui.label(id);
                    }
                    ui.label(format!(
                        "Capacidades: {}",
                        impact.affected_capability_ids.join(", ")
                    ));
                },
            );
        }
    }

    fn reference_form(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        actions: &mut Vec<UiAction>,
    ) {
        ui.collapsing("Cadastrar ou atualizar referência normativa",|ui| {
            if let Some(catalog)=&self.catalog {
                let before=self.selected_norm.clone();
                egui::ComboBox::from_label("Referência normativa").selected_text(if self.selected_norm.is_empty(){"Selecionar referência"}else{&self.selected_norm}).show_ui(ui,|ui| {
                    for norm in &catalog.norms {
                        ui.selectable_value(&mut self.selected_norm,norm.id.clone(),format!("{} | {}",norm.designation,norm.edition.as_deref().unwrap_or("Edição pendente")));
                    }
                });
                if before!=self.selected_norm {
                    self.reference=catalog.norms.iter().find(|norm|norm.id==self.selected_norm).map(NormReferenceInput::from);
                    self.amendments_text=self.reference.as_ref().map(|reference|reference.amendments.join("; ")).unwrap_or_default();
                    self.errata_text=self.reference.as_ref().map(|reference|reference.errata.join("; ")).unwrap_or_default();
                }
            }
            if ui.button("Nova referência").clicked() {
                self.selected_norm.clear();
                self.amendments_text.clear();
                self.errata_text.clear();
                self.reference=Some(NormReferenceInput {id:String::new(),designation:String::new(),part:None,edition:None,amendments:Vec::new(),errata:Vec::new(),title:String::new(),publisher:String::new(),source_url:String::new(),domain:String::new(),interpretation_owner_role:String::new()});
            }
            let Some(reference)=&mut self.reference else { return; };
            field(ui,"Identificador da referência",&mut reference.id);
            field(ui,"Designação",&mut reference.designation);
            optional_field(ui,"Parte",&mut reference.part);
            optional_field(ui,"Edição",&mut reference.edition);
            field(ui,"Título da norma",&mut reference.title);
            field(ui,"Publicador",&mut reference.publisher);
            ui.label("Fonte e domínio abaixo são os principais. Os demais vínculos existentes serão preservados e aparecem na comparação de versões.");
            field(ui,"Fonte pública",&mut reference.source_url);
            field(ui,"Domínio de aplicação",&mut reference.domain);
            field(ui,"Responsável pela interpretação",&mut reference.interpretation_owner_role);
            field(ui,"Emendas, separadas por ponto e vírgula",&mut self.amendments_text);
            field(ui,"Erratas, separadas por ponto e vírgula",&mut self.errata_text);
            ui.label("Este registro conserva metadados públicos. Ele não comprova acesso licenciado ou aprovação profissional e não substitui o pacote já fixado.");
            if ui.button("Registrar referência e nova versão").clicked() {
                reference.amendments=split_list(&self.amendments_text);
                reference.errata=split_list(&self.errata_text);
                actions.push(UiAction::command(Command::RegisterNormReference {expected_revision:project.revision,reference:reference.clone()}));
                self.confirm_migration=false;
            }
        });
    }

    fn capability_view(&mut self, ui: &mut egui::Ui, actions: &mut Vec<UiAction>) {
        ui.strong("Capacidades por função e domínio");
        field(ui, "Filtrar capacidades", &mut self.filter);
        let Some(catalog) = &self.catalog else {
            return;
        };
        let needle = self.filter.to_lowercase();
        let filtered: Vec<_> = catalog
            .capabilities
            .iter()
            .filter(|cap| {
                format!(
                    "{} {} {} {}",
                    cap.id, cap.title, cap.discipline, cap.product
                )
                .to_lowercase()
                .contains(&needle)
            })
            .collect();
        ui.label(format!(
            "{} registros. Selecione uma capacidade para conferir o impedimento de emissão.",
            filtered.len()
        ));
        egui::ScrollArea::vertical()
            .max_height(250.0)
            .id_salt("capabilities")
            .show(ui, |ui| {
                for capability in filtered {
                    ui.push_id(&capability.id, |ui| {
                        if ui
                            .button(format!("{} | {}", capability.id, capability.title))
                            .clicked()
                        {
                            actions.push(UiAction::AssessCapability(capability.id.clone()));
                        }
                        ui.label(format!(
                            "Referência: {} / {} | {}",
                            capability.product,
                            capability.module,
                            capability
                                .reference_version
                                .as_deref()
                                .unwrap_or("Versão não informada")
                        ));
                        ui.label(&capability.owner_role);
                        ui.label(format!(
                            "Método: {} | Material: {} | Issues: {:?}",
                            metadata_label(&capability.method),
                            metadata_label(&capability.material),
                            capability.implementation_issues
                        ));
                        ui.label(format!(
                            "Normas: {}",
                            if capability.norm_ids.is_empty() {
                                "Sem vínculo normativo declarado".into()
                            } else {
                                capability.norm_ids.join(", ")
                            }
                        ));
                        let state = match capability.state {
                            CapabilityState::Planned => "Planejado",
                            CapabilityState::Experimental => "Experimental",
                            CapabilityState::Verified => "Verificado",
                            CapabilityState::Released => "Liberado",
                        };
                        ui.label(format!("Estado do Prumo: {state}"));
                        ui.small(&capability.version_note);
                        ui.separator();
                    });
                }
            });
        if let Some(gate) = &self.gate {
            ui.label(
                RichText::new(format!("Capacidade selecionada: {}", gate.capability_id)).strong(),
            );
            for reason in &gate.reasons {
                ui.colored_label(Color32::YELLOW, &reason.message);
            }
            ui.label("Nenhum cálculo ou documento final é produzido por esta conferência.");
        }
    }
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        let name = ui.label(label);
        ui.add(egui::TextEdit::singleline(value).desired_width(380.0))
            .labelled_by(name.id);
    });
}
fn optional_field(ui: &mut egui::Ui, label: &str, value: &mut Option<String>) {
    let mut text = value.clone().unwrap_or_default();
    field(ui, label, &mut text);
    *value = if text.trim().is_empty() {
        None
    } else {
        Some(text)
    };
}
fn split_list(text: &str) -> Vec<String> {
    text.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

fn metadata_label(value: &str) -> &str {
    match value {
        "pending" => "a definir",
        "unspecified" => "não definido",
        "planned" => "planejado",
        _ => value,
    }
}
