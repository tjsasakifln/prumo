//! Read-only conversion of the complete v1 wire contract; identities never disappear.
use super::*;
use crate::identity::IfcGuid;
use crate::{
    ChangeRecord, Connection, Environment, ModelState, Network, PROJECT_SCHEMA_VERSION, Port,
};
use nalgebra::Vector3;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPhysical {
    id: Uuid,
    analytical_id: Uuid,
    name: String,
    discipline: Discipline,
    start_m: Point3,
    end_m: Point3,
    state: ModelState,
    #[serde(default)]
    extensions: BTreeMap<String, Value>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyNetwork {
    id: Uuid,
    name: String,
    discipline: Discipline,
    environment_id: Option<Uuid>,
    port_ids: Vec<Uuid>,
    #[serde(default)]
    extensions: BTreeMap<String, Value>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyProject {
    schema_version: u32,
    id: Uuid,
    name: String,
    revision: u64,
    units: LengthUnit,
    objects: Vec<LegacyPhysical>,
    environments: Vec<Environment>,
    networks: Vec<LegacyNetwork>,
    ports: Vec<Port>,
    connections: Vec<Connection>,
    history: Vec<ChangeRecord>,
    #[serde(default)]
    normative: crate::registry::NormativeState,
    #[serde(default)]
    extensions: BTreeMap<String, Value>,
}

fn derived_id(source: Uuid, label: &str) -> Uuid {
    let mut digest = Sha256::new();
    digest.update(b"prumo-native-v1-migration\0");
    digest.update(source.as_bytes());
    digest.update(label.as_bytes());
    let hash = digest.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub fn decode_project(value: Value) -> Result<Project, DomainError> {
    let schema = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            DomainError::InvalidModel("O formato deve informar uma versão inteira.".into())
        })?;
    let project = match schema {
        1 => convert(
            serde_json::from_value(value)
                .map_err(|error| DomainError::InvalidModel(error.to_string()))?,
        )?,
        2 => serde_json::from_value(value)
            .map_err(|error| DomainError::InvalidModel(error.to_string()))?,
        other => {
            return Err(DomainError::UnsupportedSchema(
                other.try_into().unwrap_or(u32::MAX),
            ));
        }
    };
    project.validate()?;
    Ok(project)
}

fn convert(legacy: LegacyProject) -> Result<Project, DomainError> {
    if legacy.schema_version != 1 {
        return Err(DomainError::UnsupportedSchema(legacy.schema_version));
    }
    let mut project = Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: legacy.id,
        name: legacy.name,
        revision: legacy.revision,
        units: legacy.units,
        objects: vec![],
        environments: legacy.environments,
        networks: legacy
            .networks
            .into_iter()
            .map(|x| Network {
                id: x.id,
                name: x.name,
                discipline: x.discipline,
                kind: NetworkKind::Network,
                environment_id: x.environment_id,
                port_ids: x.port_ids,
                extensions: x.extensions,
            })
            .collect(),
        ports: legacy.ports,
        connections: legacy.connections,
        history: legacy.history,
        normative: legacy.normative,
        model: SemanticModel::default(),
        dependencies: crate::dependencies::DependencyGraph::default(),
        extensions: legacy.extensions,
    };
    let mut derived_origins = BTreeMap::new();
    for old in legacy.objects {
        let start_id = derived_id(old.analytical_id, "start");
        let end_id = derived_id(old.analytical_id, "end");
        let link_id = derived_id(old.id, "link");
        derived_origins.insert(
            start_id,
            SourceOrigin::Derived {
                operation: format!("migração v1: nó inicial do analítico {}", old.analytical_id),
            },
        );
        derived_origins.insert(
            end_id,
            SourceOrigin::Derived {
                operation: format!("migração v1: nó final do analítico {}", old.analytical_id),
            },
        );
        derived_origins.insert(
            link_id,
            SourceOrigin::Derived {
                operation: format!("migração v1: vínculo do físico {}", old.id),
            },
        );
        let axis = Vector3::from(old.end_m.as_array()) - Vector3::from(old.start_m.as_array());
        if axis.norm() <= crate::MIN_MEMBER_LENGTH_M {
            return Err(DomainError::DegenerateMember);
        }
        let x = axis.normalize();
        let basis = if x.y.abs() < 0.9 {
            Vector3::y()
        } else {
            Vector3::z()
        };
        let y = (basis - x * x.dot(&basis)).normalize();
        project.model.analytical_nodes.extend([
            AnalyticalNode {
                id: start_id,
                name: "Nó inicial legado".into(),
                position_m: old.start_m,
            },
            AnalyticalNode {
                id: end_id,
                name: "Nó final legado".into(),
                position_m: old.end_m,
            },
        ]);
        project.model.analytical_members.push(AnalyticalMember {
            id: old.analytical_id,
            name: old.name.clone(),
            start_node_id: start_id,
            end_node_id: end_id,
            local_y: [y.x, y.y, y.z],
        });
        project.model.links.push(PhysicalAnalyticalLink {
            id: link_id,
            physical_id: old.id,
            analytical_id: old.analytical_id,
        });
        project.objects.push(PhysicalObject {
            id: old.id,
            name: old.name,
            discipline: old.discipline,
            start_m: old.start_m,
            end_m: old.end_m,
            state: old.state,
            role: PhysicalRole::AnalyticalSource,
            material_id: None,
            environment_id: None,
            extensions: old.extensions,
        });
    }
    let mut entities = Vec::new();
    entities.extend(project.objects.iter().map(|x| (x.id, true)));
    entities.extend(project.environments.iter().map(|x| (x.id, true)));
    entities.extend(project.networks.iter().map(|x| (x.id, true)));
    entities.extend(project.ports.iter().map(|x| (x.id, true)));
    entities.extend(project.connections.iter().map(|x| (x.id, false)));
    entities.extend(project.model.analytical_nodes.iter().map(|x| (x.id, true)));
    entities.extend(
        project
            .model
            .analytical_members
            .iter()
            .map(|x| (x.id, true)),
    );
    entities.extend(project.model.links.iter().map(|x| (x.id, false)));
    for (id, ifc) in entities {
        let identity = EntityIdentity {
            ifc_guid: ifc.then(|| IfcGuid::from_uuid(id)),
            origin: derived_origins
                .get(&id)
                .cloned()
                .unwrap_or_else(|| SourceOrigin::Imported {
                    system: "Prumo v1".into(),
                    reference: project.id.to_string(),
                    external_id: id.to_string(),
                }),
            created_revision: None,
            created_by: None,
            retired_revision: None,
        };
        if project.model.identities.insert(id, identity).is_some() {
            return Err(DomainError::DuplicateIdentity(id));
        }
    }
    Ok(project)
}
