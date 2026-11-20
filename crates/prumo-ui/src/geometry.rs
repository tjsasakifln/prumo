//! Numeric geometry review and traceable verification without gesture requirements.
use crate::{UiAction, numeric};
use prumo_domain::{Command, Project, geometry::*, model::ModelEdit};

pub struct GeometryView {
    pub report: Option<GeometryReport>,
    pub preview: Option<GeometryRepairPreview>,
    pub face_report: Option<ProjectFaceInspection>,
    pub verification: Option<serde_json::Value>,
    pub tolerances: TolerancePolicy,
    loaded_revision: Option<String>,
    object_index: usize,
    endpoint: Endpoint,
    target_m: [f64; 3],
    reason: String,
    repair_mode: usize,
    network_index: usize,
    from_port_index: usize,
    to_port_index: usize,
    environment_index: usize,
    network_filter_index: usize,
    discipline_index: usize,
    face_json: String,
}
impl Default for GeometryView {
    fn default() -> Self {
        Self {
            report: None,
            preview: None,
            face_report: None,
            verification: None,
            tolerances: Default::default(),
            loaded_revision: None,
            object_index: 0,
            endpoint: Endpoint::End,
            target_m: [0.; 3],
            reason: "Ajuste geométrico conferido".into(),
            repair_mode: 0,
            network_index: 0,
            from_port_index: 0,
            to_port_index: 1,
            environment_index: 0,
            network_filter_index: 0,
            discipline_index: 0,
            face_json:
                r#"{"vertices_m":[[0,0,0],[2,0,0],[2,1,0],[0,1,0]],"expected_normal":[0,0,1]}"#
                    .into(),
        }
    }
}

