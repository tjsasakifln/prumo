//! Accessible surface authoring. Geometry and quantities come from the domain.
use crate::{UiAction, numeric};
use prumo_domain::model::{ModelEdit, PhysicalRole};
use prumo_domain::surfaces::*;
use prumo_domain::{Command, Project};
use std::collections::BTreeMap;
use uuid::Uuid;

pub struct SurfacePanel {
    pub draft: SurfaceSpec,
    pub selected: Option<Uuid>,
    prepared: Option<(u64, SurfaceSpec)>,
    rectangle_width: f64,
    rectangle_height: f64,
}

impl Default for SurfacePanel {
    fn default() -> Self {
        Self {
            draft: SurfaceSpec {
                name: "Laje 01".into(),
                role: PhysicalRole::AnalyticalSource,
                material_id: None,
                environment_id: None,
                frame: SurfaceFrame::default(),
                kind: SurfaceKind::SolidSlab {},
                outline: rectangle(0., 0., 6., 4.),
                openings: vec![],
                thickness_m: 0.15,
                regions: vec![],
                supports: vec![],
                edge_conditions: vec![],
            },
            selected: None,
            prepared: None,
            rectangle_width: 6.,
            rectangle_height: 4.,
        }
    }
}

impl SurfacePanel {
    pub fn load(&mut self, project: &Project, id: Uuid) -> Result<(), prumo_domain::DomainError> {
        self.draft = inspect(project, id)?.surface;
        self.selected = Some(id);
        self.prepared = None;
        Ok(())
    }

    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.collapsing("Lajes, painéis e aberturas", |ui| {
            self.show_editor(ui, project, actions)
        });
    }

    fn show_editor(&mut self, ui: &mut egui::Ui, project: &Project, actions: &mut Vec<UiAction>) {
        let form_scope = format!("{}:surfaces", project.id);

        numeric::begin_form(ui, form_scope.clone());
        point_scope(ui, form_scope.clone());

        ui.label("Defina contorno, aberturas e espessuras em metros. O modelo conserva as regiões materiais e o peso próprio por componente. Não há cálculo de esforços ou dimensionamento nesta etapa.");

        egui::ComboBox::from_label("Superfície para editar")
            .selected_text(
                self.selected
                    .and_then(|id| project.objects.iter().find(|o| o.id == id))
                    .map_or("Nova superfície", |o| o.name.as_str()),
            )
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(self.selected.is_none(), "Nova superfície")
                    .clicked()
                {
                    *self = Self::default();
                    numeric::discard_form(ui, &form_scope);
                    clear_point_entries(ui, &form_scope);
                }

                for object in &project.objects {
                    if metadata(object).ok().flatten().is_some()
                        && !prumo_domain::assemblies::is_child(project, object.id).unwrap_or(true)
                        && ui
                            .selectable_label(self.selected == Some(object.id), &object.name)
                            .clicked()
                    {
                        let _ = self.load(project, object.id);
                        numeric::discard_form(ui, &form_scope);
                        clear_point_entries(ui, &form_scope);
                    }
                }
            });

        field(ui, "Nome da superfície", &mut self.draft.name);

        common_material(
            ui,
            project,
            &mut self.draft.role,
            &mut self.draft.material_id,
        );

        numeric::context(ui, "surface-main".into());

        let mut kind = match self.draft.kind {
            SurfaceKind::SolidSlab { .. } => 0,
            SurfaceKind::Panel { .. } => 1,
            SurfaceKind::Cantilever { .. } => 2,
            SurfaceKind::Ribbed { .. } => 3,
            SurfaceKind::Grillage { .. } => 4,
        };

        let labels = [
            "Laje maciça",
            "Painel",
            "Laje em balanço",
            "Laje nervurada",
            "Grelha sem capa",
        ];

        let before = kind;

        egui::ComboBox::from_label("Tipo de superfície")
            .selected_text(labels[kind])
            .show_ui(ui, |ui| {
                for (i, label) in labels.iter().enumerate() {
                    ui.selectable_value(&mut kind, i, *label);
                }
            });

        if kind != before {
            self.draft.kind = match kind {
                1 => SurfaceKind::Panel {},
                2 => SurfaceKind::Cantilever {},
                3 => SurfaceKind::Ribbed {
                    rib_width_m: 0.12,
                    spacing_x_m: 0.6,
                    spacing_y_m: Some(0.6),
                    extra_depth_m: 0.2,
                },
                4 => SurfaceKind::Grillage {
                    rib_width_m: 0.2,
                    spacing_x_m: 1.,
                    spacing_y_m: 1.,
                    depth_m: 0.4,
                },
                _ => SurfaceKind::SolidSlab {},
            };
            self.draft.thickness_m = if kind == 4 { 0. } else { 0.15 };
            numeric::discard_contexts(ui, |c| c == "surface-main" || c == "surface-spacing-y");
            self.prepared = None;
        }

        if kind != 4 {
            numeric::field(
                ui,
                if kind == 3 {
                    "Espessura da capa (m)"
                } else {
                    "Espessura da superfície (m)"
                },
                &mut self.draft.thickness_m,
            );
        }

        match &mut self.draft.kind {
            SurfaceKind::Ribbed {
                rib_width_m,
                spacing_x_m,
                spacing_y_m,
                extra_depth_m,
            } => {
                numeric::field(ui, "Largura das nervuras (m)", rib_width_m);
                numeric::field(ui, "Espaçamento X (m)", spacing_x_m);

                let mut crossed = spacing_y_m.is_some();
                if ui
                    .checkbox(&mut crossed, "Nervuras nas duas direções")
                    .changed()
                {
                    *spacing_y_m = if crossed { Some(*spacing_x_m) } else { None };
                    numeric::discard_contexts(ui, |c| c == "surface-spacing-y");
                }

                if let Some(y) = spacing_y_m {
                    numeric::context(ui, "surface-spacing-y".into());
                    numeric::field(ui, "Espaçamento Y (m)", y);
                    numeric::context(ui, "surface-main".into());
                }

                numeric::field(ui, "Altura da nervura abaixo da capa (m)", extra_depth_m);
            }

            SurfaceKind::Grillage {
                rib_width_m,
                spacing_x_m,
                spacing_y_m,
                depth_m,
            } => {
                numeric::field(ui, "Largura das barras da grelha (m)", rib_width_m);
                numeric::field(ui, "Espaçamento X (m)", spacing_x_m);
                numeric::field(ui, "Espaçamento Y (m)", spacing_y_m);
                numeric::field(ui, "Altura da grelha (m)", depth_m);
            }

            _ => {}
        }

        ui.collapsing("Posição, inclinação e eixos locais", |ui| {
            frame_editor(ui, "surface-frame", &mut self.draft.frame)
        });

        ui.collapsing("Criar contorno retangular", |ui| {
            numeric::context(ui, "surface-rectangle".into());
            numeric::field(ui, "Largura do retângulo (m)", &mut self.rectangle_width);
            numeric::field(
                ui,
                "Comprimento do retângulo (m)",
                &mut self.rectangle_height,
            );
            if ui
                .add_enabled(
                    !numeric::context_has_errors(ui, "surface-rectangle"),
                    egui::Button::new("Usar este retângulo"),
                )
                .clicked()
            {
                self.draft.outline = rectangle(0., 0., self.rectangle_width, self.rectangle_height);
                discard_collection(ui, "surface-outline");
            }
        });

        ui.collapsing("Vértices do contorno, inclusive formas côncavas", |ui| {
            outline_editor(ui, "surface-outline", &mut self.draft.outline)
        });

        ui.collapsing("Aberturas", |ui| {
            let mut remove = None;

            for (i, opening) in self.draft.openings.iter_mut().enumerate() {
                ui.push_id(("opening", i), |ui| {
                    ui.separator();
                    field(ui, "Nome da abertura", &mut opening.name);
                    outline_editor(ui, &format!("surface-opening-{i}"), &mut opening.outline);
                    if ui
                        .add_enabled(
                            editor_input_ok(ui),
                            egui::Button::new("Retirar esta abertura"),
                        )
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            }

            if let Some(i) = remove {
                self.draft.openings.remove(i);
                discard_collection(ui, "surface-opening-");
            }

            if ui.button("Adicionar abertura retangular").clicked() {
                self.draft.openings.push(SurfaceOpening {
                    name: format!("Abertura {}", self.draft.openings.len() + 1),
                    outline: rectangle(0.5, 0.5, 1., 1.),
                });
            }
        });

        ui.collapsing("Rebaixos, capitéis e regiões de espessura", |ui| {
            let mut remove = None;

            for (i, region) in self.draft.regions.iter_mut().enumerate() {
                ui.push_id(("thickness-region", i), |ui| {
                    field(ui, "Nome da região", &mut region.name);
                    numeric::context(ui, format!("surface-region-{i}"));
                    numeric::field(ui, "Espessura da região (m)", &mut region.thickness_m);
                    numeric::field(
                        ui,
                        "Deslocamento normal da região (m)",
                        &mut region.offset_m,
                    );
                    outline_editor(
                        ui,
                        &format!("surface-region-outline-{i}"),
                        &mut region.outline,
                    );
                    if ui
                        .add_enabled(
                            editor_input_ok(ui),
                            egui::Button::new("Retirar esta região"),
                        )
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            }

            if let Some(i) = remove {
                self.draft.regions.remove(i);
                discard_collection(ui, "surface-region-");
            }

            if ui.button("Adicionar região de espessura").clicked() {
                self.draft.regions.push(ThicknessRegion {
                    name: format!("Região {}", self.draft.regions.len() + 1),
                    outline: rectangle(0., 0., 1., 1.),
                    thickness_m: 0.3,
                    offset_m: -0.075,
                });
            }
        });

        ui.collapsing("Apoios candidatos", |ui| self.show_supports(ui, project));

        ui.collapsing("Bordas livres ou apoiadas", |ui| {
            self.show_edges(ui, project)
        });

        let preview = prumo_domain::surfaces::preview(project, &self.draft);

        if let Err(error) = &preview {
            ui.label(format!("Corrigir antes de aplicar: {error}"));
        }

        let numeric_ok = editor_input_ok(ui);
        show_input_errors(ui);

        if ui
            .add_enabled(
                preview.is_ok() && numeric_ok,
                egui::Button::new("Conferir prévia da superfície"),
            )
            .clicked()
        {
            self.prepared = Some((project.revision, self.draft.clone()));
        }

        let ready =
            self.prepared.as_ref().is_some_and(|(revision, draft)| {
                *revision == project.revision && *draft == self.draft
            }) && numeric_ok;

        if ready && let Ok(r) = preview {
            ui.strong("Prévia conferida para a revisão atual");
            ui.label(format!(
                "Área líquida: {:.4} m². Aberturas: {:.4} m². Volume: {:.5} m³.",
                r.net_area_m2, r.opening_area_m2, r.quantities.total_volume_m3
            ));

            for c in &r.quantities.components {
                ui.label(format!(
                    "{}: {:.5} m³{}",
                    c.component,
                    c.volume_m3,
                    c.mass_kg.map_or(String::new(), |m| format!(", {m:.2} kg"))
                ));
            }

            if self.selected.is_some() {
                ui.label("A edição reconstrói as regiões analíticas e torna obsoletos os resultados e documentos dependentes.");
            }

            for warning in &r.warnings {
                ui.label(format!("Atenção: {warning}"));
            }

            if r.punching_review_required {
                ui.label("Apoios declarados exigirão nova verificação de punção quando o motor correspondente estiver disponível.");
            }
        }

        if ui
            .add_enabled(ready, egui::Button::new("Aplicar superfície conferida"))
            .clicked()
        {
            let command = match self.selected {
                Some(object_id) => SurfaceEdit::Update {
                    object_id,
                    surface: self.draft.clone(),
                },
                None => SurfaceEdit::Create {
                    surface: self.draft.clone(),
                },
            };
            actions.push(UiAction::command(Command::EditModel {
                expected_revision: project.revision,
                edit: ModelEdit::EditSurfaces { command },
            }));
            self.prepared = None;
        }
    }

    fn show_supports(&mut self, ui: &mut egui::Ui, project: &Project) {
        ui.label("Apoios são declarados. Nenhum encontro visual cria ligação resistente. A prévia avisa quando uma abertura remove o contato.");

        let mut remove = None;

        for (i, support) in self.draft.supports.iter_mut().enumerate() {
            ui.push_id(("surface-support", i), |ui| {
                support_selector(ui, project, &mut support.object_id, self.selected);
                numeric::context(ui, format!("surface-support-{i}"));
                numeric::field(ui, "X local do apoio (m)", &mut support.point.x_m);
                numeric::field(ui, "Y local do apoio (m)", &mut support.point.y_m);
                if ui
                    .add_enabled(editor_input_ok(ui), egui::Button::new("Retirar apoio"))
                    .clicked()
                {
                    remove = Some(i);
                }
            });
        }

        if let Some(i) = remove {
            self.draft.supports.remove(i);
            discard_collection(ui, "surface-support-");
        }

        if let Some(target) = project.objects.iter().find(|o| {
            Some(o.id) != self.selected
                && o.role == PhysicalRole::AnalyticalSource
                && o.discipline == prumo_domain::Discipline::Structure
        }) && ui.button("Declarar novo apoio").clicked()
        {
            self.draft.supports.push(SurfaceSupport {
                object_id: target.id,
                point: LocalPoint::new(0., 0.),
            });
        }
    }

    fn show_edges(&mut self, ui: &mut egui::Ui, project: &Project) {
        ui.label("Todas as bordas externas começam livres. Um apoio de borda precisa indicar a peça que o recebe.");

        for edge in 0..self.draft.outline.len() {
            ui.push_id(("surface-edge", edge), |ui| {
                let mut target = self
                    .draft
                    .edge_conditions
                    .iter()
                    .find(|c| c.edge == edge)
                    .and_then(|c| match c.behavior {
                        EdgeBehavior::CandidateSupport { object_id } => Some(object_id),
                        _ => None,
                    });

                let before = target;

                egui::ComboBox::from_label(format!("Borda {}", edge + 1))
                    .selected_text(
                        target
                            .and_then(|id| project.objects.iter().find(|o| o.id == id))
                            .map_or("Livre", |o| o.name.as_str()),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut target, None, "Livre");
                        for o in &project.objects {
                            if Some(o.id) != self.selected
                                && o.discipline == prumo_domain::Discipline::Structure
                                && o.role == PhysicalRole::AnalyticalSource
                            {
                                ui.selectable_value(
                                    &mut target,
                                    Some(o.id),
                                    format!("Apoio candidato: {}", o.name),
                                );
                            }
                        }
                    });

                if target != before {
                    self.draft.edge_conditions.retain(|c| c.edge != edge);
                    self.draft.edge_conditions.push(EdgeCondition {
                        edge,
                        behavior: target.map_or(EdgeBehavior::Free {}, |object_id| {
                            EdgeBehavior::CandidateSupport { object_id }
                        }),
                    });
                }
            });
        }
    }
}

