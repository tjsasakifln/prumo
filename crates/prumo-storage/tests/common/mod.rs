#![allow(dead_code)]

use prumo_domain::{ChangeRecord, LengthUnit, PROJECT_SCHEMA_VERSION, Project};
use prumo_storage::{CommitRecord, PackageOrigin};
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

pub fn project() -> Project {
    Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name: "Fixture de persistência, sem cálculo".into(),
        revision: 1,
        units: LengthUnit::Millimetre,
        objects: vec![],
        environments: vec![],
        networks: vec![],
        ports: vec![],
        connections: vec![],
        history: vec![ChangeRecord {
            revision: 1,
            actor: "fixture".into(),
            operation: "create".into(),
            timestamp_unix_ms: 1000,
        }],
        normative: Default::default(),
        model: Default::default(),
        dependencies: Default::default(),
        extensions: BTreeMap::from([(
            "future.mep".into(),
            json!({"version":17,"air_flow":{"value":1.9109059804556539,"unit":"m3/s"},"unrecognized":[null,false,{"label":"conservar"}]}),
        )]),
    }
}

pub fn rename(project: &Project, name: &str) -> Project {
    let mut result = project.clone();
    result.name = name.into();
    result.revision += 1;
    result.history.push(ChangeRecord {
        revision: result.revision,
        actor: "fixture".into(),
        operation: "rename_project".into(),
        timestamp_unix_ms: 1000 + result.revision,
    });
    result.validate().unwrap();
    result
}

pub fn record(project: &Project) -> CommitRecord {
    let change = project.history.last().unwrap();
    CommitRecord {
        command_id: Uuid::new_v4(),
        actor: change.actor.clone(),
        operation: change.operation.clone(),
        reason: Some("Ensaio automatizado de armazenamento, sem resultado de engenharia.".into()),
        timestamp_unix_ms: change.timestamp_unix_ms,
        command: Some(json!({"operation":change.operation,"name":project.name})),
    }
}

pub fn origin() -> PackageOrigin {
    PackageOrigin {
        application: "Prumo".into(),
        version: "0.1.0-test-fixture".into(),
        platform: std::env::consts::OS.into(),
    }
}
