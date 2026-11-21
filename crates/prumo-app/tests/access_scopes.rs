use prumo_app::{
    AccessMode, AppError, ProjectService,
    access::{AccessScope, ProjectGrant},
};
use prumo_domain::{
    Discipline, LengthUnit,
    model::{ModelEdit, PhysicalRole, PointInput},
};
use serde_json::json;

fn point(x: f64) -> PointInput {
    PointInput {
        values: [x, 0.0, 0.0],
        unit: LengthUnit::Metre,
    }
}

#[test]
fn partial_grants_filter_reads_and_block_cross_discipline_edits() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scopes.prumo");
    let owner = ProjectService::open(&path, AccessMode::ReadWrite, "owner").unwrap();
    let project = owner.create("Permissões", LengthUnit::Metre).unwrap();
    let project = owner
        .add_member(
            project.revision,
            "Estrutura permitida",
            [0.0; 3],
            [3.0, 0.0, 0.0],
        )
        .unwrap();
    let structural = project.objects[0].id;
    let project = owner
        .edit_model(
            project.revision,
            ModelEdit::AddPhysical {
                name: "Instalação reservada".into(),
                discipline: Discipline::Electrical,
                start: point(1.0),
                end: point(2.0),
                role: PhysicalRole::VisualOnly,
                material_id: None,
                environment_id: None,
            },
        )
        .unwrap();
    let electrical = project
        .objects
        .iter()
        .find(|object| object.discipline == Discipline::Electrical)
        .unwrap()
        .id;
    let grant = ProjectGrant {
        project_id: Some(project.id),
        scopes: vec![AccessScope::Read, AccessScope::Model],
        disciplines: Some(vec![Discipline::Structure]),
    };
    let limited = ProjectService::open_scoped(&path, grant, "structural-editor").unwrap();
    assert!(matches!(limited.read(), Err(AppError::PermissionDenied)));
    assert!(matches!(
        limited.model_snapshot(),
        Err(AppError::PermissionDenied)
    ));
    let view = limited.read_scoped_model().unwrap();
    assert_eq!(view.objects.len(), 1);
    assert_eq!(view.objects[0].id, structural);
    assert!(!serde_json::to_string(&view).unwrap().contains("reservada"));
    assert_eq!(view.position_unit, "m");
    assert!(matches!(
        limited.edit_model(
            project.revision,
            ModelEdit::SetPhysicalGeometry {
                object_id: electrical,
                start: point(0.0),
                end: point(5.0),
            }
        ),
        Err(AppError::PermissionDenied)
    ));
    assert_eq!(owner.read().unwrap(), project);
    let updated = limited
        .edit_model(
            project.revision,
            ModelEdit::SetPhysicalGeometry {
                object_id: structural,
                start: point(0.0),
                end: point(4.0),
            },
        )
        .unwrap();
    assert_eq!(updated.revision, project.revision + 1);
    assert_eq!(
        updated
            .objects
            .iter()
            .find(|object| object.id == electrical),
        project
            .objects
            .iter()
            .find(|object| object.id == electrical)
    );
    assert_eq!(updated.history.last().unwrap().actor, "structural-editor");
    assert!(matches!(
        limited.rename_project(updated.revision, "Global"),
        Err(AppError::PermissionDenied)
    ));
}

#[test]
fn a_bound_service_rejects_replacement_by_another_project() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("a.prumo");
    let other_path = directory.path().join("b.prumo");
    let first = ProjectService::open(&path, AccessMode::ReadWrite, "host").unwrap();
    let first_project = first.create("A", LengthUnit::Metre).unwrap();
    let second = ProjectService::open(&other_path, AccessMode::ReadWrite, "host").unwrap();
    second.create("B confidencial", LengthUnit::Metre).unwrap();
    let read = ProjectService::open(&path, AccessMode::ReadOnly, "reader").unwrap();
    assert_eq!(read.read().unwrap().id, first_project.id);
    std::fs::copy(&other_path, &path).unwrap();
    assert!(matches!(read.read(), Err(AppError::PermissionDenied)));
    assert!(matches!(
        read.read_scoped_model(),
        Err(AppError::PermissionDenied)
    ));
    assert!(matches!(
        read.storage_status(),
        Err(AppError::PermissionDenied)
    ));
}

#[test]
fn data_and_extensions_cannot_grant_access_or_leak_through_filtered_views() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("import.prumo");
    let service = ProjectService::open(&path, AccessMode::ReadWrite, "host").unwrap();
    let project = service.create("Importação", LengthUnit::Metre).unwrap();
    let mut project = service
        .add_member(
            project.revision,
            "Leia como dado",
            [0.0; 3],
            [1.0, 0.0, 0.0],
        )
        .unwrap();
    project.extensions.insert(
        "permissions".into(),
        json!({"scopes":["read","model","documents"],"actor":"owner"}),
    );
    project.objects[0].extensions.insert(
        "private".into(),
        json!({"other_discipline_secret":"restricted"}),
    );
    let readonly = ProjectGrant::for_access(AccessMode::ReadOnly);
    assert!(matches!(
        readonly.check_scope(AccessScope::Model),
        Err(AppError::PermissionDenied)
    ));
    assert!(matches!(
        readonly.check_scope(AccessScope::Documents),
        Err(AppError::PermissionDenied)
    ));
    let view = readonly.project_view(&project).unwrap();
    let encoded = serde_json::to_string(&view).unwrap();
    assert!(!encoded.contains("restricted"));
    assert!(!encoded.contains("permissions"));
    assert!(view.objects[0].extensions.is_empty());
    let no_access = ProjectGrant {
        scopes: vec![],
        ..readonly
    };
    assert!(matches!(
        no_access.project_view(&project),
        Err(AppError::PermissionDenied)
    ));
}