pub(crate) fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Vec<LocalPoint> {
    vec![
        LocalPoint::new(x, y),
        LocalPoint::new(x + width, y),
        LocalPoint::new(x + width, y + height),
        LocalPoint::new(x, y + height),
    ]
}
pub(crate) fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    let label = ui.label(label);
    ui.text_edit_singleline(value).labelled_by(label.id);
}
pub(crate) fn common_material(
    ui: &mut egui::Ui,
    project: &Project,
    role: &mut PhysicalRole,
    material: &mut Option<Uuid>,
) {
    let mut structural = *role == PhysicalRole::AnalyticalSource;
    ui.checkbox(&mut structural, "Elemento estrutural promovido");
    *role = if structural {
        PhysicalRole::AnalyticalSource
    } else {
        PhysicalRole::VisualOnly
    };
    egui::ComboBox::from_label("Material da peça")
        .selected_text(
            material
                .and_then(|id| project.model.materials.iter().find(|m| m.id == id))
                .map_or("Escolha o material", |m| m.name.as_str()),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(material, None, "Sem material, referência arquitetônica");
            for m in &project.model.materials {
                ui.selectable_value(material, Some(m.id), &m.name);
            }
        });
    if project.model.materials.is_empty() {
        ui.label(
            "Cadastre o material em Modelo físico e analítico antes de promover a peça estrutural.",
        );
    }
}
pub(crate) fn point_editor(
    ui: &mut egui::Ui,
    key: &str,
    label: &str,
    point: &mut prumo_domain::units::Point3,
) {
    numeric::context(ui, key.into());
    let scope = ui.data(|data| {
        data.get_temp::<String>(egui::Id::unique("parametric-point-scope"))
            .unwrap_or_default()
    });
    let map_id = egui::Id::unique("parametric-point-entries");
    let mut entries = ui.data(|data| {
        data.get_temp::<BTreeMap<(String, String), PointEntry>>(map_id)
            .unwrap_or_default()
    });
    let entry = entries.entry((scope, key.into())).or_insert(PointEntry {
        basis: *point,
        values: point.as_array(),
        error: None,
    });
    if entry.basis != *point {
        *entry = PointEntry {
            basis: *point,
            values: point.as_array(),
            error: None,
        };
    }
    for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
        numeric::field(ui, &format!("{label}: {axis} (m)"), &mut entry.values[i]);
    }
    if numeric::context_has_errors(ui, key) {
        entry.error = Some("Preencha todas as coordenadas com números válidos.".into());
    } else {
        match prumo_domain::units::Point3::try_from(entry.values) {
            Ok(p) => {
                *point = p;
                entry.basis = p;
                entry.error = None;
            }
            Err(error) => entry.error = Some(error.to_string()),
        }
    }
    if let Some(error) = &entry.error {
        ui.label(format!("Coordenada não aplicada: {error}"));
    }
    ui.data_mut(|data| data.insert_temp(map_id, entries));
}

