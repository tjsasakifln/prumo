use super::*;

pub struct TransformationForm {
    pub selected: String,
    pub group: Vec<String>,
    pub copy_name: String,
    pub merge_name: String,
    pub split_names: [String; 2],
    pub fraction: f64,
    pub translation: [f64; 3],
    pub unit: LengthUnit,
    pub rotation_degrees: [f64; 3],
}
impl Default for TransformationForm {
    fn default() -> Self {
        Self {
            selected: String::new(),
            group: Vec::new(),
            copy_name: "Cópia 01".into(),
            merge_name: "Peça reunida".into(),
            split_names: ["Trecho A".into(), "Trecho B".into()],
            fraction: 0.5,
            translation: [0.0, 1.0, 0.0],
            unit: LengthUnit::Metre,
            rotation_degrees: [0.0; 3],
        }
    }
}
impl TransformationForm {
    pub(super) fn show(&mut self, ui: &mut Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.strong("Cópia, divisão, fusão e movimento rígido");
        ui.label("As novas peças recebem identidades próprias. O histórico preserva as origens e os vínculos tratados pela operação.");
        choice(
            ui,
            "Peça de origem",
            &mut self.selected,
            objects(project),
            false,
        );
        text(ui, "Nome da cópia", &mut self.copy_name);
        length_unit(ui, "Unidade da translação", &mut self.unit);
        vector(ui, "Translação", &mut self.translation);
        if ui
            .add_enabled(
                !self.selected.is_empty(),
                egui::Button::new("Copiar peça física"),
            )
            .clicked()
            && let Ok(source_id) = self.selected.parse()
        {
            emit(
                actions,
                project,
                ModelEdit::CopyPhysical {
                    source_id,
                    name: self.copy_name.clone(),
                    translation: point(self.translation, self.unit),
                },
            );
        }
        ui.small("A cópia não cria nós, ações, portas ou conexões automaticamente.");
        ui.separator();
        number(ui, "Fração da divisão", &mut self.fraction);
        text(ui, "Nome do primeiro trecho", &mut self.split_names[0]);
        text(ui, "Nome do segundo trecho", &mut self.split_names[1]);
        if ui
            .add_enabled(
                !self.selected.is_empty(),
                egui::Button::new("Dividir peça selecionada"),
            )
            .clicked()
            && let Ok(object_id) = self.selected.parse()
        {
            emit(
                actions,
                project,
                ModelEdit::SplitPhysical {
                    object_id,
                    fraction: self.fraction,
                    names: self.split_names.clone(),
                },
            );
        }
        ui.separator();
        ui.strong("Seleção para fusão ou transformação");
        self.group.retain(|id| {
            project
                .objects
                .iter()
                .any(|object| object.id.to_string() == *id)
        });
        egui::ScrollArea::vertical()
            .max_height(150.0)
            .id_salt("transform-group")
            .show(ui, |ui| {
                for object in &project.objects {
                    let id = object.id.to_string();
                    let mut selected = self.group.contains(&id);
                    if ui
                        .checkbox(
                            &mut selected,
                            format!("Selecionar {} ({})", object.name, short(object.id)),
                        )
                        .changed()
                    {
                        if selected {
                            self.group.push(id);
                        } else {
                            self.group.retain(|value| *value != id);
                        }
                    }
                }
            });
        text(ui, "Nome após fusão", &mut self.merge_name);
        if ui
            .add_enabled(
                self.group.len() >= 2,
                egui::Button::new("Fundir peças selecionadas"),
            )
            .clicked()
        {
            let source_ids = self.group.iter().filter_map(|id| id.parse().ok()).collect();
            emit(
                actions,
                project,
                ModelEdit::MergePhysical {
                    source_ids,
                    name: self.merge_name.clone(),
                },
            );
        }
        ui.small("A fusão exige continuidade e compatibilidade. Nenhuma ligação estrutural é criada por tolerância visual.");
        vector(ui, "Rotação em graus", &mut self.rotation_degrees);
        ui.label(
            "Rotação sucessiva em X, Y e Z, seguida da translação acima, nos eixos de engenharia.",
        );
        if ui
            .add_enabled(
                !self.group.is_empty(),
                egui::Button::new("Aplicar transformação rígida"),
            )
            .clicked()
        {
            let object_ids = self.group.iter().filter_map(|id| id.parse().ok()).collect();
            emit(
                actions,
                project,
                ModelEdit::RigidMove {
                    object_ids,
                    rotation: rotation(self.rotation_degrees),
                    translation: point(self.translation, self.unit),
                },
            );
        }
        ui.separator();
        ui.collapsing("Histórico de cópia, fusão e divisão", |ui| {
            for record in &project.model.lineage {
                let operation = match record.kind {
                    prumo_domain::model::LineageKind::Copy => "Cópia",
                    prumo_domain::model::LineageKind::Merge => "Fusão",
                    prumo_domain::model::LineageKind::Split => "Divisão",
                };
                ui.label(format!(
                    "{} | revisão {} | autor {}",
                    operation, record.revision, record.actor
                ));
                ui.small(format!(
                    "Origem: {:?}\nDestino: {:?}",
                    record.parent_ids, record.child_ids
                ));
            }
        });
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum OriginKind {
    Native,
    Imported,
    Derived,
}

pub struct CoordinateForm {
    pub unit: LengthUnit,
    pub frame_origin: [f64; 3],
    pub frame_rotation: [[f64; 3]; 3],
    pub frame_angles: [f64; 3],
    pub map_origin: [f64; 3],
    pub crs: String,
    pub map_rotation: f64,
    pub map_scale: f64,
    pub entity: String,
    pub origin_kind: OriginKind,
    pub origin_system: String,
    pub origin_reference: String,
    pub origin_external: String,
    pub origin_operation: String,
}
impl Default for CoordinateForm {
    fn default() -> Self {
        Self {
            unit: LengthUnit::Metre,
            frame_origin: [0.0; 3],
            frame_rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            frame_angles: [0.0; 3],
            map_origin: [0.0; 3],
            crs: String::new(),
            map_rotation: 0.0,
            map_scale: 1.0,
            entity: String::new(),
            origin_kind: OriginKind::Native,
            origin_system: String::new(),
            origin_reference: String::new(),
            origin_external: String::new(),
            origin_operation: String::new(),
        }
    }
}
impl CoordinateForm {
    pub(super) fn load(&mut self, project: &Project) {
        self.unit = project.units;
        self.frame_origin = project.model.engineering_frame.origin_world_m.as_array();
        self.frame_rotation = project.model.engineering_frame.rotation_to_world;
        if let Some(geo) = &project.model.georeference {
            self.crs = geo.crs_identifier.clone();
            self.map_origin = geo.map_origin_m.as_array();
            self.map_rotation = geo.rotation_radians.si().to_degrees();
            self.map_scale = geo.scale;
        } else {
            self.crs.clear();
            self.map_origin = [0.0; 3];
            self.map_rotation = 0.0;
            self.map_scale = 1.0;
        }
        self.load_origin(project);
    }

    fn load_origin(&mut self, project: &Project) {
        let Some(identity) = project
            .model
            .identities
            .iter()
            .find(|(id, identity)| {
                id.to_string() == self.entity && identity.retired_revision.is_none()
            })
            .map(|(_, identity)| identity)
        else {
            self.entity.clear();
            return;
        };
        match &identity.origin {
            SourceOrigin::Native {} => self.origin_kind = OriginKind::Native,
            SourceOrigin::Imported {
                system,
                reference,
                external_id,
            } => {
                self.origin_kind = OriginKind::Imported;
                self.origin_system = system.clone();
                self.origin_reference = reference.clone();
                self.origin_external = external_id.clone();
            }
            SourceOrigin::Derived { operation } => {
                self.origin_kind = OriginKind::Derived;
                self.origin_operation = operation.clone();
            }
        }
    }

    pub(super) fn show(
        &mut self,
        ui: &mut Ui,
        project: &Project,
        actions: &mut Vec<UiAction>,
        error: &mut Option<String>,
    ) {
        ui.strong("Unidade de apresentação");
        ui.label(format!(
            "Unidade atual: {}. O modelo persistido usa grandezas SI.",
            project.units.symbol()
        ));
        length_unit(ui, "Nova unidade de apresentação", &mut self.unit);
        if ui.button("Alterar unidade de apresentação").clicked() {
            emit(
                actions,
                project,
                ModelEdit::SetDisplayUnit { unit: self.unit },
            );
        }
        ui.separator();
        ui.strong("Referência de engenharia");
        ui.label("A mudança de referência preserva a posição física mundial, transformando coordenadas e ações. O mapa é configurado separadamente.");
        ui.label(format!(
            "Origem mundial atual: {:?} m",
            project.model.engineering_frame.origin_world_m.as_array()
        ));
        if ui.button("Carregar referência atual").clicked() {
            self.frame_origin = project.model.engineering_frame.origin_world_m.as_array();
            self.frame_rotation = project.model.engineering_frame.rotation_to_world;
        }
        vector(ui, "Nova origem mundial (m)", &mut self.frame_origin);
        vector(ui, "Ângulos da referência (graus)", &mut self.frame_angles);
        if ui.button("Preencher matriz com os ângulos").clicked() {
            self.frame_rotation = rotation(self.frame_angles);
        }
        for (index, row) in self.frame_rotation.iter_mut().enumerate() {
            vector(ui, &format!("Matriz, linha {}", index + 1), row);
        }
        if ui.button("Aplicar referência de engenharia").clicked() {
            match point(self.frame_origin, LengthUnit::Metre).to_si() {
                Ok(origin_world_m) => {
                    *error = None;
                    emit(
                        actions,
                        project,
                        ModelEdit::ChangeEngineeringFrame {
                            frame: EngineeringFrame {
                                origin_world_m,
                                rotation_to_world: self.frame_rotation,
                            },
                        },
                    );
                }
                Err(problem) => *error = Some(problem.to_string()),
            }
        }
        ui.separator();
        ui.strong("Georreferenciamento");
        if let Some(geo) = &project.model.georeference {
            ui.label(format!(
                "Referência atual: {} | origem {:?} m",
                geo.crs_identifier,
                geo.map_origin_m.as_array()
            ));
            if ui.button("Carregar referência do mapa").clicked() {
                self.crs = geo.crs_identifier.clone();
                self.map_origin = geo.map_origin_m.as_array();
                self.map_rotation = geo.rotation_radians.si().to_degrees();
                self.map_scale = geo.scale;
            }
        } else {
            ui.label("Nenhum sistema de mapa foi atribuído.");
        }
        text(ui, "Identificador do sistema de mapa", &mut self.crs);
        vector(ui, "Origem no mapa (m)", &mut self.map_origin);
        number(ui, "Rotação no mapa (graus)", &mut self.map_rotation);
        number(ui, "Escala do mapa", &mut self.map_scale);
        ui.horizontal(|ui| {
            if ui.button("Salvar georreferenciamento").clicked() {
                match (
                    point(self.map_origin, LengthUnit::Metre).to_si(),
                    prumo_domain::units::Angle::try_from(self.map_rotation.to_radians()),
                ) {
                    (Ok(map_origin_m), Ok(rotation_radians)) => {
                        *error = None;
                        emit(
                            actions,
                            project,
                            ModelEdit::SetGeoreference {
                                georeference: Some(Georeference {
                                    crs_identifier: self.crs.clone(),
                                    map_origin_m,
                                    rotation_radians,
                                    scale: self.map_scale,
                                }),
                            },
                        );
                    }
                    (Err(problem), _) | (_, Err(problem)) => *error = Some(problem.to_string()),
                }
            }
            if ui
                .add_enabled(
                    project.model.georeference.is_some(),
                    egui::Button::new("Remover georreferenciamento"),
                )
                .clicked()
            {
                emit(
                    actions,
                    project,
                    ModelEdit::SetGeoreference { georeference: None },
                );
            }
        });
        ui.separator();
        ui.strong("Identidade e procedência");
        let entities = project
            .model
            .identities
            .iter()
            .filter(|(_, identity)| identity.retired_revision.is_none())
            .map(|(id, _)| (id.to_string(), entity_name(project, *id)))
            .collect();
        if choice(ui, "Entidade de origem", &mut self.entity, entities, false) {
            self.load_origin(project);
        }
        if let Some(identity) = project
            .model
            .identities
            .iter()
            .find(|(id, _)| id.to_string() == self.entity)
            .map(|(_, identity)| identity)
        {
            ui.label(format!(
                "UUID: {} | GUID IFC: {}",
                self.entity,
                identity
                    .ifc_guid
                    .as_ref()
                    .map(|id| id.as_str())
                    .unwrap_or("Não aplicável")
            ));
            ui.small(format!(
                "Criado por {} na revisão {}",
                identity
                    .created_by
                    .as_deref()
                    .unwrap_or("Autor não registrado"),
                identity
                    .created_revision
                    .map(|revision| revision.to_string())
                    .unwrap_or_else(|| "não registrada".into())
            ));
        }
        enum_choice(
            ui,
            "Procedência",
            &mut self.origin_kind,
            &[
                (OriginKind::Native, "Nativa"),
                (OriginKind::Imported, "Importada"),
                (OriginKind::Derived, "Derivada"),
            ],
        );
        match self.origin_kind {
            OriginKind::Native => {}
            OriginKind::Imported => {
                text(ui, "Sistema de origem", &mut self.origin_system);
                text(ui, "Referência de origem", &mut self.origin_reference);
                text(ui, "Identificador externo", &mut self.origin_external);
            }
            OriginKind::Derived => text(ui, "Operação de origem", &mut self.origin_operation),
        }
        if ui
            .add_enabled(
                !self.entity.is_empty(),
                egui::Button::new("Registrar procedência"),
            )
            .clicked()
            && let Ok(entity_id) = self.entity.parse()
        {
            let origin = match self.origin_kind {
                OriginKind::Native => SourceOrigin::Native {},
                OriginKind::Imported => SourceOrigin::Imported {
                    system: self.origin_system.clone(),
                    reference: self.origin_reference.clone(),
                    external_id: self.origin_external.clone(),
                },
                OriginKind::Derived => SourceOrigin::Derived {
                    operation: self.origin_operation.clone(),
                },
            };
            emit(
                actions,
                project,
                ModelEdit::SetSourceOrigin { entity_id, origin },
            );
        }
    }
}

fn entity_name(project: &Project, id: impl std::fmt::Display) -> String {
    let id = id.to_string();
    for (entity, name) in objects(project)
        .into_iter()
        .chain(nodes(project))
        .chain(members(project))
        .chain(materials(project))
        .chain(environments(project))
    {
        if entity == id {
            return name;
        }
    }
    format!("Entidade ({})", short(id))
}

/// Presentation-only conversion. The domain validates that the matrix is a proper rotation.
fn rotation(degrees: [f64; 3]) -> [[f64; 3]; 3] {
    let [x, y, z] = degrees.map(f64::to_radians);
    let (sx, cx) = x.sin_cos();
    let (sy, cy) = y.sin_cos();
    let (sz, cz) = z.sin_cos();
    [
        [cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx],
        [sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx],
        [-sy, cy * sx, cy * cx],
    ]
}
