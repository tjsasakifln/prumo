//! Catalogue editing uses the shared typed commands; analysis runs in the service.
use crate::{UiAction, numeric};
use prumo_domain::{Command, Project, catalogue::*, model::ModelEdit};
use std::collections::BTreeMap;
use uuid::Uuid;

pub struct CatalogueView {
    pub section_report: Option<serde_json::Value>,
    pub material_report: Option<serde_json::Value>,
    selected_id: Option<Uuid>,
    object_index: usize,
    shape_index: usize,
    name: String,
    width: f64,
    height: f64,
    thickness: f64,
    web: f64,
    publisher: String,
    reference: String,
    edition: String,
    source_revision: String,
    data_license: String,
    entry_json: String,
    conditions_json: String,
    customization_reason: String,
}
impl Default for CatalogueView {
    fn default() -> Self {
        Self {
            section_report: None,
            material_report: None,
            selected_id: None,
            object_index: 0,
            shape_index: 0,
            name: "Seção 01".into(),
            width: 0.2,
            height: 0.4,
            thickness: 0.02,
            web: 0.01,
            publisher: "Projeto".into(),
            reference: "Dimensões informadas no editor".into(),
            edition: "1".into(),
            source_revision: "1".into(),
            data_license: "Dados próprios".into(),
            entry_json: String::new(),
            conditions_json: "{}".into(),
            customization_reason: String::new(),
        }
    }
}
impl CatalogueView {
    pub fn clear_evidence(&mut self) {
        self.section_report = None;
        self.material_report = None;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.collapsing("Catálogos de materiais e seções", |ui| {
            let catalogue=match project_catalogue(project) {Ok(value)=>value, Err(error)=>{ui.colored_label(egui::Color32::LIGHT_RED,error.to_string());return;}};
            ui.label("Cada entrada conserva origem, unidades e versões. As associações das peças usam a versão atual da entrada e invalidam resultados dependentes quando ela muda.");
            ui.collapsing("Criar seção por dimensões", |ui| {
                field(ui,"Nome da seção",&mut self.name);
                let shapes=["Retangular","Circular","Perfil I","Caixão"];
                egui::ComboBox::from_label("Formato da seção").selected_text(shapes[self.shape_index]).show_ui(ui,|ui| {for(i,label)in shapes.iter().enumerate(){ui.selectable_value(&mut self.shape_index,i,*label);}});
                numeric::begin(ui); numeric::context(ui,"catalogue-shape-values".into());
                numeric::field(ui,if self.shape_index==1{"Diâmetro (m)"}else{"Largura (m)"},&mut self.width);
                if self.shape_index!=1{numeric::field(ui,"Altura (m)",&mut self.height);}
                if self.shape_index>=2{numeric::field(ui,if self.shape_index==2{"Espessura da mesa (m)"}else{"Espessura da parede (m)"},&mut self.thickness);}
                if self.shape_index==2{numeric::field(ui,"Espessura da alma (m)",&mut self.web);}
                field(ui,"Responsável ou fabricante",&mut self.publisher);
                field(ui,"Referência de origem",&mut self.reference);
                field(ui,"Edição da fonte",&mut self.edition);
                field(ui,"Revisão da fonte",&mut self.source_revision);
                field(ui,"Licença dos dados",&mut self.data_license);
                let shape=match self.shape_index {
                    1=>SectionShape::Circle{diameter_m:self.width},
                    2=>SectionShape::ISection{width_m:self.width,height_m:self.height,flange_m:self.thickness,web_m:self.web},
                    3=>SectionShape::Box{width_m:self.width,height_m:self.height,wall_m:self.thickness},
                    _=>SectionShape::Rectangle{width_m:self.width,height_m:self.height},
                };
                let entry=CatalogEntry{id:Uuid::new_v4(),revision:1,name:self.name.clone(),source:CatalogSource{publisher:self.publisher.clone(),reference:self.reference.clone(),edition:self.edition.clone(),revision:self.source_revision.clone(),data_license:self.data_license.clone(),original_record:None},applicability:vec![],customization:None,payload:CatalogPayload::Section{shape}};
                let valid=entry.validate();
                if let Err(error)=&valid{ui.colored_label(egui::Color32::LIGHT_RED,error.to_string());}
                if ui.add_enabled(valid.is_ok()&&numeric::errors(ui).is_empty(),egui::Button::new("Salvar nova seção")).clicked(){
                    self.selected_id=Some(entry.id);
                    actions.push(UiAction::command(Command::EditModel{expected_revision:project.revision,edit:ModelEdit::PutCatalogEntry{entry}}));
                }
            });

            egui::ComboBox::from_label("Entrada do catálogo").selected_text(self.selected_id.and_then(|id|catalogue.latest(id).ok()).map_or("Selecione uma entrada",|entry|entry.name.as_str())).show_ui(ui,|ui| {
                for(id,versions)in &catalogue.entries{if let Some(version)=versions.last()&&ui.selectable_value(&mut self.selected_id,Some(*id),format!("{} · v{}",version.entry.name,version.entry.revision)).changed(){self.clear_evidence();}}
            });
            if let Some(entry)=self.selected_id.and_then(|id|catalogue.latest(id).ok()) {
                ui.label(format!("Origem: {} · {} · edição {} · revisão {}",entry.source.publisher,entry.source.reference,entry.source.edition,entry.source.revision));
                ui.label(format!("Licença: {}",entry.source.data_license));
                if let Some(custom)=&entry.customization{ui.label(format!("Customização da entrada {} v{}: {}",custom.base_id,custom.base_revision,custom.reason));}
                match &entry.payload {
                    CatalogPayload::Section{..}=>{if ui.button("Calcular propriedades da seção").clicked(){actions.push(UiAction::AnalyseCatalogueSection(entry.id));}}
                    CatalogPayload::Material{..}=>{
                        ui.label("Condições de consulta com grandeza, decimal original, unidade e natureza do valor.");
                        let label=ui.label("Condições do material (JSON)");
                        if ui.add(egui::TextEdit::multiline(&mut self.conditions_json).desired_rows(3)).labelled_by(label.id).changed(){self.material_report=None;}
                        let parsed=serde_json::from_str::<BTreeMap<PhysicalQuantity,PhysicalValue>>(&self.conditions_json);
                        if let Err(error)=&parsed{ui.colored_label(egui::Color32::LIGHT_RED,error.to_string());}
                        if ui.add_enabled(parsed.is_ok(),egui::Button::new("Consultar estado do material")).clicked()&&let Ok(conditions)=parsed{actions.push(UiAction::InspectCatalogueMaterial(entry.id,conditions));}
                    }
                    CatalogPayload::Properties{values}=>{for(name,value)in values{ui.label(format!("{name}: {} {:?} ({:?})",value.decimal,value.unit,value.basis));}}
                }
                ui.horizontal(|ui|{
                    if ui.button("Preparar próxima versão").clicked(){let mut draft=entry.clone();if let Some(revision)=draft.revision.checked_add(1){draft.revision=revision;self.entry_json=serde_json::to_string_pretty(&draft).unwrap_or_default();}}
                    if ui.add_enabled(!self.customization_reason.trim().is_empty(),egui::Button::new("Preparar customização")).clicked()&&let Ok(hash)=entry.sha256(){let mut draft=entry.clone();draft.id=Uuid::new_v4();draft.revision=1;draft.customization=Some(CatalogCustomization{base_id:entry.id,base_revision:entry.revision,base_sha256:hash,reason:self.customization_reason.clone()});self.entry_json=serde_json::to_string_pretty(&draft).unwrap_or_default();
                    }
                });
                field(ui,"Motivo da customização",&mut self.customization_reason);
                ui.collapsing("Versões e dados da fonte",|ui|{if let Some(versions)=catalogue.entries.get(&entry.id){for version in versions{ui.label(format!("v{} · projeto r{} · {}",version.entry.revision,version.project_revision,version.actor));json_view(ui,&serde_json::to_value(version).unwrap_or_default());}}});
                ui.separator();
                egui::ComboBox::from_label("Peça para associar").selected_text(project.objects.get(self.object_index).map_or("Selecione uma peça",|o|o.name.as_str())).show_ui(ui,|ui|{for(i,object)in project.objects.iter().enumerate(){ui.selectable_value(&mut self.object_index,i,&object.name);}});
                if let Some(object)=project.objects.get(self.object_index){
                    if ui.button("Associar entrada à peça").clicked(){actions.push(UiAction::command(Command::EditModel{expected_revision:project.revision,edit:ModelEdit::AssignCatalogEntry{object_id:object.id,entry_id:entry.id}}));}
                    if let Ok(assignments)=assignments(object){for(kind,id)in assignments{ui.horizontal(|ui|{ui.label(format!("{:?}: {}",kind,catalogue.latest(id).map_or("Entrada ausente",|entry|entry.name.as_str())));if ui.button(format!("Remover associação {kind:?}")).clicked(){actions.push(UiAction::command(Command::EditModel{expected_revision:project.revision,edit:ModelEdit::ClearCatalogAssignment{object_id:object.id,kind}}));}});}}
                }
            }

            ui.collapsing("Importar ou editar entrada completa",|ui|{
                ui.label("Use o registro completo para materiais isotrópicos ou ortotrópicos, estados por condição, polígonos, seções compostas e propriedades. Os valores físicos conservam o decimal original como texto.");
                if ui.button("Preparar exemplo de material para preencher").clicked(){self.entry_json=material_example();}
                let label=ui.label("Entrada de catálogo (JSON)");
                ui.add(egui::TextEdit::multiline(&mut self.entry_json).desired_rows(10)).labelled_by(label.id);
                let entry=CatalogEntry::from_json(self.entry_json.as_bytes());
                if !self.entry_json.is_empty()&&let Err(error)=&entry{ui.colored_label(egui::Color32::LIGHT_RED,error.to_string());}
                if ui.add_enabled(entry.is_ok(),egui::Button::new("Salvar entrada conferida")).clicked()&&let Ok(entry)=entry{
                    self.selected_id=Some(entry.id);
                    actions.push(UiAction::command(Command::EditModel{expected_revision:project.revision,edit:ModelEdit::PutCatalogEntry{entry}}));
                }
            });
            if let Some(report)=&self.section_report {
                ui.separator();ui.strong("Propriedades da seção");
                if report["revision"].as_u64()!=Some(project.revision){ui.colored_label(egui::Color32::YELLOW,"Relatório de uma revisão anterior. Calcule novamente.");}
                let geometry=&report["analysis"]["geometry"];
                for(key,label,unit)in[("area_m2","Área","m²"),("ixx_m4","Ixx","m⁴"),("iyy_m4","Iyy","m⁴"),("ixy_m4","Ixy","m⁴")]{if let Some(value)=geometry[key].as_f64(){ui.label(format!("{label}: {value:.8e} {unit}"));}}
                ui.label(format!("Centroide (m): {}",geometry["centroid_m"]));
                let torsion=&report["analysis"]["torsion"];
                if let Some(value)=torsion["j_m4"].as_f64(){ui.label(format!("Constante de torção J: {value:.8e} m⁴"));}else{ui.colored_label(egui::Color32::YELLOW,"Torção sem convergência suficiente. Estimativa não liberada.");}
                ui.label(format!("Método: {} · estado: {}",torsion["method"].as_str().unwrap_or(""),torsion["state"].as_str().unwrap_or("")));
                ui.small("Torção de Saint-Venant para seção homogênea isotrópica. Propriedades geométricas não constituem dimensionamento ou aprovação técnica.");
                ui.collapsing("Relatório e procedência",|ui|json_view(ui,report));
            }
            if let Some(report)=&self.material_report{ui.collapsing("Estado do material e procedência",|ui|json_view(ui,report));}
        });
    }
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    let label = ui.label(label);
    ui.text_edit_singleline(value).labelled_by(label.id);
}
fn json_view(ui: &mut egui::Ui, value: &serde_json::Value) {
    if let Ok(mut text) = serde_json::to_string_pretty(value) {
        ui.add(
            egui::TextEdit::multiline(&mut text)
                .desired_rows(8)
                .interactive(false),
        );
    }
}
fn material_example() -> String {
    let value = |quantity: &str, decimal: &str, unit: &str| serde_json::json!({"quantity":quantity,"decimal":decimal,"unit":unit,"basis":"nominal","design_basis":null});
    serde_json::to_string_pretty(&serde_json::json!({"id":Uuid::new_v4(),"revision":1,"name":"Exemplo para preencher","source":{"publisher":"Informar responsável","reference":"Exemplo ilustrativo, substituir pela fonte dos dados","edition":"1","revision":"1","data_license":"Informar licença","original_record":null},"applicability":[],"customization":null,"payload":{"kind":"material","material":{"class":"Informar classe","states":[{"name":"Estado de referência","conditions":[],"elastic":{"kind":"isotropic","young_modulus":value("elastic_modulus","30","gigapascal"),"poisson_ratio":0.2},"density":value("density","2500","kilogram_per_cubic_metre"),"properties":{}}]}}})).unwrap_or_default()
}
