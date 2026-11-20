use prumo_app::{AccessMode, AppError, Command, ProjectService};
use prumo_domain::canonical::model_snapshot;
use prumo_domain::dependencies::{ResultState, SourceKey, SourceKind};
use prumo_domain::model::{ModelEdit, NetworkKind, PhysicalRole, PointInput};
use prumo_domain::transactions::{
    SourcePrecondition, TransactionHistoryRequest, TransactionRequest,
};
use prumo_domain::{Discipline, LengthUnit, PortKind, Project};
use std::fs;
use std::sync::{Arc, Barrier};
use uuid::Uuid;

fn setup() -> (tempfile::TempDir, ProjectService, Project) {
    let directory = tempfile::tempdir().unwrap();
    let service = ProjectService::open(
        directory.path().join("modelo.prumo"),
        AccessMode::ReadWrite,
        "operador",
    )
    .unwrap();
    let project = service.create("Projeto", LengthUnit::Metre).unwrap();
    (directory, service, project)
}

fn member(revision: u64, name: &str, end: f64) -> Command {
    Command::AddMember {
        expected_revision: revision,
        name: name.into(),
        start: [0.0; 3],
        end: [end, 0.0, 0.0],
    }
}

#[test]
fn preview_applies_exact_ids_and_replay_survives_restart_without_losing_later_work() {
    let (directory, service, project) = setup();
    let request = TransactionRequest::new(
        &project,
        vec![
            member(project.revision, "V1", 4.0),
            Command::AddEnvironment {
                expected_revision: project.revision,
                name: "Sala".into(),
            },
        ],
    );
    let before = fs::read(service.project_path()).unwrap();
    let preview = service.preview_transaction(request.clone()).unwrap();
    let repeated_preview = service.preview_transaction(request.clone()).unwrap();
    assert_eq!(preview.plan_sha256, repeated_preview.plan_sha256);
    assert_eq!(fs::read(service.project_path()).unwrap(), before);
    let applied = service.apply_transaction(&preview.plan_sha256).unwrap();
    assert_eq!(applied.project.revision, project.revision + 1);
    assert_eq!(applied.project.objects.len(), 1);
    assert_eq!(applied.project.environments.len(), 1);
    assert_eq!(
        model_snapshot(&applied.project).unwrap().sha256,
        preview.after_snapshot_sha256
    );
    let changed = service
        .rename_project(applied.project.revision, "Trabalho posterior")
        .unwrap();
    drop(service);
    let reopened = ProjectService::open(
        directory.path().join("modelo.prumo"),
        AccessMode::ReadWrite,
        "operador",
    )
    .unwrap();
    let replay = reopened.execute_transaction(request).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.receipt.committed_revision, applied.project.revision);
    assert_eq!(replay.project, changed);
    let repeated_apply = reopened.apply_transaction(&preview.plan_sha256).unwrap();
    assert!(repeated_apply.replayed);
    assert_eq!(repeated_apply.project, changed);
}

#[test]
fn failed_later_operation_keeps_project_package_and_database_head_unchanged() {
    let (_directory, service, project) = setup();
    let before = fs::read(service.project_path()).unwrap();
    let request = TransactionRequest::new(
        &project,
        vec![member(1, "Válida", 4.0), member(1, "Degenerada", 0.0)],
    );
    assert!(service.execute_transaction(request.clone()).is_err());
    assert!(service.preview_transaction(request).is_err());
    assert_eq!(fs::read(service.project_path()).unwrap(), before);
    assert_eq!(service.read().unwrap(), project);
    assert_eq!(
        service.storage_status().unwrap().state,
        prumo_domain::storage::StorageState::Ready
    );
}

#[test]
fn same_key_cannot_change_payload_or_authenticated_actor() {
    let (directory, service, project) = setup();
    let request = TransactionRequest::new(&project, vec![member(1, "V1", 4.0)]);
    let applied = service.execute_transaction(request.clone()).unwrap();
    let mut changed = request.clone();
    changed.operations = vec![member(1, "V2", 7.0)];
    assert!(matches!(
        service.execute_transaction(changed),
        Err(AppError::IdempotencyConflict)
    ));
    let other = ProjectService::open(
        directory.path().join("modelo.prumo"),
        AccessMode::ReadWrite,
        "outro-operador",
    )
    .unwrap();
    assert!(matches!(
        other.execute_transaction(request),
        Err(AppError::IdempotencyConflict)
    ));
    assert_eq!(service.read().unwrap(), applied.project);
}