impl GeometryView {
    pub fn clear_evidence(&mut self) {
        self.report = None;
        self.preview = None;
        self.face_report = None;
        self.verification = None;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        let key = format!("{}:{}", project.id, project.revision);
        if self.loaded_revision.as_ref() != Some(&key) {
            if let Ok(value) = project_tolerances(project) {
                self.tolerances = value;
            }
            self.loaded_revision = Some(key);
        }
        ui.collapsing("Geometria e conexões", |ui| {
            ui.label("Contato visual e ligação são estados diferentes. Correções mostram o impacto antes de salvar.");
            numeric::begin(ui);numeric::context(ui,"geometry-tolerances".into());
            ui.horizontal_wrapped(|ui| {
                numeric::field(ui,"Seleção (m)",&mut self.tolerances.selection_m);
                numeric::field(ui,"União topológica (m)",&mut self.tolerances.topology_m);
                numeric::field(ui,"Malha (m)",&mut self.tolerances.mesh_m);
                numeric::field(ui,"Análise (m)",&mut self.tolerances.analysis_m);
            });
            if ui.add_enabled(numeric::errors(ui).is_empty(),egui::Button::new("Salvar tolerâncias")).clicked() {
                actions.push(UiAction::command(Command::EditModel{expected_revision:project.revision,edit:ModelEdit::SetGeometryTolerances{tolerances:self.tolerances}}));
            }
            let tolerances_saved=numeric::errors(ui).is_empty()&&project_tolerances(project).is_ok_and(|value|value==self.tolerances);
            if !tolerances_saved{self.report=None;self.preview=None;ui.label("Salve as tolerâncias em edição antes de inspecionar ou corrigir.");}
            let old_filter=(self.discipline_index,self.environment_index,self.network_filter_index);
            let disciplines=[prumo_domain::Discipline::Structure,prumo_domain::Discipline::WaterSanitation,prumo_domain::Discipline::Gas,prumo_domain::Discipline::FireProtection,prumo_domain::Discipline::Electrical,prumo_domain::Discipline::Photovoltaic,prumo_domain::Discipline::LightningProtection,prumo_domain::Discipline::Cabling,prumo_domain::Discipline::Hvac];
            let labels=["Todas as disciplinas","Estrutura","Hidrossanitária","Gás","Incêndio","Elétrica","Fotovoltaica","SPDA","Cabeamento","Climatização"];
            egui::ComboBox::from_label("Disciplina da inspeção").selected_text(labels[self.discipline_index.min(9)]).show_ui(ui,|ui| { for (i,label) in labels.iter().enumerate(){ui.selectable_value(&mut self.discipline_index,i,*label);} });
            egui::ComboBox::from_label("Ambiente da inspeção").selected_text(if self.environment_index==0 {"Todos os ambientes"} else {project.environments.get(self.environment_index-1).map_or("Todos os ambientes",|e|e.name.as_str())}).show_ui(ui,|ui|{ui.selectable_value(&mut self.environment_index,0,"Todos os ambientes");for (i,e) in project.environments.iter().enumerate(){ui.selectable_value(&mut self.environment_index,i+1,&e.name);}});
            egui::ComboBox::from_label("Rede da inspeção").selected_text(if self.network_filter_index==0 {"Todas as redes"} else {project.networks.get(self.network_filter_index-1).map_or("Todas as redes",|n|n.name.as_str())}).show_ui(ui,|ui|{ui.selectable_value(&mut self.network_filter_index,0,"Todas as redes");for(i,n) in project.networks.iter().enumerate(){ui.selectable_value(&mut self.network_filter_index,i+1,&n.name);}});
            if old_filter!=(self.discipline_index,self.environment_index,self.network_filter_index){self.report=None;}
            if ui.add_enabled(tolerances_saved,egui::Button::new("Inspecionar geometria")).clicked(){actions.push(UiAction::InspectGeometry(GeometryFilter{
                discipline:self.discipline_index.checked_sub(1).and_then(|i|disciplines.get(i).copied()),
                environment_id:self.environment_index.checked_sub(1).and_then(|i|project.environments.get(i).map(|e|e.id)),
                network_id:self.network_filter_index.checked_sub(1).and_then(|i|project.networks.get(i).map(|n|n.id)),
            }));}
            if let Some(report)=&self.report {
                if report.project_id!=project.id || report.revision!=project.revision {ui.colored_label(egui::Color32::YELLOW,"Inspeção desatualizada. Execute novamente.");}
                ui.label(format!("{} peças; {} ocorrências na revisão {}",report.object_count,report.findings.len(),report.revision));
                for finding in &report.findings {
                    let name=|id: uuid::Uuid| project.objects.iter().find(|o|o.id==id).map_or_else(||id.to_string(),|o|o.name.clone());
                    match finding {
                        GeometryFinding::ShortElement{object_id,length_m}=>{ui.label(format!("{}: vão curto, {length_m:.9} m",name(*object_id)));}
                        GeometryFinding::Duplicate{first_id,second_id}=>{ui.label(format!("Possível duplicidade: {} / {}",name(*first_id),name(*second_id)));}
                        GeometryFinding::NearContact{first_id,second_id,distance_m,explicitly_connected}=>{ui.label(format!("{} / {}: distância {distance_m:.9} m, {}",name(*first_id),name(*second_id),if *explicitly_connected {"ligação explícita registrada"}else{"sem ligação explícita"}));}
                        GeometryFinding::Crossing{first_id,second_id,position_m}=>{ui.label(format!("Cruzamento visual: {} / {}, posição {position_m:?}. Sem nova ligação.",name(*first_id),name(*second_id)));}
                        GeometryFinding::Overlap{first_id,second_id,start_m,end_m}=>{ui.label(format!("Sobreposição: {} / {}, de {start_m:?} a {end_m:?} m. Sem nova ligação.",name(*first_id),name(*second_id)));}
                    }
                }
            }
            ui.separator();ui.strong("Preparar correção");
            ui.horizontal(|ui|{ui.selectable_value(&mut self.repair_mode,0,"Ajustar extremidade");ui.selectable_value(&mut self.repair_mode,1,"Conectar portas próximas");});
            let mut repair=None;
            numeric::begin(ui);numeric::context(ui,"geometry-repair".into());
            if self.repair_mode==0 {
                egui::ComboBox::from_label("Peça para corrigir").selected_text(project.objects.get(self.object_index).map_or("Selecione uma peça",|o|o.name.as_str())).show_ui(ui,|ui|{for(i,o)in project.objects.iter().enumerate(){ui.selectable_value(&mut self.object_index,i,&o.name);}});
                ui.horizontal(|ui|{ui.selectable_value(&mut self.endpoint,Endpoint::Start,"Início");ui.selectable_value(&mut self.endpoint,Endpoint::End,"Fim");});
                if let Some(object)=project.objects.get(self.object_index) {
                    if ui.button("Carregar posição atual").clicked(){self.target_m=match self.endpoint{Endpoint::Start=>object.start_m.as_array(),Endpoint::End=>object.end_m.as_array()};}
                    ui.horizontal(|ui|{numeric::field(ui,"X (m)",&mut self.target_m[0]);numeric::field(ui,"Y (m)",&mut self.target_m[1]);numeric::field(ui,"Z (m)",&mut self.target_m[2]);});
                    repair=Some(GeometryRepair::SnapEndpoint{object_id:object.id,endpoint:self.endpoint,target_m:self.target_m,reason:self.reason.clone()});
                }
            }else{
                egui::ComboBox::from_label("Rede para conectar").selected_text(project.networks.get(self.network_index).map_or("Selecione uma rede",|n|n.name.as_str())).show_ui(ui,|ui|{for(i,n)in project.networks.iter().enumerate(){ui.selectable_value(&mut self.network_index,i,&n.name);}});
                if let Some(network)=project.networks.get(self.network_index){
                    let ports:Vec<_>=project.ports.iter().filter(|p|network.port_ids.contains(&p.id)).collect();
                    for (caption,index)in[("Porta de origem",&mut self.from_port_index),("Porta de destino",&mut self.to_port_index)]{
                        egui::ComboBox::from_label(caption).selected_text(ports.get(*index).map_or_else(||caption.into(),|p|p.id.to_string())).show_ui(ui,|ui|{for(i,p)in ports.iter().enumerate(){ui.selectable_value(index,i,p.id.to_string());}});
                    }
                    if let(Some(a),Some(b))=(ports.get(self.from_port_index),ports.get(self.to_port_index)){repair=Some(GeometryRepair::ConnectPorts{network_id:network.id,from_port:a.id,to_port:b.id,reason:self.reason.clone()});}
                }
            }
            let label=ui.label("Motivo da correção");ui.text_edit_singleline(&mut self.reason).labelled_by(label.id);
            // The reason is part of the bound plan, including edits made in this frame.
            if let Some(r)=&mut repair{match r {GeometryRepair::SnapEndpoint{reason,..}|GeometryRepair::ConnectPorts{reason,..}=>*reason=self.reason.clone()}}
            if !numeric::errors(ui).is_empty(){self.preview=None;}
            if self.preview.as_ref().is_some_and(|p|repair.as_ref().is_none_or(|r|p.repairs.as_slice()!=std::slice::from_ref(r))){self.preview=None;}
            if ui.add_enabled(tolerances_saved&&repair.is_some()&&numeric::errors(ui).is_empty(),egui::Button::new("Conferir correção")).clicked()&&let Some(repair)=repair{actions.push(UiAction::PreviewGeometry(vec![repair]));}
            if let Some(preview)=&self.preview{
                for d in &preview.dimensions_changed{ui.label(format!("Comprimento: de {:.9} m para {:.9} m",d.before_m,d.after_m));}
                ui.label(format!("Novas conexões explícitas: {}",preview.explicit_connections_added));
                if ui.add_enabled(preview.project_id==project.id&&preview.revision==project.revision&&numeric::errors(ui).is_empty(),egui::Button::new("Aplicar correção conferida")).clicked(){actions.push(UiAction::command(Command::EditModel{expected_revision:project.revision,edit:ModelEdit::ApplyGeometryRepair{plan_hash:preview.plan_hash.clone(),repairs:preview.repairs.clone()}}));}
            }
            ui.collapsing("Conferir face por coordenadas",|ui|{
                ui.label("Cole vértices em metros e a normal esperada. A conferência não insere uma face no modelo.");
                let label=ui.label("Coordenadas da face (JSON)");if ui.add(egui::TextEdit::multiline(&mut self.face_json).desired_rows(3)).labelled_by(label.id).changed(){self.face_report=None;}
                let parsed=serde_json::from_str::<FaceInput>(&self.face_json);
                if let Err(e)=&parsed{ui.colored_label(egui::Color32::LIGHT_RED,e.to_string());}
                if ui.add_enabled(tolerances_saved&&parsed.is_ok(),egui::Button::new("Conferir orientação e forma")).clicked()&&let Ok(face)=parsed{actions.push(UiAction::InspectFace(face));}
                if let Some(report)=&self.face_report{let r=&report.inspection;ui.label(format!("Revisão {} · tolerância topológica {:.9} m",report.revision,report.tolerances.topology_m));ui.label(format!("Área {:.9} m²; normal {:?}; {}",r.area_m2,r.normal,if r.valid{"geometria consistente"}else{"correção necessária"}));for problem in &r.problems{ui.label(problem);}}
            });
        });
        ui.collapsing("Verificação com casos de referência",|ui|{
            ui.label("Ensaios independentes de viga, treliça e pórtico. Esta bateria verifica o software e não calcula o projeto aberto.");
            if ui.button("Executar casos de referência").clicked(){actions.push(UiAction::RunVerification);}
            if let Some(report)=&self.verification{
                let evidence=&report["evidence"];
                ui.strong(if evidence["verification_passed"].as_bool()==Some(true){"Bateria aprovada"}else{"Bateria com falhas ou bloqueios"});
                if let Some(cases)=evidence["cases"].as_array(){for case in cases{ui.label(format!("{}: {}",case["prepared"]["protocol"]["id"].as_str().unwrap_or("Caso"),case["decision"].as_str().unwrap_or("sem decisão")));}}
                ui.small("Não libera dimensionamento, emissão técnica ou aprovação profissional.");
                ui.collapsing("Dados completos e procedência",|ui|{if let Ok(mut text)=serde_json::to_string_pretty(report){ui.add(egui::TextEdit::multiline(&mut text).desired_rows(12).interactive(false));}});
            }
        });
    }
}
