#![forbid(unsafe_code)]

//! Native project contracts. Geometry is explicitly modelled, never a calculated result.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use uuid::Uuid;

pub mod assemblies;
pub mod canonical;
pub mod catalogue;
pub mod components;
pub mod dependencies;
pub mod framing;
pub mod geometry;
pub mod identity;
pub mod layout;
pub mod model;
mod numeric;
pub mod precalculation;
pub mod registry;
pub mod storage;
pub mod surfaces;
pub mod transactions;
pub mod units;
pub mod view;

pub const PROJECT_SCHEMA_VERSION: u32 = 2;
pub const MAX_NAME_CHARACTERS: usize = 160;
pub const MAX_OBJECTS: usize = 10_000;
pub const MIN_MEMBER_LENGTH_M: f64 = 1.0e-9;
pub const MAX_COORDINATE_M: f64 = 1.0e9;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Create {
        name: String,
        units: LengthUnit,
    },
    AddMember {
        expected_revision: u64,
        name: String,
        #[serde(deserialize_with = "numeric::vector3")]
        start: [f64; 3],
        #[serde(deserialize_with = "numeric::vector3")]
        end: [f64; 3],
    },
    RenameProject {
        expected_revision: u64,
        name: String,
    },
    AddEnvironment {
        expected_revision: u64,
        name: String,
    },
    RegisterNormReference {
        expected_revision: u64,
        reference: registry::NormReferenceInput,
    },
    PinNormativePackage {
        expected_revision: u64,
        package_hash: String,
    },
    MigrateNormativePackage {
        expected_revision: u64,
        from_package_hash: String,
        to_package_hash: String,
        confirm: bool,
    },
    EditModel {
        expected_revision: u64,
        edit: model::ModelEdit,
    },
    InspectModel {
        expected_revision: u64,
    },
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LengthUnit {
    #[default]
    Metre,
    Centimetre,
    Millimetre,
}

