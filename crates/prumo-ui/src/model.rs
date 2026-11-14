//! Forms for the shared model commands. Values are converted explicitly at the boundary.
use crate::UiAction;
use egui::{Color32, Ui};
use prumo_domain::model::{
    ActionFrame, DensityInput, EngineeringFrame, ForceInput, Georeference, MaterialFamily,
    MaterialInput, ModelEdit, MomentInput, NetworkKind, NodalActionInput, PhysicalRole, PointInput,
    PressureInput, SourceOrigin,
};
use prumo_domain::units::{DensityUnit, ForceUnit, MomentUnit, PressureUnit};
use prumo_domain::{Command, Discipline, LengthUnit, PortKind, Project};

mod networks;
mod trace;
mod transformations;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ModelTab {
    #[default]
    Physical,
    Analytical,
    Materials,
    Actions,
    Transformations,
    Networks,
    Coordinates,
    Trace,
}

#[derive(Default)]
pub struct SemanticView {
    pub tab: ModelTab,
    pub physical: PhysicalForm,
    pub analytical: AnalyticalForm,
    pub material: MaterialForm,
    pub action: ActionForm,
    pub transformations: transformations::TransformationForm,
    pub network: networks::NetworkForm,
    pub coordinates: transformations::CoordinateForm,
    pub trace: trace::TraceView,
    pub error: Option<String>,
    loaded: Option<(String, u64)>,
}

impl SemanticView {
    pub fn show(&mut self, ui: &mut Ui, project: &Project, actions: &mut Vec<UiAction>) {
        let version = (project.id.to_string(), project.revision);
        if self.loaded.as_ref() != Some(&version) {
            self.physical.load(project);
            self.material.load(project);
            self.action.load(project);
            self.coordinates.load(project);
            self.loaded = Some(version);
            crate::numeric::discard_edit_buffers(ui);
        }
        ui.collapsing("Modelo físico e analítico", |ui| {
            ui.label("Selecione a operação. As alterações usam a revisão exibida e são salvas após a validação.");
            ui.horizontal_wrapped(|ui| {
                for (tab, label) in [
                    (ModelTab::Physical, "Peças"),
                    (ModelTab::Analytical, "Analítico"),
                    (ModelTab::Materials, "Materiais"),
                    (ModelTab::Actions, "Ações"),
                    (ModelTab::Transformations, "Cópia e transformação"),
                    (ModelTab::Networks, "Redes e portas"),
                    (ModelTab::Coordinates, "Origem e unidades"),
                    (ModelTab::Trace, "Medidas e dependências"),
                ] {
                    ui.selectable_value(&mut self.tab, tab, label);
                }
            });
            ui.separator();
            let previous_actions=actions.len();
            crate::numeric::begin(ui);
            crate::numeric::context(ui,format!("{}:{:?}",project.id,self.tab));
            match self.tab {
                ModelTab::Physical => self.physical.show(ui, project, actions),
                ModelTab::Analytical => self.analytical.show(ui, project, actions),
                ModelTab::Materials => self.material.show(ui, project, actions),
                ModelTab::Actions => self.action.show(ui, project, actions),
                ModelTab::Transformations => self.transformations.show(ui, project, actions),
                ModelTab::Networks => self.network.show(ui, project, actions),
                ModelTab::Coordinates => self.coordinates.show(ui, project, actions, &mut self.error),
                ModelTab::Trace => self.trace.show(ui, project, actions),
            }
            let invalid=crate::numeric::errors(ui);
            if !invalid.is_empty() {
                actions.truncate(previous_actions);
                ui.colored_label(Color32::LIGHT_RED,format!("Corrija os campos numéricos antes de salvar: {}.",invalid.join(", ")));
            }
            if let Some(error) = &self.error {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
        });
    }
}

pub struct PhysicalForm {
    pub selected: String,
    pub name: String,
    pub discipline: Discipline,
    pub role: PhysicalRole,
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub unit: LengthUnit,
    pub material: String,
    pub environment: String,
}

impl Default for PhysicalForm {
    fn default() -> Self {
        Self {
            selected: String::new(),
            name: "Objeto 01".into(),
            discipline: Discipline::Structure,
            role: PhysicalRole::VisualOnly,
            start: [0.0; 3],
            end: [4.0, 0.0, 0.0],
            unit: LengthUnit::Metre,
            material: String::new(),
            environment: String::new(),
        }
    }
}

impl PhysicalForm {
    fn load(&mut self, project: &Project) {
        if let Some(object) = project
            .objects
            .iter()
            .find(|object| object.id.to_string() == self.selected)
        {
            self.name = object.name.clone();
            self.discipline = object.discipline;
            self.role = object.role;
            self.start = object.start_m.as_array();
            self.end = object.end_m.as_array();
            self.unit = LengthUnit::Metre;
            self.material = object
                .material_id
                .map(|id| id.to_string())
                .unwrap_or_default();
            self.environment = object
                .environment_id
                .map(|id| id.to_string())
                .unwrap_or_default();
        } else {
            self.selected.clear();
        }
    }