#[test]
fn optimistic_concurrency_allows_only_one_of_two_real_clients() {
    let (directory, service, project) = setup();
    let other = ProjectService::open(
        directory.path().join("modelo.prumo"),
        AccessMode::ReadWrite,
        "outro-cliente",
    )
    .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let first_request = TransactionRequest::new(&project, vec![member(1, "Interface", 3.0)]);
    let second_request = TransactionRequest::new(&project, vec![member(1, "MCP", 5.0)]);
    let first_barrier = barrier.clone();
    let first = std::thread::spawn(move || {
        first_barrier.wait();
        service.execute_transaction(first_request)
    });
    let second = std::thread::spawn(move || {
        barrier.wait();
        other.execute_transaction(second_request)
    });
    let outcomes = [first.join().unwrap(), second.join().unwrap()];
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(
                outcome,
                Err(AppError::Conflict {
                    expected: 1,
                    actual: 2
                })
            ))
            .count(),
        1
    );
    let project = ProjectService::open(
        directory.path().join("modelo.prumo"),
        AccessMode::ReadOnly,
        "leitor",
    )
    .unwrap()
    .read()
    .unwrap();
    assert_eq!(project.objects.len(), 1);
}

#[test]
fn stale_review_and_wrong_element_precondition_are_rejected() {
    let (_directory, service, project) = setup();
    let request = TransactionRequest::new(&project, vec![member(1, "V1", 4.0)]);
    let preview = service.preview_transaction(request).unwrap();
    service.rename_project(1, "Outra decisão").unwrap();
    assert!(matches!(
        service.apply_transaction(&preview.plan_sha256),
        Err(AppError::Conflict { .. })
    ));
    let current = service.read().unwrap();
    let mut request = TransactionRequest::new(&current, vec![member(current.revision, "V2", 4.0)]);
    request.preconditions.push(SourcePrecondition {
        source: SourceKey {
            kind: SourceKind::ProjectMetadata,
            id: current.id,
        },
        sha256: Some("0".repeat(64)),
    });
    assert!(matches!(
        service.execute_transaction(request),
        Err(AppError::PreconditionFailed)
    ));
    assert_eq!(service.read().unwrap(), current);
}

#[test]
fn plan_tampering_and_cross_actor_application_cannot_write() {
    let (directory, service, project) = setup();
    let preview = service
        .preview_transaction(TransactionRequest::new(
            &project,
            vec![member(1, "V1", 4.0)],
        ))
        .unwrap();
    let other = ProjectService::open(
        directory.path().join("modelo.prumo"),
        AccessMode::ReadWrite,
        "outro-operador",
    )
    .unwrap();
    assert!(matches!(
        other.apply_transaction(&preview.plan_sha256),
        Err(AppError::PermissionDenied)
    ));
    let work = fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.is_dir())
        .unwrap();
    let path = work
        .join("prepared-transactions")
        .join(format!("{}.json", preview.plan_sha256));
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["after"]["name"] = "Adulterado".into();
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(matches!(
        service.apply_transaction(&preview.plan_sha256),
        Err(AppError::PlanConflict)
    ));
    assert_eq!(service.read().unwrap(), project);
}

fn edit(service: &ProjectService, project: Project, edit: ModelEdit) -> Project {
    service.edit_model(project.revision, edit).unwrap()
}