impl LengthUnit {
    pub const fn metres_per_unit(self) -> f64 {
        match self {
            Self::Metre => 1.0,
            Self::Centimetre => 0.01,
            Self::Millimetre => 0.001,
        }
    }

    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Metre => "m",
            Self::Centimetre => "cm",
            Self::Millimetre => "mm",
        }
    }

    pub fn to_metres(self, point: [f64; 3]) -> Result<[f64; 3], DomainError> {
        Ok(units::PointInput {
            values: point,
            unit: self,
        }
        .to_si()?
        .as_array())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Discipline {
    Structure,
    WaterSanitation,
    Gas,
    FireProtection,
    Electrical,
    Photovoltaic,
    LightningProtection,
    Cabling,
    Hvac,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelState {
    Modelled,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalObject {
    pub id: Uuid,
    pub name: String,
    pub discipline: Discipline,
    pub start_m: units::Point3,
    pub end_m: units::Point3,
    pub state: ModelState,
    pub role: model::PhysicalRole,
    pub material_id: Option<Uuid>,
    pub environment_id: Option<Uuid>,
    #[serde(default)]
    pub extensions: BTreeMap<String, Value>,
}

impl PhysicalObject {
    pub fn length_m(&self) -> f64 {
        if let Ok(Some(length)) = framing::object_length_m(self) {
            return length;
        }
        self.start_m.distance(self.end_m)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortKind {
    Structural,
    Hydraulic,
    Gas,
    Electrical,
    Signal,
    Thermal,
}

impl PortKind {
    pub const fn accepts(self, discipline: Discipline) -> bool {
        matches!(
            (self, discipline),
            (Self::Structural, Discipline::Structure)
                | (
                    Self::Hydraulic,
                    Discipline::WaterSanitation | Discipline::FireProtection
                )
                | (Self::Gas, Discipline::Gas)
                | (
                    Self::Electrical,
                    Discipline::Electrical
                        | Discipline::Photovoltaic
                        | Discipline::LightningProtection
                )
                | (Self::Signal, Discipline::Cabling)
                | (Self::Thermal, Discipline::Hvac)
        )
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub id: Uuid,
    pub object_id: Uuid,
    pub discipline: Discipline,
    pub kind: PortKind,
    pub position_m: units::Point3,
    #[serde(default)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub id: Uuid,
    pub name: String,
    pub discipline: Discipline,
    pub kind: model::NetworkKind,
    pub environment_id: Option<Uuid>,
    pub port_ids: Vec<Uuid>,
    #[serde(default)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    pub id: Uuid,
    pub network_id: Uuid,
    pub from_port: Uuid,
    pub to_port: Uuid,
    #[serde(default)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeRecord {
    pub revision: u64,
    pub actor: String,
    pub operation: String,
    pub timestamp_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    pub id: Uuid,
    pub name: String,
    pub revision: u64,
    pub units: LengthUnit,
    pub objects: Vec<PhysicalObject>,
    pub environments: Vec<Environment>,
    pub networks: Vec<Network>,
    pub ports: Vec<Port>,
    pub connections: Vec<Connection>,
    pub history: Vec<ChangeRecord>,
    #[serde(default)]
    pub normative: registry::NormativeState,
    pub model: model::SemanticModel,
    #[serde(default)]
    pub dependencies: dependencies::DependencyGraph,
    #[serde(default)]
    pub extensions: BTreeMap<String, Value>,
}

impl Project {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.schema_version != PROJECT_SCHEMA_VERSION {
            return Err(DomainError::UnsupportedSchema(self.schema_version));
        }
        validate_name(&self.name)?;
        self.normative
            .validate(self.revision)
            .map_err(|error| DomainError::InvalidNormative(error.to_string()))?;
        if self.revision == 0 || self.history.len() as u64 != self.revision {
            return Err(DomainError::InvalidHistory);
        }
        for (index, change) in self.history.iter().enumerate() {
            if change.revision != index as u64 + 1 {
                return Err(DomainError::InvalidHistory);
            }
            validate_name(&change.actor)?;
            validate_name(&change.operation)?;
        }
        self.validate_model_contents(self.revision)?;
        self.dependencies.validate(
            self.revision,
            &crate::canonical::source_index_unchecked(self)?,
        )?;
        Ok(())
    }

    /// Model edits validate their next-revision metadata before the service stamps history.
    pub fn validate_model_contents(&self, metadata_revision: u64) -> Result<(), DomainError> {
        geometry::project_tolerances(self)?.validate()?;
        let mut all_ids = BTreeSet::new();
        insert_id(&mut all_ids, self.id)?;
        if self.objects.len() > MAX_OBJECTS
            || self.ports.len() > MAX_OBJECTS * 16
            || self.networks.len() > MAX_OBJECTS
            || self.environments.len() > MAX_OBJECTS
            || self.connections.len() > MAX_OBJECTS * 16
        {
            return Err(DomainError::ModelLimit);
        }
        for object in &self.objects {
            insert_id(&mut all_ids, object.id)?;
            validate_name(&object.name)?;
            validate_point(object.start_m.as_array())?;
            validate_point(object.end_m.as_array())?;
            if object.length_m() <= MIN_MEMBER_LENGTH_M {
                return Err(DomainError::DegenerateMember);
            }
            if let Some(id) = object.material_id
                && !self.model.materials.iter().any(|x| x.id == id)
            {
                return Err(DomainError::UnknownReference(id));
            }
            if let Some(id) = object.environment_id
                && !self.environments.iter().any(|x| x.id == id)
            {
                return Err(DomainError::UnknownReference(id));
            }
        }
        for environment in &self.environments {
            insert_id(&mut all_ids, environment.id)?;
            validate_name(&environment.name)?;
        }
        for port in &self.ports {
            insert_id(&mut all_ids, port.id)?;
            validate_point(port.position_m.as_array())?;
            let object = self
                .objects
                .iter()
                .find(|object| object.id == port.object_id)
                .ok_or(DomainError::UnknownReference(port.object_id))?;
            if object.discipline != port.discipline || !port.kind.accepts(port.discipline) {
                return Err(DomainError::IncompatiblePort);
            }
        }
        let mut assigned_ports = BTreeSet::new();
        for network in &self.networks {
            insert_id(&mut all_ids, network.id)?;
            validate_name(&network.name)?;
            if network.kind == model::NetworkKind::Circuit
                && !matches!(
                    network.discipline,
                    Discipline::Electrical
                        | Discipline::Photovoltaic
                        | Discipline::LightningProtection
                        | Discipline::Cabling
                )
            {
                return Err(DomainError::IncompatiblePort);
            }
            if let Some(environment_id) = network.environment_id
                && !self
                    .environments
                    .iter()
                    .any(|item| item.id == environment_id)
            {
                return Err(DomainError::UnknownReference(environment_id));
            }
            for port_id in &network.port_ids {
                let port = self
                    .ports
                    .iter()
                    .find(|port| port.id == *port_id)
                    .ok_or(DomainError::UnknownReference(*port_id))?;
                if port.discipline != network.discipline || !assigned_ports.insert(*port_id) {
                    return Err(DomainError::IncompatiblePort);
                }
            }
        }
        let mut edges = BTreeSet::new();
        for connection in &self.connections {
            insert_id(&mut all_ids, connection.id)?;
            let network = self
                .networks
                .iter()
                .find(|network| network.id == connection.network_id)
                .ok_or(DomainError::UnknownReference(connection.network_id))?;
            let endpoints = [connection.from_port, connection.to_port];
            if connection.from_port == connection.to_port
                || endpoints.iter().any(|id| !network.port_ids.contains(id))
            {
                return Err(DomainError::InvalidConnection);
            }
            let edge = (
                connection.network_id,
                connection.from_port.min(connection.to_port),
                connection.from_port.max(connection.to_port),
            );
            if !edges.insert(edge) {
                return Err(DomainError::InvalidConnection);
            }
        }
        layout::register_ids(self, &mut all_ids)?;
        surfaces::register_ids(self, &mut all_ids)?;
        assemblies::register_ids(self, &mut all_ids)?;
        model::validate_semantics(self, metadata_revision, &mut all_ids)?;
        catalogue::validate_catalogue(self, metadata_revision, &mut all_ids)?;
        layout::validate_project(self)?;
        framing::validate_project(self)?;
        surfaces::validate_project(self)?;
        assemblies::validate_project(self)?;
        precalculation::validate_project(self)
    }
}

pub fn validate_name(value: &str) -> Result<(), DomainError> {
    if value.trim().is_empty()
        || value.chars().count() > MAX_NAME_CHARACTERS
        || value.chars().any(char::is_control)
    {
        Err(DomainError::InvalidName)
    } else {
        Ok(())
    }
}

pub fn validate_point(point: [f64; 3]) -> Result<(), DomainError> {
    if point
        .into_iter()
        .any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE_M)
    {
        Err(DomainError::InvalidCoordinate)
    } else {
        Ok(())
    }
}

fn insert_id(ids: &mut BTreeSet<Uuid>, id: Uuid) -> Result<(), DomainError> {
    if id.is_nil() || !ids.insert(id) {
        Err(DomainError::DuplicateIdentity(id))
    } else {
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Grandeza não finita, não positiva quando exigido, ou fora do domínio dimensional.")]
    InvalidQuantity,
    #[error("GUID IFC inválido; use a codificação de 22 caracteres especificada pelo IFC.")]
    InvalidIfcGuid,
    #[error("Modelo semântico inválido: {0}")]
    InvalidModel(String),
    #[error("Referencial ou transformação rígida inválida.")]
    InvalidFrame,
    #[error("A seleção não contém todas as peças e membros que compartilham os nós afetados.")]
    SharedTopology,
    #[error("Dependência inválida: {0}")]
    InvalidDependency(String),
    #[error("Snapshot inválido: {0}")]
    InvalidSnapshot(String),
    #[error("Registro normativo inválido: {0}")]
    InvalidNormative(String),
    #[error("O nome deve conter de 1 a 160 caracteres, sem caracteres de controle.")]
    InvalidName,
    #[error("Informe coordenadas finitas entre -1e9 e 1e9 metros.")]
    InvalidCoordinate,
    #[error("O início e o fim da peça devem definir comprimento maior que 1e-9 metro.")]
    DegenerateMember,
    #[error("Identidade nula ou repetida: {0}.")]
    DuplicateIdentity(Uuid),
    #[error("Referência inexistente no projeto: {0}.")]
    UnknownReference(Uuid),
    #[error("A porta não é compatível com a disciplina ou pertence a mais de uma rede.")]
    IncompatiblePort,
    #[error("Conexão inválida ou repetida; informe duas portas distintas da mesma rede.")]
    InvalidConnection,
    #[error("O histórico de revisões não corresponde à revisão do projeto.")]
    InvalidHistory,
    #[error("O modelo excede os limites desta versão de desenvolvimento.")]
    ModelLimit,
    #[error("A versão {0} do formato de projeto não é suportada.")]
    UnsupportedSchema(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_units_convert_to_the_same_physical_coordinates() {
        assert_eq!(
            LengthUnit::Metre.to_metres([2.5, 0.0, -1.0]).unwrap(),
            [2.5, 0.0, -1.0]
        );
        assert_eq!(
            LengthUnit::Centimetre
                .to_metres([250.0, 0.0, -100.0])
                .unwrap(),
            [2.5, 0.0, -1.0]
        );
        assert_eq!(
            LengthUnit::Millimetre
                .to_metres([2500.0, 0.0, -1000.0])
                .unwrap(),
            [2.5, 0.0, -1.0]
        );
    }

    #[test]
    fn invalid_values_are_rejected_before_serialization() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0e20] {
            assert!(LengthUnit::Metre.to_metres([value, 0.0, 0.0]).is_err());
        }
        for name in ["", "   ", "Viga\n1", "\0"] {
            assert!(validate_name(name).is_err());
        }
        assert!(validate_name(&"é".repeat(160)).is_ok());
        assert!(validate_name(&"é".repeat(161)).is_err());
    }

    #[test]
    fn all_installation_disciplines_have_a_typed_contract() {
        let pairs = [
            (Discipline::Structure, PortKind::Structural),
            (Discipline::WaterSanitation, PortKind::Hydraulic),
            (Discipline::Gas, PortKind::Gas),
            (Discipline::FireProtection, PortKind::Hydraulic),
            (Discipline::Electrical, PortKind::Electrical),
            (Discipline::Photovoltaic, PortKind::Electrical),
            (Discipline::LightningProtection, PortKind::Electrical),
            (Discipline::Cabling, PortKind::Signal),
            (Discipline::Hvac, PortKind::Thermal),
        ];
        for (discipline, kind) in pairs {
            assert!(kind.accepts(discipline));
        }
        assert!(!PortKind::Hydraulic.accepts(Discipline::Gas));
        assert!(!PortKind::Signal.accepts(Discipline::Electrical));
    }
}
