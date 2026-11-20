//! Accessible project navigation. All actions go through the shared view command service.
use crate::{UiAction, numeric};
use egui::{Color32, RichText, Stroke, Vec2};
use prumo_domain::{
    Project,
    view::{CutAxis, ViewCommand, ViewCut, ViewKind, ViewState},
};

pub struct ViewportView {
    pub state: ViewState,
    pub kind: ViewKind,
    pub filter: String,
    pub cut_axis: CutAxis,
    pub cut_min_m: f64,
    pub cut_max_m: f64,
    pub pan_step_m: f64,
    pub notice: String,
    pub three_views: bool,
    pub export_available: bool,
}

impl Default for ViewportView {
    fn default() -> Self {
        Self {
            state: ViewState::default(),
            kind: ViewKind::ThreeD,
            filter: String::new(),
            cut_axis: CutAxis::Z,
            cut_min_m: 0.0,
            cut_max_m: 3.0,
            pan_step_m: 1.0,
            notice: String::new(),
            three_views: false,
            export_available: true,
        }
    }
}

impl ViewportView {
    pub fn discipline_label(discipline: prumo_domain::Discipline) -> &'static str {
        use prumo_domain::Discipline;
        match discipline {
            Discipline::Structure => "Estruturas",
            Discipline::WaterSanitation => "Água e esgoto",
            Discipline::Gas => "Gás",
            Discipline::FireProtection => "Proteção contra incêndio",
            Discipline::Electrical => "Elétrica",
            Discipline::Photovoltaic => "Fotovoltaica",
            Discipline::LightningProtection => "Proteção contra descargas atmosféricas",
            Discipline::Cabling => "Cabeamento",
            Discipline::Hvac => "Climatização e ventilação",
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        self.state.synchronize(project);
        self.kind = self.state.active_view;
        ui.heading("Ambiente 2D/3D");
        ui.label("A mesma peça em planta, corte e 3D. Modelado, não calculado.");
        ui.horizontal_wrapped(|ui| {
            for (view, label) in [
                (ViewKind::Plan, "Planta XY"),
                (ViewKind::Section, "Corte XZ"),
                (ViewKind::ThreeD, "Modelo 3D"),
            ] {
                if ui
                    .selectable_label(self.state.active_view == view, label)
                    .clicked()
                {
                    actions.push(UiAction::View(ViewCommand::SetView { view }));
                }
            }
            ui.checkbox(&mut self.three_views, "Três vistas");
        });
        ui.horizontal_wrapped(|ui| {
            button(ui, actions, "Enquadrar tudo", ViewCommand::Fit {});
            button(ui, actions, "Ampliar +", ViewCommand::Zoom { factor: 1.25 });
            button(ui, actions, "Reduzir −", ViewCommand::Zoom { factor: 0.8 });
            button(ui, actions, "Mostrar tudo", ViewCommand::ShowAll {});
        });
        if let Some(reason) = &self.state.graphics_failure {
            ui.strong("Vista gráfica indisponível");
            ui.label(reason);
            ui.label(
                "Seu projeto foi preservado. Continue pela lista de objetos ou restaure a vista.",
            );
            button(
                ui,
                actions,
                "Restaurar vista",
                ViewCommand::RestoreGraphics {},
            );
        } else if self.three_views {
            for kind in [ViewKind::Plan, ViewKind::Section, ViewKind::ThreeD] {
                ui.push_id(format!("{kind:?}"), |ui| {
                    self.canvas(ui, project, kind, 140.0, actions)
                });
            }
        } else {
            self.canvas(ui, project, self.kind, 225.0, actions);
        }
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        ui.label("Guias tracejadas são referências de eixos e níveis, sem conexão automática.");
        ui.label("Clique em uma peça. Cliques repetidos alternam peças sobrepostas. Tab e Enter operam os controles; na vista, use setas, +, − e Home.");
        self.navigation(ui, actions);
        self.object_selector(ui, project, actions);
        self.properties(ui, project);
        self.accessibility(ui, actions);
        if self.export_available && ui.button("Exportar projeto para vista web").on_hover_text("Salva uma cópia JSON do modelo no diretório do projeto para abrir no visualizador web.").clicked() {
            actions.push(UiAction::ExportWebProject);
        }
    }