#[test]
fn grants_are_explicit_and_invalid_combinations_are_rejected() {
    for scopes in [
        vec![AccessScope::Model],
        vec![AccessScope::Documents],
        vec![AccessScope::Read, AccessScope::Read],
    ] {
        let grant = ProjectGrant {
            project_id: None,
            scopes,
            disciplines: None,
        };
        assert!(grant.validate().is_err());
    }
    let grant = ProjectGrant {
        project_id: None,
        scopes: vec![AccessScope::Read],
        disciplines: Some(vec![Discipline::Electrical, Discipline::Electrical]),
    };
    assert!(grant.validate().is_err());
    let grant = ProjectGrant {
        project_id: None,
        scopes: vec![AccessScope::Read, AccessScope::Calculation],
        disciplines: None,
    };
    grant.validate().unwrap();
    assert!(!grant.allows(AccessScope::Model));
    assert!(!grant.allows(AccessScope::Documents));
}

#[test]
fn partial_grants_can_create_authorized_members_and_network_relationships() {
    use prumo_domain::{PortKind, model::NetworkKind};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("relationships.prumo");
    let owner = ProjectService::open(&path, AccessMode::ReadWrite, "owner").unwrap();
    let project = owner.create("Relações", LengthUnit::Metre).unwrap();
    let structure = ProjectService::open_scoped(
        &path,
        ProjectGrant {
            project_id: Some(project.id),
            scopes: vec![AccessScope::Read, AccessScope::Model],
            disciplines: Some(vec![Discipline::Structure]),
        },
        "structure",
    )
    .unwrap();
    let mut project = structure
        .add_member(project.revision, "V1", [0.0; 3], [3.0, 0.0, 0.0])
        .unwrap();
    assert_eq!(project.model.links.len(), 1);
    let member = project.objects[0].clone();
    let electrical = ProjectService::open_scoped(
        &path,
        ProjectGrant {
            project_id: Some(project.id),
            scopes: vec![AccessScope::Read, AccessScope::Model],
            disciplines: Some(vec![Discipline::Electrical]),
        },
        "electrical",
    )
    .unwrap();
    let mut object_ids = Vec::new();
    for (name, start, end) in [("Equipamento A", 0.0, 1.0), ("Equipamento B", 2.0, 3.0)] {
        project = electrical
            .edit_model(
                project.revision,
                ModelEdit::AddPhysical {
                    name: name.into(),
                    discipline: Discipline::Electrical,
                    start: point(start),
                    end: point(end),
                    role: PhysicalRole::VisualOnly,
                    material_id: None,
                    environment_id: None,
                },
            )
            .unwrap();
        object_ids.push(project.objects.last().unwrap().id);
    }
    project = electrical
        .edit_model(
            project.revision,
            ModelEdit::AddNetwork {
                name: "Circuito".into(),
                discipline: Discipline::Electrical,
                kind: NetworkKind::Circuit,
                environment_id: None,
            },
        )
        .unwrap();
    let network_id = project.networks[0].id;
    let mut port_ids = Vec::new();
    for (i, object_id) in object_ids.into_iter().enumerate() {
        project = electrical
            .edit_model(
                project.revision,
                ModelEdit::AddPort {
                    object_id,
                    discipline: Discipline::Electrical,
                    kind: PortKind::Electrical,
                    position: point(i as f64 * 2.0),
                },
            )
            .unwrap();
        let port_id = project.ports.last().unwrap().id;
        project = electrical
            .edit_model(
                project.revision,
                ModelEdit::AssignPort {
                    network_id,
                    port_id,
                },
            )
            .unwrap();
        port_ids.push(port_id);
    }
    project = electrical
        .edit_model(
            project.revision,
            ModelEdit::ConnectPorts {
                network_id,
                from_port: port_ids[0],
                to_port: port_ids[1],
            },
        )
        .unwrap();
    assert_eq!(project.connections.len(), 1);
    assert_eq!(
        project.objects.iter().find(|object| object.id == member.id),
        Some(&member)
    );
    let filtered = electrical.read_scoped_model().unwrap();
    assert_eq!(filtered.objects.len(), 2);
    assert_eq!(filtered.connections.len(), 1);
    assert!(!filtered.objects.iter().any(|object| object.id == member.id));
}

#[test]
fn preparatory_edits_do_not_bypass_scoped_grants() {
    use prumo_domain::precalculation::PrecalculationEdit;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("precalculation-scopes.prumo");
    let owner = ProjectService::open(&path, AccessMode::ReadWrite, "owner").unwrap();
    let initial = owner
        .create("Escopos da revisão", LengthUnit::Metre)
        .unwrap();
    let project = owner
        .add_member(initial.revision, "Pilar", [0.0, 0.0, 0.0], [0.0, 0.0, 4.0])
        .unwrap();
    let object_id = project.objects[0].id;
    let restricted = ProjectService::open_scoped(
        &path,
        ProjectGrant {
            project_id: Some(project.id),
            scopes: vec![AccessScope::Read, AccessScope::Model],
            disciplines: Some(vec![Discipline::Structure]),
        },
        "structural-editor",
    )
    .unwrap();
    let edit = ModelEdit::EditPrecalculation {
        command: PrecalculationEdit::SetMaterialBatch {
            object_ids: vec![object_id],
            material_id: None,
        },
    };
    assert!(matches!(
        restricted.edit_model(project.revision, edit.clone()),
        Err(AppError::PermissionDenied)
    ));
    assert_eq!(owner.read().unwrap(), project);
    let updated = owner.edit_model(project.revision, edit).unwrap();
    assert_eq!(updated.revision, project.revision + 1);
    assert_eq!(updated.objects[0].id, object_id);
}
