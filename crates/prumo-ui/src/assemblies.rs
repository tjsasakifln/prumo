//! Walls, cores, stairs and ramps use the same domain generators as MCP.
use crate::surfaces::{
    common_material, discard_collection, editor_input_ok, field, frame_editor, outline_editor,
    point_editor, point_scope, rectangle, show_input_errors, support_selector,
};
use crate::{UiAction, numeric};
use prumo_domain::assemblies::*;
use prumo_domain::model::{ModelEdit, PhysicalRole};
use prumo_domain::surfaces::{LocalPoint, SurfaceFrame, SurfaceOpening};
use prumo_domain::units::Point3;
use prumo_domain::{Command, Project};
use uuid::Uuid;

pub struct AssemblyPanel {
    pub draft: AssemblySpec,
    pub selected: Option<Uuid>,
    prepared: Option<(u64, AssemblySpec, bool, Vec<Uuid>)>,
    adopt: bool,
    adopted: Vec<Uuid>,
    notice: String,
}

impl Default for AssemblyPanel {
    fn default() -> Self {
        Self {
            draft: AssemblySpec {
                name: "Escada 01".into(),
                role: PhysicalRole::AnalyticalSource,
                material_id: None,
                environment_id: None,
                shape: starter(2),
                supports: vec![],
                continuities: vec![],
            },
            selected: None,
            prepared: None,
            adopt: false,
            adopted: vec![],
            notice: String::new(),
        }
    }
}