    pub fn diagnostics(
        &self,
        ui: &mut egui::Ui,
        project: &Project,
        report: Option<&prumo_domain::geometry::GeometryReport>,
        actions: &mut Vec<UiAction>,
    ) {
        ui.collapsing("Localizar problemas geométricos", |ui| {
            let Some(report) = report.filter(|report| report.project_id == project.id && report.revision == project.revision) else {
                ui.label("Abra Geometria e conexões e execute a inspeção desta revisão. Os problemas poderão ser localizados aqui pelo nome da peça.");
                return;
            };
            use prumo_domain::geometry::GeometryFinding;
            for (index, finding) in report.findings.iter().enumerate() {
                let (description, ids) = match finding {
                    GeometryFinding::ShortElement { object_id, .. } => ("Peça curta", vec![*object_id]),
                    GeometryFinding::Duplicate { first_id, second_id } => ("Peças duplicadas", vec![*first_id, *second_id]),
                    GeometryFinding::NearContact { first_id, second_id, .. } => ("Proximidade sem ligação presumida", vec![*first_id, *second_id]),
                    GeometryFinding::Crossing { first_id, second_id, .. } => ("Cruzamento a conferir", vec![*first_id, *second_id]),
                    GeometryFinding::Overlap { first_id, second_id, .. } => ("Sobreposição a conferir", vec![*first_id, *second_id]),
                };
                ui.label(format!("Apontamento {}: {}", index + 1, description));
                for id in ids {
                    if let Some(object) = project.objects.iter().find(|object| object.id == id) {
                        ui.push_id((index,id), |ui| {
                            if ui.button(format!("Localizar {}", object.name)).clicked() {
                                actions.push(UiAction::View(ViewCommand::Select { object_id: id, reveal: true }));
                                actions.push(UiAction::View(ViewCommand::LocateSelected {}));
                            }
                        });
                    }
                }
            }
            if report.findings.is_empty() { ui.label("A inspeção desta revisão não encontrou apontamentos geométricos no filtro usado. Isso não aprova o cálculo."); }
        });
    }