    fn show(&mut self, ui: &mut Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.strong("Geometria física");
        ui.label("Um objeto visual não cria elemento analítico. A origem analítica permite vínculos explícitos na aba Analítico.");
        if choice(
            ui,
            "Peça para editar",
            &mut self.selected,
            objects(project),
            true,
        ) {
            self.load(project);
        }
        crate::numeric::context(ui, format!("{}:physical:{}", project.id, self.selected));
        text(ui, "Nome do objeto físico", &mut self.name);
        discipline(ui, "Disciplina do objeto", &mut self.discipline);
        enum_choice(
            ui,
            "Função do objeto",
            &mut self.role,
            &[
                (PhysicalRole::VisualOnly, "Somente visual"),
                (PhysicalRole::AnalyticalSource, "Origem para análise"),
            ],
        );
        length_unit(ui, "Unidade da geometria", &mut self.unit);
        vector(ui, "Início físico", &mut self.start);
        vector(ui, "Fim físico", &mut self.end);
        choice(
            ui,
            "Material da peça",
            &mut self.material,
            materials(project),
            true,
        );
        choice(
            ui,
            "Ambiente da peça",
            &mut self.environment,
            environments(project),
            true,
        );
        ui.horizontal_wrapped(|ui| {
            if ui.button("Criar objeto físico").clicked() {
                emit(
                    actions,
                    project,
                    ModelEdit::AddPhysical {
                        name: self.name.clone(),
                        discipline: self.discipline,
                        start: point(self.start, self.unit),
                        end: point(self.end, self.unit),
                        role: self.role,
                        material_id: self.material.parse().ok(),
                        environment_id: self.environment.parse().ok(),
                    },
                );
            }
            if let Ok(object_id) = self.selected.parse() {
                if ui.button("Salvar geometria da peça").clicked() {
                    emit(
                        actions,
                        project,
                        ModelEdit::SetPhysicalGeometry {
                            object_id,
                            start: point(self.start, self.unit),
                            end: point(self.end, self.unit),
                        },
                    );
                }
                if ui.button("Salvar função da peça").clicked() {
                    emit(
                        actions,
                        project,
                        ModelEdit::SetPhysicalRole {
                            object_id,
                            role: self.role,
                        },
                    );
                }
                if ui.button("Aplicar material à peça").clicked() {
                    emit(
                        actions,
                        project,
                        ModelEdit::SetPhysicalMaterial {
                            object_id,
                            material_id: self.material.parse().ok(),
                        },
                    );
                }
            }
        });
        ui.small("Disciplina e ambiente são definidos na criação. Os botões de edição alteram somente o campo indicado.");
        for object in &project.objects {
            let count = project
                .model
                .links
                .iter()
                .filter(|link| link.physical_id == object.id)
                .count();
            ui.label(format!(
                "{} | {} | {count} vínculos analíticos | {:.4} m",
                object.name,
                role_label(object.role),
                object.length_m()
            ));
        }
    }
}

pub struct AnalyticalForm {
    pub node_name: String,
    pub position: [f64; 3],
    pub unit: LengthUnit,
    pub member_name: String,
    pub start_node: String,
    pub end_node: String,
    pub local_y: [f64; 3],
    pub physical: String,
    pub member: String,
    pub link: String,
}
impl Default for AnalyticalForm {
    fn default() -> Self {
        Self {
            node_name: "Nó 01".into(),
            position: [0.0; 3],
            unit: LengthUnit::Metre,
            member_name: "Barra analítica 01".into(),
            start_node: String::new(),
            end_node: String::new(),
            local_y: [0.0, 1.0, 0.0],
            physical: String::new(),
            member: String::new(),
            link: String::new(),
        }
    }
}
impl AnalyticalForm {
    fn show(&mut self, ui: &mut Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.strong("Nós e barras analíticas");
        text(ui, "Nome do nó", &mut self.node_name);
        length_unit(ui, "Unidade do nó", &mut self.unit);
        vector(ui, "Posição do nó", &mut self.position);
        if ui.button("Criar nó analítico").clicked() {
            emit(
                actions,
                project,
                ModelEdit::AddAnalyticalNode {
                    name: self.node_name.clone(),
                    position: point(self.position, self.unit),
                },
            );
        }
        ui.separator();
        text(ui, "Nome da barra analítica", &mut self.member_name);
        choice(
            ui,
            "Nó inicial",
            &mut self.start_node,
            nodes(project),
            false,
        );
        choice(ui, "Nó final", &mut self.end_node, nodes(project), false);
        vector(ui, "Direção local Y", &mut self.local_y);
        ui.small(
            "A direção Y deve ser transversal ao eixo da barra. O domínio valida a base local.",
        );
        if ui
            .add_enabled(
                !self.start_node.is_empty() && !self.end_node.is_empty(),
                egui::Button::new("Criar barra analítica"),
            )
            .clicked()
            && let (Ok(start_node_id), Ok(end_node_id)) =
                (self.start_node.parse(), self.end_node.parse())
        {
            emit(
                actions,
                project,
                ModelEdit::AddAnalyticalMember {
                    name: self.member_name.clone(),
                    start_node_id,
                    end_node_id,
                    local_y: self.local_y,
                },
            );
        }
        ui.separator();
        ui.strong("Relações físicas e analíticas");
        ui.label("Uma peça pode apontar para várias barras. Várias peças podem apontar para a mesma barra.");
        choice(
            ui,
            "Peça do vínculo",
            &mut self.physical,
            objects(project),
            false,
        );
        choice(
            ui,
            "Barra do vínculo",
            &mut self.member,
            members(project),
            false,
        );
        if ui
            .add_enabled(
                !self.physical.is_empty() && !self.member.is_empty(),
                egui::Button::new("Vincular peça e barra"),
            )
            .clicked()
            && let (Ok(physical_id), Ok(analytical_id)) =
                (self.physical.parse(), self.member.parse())
        {
            emit(
                actions,
                project,
                ModelEdit::LinkPhysicalAnalytical {
                    physical_id,
                    analytical_id,
                },
            );
        }
        let links = project
            .model
            .links
            .iter()
            .map(|link| {
                let physical = project
                    .objects
                    .iter()
                    .find(|object| object.id == link.physical_id)
                    .map(|object| object.name.as_str())
                    .unwrap_or("Ausente");
                let analytical = project
                    .model
                    .analytical_members
                    .iter()
                    .find(|member| member.id == link.analytical_id)
                    .map(|member| member.name.as_str())
                    .unwrap_or("Ausente");
                (
                    link.id.to_string(),
                    format!("{physical} com {analytical} ({})", short(link.id)),
                )
            })
            .collect();
        choice(ui, "Vínculo para remover", &mut self.link, links, false);
        if ui
            .add_enabled(
                !self.link.is_empty(),
                egui::Button::new("Remover vínculo explícito"),
            )
            .clicked()
            && let Ok(link_id) = self.link.parse()
        {
            emit(
                actions,
                project,
                ModelEdit::UnlinkPhysicalAnalytical { link_id },
            );
        }
        ui.label(format!(
            "{} nós | {} barras | {} vínculos",
            project.model.analytical_nodes.len(),
            project.model.analytical_members.len(),
            project.model.links.len()
        ));
    }
}

pub struct MaterialForm {
    pub selected: String,
    pub name: String,
    pub family: MaterialFamily,
    pub elastic_modulus: f64,
    pub pressure_unit: PressureUnit,
    pub density: f64,
    pub poisson_ratio: f64,
}
impl Default for MaterialForm {
    fn default() -> Self {
        Self {
            selected: String::new(),
            name: "Material de estudo".into(),
            family: MaterialFamily::Generic,
            elastic_modulus: 30.0,
            pressure_unit: PressureUnit::Gigapascal,
            density: 2500.0,
            poisson_ratio: 0.2,
        }
    }
}
impl MaterialForm {
    fn load(&mut self, project: &Project) {
        if let Some(material) = project
            .model
            .materials
            .iter()
            .find(|material| material.id.to_string() == self.selected)
        {
            self.name = material.name.clone();
            self.family = material.family;
            self.elastic_modulus = material.elastic_modulus.si();
            self.pressure_unit = PressureUnit::Pascal;
            self.density = material.density.si();
            self.poisson_ratio = material.poisson_ratio;
        } else {
            self.selected.clear();
        }
    }