impl AssemblyPanel {
    pub fn load(&mut self, project: &Project, id: Uuid) -> Result<(), prumo_domain::DomainError> {
        self.draft = inspect(project, id)?.assembly;
        self.selected = Some(id);
        self.prepared = None;
        self.adopt = false;
        self.adopted.clear();
        Ok(())
    }
    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.collapsing("Paredes, núcleos, escadas e rampas", |ui| {
            self.show_editor(ui, project, actions)
        });
    }

    fn show_editor(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        let form_scope = format!("{}:assemblies", project.id);

        numeric::begin_form(ui, form_scope.clone());
        point_scope(ui, form_scope.clone());

        ui.label("Crie as partes por dimensões e declare seus apoios e continuidades. O desenho mostra regiões candidatas; não comprova transferência de esforços.");

        egui::ComboBox::from_label("Conjunto para editar")
            .selected_text(
                self.selected
                    .and_then(|id| project.objects.iter().find(|o| o.id == id))
                    .map_or("Novo conjunto", |o| o.name.as_str()),
            )
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(self.selected.is_none(), "Novo conjunto")
                    .clicked()
                {
                    *self = Self::default();
                    numeric::discard_form(ui, &form_scope);
                    crate::surfaces::clear_point_entries(ui, &form_scope);
                }

                for o in &project.objects {
                    if metadata(o).ok().flatten().is_some()
                        && ui
                            .selectable_label(self.selected == Some(o.id), &o.name)
                            .clicked()
                    {
                        let _ = self.load(project, o.id);
                        numeric::discard_form(ui, &form_scope);
                        crate::surfaces::clear_point_entries(ui, &form_scope);
                    }
                }
            });

        field(ui, "Nome do conjunto", &mut self.draft.name);
        common_material(
            ui,
            project,
            &mut self.draft.role,
            &mut self.draft.material_id,
        );

        let mut kind = match self.draft.shape {
            AssemblyShape::Wall { .. } => 0,
            AssemblyShape::Core { .. } => 1,
            AssemblyShape::Stair {
                form: StairForm::Conventional,
                ..
            } => 2,
            AssemblyShape::Stair {
                form: StairForm::Folded,
                ..
            } => 3,
            AssemblyShape::Stair {
                form: StairForm::SelfSupporting,
                ..
            } => 4,
            AssemblyShape::Ramp { .. } => 5,
        };
        let previous = kind;

        let labels = [
            "Parede",
            "Núcleo aberto ou fechado",
            "Escada convencional",
            "Escada plissada",
            "Escada autoportante",
            "Rampa",
        ];

        egui::ComboBox::from_label("Tipo de conjunto")
            .selected_text(labels[kind])
            .show_ui(ui, |ui| {
                for (i, label) in labels.iter().enumerate() {
                    ui.selectable_value(&mut kind, i, *label);
                }
            });

        if kind != previous {
            self.draft.shape = starter(kind);
            self.draft.supports.clear();
            self.draft.continuities.clear();
            self.prepared = None;
            self.adopted.clear();
            numeric::discard_form(ui, &form_scope);
            crate::surfaces::clear_point_entries(ui, &form_scope);
        }

        match &mut self.draft.shape {
            AssemblyShape::Wall { panel } => wall_editor(ui, "assembly-wall", panel),
            AssemblyShape::Core {
                panels,
                coupling_beams,
            } => {
                let mut remove = None;

                for (i, panel) in panels.iter_mut().enumerate() {
                    ui.push_id(("core-panel", i), |ui| {
                        ui.collapsing(format!("Parede {}", i + 1), |ui| {
                            wall_editor(ui, &format!("assembly-core-{i}"), panel);
                            if ui
                                .add_enabled(
                                    editor_input_ok(ui),
                                    egui::Button::new("Retirar parede"),
                                )
                                .clicked()
                            {
                                remove = Some(i);
                            }
                        });
                    });
                }

                if let Some(i) = remove
                    && panels.len() > 2
                {
                    panels.remove(i);
                    discard_collection(ui, "assembly-core-");
                }

                if ui.button("Adicionar parede ao núcleo").clicked() {
                    let mut p = wall();
                    p.name = format!("Parede {}", panels.len() + 1);
                    panels.push(p);
                }

                ui.collapsing("Vigas de acoplamento", |ui| {
                    let mut remove = None;

                    for (i, beam) in coupling_beams.iter_mut().enumerate() {
                        ui.push_id(("coupling-beam", i), |ui| {
                            field(ui, "Nome da viga de acoplamento", &mut beam.name);
                            point_editor(
                                ui,
                                &format!("assembly-beam-{i}-start"),
                                "Início",
                                &mut beam.start_m,
                            );
                            point_editor(
                                ui,
                                &format!("assembly-beam-{i}-end"),
                                "Fim",
                                &mut beam.end_m,
                            );
                            numeric::context(ui, format!("assembly-beam-{i}"));
                            numeric::field(ui, "Altura da viga (m)", &mut beam.height_m);
                            numeric::field(ui, "Espessura da viga (m)", &mut beam.thickness_m);
                            if ui
                                .add_enabled(
                                    editor_input_ok(ui),
                                    egui::Button::new("Retirar viga de acoplamento"),
                                )
                                .clicked()
                            {
                                remove = Some(i);
                            }
                        });
                    }

                    if let Some(i) = remove {
                        coupling_beams.remove(i);
                        discard_collection(ui, "assembly-beam-");
                    }

                    if ui.button("Adicionar viga de acoplamento").clicked() {
                        coupling_beams.push(CouplingBeam {
                            name: format!("Acoplamento {}", coupling_beams.len() + 1),
                            start_m: point([0., 0., 2.5]),
                            end_m: point([2., 0., 2.5]),
                            height_m: 0.5,
                            thickness_m: 0.2,
                        });
                    }
                });
            }

            AssemblyShape::Stair {
                flights, landings, ..
            }
            | AssemblyShape::Ramp { flights, landings } => {
                ui.label("Nos lances convencionais e rampas, os pontos inferior e superior pertencem à face superior da cintura. O concreto da laje fica abaixo dela, na direção oposta à normal. O patamar também é informado pela face superior.");

                let mut remove = None;

                for (i, flight) in flights.iter_mut().enumerate() {
                    ui.push_id(("flight", i), |ui| {
                        ui.strong(format!("Lance {}", i + 1));
                        field(ui, "Nome do lance", &mut flight.name);
                        point_editor(
                            ui,
                            &format!("assembly-flight-{i}-start"),
                            "Ponto inferior",
                            &mut flight.start_m,
                        );
                        point_editor(
                            ui,
                            &format!("assembly-flight-{i}-end"),
                            "Ponto superior",
                            &mut flight.end_m,
                        );
                        numeric::context(ui, format!("assembly-flight-{i}"));
                        numeric::field(ui, "Largura do lance (m)", &mut flight.width_m);
                        numeric::field(
                            ui,
                            "Espessura da laje do lance (m)",
                            &mut flight.thickness_m,
                        );
                        if kind != 5 {
                            ui.horizontal(|ui| {
                                let label = ui.label("Quantidade de degraus");
                                ui.add(egui::DragValue::new(&mut flight.steps).range(1..=128))
                                    .labelled_by(label.id);
                                if ui.button("Menos um degrau").clicked() {
                                    flight.steps = flight.steps.saturating_sub(1).max(1);
                                }
                                if ui.button("Mais um degrau").clicked() {
                                    flight.steps = flight.steps.saturating_add(1).min(128);
                                }
                            });
                        }
                        if ui
                            .add_enabled(editor_input_ok(ui), egui::Button::new("Retirar lance"))
                            .clicked()
                        {
                            remove = Some(i);
                        }
                    });
                }

                if let Some(i) = remove
                    && flights.len() > 1
                {
                    flights.remove(i);
                    discard_collection(ui, "assembly-flight-");
                }

                if ui.button("Adicionar lance").clicked() {
                    let mut f = flight(kind == 5);
                    f.name = format!("Lance {}", flights.len() + 1);
                    flights.push(f);
                }

                ui.collapsing("Patamares", |ui| {
                    let mut remove = None;

                    for (i, landing) in landings.iter_mut().enumerate() {
                        ui.push_id(("landing", i), |ui| {
                            field(ui, "Nome do patamar", &mut landing.name);
                            frame_editor(
                                ui,
                                &format!("assembly-landing-frame-{i}"),
                                &mut landing.frame,
                            );
                            numeric::context(ui, format!("assembly-landing-{i}"));
                            numeric::field(
                                ui,
                                "Espessura do patamar (m)",
                                &mut landing.thickness_m,
                            );
                            outline_editor(
                                ui,
                                &format!("assembly-landing-outline-{i}"),
                                &mut landing.outline,
                            );
                            if ui
                                .add_enabled(
                                    editor_input_ok(ui),
                                    egui::Button::new("Retirar patamar"),
                                )
                                .clicked()
                            {
                                remove = Some(i);
                            }
                        });
                    }

                    if let Some(i) = remove {
                        landings.remove(i);
                        discard_collection(ui, "assembly-landing-");
                    }

                    if ui.button("Adicionar patamar").clicked() {
                        let mut p = landing();
                        p.name = format!("Patamar {}", landings.len() + 1);
                        landings.push(p);
                    }
                });
            }
        }

        ui.separator();

        ui.collapsing("Apoios e continuidade declarada", |ui| {
            self.show_supports(ui, project)
        });

        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }

        let preview = prumo_domain::assemblies::preview(project, &self.draft);

        if let Err(e) = &preview {
            ui.label(format!("Corrigir antes de aplicar: {e}"));
        }

        if let Ok(p) = &preview {
            ui.collapsing("Partes geradas e relação analítica", |ui| {
                for (i, part) in p.parts.iter().enumerate() {
                    ui.label(format!(
                        "{}: {}. Espessura {:.3} m. {} abertura(s).",
                        i + 1,
                        part.surface.name,
                        part.surface.thickness_m,
                        part.surface.openings.len()
                    ));
                }
                for v in &p.additional_step_volumes {
                    ui.label(format!(
                        "Degraus de {}: {:.5} m³. {}",
                        v.flight, v.volume_m3, v.hypothesis
                    ));
                }
            });

            if self.selected.is_none() {
                ui.collapsing("Adotar superfícies criadas manualmente", |ui| {
                    ui.checkbox(
                        &mut self.adopt,
                        "Usar peças existentes que correspondam exatamente aos parâmetros",
                    );

                    if self.adopt {
                        self.adopted.resize(p.parts.len(), Uuid::nil());
                        for (i, part) in p.parts.iter().enumerate() {
                            egui::ComboBox::from_label(format!("Peça para {}", part.surface.name))
                                .selected_text(
                                    project
                                        .objects
                                        .iter()
                                        .find(|o| o.id == self.adopted[i])
                                        .map_or("Escolha uma superfície equivalente", |o| {
                                            o.name.as_str()
                                        }),
                                )
                                .show_ui(ui, |ui| {
                                    for o in &project.objects {
                                        if prumo_domain::surfaces::metadata(o)
                                            .ok()
                                            .flatten()
                                            .is_some_and(|r| r.surface == part.surface)
                                            && !is_child(project, o.id).unwrap_or(true)
                                        {
                                            ui.selectable_value(
                                                &mut self.adopted[i],
                                                o.id,
                                                &o.name,
                                            );
                                        }
                                    }
                                });
                        }
                    }
                });
            }
        }

        let numeric_ok = editor_input_ok(ui);
        show_input_errors(ui);

        let can_prepare = preview.is_ok()
            && numeric_ok
            && (!self.adopt || self.adopted.iter().all(|id| !id.is_nil()));

        if ui
            .add_enabled(
                can_prepare,
                egui::Button::new("Conferir prévia do conjunto"),
            )
            .clicked()
        {
            self.prepared = Some((
                project.revision,
                self.draft.clone(),
                self.adopt,
                self.adopted.clone(),
            ));
        }

        let ready = can_prepare
            && self.prepared.as_ref().is_some_and(|(r, s, adopt, ids)| {
                *r == project.revision
                    && *s == self.draft
                    && *adopt == self.adopt
                    && *ids == self.adopted
            });

        if ready && let Ok(p) = preview {
            ui.strong("Prévia pronta para a revisão atual");
            ui.label(format!(
                "{} parte(s), {} continuidade(s), {} apoio(s) declarado(s).",
                p.parts.len(),
                p.declared_continuities.len(),
                p.declared_supports.len()
            ));
            for warning in &p.warnings {
                ui.label(format!("Atenção: {warning}"));
            }
            if self.selected.is_some() {
                ui.label("As regiões analíticas serão atualizadas e os resultados e documentos dependentes ficarão obsoletos.");
            }
        }

        if ui
            .add_enabled(ready, egui::Button::new("Aplicar conjunto conferido"))
            .clicked()
        {
            let command = if let Some(object_id) = self.selected {
                AssemblyEdit::Update {
                    object_id,
                    assembly: self.draft.clone(),
                }
            } else if self.adopt {
                AssemblyEdit::Adopt {
                    assembly: self.draft.clone(),
                    part_ids: self.adopted.clone(),
                }
            } else {
                AssemblyEdit::Create {
                    assembly: self.draft.clone(),
                }
            };
            actions.push(UiAction::command(Command::EditModel {
                expected_revision: project.revision,
                edit: ModelEdit::EditAssemblies { command },
            }));
            self.prepared = None;
        }
    }

    fn show_supports(&mut self, ui: &mut egui::Ui, project: &Project) {
        ui.label("As partes são numeradas pela lista da prévia. Apoios externos e uniões entre bordas são escolhas explícitas.");

        let part_names = prumo_domain::assemblies::preview(project, &self.draft)
            .map(|p| {
                p.parts
                    .iter()
                    .map(|x| x.surface.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut remove = None;

        for (i, support) in self.draft.supports.iter_mut().enumerate() {
            ui.push_id(("assembly-support", i), |ui| {
                part_selector(ui, "Parte apoiada", &part_names, &mut support.part);
                support_selector(ui, project, &mut support.object_id, self.selected);
                egui::ComboBox::from_label("Condição declarada do apoio")
                    .selected_text(match support.kind {
                        SupportKind::Bearing => "Apoio",
                        SupportKind::Fixed => "Engaste",
                        SupportKind::Sliding => "Apoio deslizante",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut support.kind, SupportKind::Bearing, "Apoio");
                        ui.selectable_value(&mut support.kind, SupportKind::Fixed, "Engaste");
                        ui.selectable_value(
                            &mut support.kind,
                            SupportKind::Sliding,
                            "Apoio deslizante",
                        );
                    });
                numeric::context(ui, format!("assembly-support-{i}"));
                numeric::field(ui, "X local do apoio na parte (m)", &mut support.point.x_m);
                numeric::field(ui, "Y local do apoio na parte (m)", &mut support.point.y_m);
                if ui
                    .add_enabled(
                        editor_input_ok(ui),
                        egui::Button::new("Retirar apoio do conjunto"),
                    )
                    .clicked()
                {
                    remove = Some(i);
                }
            });
        }

        if let Some(i) = remove {
            self.draft.supports.remove(i);
            discard_collection(ui, "assembly-support-");
        }

        if let Some(target) = project.objects.iter().find(|o| {
            Some(o.id) != self.selected
                && o.role == PhysicalRole::AnalyticalSource
                && o.discipline == prumo_domain::Discipline::Structure
        }) && ui.button("Declarar apoio externo").clicked()
        {
            self.draft.supports.push(AssemblySupport {
                part: 0,
                object_id: target.id,
                point: LocalPoint::new(0., 0.),
                kind: SupportKind::Bearing,
            });
        }

        if ui
            .button("Identificar bordas coincidentes para conferir")
            .clicked()
        {
            match proposed_continuities(project, &self.draft) {
                Ok(links) => {
                    self.draft.continuities = links;
                    self.notice="Uniões propostas abaixo. Confira e retire as que não devem existir antes de aplicar.".into();
                }
                Err(e) => self.notice = e.to_string(),
            }
        }

        let mut remove = None;

        for (i, c) in self.draft.continuities.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Parte {}, borda {} para a parte {}, borda {}",
                    c.from_part + 1,
                    c.from_edge + 1,
                    c.to_part + 1,
                    c.to_edge + 1
                ));
                if ui.button(format!("Retirar união {}", i + 1)).clicked() {
                    remove = Some(i);
                }
            });
        }

        if let Some(i) = remove {
            self.draft.continuities.remove(i);
        }
    }
}

