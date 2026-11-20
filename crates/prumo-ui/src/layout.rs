//! Axes and levels are authored with accessible controls and explicit units.
use crate::UiAction;
use prumo_domain::{
    Command, LengthUnit, Project,
    layout::{self, AxisInput, LayoutEdit, LayoutPoint, SnapProposal, SnapRequest},
    model::ModelEdit,
    transactions::TransactionRequest,
    units::PointInput,
};
use uuid::Uuid;

pub struct LayoutPanel {
    pub level_name: String,
    pub level_elevation: String,
    pub level_id: Option<Uuid>,
    axis_name: String,
    axis_id: Option<Uuid>,
    axis_origin: [String; 3],
    axis_angle: String,
    axis_offset: String,
    axis_extent: String,
    axis_frame: Option<Uuid>,
    frame_name: String,
    frame_id: Option<Uuid>,
    frame_origin: [String; 3],
    frame_angle: String,
    bind_object: Option<Uuid>,
    bind_level: Option<Uuid>,
    bind_offset: String,
    snap_mode: usize,
    snap_object: Option<Uuid>,
    snap_end: bool,
    snap_axis: Option<Uuid>,
    snap_second_axis: Option<Uuid>,
    snap_level: Option<Uuid>,
    snap_position: [String; 3],
    snap_towards: [String; 3],
    snap_distance: String,
    pub snap: Option<SnapProposal>,
    pub message: String,
}
impl Default for LayoutPanel {
    fn default() -> Self {
        Self {
            level_name: "Térreo".into(),
            level_elevation: "0 m".into(),
            level_id: None,
            axis_name: "A".into(),
            axis_id: None,
            axis_origin: zero_point(),
            axis_angle: "0".into(),
            axis_offset: "0 m".into(),
            axis_extent: "20 m".into(),
            axis_frame: None,
            frame_name: "Referencial local".into(),
            frame_id: None,
            frame_origin: zero_point(),
            frame_angle: "0".into(),
            bind_object: None,
            bind_level: None,
            bind_offset: "0 m".into(),
            snap_mode: 0,
            snap_object: None,
            snap_end: false,
            snap_axis: None,
            snap_second_axis: None,
            snap_level: None,
            snap_position: zero_point(),
            snap_towards: ["1 m".into(), "0 m".into(), "0 m".into()],
            snap_distance: "1 m".into(),
            snap: None,
            message: String::new(),
        }
    }
}
pub(crate) fn zero_point() -> [String; 3] {
    ["0 m".into(), "0 m".into(), "0 m".into()]
}
pub(crate) fn text_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        let response = ui.label(label);
        ui.text_edit_singleline(value).labelled_by(response.id);
    });
}
pub(crate) fn point_fields(ui: &mut egui::Ui, label: &str, values: &mut [String; 3]) {
    ui.push_id(label, |ui| {
        ui.label(label);
        for (index, axis) in ["X", "Y", "Z"].iter().enumerate() {
            text_field(
                ui,
                &format!("{label}: {axis} (com unidade)"),
                &mut values[index],
            );
        }
    });
}
pub(crate) fn point_input(values: &[String; 3]) -> Result<PointInput, String> {
    Ok(PointInput {
        values: [
            layout::parse_length(&values[0]).map_err(|e| e.to_string())?,
            layout::parse_length(&values[1]).map_err(|e| e.to_string())?,
            layout::parse_length(&values[2]).map_err(|e| e.to_string())?,
        ],
        unit: LengthUnit::Metre,
    })
}
pub(crate) fn number(value: &str) -> Result<f64, String> {
    let value = value.trim();
    if value.contains(',') && value.contains('.') {
        return Err("Use vírgula ou ponto decimal, sem separador de milhares.".into());
    }
    value
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite())
        .ok_or_else(|| "Informe número finito, sem unidade neste campo.".into())
}
pub(crate) fn choose(
    ui: &mut egui::Ui,
    label: &str,
    selection: &mut Option<Uuid>,
    items: &[(Uuid, String)],
    none: &str,
) {
    egui::ComboBox::from_label(label)
        .selected_text(
            selection
                .and_then(|id| {
                    items
                        .iter()
                        .find(|(item, _)| *item == id)
                        .map(|(_, label)| label.as_str())
                })
                .unwrap_or(none),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(selection, None, none);
            for (id, name) in items {
                ui.selectable_value(selection, Some(*id), name);
            }
        });
}
fn selected(value: Option<Uuid>, name: &str) -> Result<Uuid, String> {
    value.ok_or_else(|| format!("Selecione {name}."))
}
fn edit(project: &Project, command: LayoutEdit) -> Command {
    Command::EditModel {
        expected_revision: project.revision,
        edit: ModelEdit::EditLayout { command },
    }
}
fn submit(project: &Project, command: LayoutEdit, preview: bool, actions: &mut Vec<UiAction>) {
    let command = edit(project, command);
    if preview {
        actions.push(UiAction::PreviewTransaction(Box::new(
            TransactionRequest::new(project, vec![command]),
        )));
    } else {
        actions.push(UiAction::command(command));
    }
}