    fn input(&self) -> MaterialInput {
        MaterialInput {
            name: self.name.clone(),
            family: self.family,
            elastic_modulus: PressureInput {
                value: self.elastic_modulus,
                unit: self.pressure_unit,
            },
            density: DensityInput {
                value: self.density,
                unit: DensityUnit::KilogramPerCubicMetre,
            },
            poisson_ratio: self.poisson_ratio,
        }
    }
    fn show(&mut self, ui: &mut Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.strong("Propriedades do material");
        ui.label("Dados informados pelo usuário. O cadastro não atribui classe normativa nem qualificação de resistência.");
        if choice(
            ui,
            "Material para revisar",
            &mut self.selected,
            materials(project),
            true,
        ) {
            self.load(project);
        }
        crate::numeric::context(ui, format!("{}:material:{}", project.id, self.selected));
        text(ui, "Nome do material", &mut self.name);
        enum_choice(
            ui,
            "Família do material",
            &mut self.family,
            &[
                (MaterialFamily::Generic, "Genérico"),
                (MaterialFamily::Concrete, "Concreto"),
                (MaterialFamily::Steel, "Aço"),
                (MaterialFamily::Timber, "Madeira"),
            ],
        );
        number(ui, "Módulo de elasticidade", &mut self.elastic_modulus);
        enum_choice(
            ui,
            "Unidade do módulo",
            &mut self.pressure_unit,
            &[
                (PressureUnit::Pascal, "Pa"),
                (PressureUnit::Kilopascal, "kPa"),
                (PressureUnit::Megapascal, "MPa"),
                (PressureUnit::Gigapascal, "GPa"),
            ],
        );
        number(ui, "Massa específica (kg/m³)", &mut self.density);
        number(ui, "Coeficiente de Poisson", &mut self.poisson_ratio);
        ui.horizontal(|ui| {
            if ui.button("Cadastrar material").clicked() {
                emit(
                    actions,
                    project,
                    ModelEdit::AddMaterial {
                        material: self.input(),
                    },
                );
            }
            if ui
                .add_enabled(
                    !self.selected.is_empty(),
                    egui::Button::new("Revisar material selecionado"),
                )
                .clicked()
                && let Ok(material_id) = self.selected.parse()
            {
                emit(
                    actions,
                    project,
                    ModelEdit::SetMaterial {
                        material_id,
                        material: self.input(),
                    },
                );
            }
        });
        for material in &project.model.materials {
            ui.label(format!(
                "{} | E = {:.4e} Pa | ρ = {:.3} kg/m³ | ν = {:.4}",
                material.name,
                material.elastic_modulus.si(),
                material.density.si(),
                material.poisson_ratio
            ));
        }
    }
}

pub struct ActionForm {
    pub selected: String,
    pub name: String,
    pub node: String,
    pub force: [f64; 3],
    pub force_unit: ForceUnit,
    pub couple: [f64; 3],
    pub moment_unit: MomentUnit,
    pub local: bool,
    pub member: String,
}
impl Default for ActionForm {
    fn default() -> Self {
        Self {
            selected: String::new(),
            name: "Ação nodal 01".into(),
            node: String::new(),
            force: [0.0, 0.0, -10.0],
            force_unit: ForceUnit::Kilonewton,
            couple: [0.0; 3],
            moment_unit: MomentUnit::KilonewtonMetre,
            local: false,
            member: String::new(),
        }
    }
}
impl ActionForm {
    fn load(&mut self, project: &Project) {
        if let Some(action) = project
            .model
            .actions
            .iter()
            .find(|action| action.id.to_string() == self.selected)
        {
            self.name = action.name.clone();
            self.node = action.application_node_id.to_string();
            self.force = action.force.si();
            self.force_unit = ForceUnit::Newton;
            self.couple = action.couple.si();
            self.moment_unit = MomentUnit::NewtonMetre;
            match action.frame {
                ActionFrame::Engineering {} => {
                    self.local = false;
                    self.member.clear();
                }
                ActionFrame::MemberLocal { member_id } => {
                    self.local = true;
                    self.member = member_id.to_string();
                }
            }
        } else {
            self.selected.clear();
        }
    }