fn wall_editor(ui: &mut egui::Ui, key: &str, panel: &mut WallPanel) {
    field(ui, "Nome da parede", &mut panel.name);
    frame_editor(ui, &format!("{key}-frame"), &mut panel.frame);
    numeric::context(ui, key.into());
    numeric::field(ui, "Espessura da parede (m)", &mut panel.thickness_m);
    outline_editor(ui, &format!("{key}-outline"), &mut panel.outline);
    ui.collapsing("Portas e janelas na parede", |ui| {
        let mut remove = None;
        for (i, opening) in panel.openings.iter_mut().enumerate() {
            ui.push_id(("wall-opening", i), |ui| {
                field(ui, "Nome da abertura na parede", &mut opening.name);
                outline_editor(ui, &format!("{key}-opening-{i}"), &mut opening.outline);
                if ui
                    .add_enabled(
                        editor_input_ok(ui),
                        egui::Button::new("Retirar abertura da parede"),
                    )
                    .clicked()
                {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            panel.openings.remove(i);
            discard_collection(ui, &format!("{key}-opening-"));
        }
        if ui.button("Adicionar abertura na parede").clicked() {
            panel.openings.push(SurfaceOpening {
                name: format!("Abertura {}", panel.openings.len() + 1),
                outline: rectangle(1., 1., 1., 1.),
            });
        }
    });
}
fn part_selector(ui: &mut egui::Ui, label: &str, names: &[String], selected: &mut usize) {
    egui::ComboBox::from_label(label)
        .selected_text(
            names
                .get(*selected)
                .map_or("Parte inexistente", String::as_str),
        )
        .show_ui(ui, |ui| {
            for (i, name) in names.iter().enumerate() {
                ui.selectable_value(selected, i, format!("{}: {name}", i + 1));
            }
        });
}
fn point(values: [f64; 3]) -> Point3 {
    Point3::try_from(values).expect("finite built-in editor defaults")
}
fn wall() -> WallPanel {
    WallPanel {
        name: "Parede 01".into(),
        frame: SurfaceFrame {
            origin_m: Point3::ORIGIN,
            u: [1., 0., 0.],
            v: [0., 0., 1.],
        },
        outline: rectangle(0., 0., 4., 3.),
        openings: vec![],
        thickness_m: 0.2,
    }
}
fn flight(ramp: bool) -> StairFlight {
    StairFlight {
        name: "Lance 01".into(),
        start_m: Point3::ORIGIN,
        end_m: point(if ramp { [6., 0., 0.5] } else { [3., 0., 3.] }),
        width_m: 1.2,
        thickness_m: 0.15,
        steps: if ramp { 0 } else { 16 },
    }
}
fn landing() -> Landing {
    Landing {
        name: "Patamar superior".into(),
        frame: SurfaceFrame {
            origin_m: point([3., 0., 3.]),
            ..Default::default()
        },
        outline: rectangle(0., 0., 1.2, 1.2),
        thickness_m: 0.15,
    }
}
fn starter(kind: usize) -> AssemblyShape {
    match kind {
        0 => AssemblyShape::Wall { panel: wall() },
        1 => {
            let a = wall();
            let mut b = wall();
            b.name = "Parede 02".into();
            b.frame.origin_m = point([4., 0., 0.]);
            b.frame.u = [0., 1., 0.];
            b.outline = rectangle(0., 0., 3., 3.);
            AssemblyShape::Core {
                panels: vec![a, b],
                coupling_beams: vec![],
            }
        }
        5 => {
            let mut l = landing();
            l.frame.origin_m = point([6., 0., 0.5]);
            AssemblyShape::Ramp {
                flights: vec![flight(true)],
                landings: vec![l],
            }
        }
        _ => AssemblyShape::Stair {
            form: match kind {
                3 => StairForm::Folded,
                4 => StairForm::SelfSupporting,
                _ => StairForm::Conventional,
            },
            flights: vec![flight(false)],
            landings: vec![landing()],
        },
    }
}
