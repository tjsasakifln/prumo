//! Form-based structural members, including axis references and explicit connections.
use crate::{
    UiAction,
    layout::{choose, number, point_fields, point_input, text_field, zero_point},
};
use prumo_domain::{
    Command, Project,
    catalogue::{self, CatalogPayload},
    framing::{
        self, FramingEdit, FramingInput, MemberKind, MemberPath, SectionAlignment, SectionStation,
        SectionVariation,
    },
    layout::{self, LayoutPoint},
    model::ModelEdit,
    transactions::TransactionRequest,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct PointEditor {
    mode: usize,
    coordinates: [String; 3],
    frame: Option<Uuid>,
    axis: Option<Uuid>,
    second_axis: Option<Uuid>,
    level: Option<Uuid>,
    station: String,
    offset: String,
    node: Option<Uuid>,
}
impl Default for PointEditor {
    fn default() -> Self {
        Self {
            mode: 0,
            coordinates: zero_point(),
            frame: None,
            axis: None,
            second_axis: None,
            level: None,
            station: "0 m".into(),
            offset: "0 m".into(),
            node: None,
        }
    }
}
impl PointEditor {
    fn from_point(point: &LayoutPoint) -> Self {
        let mut editor = Self::default();
        match point {
            LayoutPoint::Coordinates { position } => {
                editor.coordinates = position
                    .to_si()
                    .map(|p| p.as_array().map(|v| format!("{v} m")))
                    .unwrap_or_else(|_| zero_point())
            }
            LayoutPoint::Local { frame_id, position } => {
                editor.mode = 1;
                editor.frame = Some(*frame_id);
                editor.coordinates = position
                    .to_si()
                    .map(|p| p.as_array().map(|v| format!("{v} m")))
                    .unwrap_or_else(|_| zero_point());
            }
            LayoutPoint::AxisStation {
                axis_id,
                station,
                level_id,
                offset,
            } => {
                editor.mode = 2;
                editor.axis = Some(*axis_id);
                editor.level = Some(*level_id);
                editor.station = station.clone();
                editor.offset = offset.clone();
            }
            LayoutPoint::GridIntersection {
                first_axis_id,
                second_axis_id,
                level_id,
                offset,
            } => {
                editor.mode = 3;
                editor.axis = Some(*first_axis_id);
                editor.second_axis = Some(*second_axis_id);
                editor.level = Some(*level_id);
                editor.offset = offset.clone();
            }
            LayoutPoint::Node { node_id } => {
                editor.mode = 4;
                editor.node = Some(*node_id);
            }
        }
        editor
    }
    pub fn point(&self) -> Result<LayoutPoint, String> {
        let require = |id: Option<Uuid>, label: &str| {
            id.ok_or_else(|| format!("Selecione {label} do ponto."))
        };
        Ok(match self.mode {
            0 => LayoutPoint::Coordinates {
                position: point_input(&self.coordinates)?,
            },
            1 => LayoutPoint::Local {
                frame_id: require(self.frame, "o referencial")?,
                position: point_input(&self.coordinates)?,
            },
            2 => LayoutPoint::AxisStation {
                axis_id: require(self.axis, "o eixo")?,
                station: self.station.clone(),
                level_id: require(self.level, "o nível")?,
                offset: self.offset.clone(),
            },
            3 => LayoutPoint::GridIntersection {
                first_axis_id: require(self.axis, "o primeiro eixo")?,
                second_axis_id: require(self.second_axis, "o segundo eixo")?,
                level_id: require(self.level, "o nível")?,
                offset: self.offset.clone(),
            },
            _ => LayoutPoint::Node {
                node_id: require(self.node, "o nó")?,
            },
        })
    }
    pub fn show(&mut self, ui: &mut egui::Ui, label: &str, project: &Project) {
        ui.push_id(label, |ui| {
            ui.strong(label);
            let context = |field: &str| format!("{label}: {field}");
            let modes = ["Coordenadas", "Referencial local", "Posição em eixo", "Interseção de eixos", "Nó existente"];
            egui::ComboBox::from_label(context("origem do ponto"))
                .selected_text(modes[self.mode.min(4)])
                .show_ui(ui, |ui| {
                    for (index, name) in modes.iter().enumerate() {
                        ui.selectable_value(&mut self.mode, index, *name);
                    }
                });
            let layout = layout::state(project).unwrap_or_default();
            let axes = layout.axes.iter().map(|axis| (axis.id, axis.input.name.clone())).collect::<Vec<_>>();
            let levels = layout.levels.iter().map(|level| (level.id, level.name.clone())).collect::<Vec<_>>();
            match self.mode {
                0 => point_fields(ui, label, &mut self.coordinates),
                1 => {
                    let frames = layout.frames.iter().map(|frame| (frame.id, frame.name.clone())).collect::<Vec<_>>();
                    choose(ui, &context("referencial"), &mut self.frame, &frames, "Selecione");
                    point_fields(ui, &context("coordenadas locais"), &mut self.coordinates);
                }
                2 => {
                    choose(ui, &context("eixo"), &mut self.axis, &axes, "Selecione");
                    choose(ui, &context("nível"), &mut self.level, &levels, "Selecione");
                    text_field(ui, &context("distância ao longo do eixo"), &mut self.station);
                    text_field(ui, &context("desnível"), &mut self.offset);
                }
                3 => {
                    choose(ui, &context("primeiro eixo"), &mut self.axis, &axes, "Selecione");
                    choose(ui, &context("segundo eixo"), &mut self.second_axis, &axes, "Selecione outro eixo");
                    choose(ui, &context("nível"), &mut self.level, &levels, "Selecione");
                    text_field(ui, &context("desnível"), &mut self.offset);
                }
                _ => {
                    let nodes = project.model.analytical_nodes.iter().map(|node| (node.id, node.name.clone())).collect::<Vec<_>>();
                    choose(ui, &context("nó de referência"), &mut self.node, &nodes, "Selecione");
                    ui.small("A posição segue o nó. Para compartilhar a ligação, selecione-o também em Conexões explícitas.");
                }
            }
        });
    }
}
pub struct FramingPanel {
    pub name: String,
    pub editing_id: Option<Uuid>,
    kind: MemberKind,
    curved: bool,
    start: PointEditor,
    end: PointEditor,
    through: PointEditor,
    segments: String,
    material: Option<Uuid>,
    section_stations: Vec<(String, Option<Uuid>)>,
    variation: SectionVariation,
    alignment: SectionAlignment,
    eccentricity: [String; 3],
    roll: String,
    connections: [Option<Uuid>; 2],
    pub message: String,
    truss_name: String,
    truss_points: Vec<PointEditor>,
    truss_edges: Vec<(String, String)>,
    pub preview: Option<FramingInput>,
}
impl Default for FramingPanel {
    fn default() -> Self {
        let end = PointEditor {
            coordinates: ["4 m".into(), "0 m".into(), "0 m".into()],
            ..Default::default()
        };
        let through = PointEditor {
            coordinates: ["2 m".into(), "1 m".into(), "0 m".into()],
            ..Default::default()
        };
        Self {
            name: "Viga 1".into(),
            editing_id: None,
            kind: MemberKind::Beam,
            curved: false,
            start: Default::default(),
            end: end.clone(),
            through: through.clone(),
            segments: "24".into(),
            material: None,
            section_stations: vec![("0".into(), None)],
            variation: SectionVariation::Constant,
            alignment: SectionAlignment::Centroid,
            eccentricity: zero_point(),
            roll: "0".into(),
            connections: [None, None],
            message: String::new(),
            truss_name: "Treliça 1".into(),
            truss_points: vec![Default::default(), end, through],
            truss_edges: vec![
                ("1".into(), "2".into()),
                ("1".into(), "3".into()),
                ("3".into(), "2".into()),
            ],
            preview: None,
        }
    }
}
impl FramingPanel {
    pub fn input(&self) -> Result<FramingInput, String> {
        let start = self.start.point()?;
        let end = self.end.point()?;
        let path = if self.curved {
            MemberPath::CircularArc {
                start,
                through: self.through.point()?,
                end,
                segments: self
                    .segments
                    .parse()
                    .map_err(|_| "Informe de 2 a 256 segmentos inteiros.".to_string())?,
            }
        } else {
            MemberPath::Straight { start, end }
        };
        let sections = self
            .section_stations
            .iter()
            .map(|(fraction, id)| {
                Ok(SectionStation {
                    fraction: number(fraction)?,
                    section_id: id
                        .ok_or_else(|| "Selecione uma seção em cada estação.".to_string())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(FramingInput{name:self.name.clone(),kind:self.kind,path,material_id:self.material.ok_or_else(||"Selecione material do modelo. Cadastre-o em Modelo físico e analítico se necessário.".to_string())?,sections,variation:self.variation,alignment:self.alignment,eccentricity:point_input(&self.eccentricity)?,roll_degrees:number(&self.roll)?,connections:self.connections})
    }
    fn load(&mut self, id: Uuid, input: &FramingInput) {
        self.editing_id = Some(id);
        self.name = input.name.clone();
        self.kind = input.kind;
        self.material = Some(input.material_id);
        self.section_stations = input
            .sections
            .iter()
            .map(|s| (s.fraction.to_string(), Some(s.section_id)))
            .collect();
        self.variation = input.variation;
        self.alignment = input.alignment;
        self.eccentricity = input
            .eccentricity
            .to_si()
            .map(|p| p.as_array().map(|v| format!("{v} m")))
            .unwrap_or_else(|_| zero_point());
        self.roll = input.roll_degrees.to_string();
        self.connections = input.connections;
        match &input.path {
            MemberPath::Straight { start, end } => {
                self.curved = false;
                self.start = PointEditor::from_point(start);
                self.end = PointEditor::from_point(end);
            }
            MemberPath::CircularArc {
                start,
                through,
                end,
                segments,
            } => {
                self.curved = true;
                self.start = PointEditor::from_point(start);
                self.through = PointEditor::from_point(through);
                self.end = PointEditor::from_point(end);
                self.segments = segments.to_string();
            }
        }
        self.preview = None;
        self.message.clear();
    }
    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.collapsing("Pilares, vigas, barras e treliças",|ui|{
            ui.label("Peças editáveis com material, seção e modelo analítico relacionados. Modelagem disponível; cálculo do projeto ainda indisponível.");
            for object in &project.objects{
                if let Ok(Some(record))=framing::metadata(object){
                    ui.horizontal(|ui|{
                        ui.label(format!("{} | {:.4} m | {}",object.name,record.physical_length_m,if record.behavior==framing::MemberBehavior::AxialOnly{
                            "Somente axial"}
                        else{
                            "Pórtico"}
                        ));
                        if ui.button(format!("Editar parâmetros de {}",object.name)).clicked(){
                            self.load(object.id,&record.input);
                        }
                    });
                }
            }
            text_field(ui,"Nome da peça estrutural",&mut self.name);
            egui::ComboBox::from_label("Tipo de peça estrutural").selected_text(kind_name(self.kind)).show_ui(ui,|ui|{
                for kind in[MemberKind::Column,MemberKind::Beam,MemberKind::Bar,MemberKind::TrussBar]{
                    ui.selectable_value(&mut self.kind,kind,kind_name(kind));
                }
            });
            ui.checkbox(&mut self.curved,"Viga curva por três pontos");
            self.start.show(ui,"Início da peça",project);
            if self.curved{
                self.through.show(ui,"Ponto intermediário do arco",project);
                text_field(ui,"Segmentos analíticos do arco",&mut self.segments);
            }
            self.end.show(ui,"Fim da peça",project);
            let materials=project.model.materials.iter().map(|m|(m.id,m.name.clone())).collect::<Vec<_>>();
            choose(ui,"Material da peça",&mut self.material,&materials,"Selecione material");
            let catalogue=catalogue::project_catalogue(project).unwrap_or_default();
            let sections=catalogue.entries.values().filter_map(|versions|versions.last()).filter(|v|matches!(v.entry.payload,CatalogPayload::Section{
                ..}
            )).map(|v|(v.entry.id,v.entry.name.clone())).collect::<Vec<_>>();
            egui::ComboBox::from_label("Variação da seção").selected_text(variation_name(self.variation)).show_ui(ui,|ui|{
                for variation in[SectionVariation::Constant,SectionVariation::Stepped,SectionVariation::Linear]{
                    ui.selectable_value(&mut self.variation,variation,variation_name(variation));
                }
            });
            ui.label("Estações crescentes entre 0 e 1. Para variação linear, informe também a estação final 1. Por trechos, cada transição deve ocorrer antes de 1.");
            let mut remove = None;
            for (index, (fraction, id)) in self.section_stations.iter_mut().enumerate() {
                ui.push_id(("station", index), |ui| {
                    text_field(ui, &format!("Posição relativa da seção {}", index + 1), fraction);
                    choose(ui, &format!("Seção {}", index + 1), id, &sections, "Selecione seção");
                    if index > 0 && ui.button("Remover estação").clicked() {
                        remove = Some(index);
                    }
                });
            }
            if let Some(index) = remove {
                self.section_stations.remove(index);
            }
            if ui.add_enabled(
                self.section_stations.len() < 64,
                egui::Button::new("Adicionar estação de seção"),
            ).clicked() {
                self.section_stations.push((
                    if self.variation == SectionVariation::Stepped {
                        "0,5".into()
                    } else {
                        "1".into()
                    },
                    self.section_stations[0].1,
                ));
            }
            ui.collapsing("Alinhamento, excentricidade e conexões explícitas",|ui|{
                egui::ComboBox::from_label("Alinhamento da seção").selected_text(if self.alignment==SectionAlignment::Centroid{
                    "Centroide sobre a linha"}
                else{
                    "Origem da seção sobre a linha"}
                ).show_ui(ui,|ui|{
                    ui.selectable_value(&mut self.alignment,SectionAlignment::Centroid,"Centroide sobre a linha");
                    ui.selectable_value(&mut self.alignment,SectionAlignment::SectionOrigin,"Origem da seção sobre a linha");
                });
                point_fields(ui,"Excentricidade física em eixos de engenharia",&mut self.eccentricity);
                text_field(ui,"Rotação da seção (graus)",&mut self.roll);
                let nodes=project.model.analytical_nodes.iter().map(|n|(n.id,n.name.clone())).collect::<Vec<_>>();
                choose(ui,"Conexão no início",&mut self.connections[0],&nodes,"Novo nó independente");
                choose(ui,"Conexão no fim",&mut self.connections[1],&nodes,"Novo nó independente");
                ui.small("Compartilhar um nó exige escolha explícita e posição coincidente. Cruzamentos não criam vínculos.");
            });
            if ui.button("Conferir peça física e analítica").clicked(){
                self.preview=None;
                match self.input().and_then(|input|framing::resolve_geometry(project,&input).map(|_|input).map_err(|e|e.to_string())){
                    Ok(input)=>{
                        self.preview=Some(input);
                        self.message.clear();
                    }
                    ,Err(error)=>self.message=error}
            }
            if let Some(input)=&self.preview{
                if self.input().ok().as_ref()!=Some(input){
                    ui.label("Parâmetros alterados. Confira a peça novamente antes de salvar.");
                }
                else if let Ok(geometry)=framing::resolve_geometry(project,input){
                    geometric_preview(ui,&geometry);
                    ui.strong(format!("Traçado analítico {:.6} m; {} trechos analíticos",geometry.analytical_path_length_m,geometry.analytical_points_m.len()-1));
                    if geometry.physical_length_is_exact{
                        ui.label(format!("Comprimento físico pela expressão geométrica: {:.6} m.",geometry.physical_length_m));
                    }
                    else{
                        ui.label(format!("Comprimento físico estimado: {:.6} m; estimativa de convergência numérica {:.3e} m, sem garantia de erro certificado.",geometry.physical_length_m,geometry.physical_length_error_estimate_m));
                    }
                    if geometry.maximum_chord_error_m>0.0{
                        ui.label(format!("Discretização analítica: desvio máximo {:.6} m; soma das cordas {:.6} m.",geometry.maximum_chord_error_m,geometry.analytical_chord_length_m));
                        ui.label(format!("Representação física: limite de desvio das cordas {:.6} m; soma das cordas {:.6} m.",geometry.maximum_physical_chord_error_m,geometry.physical_chord_length_m));
                    }
                    if !geometry.physical_breaks.is_empty(){
                        ui.label("A mudança por trechos desloca o centroide. Os saltos não são segmentos físicos nem conexões analíticas.");
                    }
                    if self.kind==MemberKind::TrussBar{
                        ui.strong("Somente esforço axial. Rigidez à flexão proibida neste contrato.");
                    }
                    for(index,point)in geometry.physical_points_m.iter().enumerate().take(4){
                        ui.label(format!("Ponto físico {}: {:.4}, {:.4}, {:.4} m",index+1,point[0],point[1],point[2]));
                    }
                    if ui.button(if self.editing_id.is_some(){
                        "Preparar alteração da peça"}
                    else{
                        "Criar peça conferida"}
                    ).clicked(){
                        let command=Command::EditModel{
                            expected_revision:project.revision,edit:ModelEdit::EditFraming{
                                command:FramingEdit::PutMember{
                                    id:self.editing_id.unwrap_or_else(Uuid::new_v4),input:Box::new(input.clone())}
                            }
                        }
                        ;
                        if self.editing_id.is_some(){
                            actions.push(UiAction::PreviewTransaction(Box::new(TransactionRequest::new(project,vec![command]))));
                            self.message="Confira a prévia em Alterações em lote e histórico antes de aplicar.".into();
                        }
                        else{
                            actions.push(UiAction::command(command));
                        }
                    }
                }
            }
            if ui.button("Preparar nova peça").clicked(){
                self.editing_id=None;
                self.preview=None;
                self.connections=[None,None];
            }
            ui.collapsing("Montar treliça por pontos e barras",|ui|{
                ui.label("A treliça usa o material e a primeira seção selecionados acima. As barras compartilham somente os pontos indicados na lista.");
                text_field(ui,"Nome da treliça",&mut self.truss_name);
                for(index,point)in self.truss_points.iter_mut().enumerate(){
                    point.show(ui,&format!("Ponto da treliça {}",index+1),project);
                }
                if ui.add_enabled(self.truss_points.len()<512,egui::Button::new("Adicionar ponto da treliça")).clicked(){
                    self.truss_points.push(Default::default());
                }
                if ui.add_enabled(self.truss_points.len()>2,egui::Button::new("Remover último ponto")).clicked(){
                    self.truss_points.pop();
                }
                for(index,(start,end))in self.truss_edges.iter_mut().enumerate(){
                    ui.push_id(("truss-edge",index),|ui|{
                        text_field(ui,&format!("Barra {}: ponto inicial",index+1),start);
                        text_field(ui,&format!("Barra {}: ponto final",index+1),end);
                    });
                }
                if ui.add_enabled(self.truss_edges.len()<1024,egui::Button::new("Adicionar barra da treliça")).clicked(){
                    self.truss_edges.push(("1".into(),"2".into()));
                }
                if ui.add_enabled(self.truss_edges.len()>1,egui::Button::new("Remover última barra")).clicked(){
                    self.truss_edges.pop();
                }
                if ui.button("Conferir montagem da treliça").clicked(){
                    let result=(||{
                        let points=self.truss_points.iter().map(PointEditor::point).collect::<Result<Vec<_>,_>>()?;
                        let edges=self.truss_edges.iter().map(|(a,b)|{
                            let parse=|text:&str|text.parse::<usize>().ok().and_then(|n|n.checked_sub(1)).ok_or_else(||"Pontos da treliça são numerados a partir de 1.".to_string());
                            Ok([parse(a)?,parse(b)?])}
                        ).collect::<Result<Vec<_>,String>>()?;
                        Ok::<_,String>(FramingEdit::CreateTruss{
                            id:Uuid::new_v4(),name:self.truss_name.clone(),points,edges,material_id:self.material.ok_or_else(||"Selecione material.".to_string())?,section_id:self.section_stations.first().and_then(|(_,id)|*id).ok_or_else(||"Selecione seção.".to_string())?}
                        )}
                    )();
                    match result{
                        Ok(command)=>{
                            let command=Command::EditModel{
                                expected_revision:project.revision,edit:ModelEdit::EditFraming{
                                    command}
                            }
                            ;
                            actions.push(UiAction::PreviewTransaction(Box::new(TransactionRequest::new(project,vec![command]))));
                            self.message="Confira a montagem atômica e seus nós no painel de alterações antes de aplicar.".into();
                        }
                        ,Err(error)=>self.message=error}
                }
            });
            if !self.message.is_empty(){
                ui.label(&self.message);
            }
        });
    }
}
fn kind_name(kind: MemberKind) -> &'static str {
    match kind {
        MemberKind::Column => "Pilar",
        MemberKind::Beam => "Viga",
        MemberKind::Bar => "Barra genérica",
        MemberKind::TrussBar => "Barra de treliça",
    }
}
fn variation_name(variation: SectionVariation) -> &'static str {
    match variation {
        SectionVariation::Constant => "Constante",
        SectionVariation::Stepped => "Por trechos",
        SectionVariation::Linear => "Linear entre estações",
    }
}

/// Reuses the same f64 projection as the project viewport. Both layers carry
/// visible labels and different line patterns, including in high contrast.
fn geometric_preview(ui: &mut egui::Ui, geometry: &framing::ResolvedGeometry) {
    use prumo_domain::view::{ViewKind, ViewState};
    let mut lower = [f64::INFINITY; 3];
    let mut upper = [f64::NEG_INFINITY; 3];
    for point in geometry
        .physical_points_m
        .iter()
        .chain(&geometry.analytical_points_m)
    {
        for axis in 0..3 {
            lower[axis] = lower[axis].min(point[axis]);
            upper[axis] = upper[axis].max(point[axis]);
        }
    }
    let state = ViewState {
        center_m: std::array::from_fn(|axis| lower[axis] + (upper[axis] - lower[axis]) * 0.5),
        span_m: ((upper[0] - lower[0])
            .hypot(upper[1] - lower[1])
            .hypot(upper[2] - lower[2])
            * 1.3)
            .max(1.0),
        ..Default::default()
    };
    ui.columns(2, |columns| {
        for (index, (title, points)) in [
            (
                "Prévia 3D física, linha contínua",
                &geometry.physical_points_m,
            ),
            (
                "Prévia 3D analítica, linha tracejada",
                &geometry.analytical_points_m,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let ui = &mut columns[index];
            ui.label(title);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width().max(100.0), 180.0),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(rect);
            let scale = f64::from(rect.width().min(rect.height()) - 24.0) / state.span_m;
            let project = |point: prumo_domain::units::Point3| {
                let p = state.project_point(point.as_array(), ViewKind::ThreeD);
                egui::pos2(
                    rect.center().x + (p[0] * scale) as f32,
                    rect.center().y - (p[1] * scale) as f32,
                )
            };
            let stroke = egui::Stroke::new(2.0, ui.visuals().text_color());
            for (segment, pair) in points.windows(2).enumerate() {
                if index == 0 && geometry.physical_breaks.contains(&(segment + 1)) {
                    continue;
                }
                let a = project(pair[0]);
                let b = project(pair[1]);
                if index == 0 {
                    painter.line_segment([a, b], stroke);
                } else {
                    for dash in 0..5 {
                        let start = dash as f32 / 5.0;
                        let end = (dash as f32 + 0.6) / 5.0;
                        painter.line_segment([a + (b - a) * start, a + (b - a) * end], stroke);
                    }
                }
            }
            for point in [points.first(), points.last()].into_iter().flatten() {
                painter.circle_stroke(project(*point), 4.0, stroke);
            }
        }
    });
}