    fn show(&mut self, ui: &mut Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.strong("Força e momento nodal");
        let options = project
            .model
            .actions
            .iter()
            .map(|action| (action.id.to_string(), named(&action.name, action.id)))
            .collect();
        if choice(ui, "Ação para revisar", &mut self.selected, options, true) {
            self.load(project);
        }
        crate::numeric::context(ui, format!("{}:action:{}", project.id, self.selected));
        text(ui, "Nome da ação", &mut self.name);
        choice(ui, "Nó de aplicação", &mut self.node, nodes(project), false);
        vector(ui, "Força", &mut self.force);
        enum_choice(
            ui,
            "Unidade da força",
            &mut self.force_unit,
            &[(ForceUnit::Newton, "N"), (ForceUnit::Kilonewton, "kN")],
        );
        vector(ui, "Momento", &mut self.couple);
        enum_choice(
            ui,
            "Unidade do momento",
            &mut self.moment_unit,
            &[
                (MomentUnit::NewtonMetre, "N·m"),
                (MomentUnit::KilonewtonMetre, "kN·m"),
            ],
        );
        ui.checkbox(&mut self.local, "Usar eixos locais de uma barra");
        if self.local {
            choice(
                ui,
                "Barra dos eixos locais",
                &mut self.member,
                members(project),
                false,
            );
        } else {
            ui.label("Componentes expressas nos eixos da origem de engenharia.");
        }
        let ready = !self.node.is_empty() && (!self.local || !self.member.is_empty());
        ui.horizontal_wrapped(|ui| {
            let add = ui
                .add_enabled(ready, egui::Button::new("Adicionar ação nodal"))
                .clicked();
            let update = ui
                .add_enabled(
                    ready && !self.selected.is_empty(),
                    egui::Button::new("Revisar ação selecionada"),
                )
                .clicked();
            if (add || update)
                && let Ok(application_node_id) = self.node.parse()
            {
                let frame = if self.local {
                    let Ok(member_id) = self.member.parse() else {
                        return;
                    };
                    ActionFrame::MemberLocal { member_id }
                } else {
                    ActionFrame::Engineering {}
                };
                let action = NodalActionInput {
                    name: self.name.clone(),
                    application_node_id,
                    force: ForceInput {
                        values: self.force,
                        unit: self.force_unit,
                    },
                    couple: MomentInput {
                        values: self.couple,
                        unit: self.moment_unit,
                    },
                    frame,
                };
                if add {
                    emit(actions, project, ModelEdit::AddNodalAction { action });
                } else if let Ok(action_id) = self.selected.parse() {
                    emit(
                        actions,
                        project,
                        ModelEdit::SetNodalAction { action_id, action },
                    );
                }
            }
        });
        for action in &project.model.actions {
            let frame = match &action.frame {
                ActionFrame::Engineering {} => "Eixos de engenharia".to_owned(),
                ActionFrame::MemberLocal { member_id } => project
                    .model
                    .analytical_members
                    .iter()
                    .find(|member| member.id == *member_id)
                    .map(|member| format!("Eixos de {}", member.name))
                    .unwrap_or_else(|| format!("Barra {}", short(member_id))),
            };
            ui.label(format!(
                "{} | F = {:?} N | M = {:?} N·m | {}",
                action.name,
                action.force.si(),
                action.couple.si(),
                frame
            ));
        }
    }
}

fn emit(actions: &mut Vec<UiAction>, project: &Project, edit: ModelEdit) {
    actions.push(UiAction::command(Command::EditModel {
        expected_revision: project.revision,
        edit,
    }));
}
fn point(values: [f64; 3], unit: LengthUnit) -> PointInput {
    PointInput { values, unit }
}
fn short(id: impl std::fmt::Display) -> String {
    id.to_string().chars().take(8).collect()
}
fn named(name: &str, id: impl std::fmt::Display) -> String {
    format!("{name} ({})", short(id))
}
fn objects(project: &Project) -> Vec<(String, String)> {
    project
        .objects
        .iter()
        .map(|o| (o.id.to_string(), named(&o.name, o.id)))
        .collect()
}
fn nodes(project: &Project) -> Vec<(String, String)> {
    project
        .model
        .analytical_nodes
        .iter()
        .map(|o| (o.id.to_string(), named(&o.name, o.id)))
        .collect()
}
fn members(project: &Project) -> Vec<(String, String)> {
    project
        .model
        .analytical_members
        .iter()
        .map(|o| (o.id.to_string(), named(&o.name, o.id)))
        .collect()
}
fn materials(project: &Project) -> Vec<(String, String)> {
    project
        .model
        .materials
        .iter()
        .map(|o| (o.id.to_string(), named(&o.name, o.id)))
        .collect()
}
fn environments(project: &Project) -> Vec<(String, String)> {
    project
        .environments
        .iter()
        .map(|o| (o.id.to_string(), named(&o.name, o.id)))
        .collect()
}
fn role_label(role: PhysicalRole) -> &'static str {
    match role {
        PhysicalRole::VisualOnly => "Somente visual",
        PhysicalRole::AnalyticalSource => "Origem para análise",
    }
}