impl LayoutPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.collapsing("Eixos, níveis e encaixes",|ui|{
            ui.label("Digite medidas como 3,2 m, 25 cm ou (3 m + 20 cm) / 2. Um encaixe visual não cria ligação estrutural.");
            let state=match layout::state(project){Ok(state)=>state,Err(error)=>{ui.label(error.to_string());return;}};
            let levels:Vec<_>=state.levels.iter().map(|l|(l.id,format!("{} ({:.3} m)",l.name,l.elevation_m))).collect();
            let axes:Vec<_>=state.axes.iter().map(|a|(a.id,a.input.name.clone())).collect();
            let frames:Vec<_>=state.frames.iter().map(|f|(f.id,f.name.clone())).collect();
            let objects:Vec<_>=project.objects.iter().map(|o|(o.id,o.name.clone())).collect();
            ui.strong("Níveis do projeto");
            for level in &state.levels{ui.horizontal(|ui|{ui.label(format!("{}: {:.4} m",level.name,level.elevation_m));if ui.button(format!("Editar nível {}",level.name)).clicked(){self.level_id=Some(level.id);self.level_name=level.name.clone();self.level_elevation=level.original_expression.clone();}});}
            text_field(ui,"Nome do nível",&mut self.level_name);text_field(ui,"Cota do nível",&mut self.level_elevation);
            ui.horizontal(|ui|{
                if ui.button(if self.level_id.is_some(){"Conferir movimento do nível"}else{"Criar nível"}).clicked(){
                    match layout::parse_length(&self.level_elevation){Ok(_)=>{submit(project,LayoutEdit::PutLevel{id:self.level_id.unwrap_or_else(Uuid::new_v4),name:self.level_name.clone(),elevation:self.level_elevation.clone()},self.level_id.is_some(),actions);self.message=if self.level_id.is_some(){"Confira as peças, os nós e os resultados afetados em Alterações em lote e histórico antes de aplicar.".into()}else{String::new()};},Err(error)=>self.message=error.to_string()}
                }
                if ui.button("Preparar novo nível").clicked(){self.level_id=None;self.level_name.clear();}
            });
            ui.collapsing("Referenciais locais",|ui|{
                for frame in &state.frames{if ui.button(format!("Editar referencial {}",frame.name)).clicked(){self.frame_id=Some(frame.id);self.frame_name=frame.name.clone();self.frame_origin=frame.origin_m.as_array().map(|v|format!("{v} m"));self.frame_angle=frame.angle_degrees.to_string();}}
                text_field(ui,"Nome do referencial",&mut self.frame_name);point_fields(ui,"Origem do referencial",&mut self.frame_origin);text_field(ui,"Rotação do referencial (graus)",&mut self.frame_angle);
                if ui.button(if self.frame_id.is_some(){"Conferir alteração do referencial"}else{"Criar referencial"}).clicked(){let result=(||Ok::<_,String>(LayoutEdit::PutFrame{id:self.frame_id.unwrap_or_else(Uuid::new_v4),name:self.frame_name.clone(),origin:point_input(&self.frame_origin)?,angle_degrees:number(&self.frame_angle)?}))();match result{Ok(command)=>submit(project,command,self.frame_id.is_some(),actions),Err(error)=>self.message=error}}
                if ui.button("Preparar novo referencial").clicked(){self.frame_id=None;self.frame_name.clear();}
            });
            ui.collapsing("Eixos ortogonais e inclinados",|ui|{
                for axis in &state.axes{if ui.button(format!("Editar eixo {}",axis.input.name)).clicked(){self.axis_id=Some(axis.id);self.axis_name=axis.input.name.clone();self.axis_origin=axis.input.origin.to_si().map(|p|p.as_array().map(|v|format!("{v} m"))).unwrap_or_else(|_|zero_point());self.axis_angle=axis.input.angle_degrees.to_string();self.axis_offset=axis.input.offset.clone();self.axis_extent=axis.input.extent.clone();self.axis_frame=axis.input.frame_id;}}
                text_field(ui,"Nome do eixo",&mut self.axis_name);point_fields(ui,"Origem do eixo",&mut self.axis_origin);text_field(ui,"Direção do eixo (graus)",&mut self.axis_angle);text_field(ui,"Afastamento perpendicular",&mut self.axis_offset);text_field(ui,"Extensão de apresentação",&mut self.axis_extent);choose(ui,"Referencial do eixo",&mut self.axis_frame,&frames,"Referencial de engenharia");
                if ui.button(if self.axis_id.is_some(){"Conferir alteração do eixo"}else{"Criar eixo"}).clicked(){let result=(||Ok::<_,String>(LayoutEdit::PutAxis{id:self.axis_id.unwrap_or_else(Uuid::new_v4),input:AxisInput{name:self.axis_name.clone(),origin:point_input(&self.axis_origin)?,angle_degrees:number(&self.axis_angle)?,offset:self.axis_offset.clone(),extent:self.axis_extent.clone(),frame_id:self.axis_frame}}))();match result{Ok(command)=>submit(project,command,self.axis_id.is_some(),actions),Err(error)=>self.message=error}}
                if ui.button("Preparar novo eixo").clicked(){self.axis_id=None;self.axis_name.clear();}
            });
            ui.collapsing("Vincular origem de uma peça ao nível",|ui|{
                ui.label("Lajes e conjuntos acompanham o nível. Pilares e vigas usam os níveis nos seus pontos paramétricos.");choose(ui,"Peça vinculada",&mut self.bind_object,&objects,"Selecione uma peça");choose(ui,"Nível de origem",&mut self.bind_level,&levels,"Selecione um nível");text_field(ui,"Desnível da origem",&mut self.bind_offset);
                if ui.button("Conferir vínculo com nível").clicked(){match(selected(self.bind_object,"a peça"),selected(self.bind_level,"o nível")){(Ok(object_id),Ok(level_id))=>submit(project,LayoutEdit::BindObject{object_id,level_id,offset:self.bind_offset.clone()},true,actions),(Err(error),_)|(_,Err(error))=>self.message=error}}
                for binding in &state.bindings{ui.horizontal(|ui|{let name=objects.iter().find(|(id,_)|*id==binding.object_id).map(|(_,name)|name.as_str()).unwrap_or("Peça");ui.label(format!("{name}, desnível {:.3} m",binding.offset_m));if ui.button(format!("Desvincular {name}")).clicked(){submit(project,LayoutEdit::UnbindObject{object_id:binding.object_id},false,actions);}});}
            });
            ui.collapsing("Prévia de encaixe",|ui|{
                let modes=["Extremidade","Alinhamento","Interseção","Distância"];
                egui::ComboBox::from_label("Tipo de encaixe").selected_text(modes[self.snap_mode.min(3)]).show_ui(ui,|ui|{for(index,name)in modes.iter().enumerate(){ui.selectable_value(&mut self.snap_mode,index,*name);}});
                match self.snap_mode{
                    0=>{choose(ui,"Peça de referência",&mut self.snap_object,&objects,"Selecione uma peça");ui.checkbox(&mut self.snap_end,"Usar extremidade final");}
                    1=>{choose(ui,"Eixo de alinhamento",&mut self.snap_axis,&axes,"Selecione um eixo");point_fields(ui,"Posição a alinhar",&mut self.snap_position);}
                    2=>{choose(ui,"Primeiro eixo",&mut self.snap_axis,&axes,"Selecione um eixo");choose(ui,"Segundo eixo",&mut self.snap_second_axis,&axes,"Selecione outro eixo");choose(ui,"Nível da interseção",&mut self.snap_level,&levels,"Selecione um nível");}
                    _=>{point_fields(ui,"Origem da distância",&mut self.snap_position);point_fields(ui,"Direção da distância",&mut self.snap_towards);text_field(ui,"Distância de encaixe",&mut self.snap_distance);}
                }
                if ui.button("Mostrar vínculo proposto").clicked(){
                    self.snap=None;
                    let request=(||match self.snap_mode{0=>Ok(SnapRequest::Endpoint{object_id:selected(self.snap_object,"a peça")?,end:self.snap_end}),1=>Ok(SnapRequest::Alignment{axis_id:selected(self.snap_axis,"o eixo")?,position:point_input(&self.snap_position)?}),2=>Ok(SnapRequest::Intersection{first_axis_id:selected(self.snap_axis,"o primeiro eixo")?,second_axis_id:selected(self.snap_second_axis,"o segundo eixo")?,level_id:selected(self.snap_level,"o nível")?,offset:"0 m".into()}),_=>Ok::<_,String>(SnapRequest::Distance{from:LayoutPoint::Coordinates{position:point_input(&self.snap_position)?},towards:LayoutPoint::Coordinates{position:point_input(&self.snap_towards)?},distance:self.snap_distance.clone()})})();
                    match request.and_then(|r|layout::propose_snap(project,&r).map_err(|e|e.to_string())){Ok(proposal)=>{self.snap=Some(proposal);self.message.clear();},Err(error)=>self.message=error}
                }
                if let Some(snap)=&self.snap{if snap.project_id==project.id&&snap.revision==project.revision{ui.strong(format!("X {:.6} m | Y {:.6} m | Z {:.6} m",snap.point_m[0],snap.point_m[1],snap.point_m[2]));ui.label(&snap.explanation);ui.label(format!("{} referências geométricas; nenhuma conexão estrutural criada.",snap.sources.len()));}else{ui.label("Prévia desatualizada. Gere novamente após a alteração do projeto.");}}
            });
            if !self.message.is_empty(){ui.label(&self.message);}
        });
    }
}