#[derive(Clone)]
struct PointEntry {
    basis: prumo_domain::units::Point3,
    values: [f64; 3],
    error: Option<String>,
}
pub(crate) fn point_scope(ui: &mut egui::Ui, scope: String) {
    ui.data_mut(|data| data.insert_temp(egui::Id::unique("parametric-point-scope"), scope));
}
pub(crate) fn point_errors(ui: &egui::Ui) -> bool {
    ui.data(|data| {
        let scope = data
            .get_temp::<String>(egui::Id::unique("parametric-point-scope"))
            .unwrap_or_default();
        data.get_temp::<BTreeMap<(String, String), PointEntry>>(egui::Id::unique(
            "parametric-point-entries",
        ))
        .unwrap_or_default()
        .iter()
        .any(|((s, _), entry)| s == &scope && entry.error.is_some())
    })
}
pub(crate) fn clear_point_entries(ui: &mut egui::Ui, scope: &str) {
    ui.data_mut(|data| {
        let id = egui::Id::unique("parametric-point-entries");
        let mut entries = data
            .get_temp::<BTreeMap<(String, String), PointEntry>>(id)
            .unwrap_or_default();
        entries.retain(|(s, _), _| s != scope);
        data.insert_temp(id, entries);
    });
}

pub(crate) fn editor_input_ok(ui: &egui::Ui) -> bool {
    numeric::errors(ui).is_empty() && !point_errors(ui)
}