fn text(ui: &mut Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        let label = ui.label(label);
        ui.add(egui::TextEdit::singleline(value).desired_width(320.0))
            .labelled_by(label.id);
    });
}
fn number(ui: &mut Ui, label: &str, value: &mut f64) {
    ui.horizontal(|ui| {
        crate::numeric::field(ui, label, value);
    });
}
fn vector(ui: &mut Ui, label: &str, values: &mut [f64; 3]) {
    ui.columns(3, |columns| {
        for (index, (axis, value)) in ["X", "Y", "Z"].into_iter().zip(values).enumerate() {
            crate::numeric::field(&mut columns[index], &format!("{label} {axis}"), value);
        }
    });
}
fn choice(
    ui: &mut Ui,
    label: &str,
    selected: &mut String,
    options: Vec<(String, String)>,
    optional: bool,
) -> bool {
    if !selected.is_empty() && !options.iter().any(|(id, _)| id == selected) {
        selected.clear();
    }
    let current = options
        .iter()
        .find(|(id, _)| id == selected)
        .map(|(_, name)| name.clone())
        .unwrap_or_else(|| {
            if optional {
                "Nenhum".into()
            } else {
                "Selecione".into()
            }
        });
    let mut changed = false;
    ui.horizontal(|ui| {
        let caption = ui.label(label);
        let response = egui::ComboBox::from_id_salt(label)
            .selected_text(current)
            .width(330.0)
            .show_ui(ui, |ui| {
                if optional {
                    changed |= ui
                        .selectable_value(selected, String::new(), "Nenhum")
                        .changed();
                }
                for (id, name) in options {
                    changed |= ui.selectable_value(selected, id, name).changed();
                }
            });
        response.response.labelled_by(caption.id);
    });
    changed
}
fn enum_choice<T: Copy + PartialEq>(
    ui: &mut Ui,
    label: &str,
    selected: &mut T,
    options: &[(T, &str)],
) {
    let current = options
        .iter()
        .find(|(value, _)| value == selected)
        .map(|(_, name)| *name)
        .unwrap_or("Selecione");
    ui.horizontal(|ui| {
        let caption = ui.label(label);
        egui::ComboBox::from_id_salt(label)
            .selected_text(current)
            .show_ui(ui, |ui| {
                for (value, name) in options {
                    ui.selectable_value(selected, *value, *name);
                }
            })
            .response
            .labelled_by(caption.id);
    });
}
fn length_unit(ui: &mut Ui, label: &str, unit: &mut LengthUnit) {
    enum_choice(
        ui,
        label,
        unit,
        &[
            (LengthUnit::Metre, "m"),
            (LengthUnit::Centimetre, "cm"),
            (LengthUnit::Millimetre, "mm"),
        ],
    );
}
fn discipline(ui: &mut Ui, label: &str, selected: &mut Discipline) {
    enum_choice(
        ui,
        label,
        selected,
        &[
            (Discipline::Structure, "Estruturas"),
            (Discipline::WaterSanitation, "Hidrossanitário"),
            (Discipline::Gas, "Gás"),
            (Discipline::FireProtection, "Incêndio"),
            (Discipline::Electrical, "Elétrica"),
            (Discipline::Photovoltaic, "Fotovoltaico"),
            (Discipline::LightningProtection, "SPDA"),
            (Discipline::Cabling, "Cabeamento"),
            (Discipline::Hvac, "Climatização"),
        ],
    );
}

fn discipline_label(discipline: Discipline) -> &'static str {
    match discipline {
        Discipline::Structure => "Estruturas",
        Discipline::WaterSanitation => "Hidrossanitário",
        Discipline::Gas => "Gás",
        Discipline::FireProtection => "Incêndio",
        Discipline::Electrical => "Elétrica",
        Discipline::Photovoltaic => "Fotovoltaico",
        Discipline::LightningProtection => "SPDA",
        Discipline::Cabling => "Cabeamento",
        Discipline::Hvac => "Climatização",
    }
}

fn port_kind_label(kind: PortKind) -> &'static str {
    match kind {
        PortKind::Structural => "Estrutural",
        PortKind::Hydraulic => "Hidráulica",
        PortKind::Gas => "Gás",
        PortKind::Electrical => "Elétrica",
        PortKind::Signal => "Sinal",
        PortKind::Thermal => "Térmica",
    }
}
