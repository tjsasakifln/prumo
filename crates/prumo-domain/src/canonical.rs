//! Canonical model inputs. Collection order is normalized; topology order is retained.
use crate::dependencies::{SourceDigest, SourceKey, SourceKind};
use crate::{DomainError, Project};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSnapshot {
    pub version: u32,
    pub project_id: Uuid,
    pub source_revision: u64,
    pub sha256: String,
    pub canonical_json: String,
    pub sources: Vec<SourceDigest>,
}

fn serialization(error: serde_json::Error) -> DomainError {
    DomainError::InvalidSnapshot(error.to_string())
}

/// Canonical form version 1: UTF-8 JSON, lexicographic object keys and normalized zero.
/// It does not claim RFC 8785 equivalence or round physical quantities to a tolerance.
pub(crate) fn canonical_json(value: &impl Serialize) -> Result<String, DomainError> {
    let value = serde_json::to_value(value).map_err(serialization)?;
    let mut output = String::new();
    write_value(&value, &mut output)?;
    Ok(output)
}

fn write_value(value: &Value, output: &mut String) -> Result<(), DomainError> {
    match value {
        Value::Object(object) => {
            output.push('{');
            let ordered: BTreeMap<_, _> = object.iter().collect();
            for (index, (key, value)) in ordered.into_iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                output.push_str(&serde_json::to_string(key).map_err(serialization)?);
                output.push(':');
                write_value(value, output)?;
            }
            output.push('}');
        }
        Value::Array(array) => {
            output.push('[');
            for (index, value) in array.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                write_value(value, output)?;
            }
            output.push(']');
        }
        Value::Number(number) if number.as_f64() == Some(0.0) => output.push('0'),
        other => output.push_str(&serde_json::to_string(other).map_err(serialization)?),
    }
    Ok(())
}

pub(crate) fn digest(value: &impl Serialize) -> Result<String, DomainError> {
    Ok(hash_bytes(canonical_json(value)?.as_bytes()))
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sorted_entities(value: impl Serialize) -> Result<Value, DomainError> {
    let mut value = serde_json::to_value(value).map_err(serialization)?;
    if let Some(array) = value.as_array_mut() {
        array.sort_by(|a, b| {
            a.get("id")
                .and_then(Value::as_str)
                .cmp(&b.get("id").and_then(Value::as_str))
        });
    }
    Ok(value)
}

fn normalized_networks(project: &Project) -> Result<Value, DomainError> {
    let mut networks = sorted_entities(&project.networks)?;
    for network in networks.as_array_mut().into_iter().flatten() {
        if let Some(ports) = network.get_mut("port_ids").and_then(Value::as_array_mut) {
            ports.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        }
    }
    Ok(networks)
}

fn payload(project: &Project) -> Result<Value, DomainError> {
    let mut model = serde_json::to_value(&project.model).map_err(serialization)?;
    // Lists of entities are sets. Member endpoints, vectors and lineage stay ordered.
    for field in [
        "analytical_nodes",
        "analytical_members",
        "links",
        "materials",
        "actions",
    ] {
        if let Some(values) = model.get_mut(field).and_then(Value::as_array_mut) {
            values.sort_by(|a, b| {
                a.get("id")
                    .and_then(Value::as_str)
                    .cmp(&b.get("id").and_then(Value::as_str))
            });
        }
    }
    Ok(json!({
        "snapshot_version": 1, "project_id": project.id, "project_name": project.name,
        "display_unit": project.units,
        "unit_system": "SI", "objects": sorted_entities(&project.objects)?,
        "environments": sorted_entities(&project.environments)?,
        "ports": sorted_entities(&project.ports)?, "networks": normalized_networks(project)?,
        "connections": sorted_entities(&project.connections)?, "model": model,
        "normative_package": project.normative.active,
        "extensions": project.extensions
    }))
}

pub fn model_snapshot(project: &Project) -> Result<ModelSnapshot, DomainError> {
    project.validate()?;
    let sources: Vec<_> = source_index_unchecked(project)?
        .into_iter()
        .map(|(source, sha256)| SourceDigest { source, sha256 })
        .collect();
    let mut content = payload(project)?;
    content["source_digests"] = serde_json::to_value(&sources).map_err(serialization)?;
    let canonical_json = canonical_json(&content)?;
    Ok(ModelSnapshot {
        version: 1,
        project_id: project.id,
        source_revision: project.revision,
        sha256: hash_bytes(canonical_json.as_bytes()),
        canonical_json,
        sources,
    })
}

fn add<T: Serialize>(
    index: &mut BTreeMap<SourceKey, String>,
    kind: SourceKind,
    id: Uuid,
    value: &T,
) -> Result<(), DomainError> {
    index.insert(SourceKey { kind, id }, digest(value)?);
    Ok(())
}

/// Called by Project::validate; deliberately does not recursively validate Project.
pub fn source_index_unchecked(
    project: &Project,
) -> Result<BTreeMap<SourceKey, String>, DomainError> {
    let mut index = BTreeMap::new();
    add(
        &mut index,
        SourceKind::ProjectMetadata,
        project.id,
        &json!({"name":project.name,"display_unit":project.units,"extensions":project.extensions}),
    )?;
    add(
        &mut index,
        SourceKind::Coordinates,
        project.id,
        &project.model.engineering_frame,
    )?;
    add(
        &mut index,
        SourceKind::Georeference,
        project.id,
        &project.model.georeference,
    )?;
    add(
        &mut index,
        SourceKind::Normative,
        project.id,
        &project.normative.active,
    )?;

    let mut membership: BTreeMap<&str, Vec<Uuid>> = BTreeMap::new();
    macro_rules! entities {
        ($kind:ident, $name:literal, $items:expr) => {{
            let mut ids = Vec::new();
            for entity in $items {
                ids.push(entity.id);
                add(&mut index, SourceKind::$kind, entity.id,
                    &json!({"entity":entity,"identity":project.model.identities.get(&entity.id)}))?;
            }
            ids.sort_unstable();
            membership.insert($name, ids);
        }};
    }
    entities!(Physical, "physical", &project.objects);
    entities!(
        AnalyticalNode,
        "analytical_nodes",
        &project.model.analytical_nodes
    );
    entities!(
        AnalyticalMember,
        "analytical_members",
        &project.model.analytical_members
    );
    entities!(Link, "links", &project.model.links);
    entities!(Material, "materials", &project.model.materials);
    entities!(Action, "actions", &project.model.actions);
    entities!(Environment, "environments", &project.environments);
    entities!(Port, "ports", &project.ports);
    entities!(Connection, "connections", &project.connections);
    let mut network_ids = Vec::new();
    for network in &project.networks {
        network_ids.push(network.id);
        let mut normalized = network.clone();
        normalized.port_ids.sort_unstable();
        add(
            &mut index,
            SourceKind::Network,
            network.id,
            &json!({"entity":normalized,"identity":project.model.identities.get(&network.id)}),
        )?;
    }
    network_ids.sort_unstable();
    membership.insert("networks", network_ids);
    let roles: BTreeMap<_, _> = project
        .objects
        .iter()
        .map(|object| (object.id, object.role))
        .collect();
    add(
        &mut index,
        SourceKind::Membership,
        project.id,
        &json!({"ids":membership,"roles":roles}),
    )?;
    for object in &project.objects {
        add(
            &mut index,
            SourceKind::PhysicalGeometry,
            object.id,
            &json!({"start_m":object.start_m,"end_m":object.end_m}),
        )?;
    }
    Ok(index)
}