#[test]
fn undo_redo_preserve_network_connections_geometry_units_ids_and_audit_after_restart() {
    let (directory, service, project) = setup();
    let mut project = edit(
        &service,
        project,
        ModelEdit::AddPhysical {
            name: "Tubo".into(),
            discipline: Discipline::WaterSanitation,
            start: PointInput {
                values: [0.0; 3],
                unit: LengthUnit::Metre,
            },
            end: PointInput {
                values: [4.0, 0.0, 0.0],
                unit: LengthUnit::Metre,
            },
            role: PhysicalRole::VisualOnly,
            material_id: None,
            environment_id: None,
        },
    );
    let object_id = project.objects[0].id;
    project = edit(
        &service,
        project,
        ModelEdit::AddNetwork {
            name: "Água".into(),
            discipline: Discipline::WaterSanitation,
            kind: NetworkKind::Network,
            environment_id: None,
        },
    );
    let network_id = project.networks[0].id;
    for x in [0.0, 4.0] {
        project = edit(
            &service,
            project,
            ModelEdit::AddPort {
                object_id,
                discipline: Discipline::WaterSanitation,
                kind: PortKind::Hydraulic,
                position: PointInput {
                    values: [x, 0.0, 0.0],
                    unit: LengthUnit::Metre,
                },
            },
        );
    }
    let ports = [project.ports[0].id, project.ports[1].id];
    for port_id in ports {
        project = edit(
            &service,
            project,
            ModelEdit::AssignPort {
                network_id,
                port_id,
            },
        );
    }
    project = edit(
        &service,
        project,
        ModelEdit::ConnectPorts {
            network_id,
            from_port: ports[0],
            to_port: ports[1],
        },
    );
    let before = project.clone();
    let request = TransactionRequest::new(
        &project,
        vec![
            Command::EditModel {
                expected_revision: project.revision,
                edit: ModelEdit::SetDisplayUnit {
                    unit: LengthUnit::Centimetre,
                },
            },
            Command::EditModel {
                expected_revision: project.revision,
                edit: ModelEdit::DisconnectPorts {
                    connection_id: project.connections[0].id,
                },
            },
            Command::EditModel {
                expected_revision: project.revision,
                edit: ModelEdit::SetPhysicalGeometry {
                    object_id,
                    start: PointInput {
                        values: [0.0; 3],
                        unit: LengthUnit::Metre,
                    },
                    end: PointInput {
                        values: [6.0, 0.0, 0.0],
                        unit: LengthUnit::Metre,
                    },
                },
            },
        ],
    );
    let applied = service.execute_transaction(request).unwrap();
    assert_eq!(applied.project.revision, before.revision + 1);
    drop(service);
    let reopened = ProjectService::open(
        directory.path().join("modelo.prumo"),
        AccessMode::ReadWrite,
        "revisor",
    )
    .unwrap();
    let undone = reopened
        .undo_transaction(TransactionHistoryRequest::new(
            &applied.project,
            applied.receipt.command_id,
        ))
        .unwrap();
    assert_eq!(undone.project.objects, before.objects);
    assert_eq!(undone.project.networks, before.networks);
    assert_eq!(undone.project.ports, before.ports);
    assert_eq!(undone.project.connections, before.connections);
    assert_eq!(undone.project.model, before.model);
    assert_eq!(undone.project.units, before.units);
    assert_eq!(undone.project.revision, before.revision + 2);
    assert_eq!(undone.project.history.last().unwrap().actor, "revisor");
    let redo_request = TransactionHistoryRequest::new(&undone.project, applied.receipt.command_id);
    let redone = reopened.redo_transaction(redo_request.clone()).unwrap();
    assert_eq!(redone.project.objects, applied.project.objects);
    assert_eq!(redone.project.connections, applied.project.connections);
    assert_eq!(redone.project.model, applied.project.model);
    assert_eq!(redone.project.units, LengthUnit::Centimetre);
    assert!(reopened.redo_transaction(redo_request).unwrap().replayed);
    assert_eq!(
        reopened.transaction_history().unwrap().undo_command_id,
        Some(applied.receipt.command_id)
    );
}

#[test]
fn undo_does_not_revalidate_obsolete_results_and_new_edit_clears_redo() {
    let (_directory, service, project) = setup();
    let project = service
        .add_member(project.revision, "V1", [0.0; 3], [4.0, 0.0, 0.0])
        .unwrap();
    let project = service.inspect_model(project.revision).unwrap();
    let request = TransactionRequest::new(
        &project,
        vec![Command::EditModel {
            expected_revision: project.revision,
            edit: ModelEdit::SetPhysicalGeometry {
                object_id: project.objects[0].id,
                start: PointInput {
                    values: [0.0; 3],
                    unit: LengthUnit::Metre,
                },
                end: PointInput {
                    values: [8.0, 0.0, 0.0],
                    unit: LengthUnit::Metre,
                },
            },
        }],
    );
    let applied = service.execute_transaction(request).unwrap();
    assert!(!applied.receipt.invalidated_result_ids.is_empty());
    let undone = service
        .undo_transaction(TransactionHistoryRequest::new(
            &applied.project,
            applied.receipt.command_id,
        ))
        .unwrap();
    assert!(
        undone
            .project
            .dependencies
            .results
            .iter()
            .all(|result| result.state == ResultState::Stale)
    );
    let changed = service
        .rename_project(undone.project.revision, "Nova decisão")
        .unwrap();
    assert_eq!(service.transaction_history().unwrap().redo_command_id, None);
    assert!(matches!(
        service.redo_transaction(TransactionHistoryRequest::new(
            &changed,
            applied.receipt.command_id
        )),
        Err(AppError::HistoryConflict)
    ));
}

