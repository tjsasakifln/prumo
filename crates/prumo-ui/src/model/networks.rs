use super::*;

pub struct NetworkForm {
    pub name: String,
    pub discipline: Discipline,
    pub kind: NetworkKind,
    pub environment: String,
    pub selected: String,
    pub object: String,
    pub port_discipline: Discipline,
    pub port_kind: PortKind,
    pub position: [f64; 3],
    pub unit: LengthUnit,
    pub port: String,
    pub second_port: String,
    pub connection: String,
}

impl Default for NetworkForm {
    fn default() -> Self {
        Self {
            name: "Rede 01".into(),
            discipline: Discipline::WaterSanitation,
            kind: NetworkKind::Network,
            environment: String::new(),
            selected: String::new(),
            object: String::new(),
            port_discipline: Discipline::WaterSanitation,
            port_kind: PortKind::Hydraulic,
            position: [0.0; 3],
            unit: LengthUnit::Metre,
            port: String::new(),
            second_port: String::new(),
            connection: String::new(),
        }
    }
}

impl NetworkForm {
    pub(super) fn show(&mut self, ui: &mut Ui, project: &Project, actions: &mut Vec<UiAction>) {
        ui.strong("Redes, circuitos e portas");
        ui.label("As portas pertencem a objetos e são atribuídas a uma rede. A conexão só existe após a ação Conectar portas.");
        text(ui, "Nome da rede ou circuito", &mut self.name);
        discipline(ui, "Disciplina da rede", &mut self.discipline);
        enum_choice(
            ui,
            "Tipo da rede",
            &mut self.kind,
            &[
                (NetworkKind::Network, "Rede"),
                (NetworkKind::Circuit, "Circuito"),
            ],
        );
        choice(
            ui,
            "Ambiente da rede",
            &mut self.environment,
            environments(project),
            true,
        );
        if ui.button("Criar rede ou circuito").clicked() {
            emit(
                actions,
                project,
                ModelEdit::AddNetwork {
                    name: self.name.clone(),
                    discipline: self.discipline,
                    kind: self.kind,
                    environment_id: self.environment.parse().ok(),
                },
            );
        }
        ui.separator();
        if choice(
            ui,
            "Objeto da nova porta",
            &mut self.object,
            objects(project),
            false,
        ) && let Some(object) = project
            .objects
            .iter()
            .find(|object| object.id.to_string() == self.object)
        {
            self.port_discipline = object.discipline;
            self.position = object.start_m.as_array();
            self.unit = LengthUnit::Metre;
            self.port_kind = match object.discipline {
                Discipline::Structure => PortKind::Structural,
                Discipline::WaterSanitation | Discipline::FireProtection => PortKind::Hydraulic,
                Discipline::Gas => PortKind::Gas,
                Discipline::Electrical
                | Discipline::Photovoltaic
                | Discipline::LightningProtection => PortKind::Electrical,
                Discipline::Cabling => PortKind::Signal,
                Discipline::Hvac => PortKind::Thermal,
            };
        }
        crate::numeric::context(ui, format!("{}:port:{}", project.id, self.object));
        discipline(ui, "Disciplina da porta", &mut self.port_discipline);
        enum_choice(
            ui,
            "Tipo da porta",
            &mut self.port_kind,
            &[
                (PortKind::Structural, "Estrutural"),
                (PortKind::Hydraulic, "Hidráulica"),
                (PortKind::Gas, "Gás"),
                (PortKind::Electrical, "Elétrica"),
                (PortKind::Signal, "Sinal"),
                (PortKind::Thermal, "Térmica"),
            ],
        );
        length_unit(ui, "Unidade da porta", &mut self.unit);
        vector(ui, "Posição da porta", &mut self.position);
        if ui
            .add_enabled(
                !self.object.is_empty(),
                egui::Button::new("Criar porta no objeto"),
            )
            .clicked()
            && let Ok(object_id) = self.object.parse()
        {
            emit(
                actions,
                project,
                ModelEdit::AddPort {
                    object_id,
                    discipline: self.port_discipline,
                    kind: self.port_kind,
                    position: point(self.position, self.unit),
                },
            );
        }
        ui.separator();
        let networks = project
            .networks
            .iter()
            .map(|network| (network.id.to_string(), named(&network.name, network.id)))
            .collect();
        choice(
            ui,
            "Rede para conectar",
            &mut self.selected,
            networks,
            false,
        );
        choice(ui, "Porta de origem", &mut self.port, ports(project), false);
        if ui
            .add_enabled(
                !self.selected.is_empty() && !self.port.is_empty(),
                egui::Button::new("Atribuir porta à rede"),
            )
            .clicked()
            && let (Ok(network_id), Ok(port_id)) = (self.selected.parse(), self.port.parse())
        {
            emit(
                actions,
                project,
                ModelEdit::AssignPort {
                    network_id,
                    port_id,
                },
            );
        }
        choice(
            ui,
            "Porta de destino",
            &mut self.second_port,
            ports(project),
            false,
        );
        if ui
            .add_enabled(
                !self.selected.is_empty() && !self.port.is_empty() && !self.second_port.is_empty(),
                egui::Button::new("Conectar portas explicitamente"),
            )
            .clicked()
            && let (Ok(network_id), Ok(from_port), Ok(to_port)) = (
                self.selected.parse(),
                self.port.parse(),
                self.second_port.parse(),
            )
        {
            emit(
                actions,
                project,
                ModelEdit::ConnectPorts {
                    network_id,
                    from_port,
                    to_port,
                },
            );
        }
        let connections = project
            .connections
            .iter()
            .map(|connection| {
                let network = project
                    .networks
                    .iter()
                    .find(|network| network.id == connection.network_id)
                    .map(|network| network.name.as_str())
                    .unwrap_or("Ausente");
                (
                    connection.id.to_string(),
                    format!(
                        "{network}: {} com {} ({})",
                        short(connection.from_port),
                        short(connection.to_port),
                        short(connection.id)
                    ),
                )
            })
            .collect();
        choice(
            ui,
            "Conexão para desfazer",
            &mut self.connection,
            connections,
            false,
        );
        if ui
            .add_enabled(
                !self.connection.is_empty(),
                egui::Button::new("Desconectar portas"),
            )
            .clicked()
            && let Ok(connection_id) = self.connection.parse()
        {
            emit(
                actions,
                project,
                ModelEdit::DisconnectPorts { connection_id },
            );
        }
        ui.separator();
        for network in &project.networks {
            let connections = project
                .connections
                .iter()
                .filter(|connection| connection.network_id == network.id)
                .count();
            ui.label(format!(
                "{} | {} | {} portas | {connections} conexões explícitas",
                network.name,
                discipline_label(network.discipline),
                network.port_ids.len()
            ));
        }
        ui.label(format!(
            "Total: {} portas e {} conexões. Cruzamentos visuais não conectam portas.",
            project.ports.len(),
            project.connections.len()
        ));
    }
}

fn ports(project: &Project) -> Vec<(String, String)> {
    project
        .ports
        .iter()
        .map(|port| {
            let owner = project
                .objects
                .iter()
                .find(|object| object.id == port.object_id)
                .map(|object| object.name.as_str())
                .unwrap_or("Ausente");
            (
                port.id.to_string(),
                format!(
                    "{owner} | {} ({})",
                    port_kind_label(port.kind),
                    short(port.id)
                ),
            )
        })
        .collect()
}