    fn canvas(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        kind: ViewKind,
        height: f32,
        actions: &mut Vec<UiAction>,
    ) {
        let label = match kind {
            ViewKind::Plan => "Planta XY",
            ViewKind::Section => "Corte XZ",
            ViewKind::ThreeD => "Modelo 3D",
        };
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().max(100.0), height),
            egui::Sense::click(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::Role::Image,
                ui.is_enabled(),
                format!(
                    "Vista {label}. Seleção e navegação também disponíveis na lista de objetos."
                ),
            )
        });
        let painter = ui.painter_at(rect);
        let background = if self.state.high_contrast {
            Color32::BLACK
        } else {
            Color32::from_rgb(18, 29, 39)
        };
        painter.rect_filled(rect, 4.0, background);
        let frame = match self
            .state
            .frame(project, kind, rect.width() as f64, rect.height() as f64)
        {
            Ok(frame) => frame,
            Err(error) => {
                actions.push(UiAction::View(ViewCommand::GraphicsUnavailable {
                    reason: "A projeção gráfica não pôde ser construída. O modelo foi preservado."
                        .into(),
                }));
                self.notice = error.to_string();
                return;
            }
        };
        for guide in &frame.guides {
            let a = rect.min + egui::vec2(guide.start_px[0] as f32, guide.start_px[1] as f32);
            let b = rect.min + egui::vec2(guide.end_px[0] as f32, guide.end_px[1] as f32);
            let color = if self.state.high_contrast {
                Color32::WHITE
            } else {
                Color32::from_rgb(155, 170, 190)
            };
            let length = a.distance(b);
            let dashes = (length / 12.0).ceil().clamp(1.0, 200.0) as usize;
            for dash in 0..dashes {
                let start = dash as f32 / dashes as f32;
                let end = (dash as f32 + 0.55) / dashes as f32;
                painter.line_segment([a.lerp(b, start), a.lerp(b, end)], Stroke::new(1.0, color));
            }
            painter.text(
                a + egui::vec2(3.0, -3.0),
                egui::Align2::LEFT_BOTTOM,
                &guide.label,
                egui::FontId::proportional(12.0),
                color,
            );
        }
        let mut labelled = std::collections::BTreeSet::new();
        for segment in &frame.segments {
            let a = rect.min + egui::vec2(segment.start_px[0] as f32, segment.start_px[1] as f32);
            let b = rect.min + egui::vec2(segment.end_px[0] as f32, segment.end_px[1] as f32);
            if segment.selected {
                painter.line_segment([a, b], Stroke::new(8.0, Color32::WHITE));
            }
            let color = if segment.selected {
                Color32::from_rgb(255, 210, 80)
            } else if self.state.high_contrast {
                Color32::WHITE
            } else {
                Color32::from_rgb(75, 211, 190)
            };
            painter.line_segment(
                [a, b],
                Stroke::new(if segment.selected { 3.0 } else { 2.5 }, color),
            );
            painter.circle_filled(a, 3.5, color);
            painter.circle_filled(b, 3.5, color);
            if segment.selected
                && labelled.insert(self.state.selected_id.unwrap_or(segment.object_id))
                && let Some(object) = project
                    .objects
                    .iter()
                    .find(|o| Some(o.id) == self.state.selected_id)
            {
                painter.text(
                    (a + b.to_vec2()) * 0.5 + egui::vec2(5.0, -9.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("Selecionado: {}", object.name),
                    egui::FontId::proportional(13.0),
                    Color32::WHITE,
                );
            }
        }
        painter.text(
            rect.left_top() + egui::vec2(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::proportional(13.0),
            Color32::WHITE,
        );
        if frame.segments.is_empty() && frame.guides.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if project.objects.is_empty() {
                    "Adicione peças para começar"
                } else {
                    "Nenhuma peça visível. Use Mostrar tudo."
                },
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect.shrink(2.0),
                4.0,
                Stroke::new(3.0, Color32::from_rgb(255, 210, 80)),
                egui::StrokeKind::Inside,
            );
            let commands = ui.input(|input| {
                let mut commands = Vec::new();
                for (key, command) in [
                    (
                        egui::Key::ArrowLeft,
                        ViewCommand::Pan {
                            view: kind,
                            right_m: -self.pan_step_m,
                            up_m: 0.0,
                        },
                    ),
                    (
                        egui::Key::ArrowRight,
                        ViewCommand::Pan {
                            view: kind,
                            right_m: self.pan_step_m,
                            up_m: 0.0,
                        },
                    ),
                    (
                        egui::Key::ArrowUp,
                        ViewCommand::Pan {
                            view: kind,
                            right_m: 0.0,
                            up_m: self.pan_step_m,
                        },
                    ),
                    (
                        egui::Key::ArrowDown,
                        ViewCommand::Pan {
                            view: kind,
                            right_m: 0.0,
                            up_m: -self.pan_step_m,
                        },
                    ),
                    (egui::Key::Plus, ViewCommand::Zoom { factor: 1.25 }),
                    (egui::Key::Minus, ViewCommand::Zoom { factor: 0.8 }),
                    (egui::Key::Home, ViewCommand::Fit {}),
                    (egui::Key::Escape, ViewCommand::ClearSelection {}),
                ] {
                    if input.key_pressed(key) {
                        commands.push(command);
                    }
                }
                commands
            });
            actions.extend(commands.into_iter().map(UiAction::View));
        }
        if response.clicked() {
            response.request_focus();
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.min;
                match frame.pick_candidates([local.x as f64, local.y as f64], 7.0) {
                    Ok(ids) if !ids.is_empty() => {
                        let index = self
                            .state
                            .selected_id
                            .and_then(|selected| ids.iter().position(|id| *id == selected))
                            .map_or(0, |index| (index + 1) % ids.len());
                        actions.push(UiAction::View(ViewCommand::Select {
                            object_id: ids[index],
                            reveal: true,
                        }));
                        self.notice = if ids.len() > 1 {
                            format!(
                                "{} peças sobrepostas. Clique novamente para alternar ou selecione pelo nome.",
                                ids.len()
                            )
                        } else {
                            String::new()
                        };
                    }
                    Ok(_) => {
                        self.notice = "Nenhuma peça no ponto. Selecione pelo nome na lista.".into()
                    }
                    Err(error) => self.notice = error.to_string(),
                }
            }
        }
    }

    fn navigation(&mut self, ui: &mut egui::Ui, actions: &mut Vec<UiAction>) {
        ui.collapsing("Navegar, girar e cortar sem arrastar", |ui| {
            numeric::begin(ui);
            numeric::context(ui,"viewport-navigation".into());
            numeric::field(ui,"Passo de deslocamento (m)",&mut self.pan_step_m);
            if self.pan_step_m.is_finite() && self.pan_step_m>0.0 {
                ui.horizontal_wrapped(|ui| {
                    button(ui,actions,"Mover vista para esquerda",ViewCommand::Pan{view:self.kind,right_m:-self.pan_step_m,up_m:0.0});
                    button(ui,actions,"Mover vista para direita",ViewCommand::Pan{view:self.kind,right_m:self.pan_step_m,up_m:0.0});
                    button(ui,actions,"Mover vista para cima",ViewCommand::Pan{view:self.kind,right_m:0.0,up_m:self.pan_step_m});
                    button(ui,actions,"Mover vista para baixo",ViewCommand::Pan{view:self.kind,right_m:0.0,up_m:-self.pan_step_m});
                });
            } else { ui.label("Erro: informe um passo positivo, em metros."); }
            ui.horizontal_wrapped(|ui| {
                button(ui,actions,"Girar 3D à esquerda",ViewCommand::SetCamera{yaw_degrees:(self.state.yaw_degrees-15.0).rem_euclid(360.0),pitch_degrees:self.state.pitch_degrees});
                button(ui,actions,"Girar 3D à direita",ViewCommand::SetCamera{yaw_degrees:(self.state.yaw_degrees+15.0).rem_euclid(360.0),pitch_degrees:self.state.pitch_degrees});
                button(ui,actions,"Elevar câmera",ViewCommand::SetCamera{yaw_degrees:self.state.yaw_degrees,pitch_degrees:(self.state.pitch_degrees+10.0).min(89.0)});
                button(ui,actions,"Abaixar câmera",ViewCommand::SetCamera{yaw_degrees:self.state.yaw_degrees,pitch_degrees:(self.state.pitch_degrees-10.0).max(-89.0)});
            });
            ui.label("Intervalo de corte em coordenadas do projeto. A geometria original permanece preservada.");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.cut_axis,CutAxis::X,"Cortar por X");
                ui.selectable_value(&mut self.cut_axis,CutAxis::Y,"Cortar por Y");
                ui.selectable_value(&mut self.cut_axis,CutAxis::Z,"Cortar por Z");
            });
            numeric::field(ui,"Cota mínima do corte (m)",&mut self.cut_min_m);
            numeric::field(ui,"Cota máxima do corte (m)",&mut self.cut_max_m);
            let valid=numeric::errors(ui).is_empty();
            if ui.add_enabled(valid,egui::Button::new("Aplicar intervalo de corte")).clicked(){
                actions.push(UiAction::View(ViewCommand::SetCut{cut:Some(ViewCut{axis:self.cut_axis,min_m:self.cut_min_m,max_m:self.cut_max_m})}));
            }
            button(ui,actions,"Retirar corte",ViewCommand::SetCut{cut:None});
            if let Some(cut)=self.state.cut { ui.label(format!("Corte ativo: {:?}, {:.3} a {:.3} m",cut.axis,cut.min_m,cut.max_m)); }
        });
    }

    fn object_selector(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        actions: &mut Vec<UiAction>,
    ) {
        ui.collapsing("Árvore de objetos e seleção",|ui| {
            let label=ui.label("Buscar peça por nome ou identidade");
            ui.text_edit_singleline(&mut self.filter).labelled_by(label.id);
            ui.label("A lista inclui peças ocultas e de outros pavimentos. Selecionar revela a peça nas vistas.");
            ui.horizontal_wrapped(|ui| {
                button(ui,actions,"Objeto anterior",ViewCommand::SelectNext{reverse:true});
                button(ui,actions,"Próximo objeto",ViewCommand::SelectNext{reverse:false});
                button(ui,actions,"Limpar seleção",ViewCommand::ClearSelection {});
            });
            let filter=self.filter.to_lowercase();
            egui::ScrollArea::vertical().max_height(190.0).id_salt("viewport-object-tree").show(ui,|ui| {
                for object in &project.objects {
                    if !filter.is_empty() && !object.name.to_lowercase().contains(&filter) && !object.id.to_string().contains(&filter) { continue; }
                    let (visible,reason)=self.state.visibility(project, object);
                    let selected=self.state.selected_id==Some(object.id);
                    let name=format!("{}{} | {} | Z {:.3} a {:.3} m",if selected{"Selecionado: "}else{""},object.name,if visible{"Visível"}else{reason},object.start_m[2].min(object.end_m[2]),object.start_m[2].max(object.end_m[2]));
                    if ui.selectable_label(selected,name).on_hover_text(format!("Identidade: {}\nDisciplina: {}",object.id,Self::discipline_label(object.discipline))).clicked() {
                        actions.push(UiAction::View(ViewCommand::Select{object_id:object.id,reveal:true}));
                    }
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(self.state.selected_id.is_some(), |ui| {
                button(
                    ui,
                    actions,
                    "Localizar seleção",
                    ViewCommand::LocateSelected {},
                );
                button(ui, actions, "Ocultar seleção", ViewCommand::HideSelected {});
                button(
                    ui,
                    actions,
                    "Isolar seleção",
                    ViewCommand::IsolateSelected {},
                );
            });
        });
    }

    fn properties(&self, ui: &mut egui::Ui, project: &Project) {
        if let Some(object) = self
            .state
            .selected_id
            .and_then(|id| project.objects.iter().find(|o| o.id == id))
        {
            ui.strong(format!("Selecionado: {}", object.name));
            ui.label(format!(
                "{} | {} | {}",
                object.id,
                Self::discipline_label(object.discipline),
                self.state.visibility(project, object).1
            ));
            ui.collapsing("Propriedades da seleção",|ui| {
                ui.label(format!("Início: {:.6}, {:.6}, {:.6} m",object.start_m[0],object.start_m[1],object.start_m[2]));
                ui.label(format!("Fim: {:.6}, {:.6}, {:.6} m",object.end_m[0],object.end_m[1],object.end_m[2]));
                ui.label(if object.role==prumo_domain::model::PhysicalRole::VisualOnly{"Referência arquitetônica ou visual. Não participa de análise."}else{"Fonte analítica modelada. Cálculo ainda não disponível."});
                let ids:Vec<_>=project.model.links.iter().filter(|link|link.physical_id==object.id).map(|link|link.analytical_id.to_string()).collect();
                ui.label(format!("Relações analíticas: {}",if ids.is_empty(){"Nenhuma".into()}else{ids.join(", ")}));
                if let Some(material)=object.material_id.and_then(|id|project.model.materials.iter().find(|m|m.id==id)) {ui.label(format!("Material: {}",material.name));}
                ui.label("Seleção e proximidade na tela não criam conexão estrutural, hidráulica ou elétrica.");
            });
        } else {
            ui.label("Nenhuma peça selecionada. Use o desenho ou a árvore de objetos.");
        }
    }

    fn accessibility(&self, ui: &mut egui::Ui, actions: &mut Vec<UiAction>) {
        ui.collapsing("Acessibilidade e ajuda",|ui| {
            let mut high_contrast=self.state.high_contrast;
            if ui.checkbox(&mut high_contrast,"Alto contraste").changed(){actions.push(UiAction::View(ViewCommand::SetPreferences{scale:self.state.scale,high_contrast}));}
            ui.horizontal_wrapped(|ui|{
                ui.label("Escala da interface");
                for (scale,label) in [(1.0,"100%"),(1.25,"125%"),(1.5,"150%"),(2.0,"200%")]{
                    if ui.selectable_label((self.state.scale-scale).abs()<1.0e-9,label).clicked(){actions.push(UiAction::View(ViewCommand::SetPreferences{scale,high_contrast:self.state.high_contrast}));}
                }
            });
            ui.label(RichText::new("Teclado: Tab para avançar, Shift+Tab para voltar, Enter ou Espaço para ativar. Todos os movimentos têm botões. O desenho tem alternativa textual na árvore e nas propriedades.").strong());
            ui.label("Estruturas e instalações usam a mesma árvore e as mesmas identidades. Ferramentas de cálculo, dimensionamento e emissão serão liberadas nas suas etapas de desenvolvimento.");
            ui.label("Leitores de tela usam os nomes dos controles e a lista de objetos. A leitura do desenho é oferecida por suas propriedades textuais.");
        });
    }
}

fn button(ui: &mut egui::Ui, actions: &mut Vec<UiAction>, label: &str, command: ViewCommand) {
    if ui.button(label).clicked() {
        actions.push(UiAction::View(command));
    }
}