pub(crate) fn show_input_errors(ui: &mut egui::Ui) {
    for label in numeric::errors(ui) {
        ui.colored_label(
            egui::Color32::LIGHT_RED,
            format!("Corrija o campo, mesmo se a seção estiver recolhida: {label}"),
        );
    }
    if point_errors(ui) {
        ui.colored_label(egui::Color32::LIGHT_RED,"Há uma coordenada inválida no formulário. Reabra a seção e corrija antes de conferir a prévia.");
    }
}

/// An indexed collection can be renumbered only after invalid entries have
/// been corrected. Its replaced widget contexts are then reset explicitly.
pub(crate) fn discard_collection(ui: &mut egui::Ui, prefix: &str) {
    numeric::discard_contexts(ui, |context| context.starts_with(prefix));
    let scope = ui.data(|data| {
        data.get_temp::<String>(egui::Id::unique("parametric-point-scope"))
            .unwrap_or_default()
    });
    ui.data_mut(|data| {
        let id = egui::Id::unique("parametric-point-entries");
        let mut entries = data
            .get_temp::<BTreeMap<(String, String), PointEntry>>(id)
            .unwrap_or_default();
        entries.retain(|(s, key), _| s != &scope || !key.starts_with(prefix));
        data.insert_temp(id, entries);
    });
}
pub(crate) fn frame_editor(ui: &mut egui::Ui, key: &str, frame: &mut SurfaceFrame) {
    point_editor(ui, &format!("{key}-origin"), "Origem", &mut frame.origin_m);
    numeric::context(ui, format!("{key}-axes"));
    for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
        numeric::field(ui, &format!("Eixo U, componente {axis}"), &mut frame.u[i]);
        numeric::field(ui, &format!("Eixo V, componente {axis}"), &mut frame.v[i]);
    }
    ui.small("U e V devem ser unitários e perpendiculares. A normal positiva é U × V.");
}
pub(crate) fn outline_editor(ui: &mut egui::Ui, key: &str, points: &mut Vec<LocalPoint>) {
    let mut remove = None;
    for (i, point) in points.iter_mut().enumerate() {
        numeric::context(ui, format!("{key}-{i}"));
        ui.horizontal(|ui| {
            numeric::field(ui, &format!("Vértice {}: X (m)", i + 1), &mut point.x_m);
            numeric::field(ui, &format!("Vértice {}: Y (m)", i + 1), &mut point.y_m);
            if ui
                .add_enabled(
                    editor_input_ok(ui),
                    egui::Button::new(format!("Remover vértice {}", i + 1)),
                )
                .clicked()
            {
                remove = Some(i);
            }
        });
    }
    if points.len() > 3
        && let Some(i) = remove
    {
        points.remove(i);
        discard_collection(ui, &format!("{key}-"));
    }
    if ui.button("Adicionar vértice").clicked() && points.len() < 512 {
        let p = points.last().copied().unwrap_or(LocalPoint::new(0., 0.));
        points.push(LocalPoint::new(p.x_m + 1., p.y_m));
    }
    ui.small("Informe o contorno no sentido positivo dos eixos locais. Não repita o ponto de fechamento.");
}
pub(crate) fn support_selector(
    ui: &mut egui::Ui,
    project: &Project,
    id: &mut Uuid,
    exclude: Option<Uuid>,
) {
    egui::ComboBox::from_label("Peça de apoio")
        .selected_text(
            project
                .objects
                .iter()
                .find(|o| o.id == *id)
                .map_or("Escolha o apoio", |o| o.name.as_str()),
        )
        .show_ui(ui, |ui| {
            for o in &project.objects {
                if Some(o.id) != exclude
                    && o.discipline == prumo_domain::Discipline::Structure
                    && o.role == PhysicalRole::AnalyticalSource
                {
                    ui.selectable_value(id, o.id, &o.name);
                }
            }
        });
}