#[test]
fn replayed_inspection_does_not_duplicate_result_jobs_and_request_contract_is_closed() {
    let (_directory, service, project) = setup();
    let request = TransactionRequest::new(
        &project,
        vec![Command::InspectModel {
            expected_revision: project.revision,
        }],
    );
    let applied = service.execute_transaction(request.clone()).unwrap();
    let replay = service.execute_transaction(request).unwrap();
    assert!(replay.replayed);
    assert_eq!(
        replay.project.dependencies.results,
        applied.project.dependencies.results
    );
    assert_eq!(replay.project.revision, applied.project.revision);
    let mut value = serde_json::to_value(TransactionRequest::new(
        &replay.project,
        vec![member(replay.project.revision, "V1", 4.0)],
    ))
    .unwrap();
    value["actor"] = "administrador".into();
    assert!(serde_json::from_value::<TransactionRequest>(value).is_err());
}

#[test]
fn readonly_and_foreign_project_requests_fail_before_mutating() {
    let (directory, service, project) = setup();
    let readonly = ProjectService::open(
        directory.path().join("modelo.prumo"),
        AccessMode::ReadOnly,
        "leitor",
    )
    .unwrap();
    let request = TransactionRequest::new(&project, vec![member(1, "V1", 4.0)]);
    assert!(matches!(
        readonly.preview_transaction(request.clone()),
        Err(AppError::PermissionDenied)
    ));
    let mut foreign = request;
    foreign.project_id = Uuid::new_v4();
    assert!(matches!(
        service.execute_transaction(foreign),
        Err(AppError::PermissionDenied)
    ));
    assert_eq!(service.read().unwrap(), project);
}

#[test]
fn a_source_precondition_requires_explicit_hash_or_null() {
    let condition = SourcePrecondition {
        source: SourceKey {
            kind: SourceKind::Physical,
            id: Uuid::new_v4(),
        },
        sha256: None,
    };
    let mut value = serde_json::to_value(&condition).unwrap();
    assert!(value.get("sha256").unwrap().is_null());
    assert!(serde_json::from_value::<SourcePrecondition>(value.clone()).is_ok());
    value.as_object_mut().unwrap().remove("sha256");
    assert!(serde_json::from_value::<SourcePrecondition>(value).is_err());
}

#[test]
fn inspections_between_undo_and_redo_remain_auditable_and_are_invalidated_again() {
    let (_directory, service, project) = setup();
    let project = service
        .add_member(project.revision, "V1", [0.0; 3], [4.0, 0.0, 0.0])
        .unwrap();
    let applied = service
        .execute_transaction(TransactionRequest::new(
            &project,
            vec![Command::EditModel {
                expected_revision: project.revision,
                edit: ModelEdit::SetPhysicalGeometry {
                    object_id: project.objects[0].id,
                    start: PointInput {
                        values: [0.0; 3],
                        unit: LengthUnit::Metre,
                    },
                    end: PointInput {
                        values: [8.0, 0.0, 0.0],
                        unit: LengthUnit::Metre,
                    },
                },
            }],
        ))
        .unwrap();
    let inspected_before_undo = service.inspect_model(applied.project.revision).unwrap();
    let previous_results = inspected_before_undo.dependencies.results.clone();
    assert!(!previous_results.is_empty());
    let undone = service
        .undo_transaction(TransactionHistoryRequest::new(
            &inspected_before_undo,
            applied.receipt.command_id,
        ))
        .unwrap();
    assert!(
        undone
            .project
            .dependencies
            .results
            .iter()
            .all(|result| result.state == ResultState::Stale)
    );
    let inspected_before_redo = service.inspect_model(undone.project.revision).unwrap();
    assert_eq!(
        service.transaction_history().unwrap().redo_command_id,
        Some(applied.receipt.command_id)
    );
    let redone = service
        .redo_transaction(TransactionHistoryRequest::new(
            &inspected_before_redo,
            applied.receipt.command_id,
        ))
        .unwrap();
    assert_eq!(redone.project.objects, applied.project.objects);
    assert_eq!(
        redone.project.dependencies.results.len(),
        inspected_before_redo.dependencies.results.len()
    );
    assert!(
        redone
            .project
            .dependencies
            .results
            .iter()
            .all(|result| result.state == ResultState::Stale)
    );
    for previous in previous_results {
        assert!(
            redone
                .project
                .dependencies
                .results
                .iter()
                .any(|result| result.id == previous.id)
        );
    }
    assert!(!redone.receipt.invalidated_result_ids.is_empty());
}
