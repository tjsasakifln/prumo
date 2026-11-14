use prumo_app::{AccessMode, AppError, Command, ProjectService};
use prumo_domain::{
    Connection, Discipline, LengthUnit, ModelState, Network, Port, PortKind, Project,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::sync::{Arc, Barrier};
use uuid::Uuid;

fn service(directory: &tempfile::TempDir, access: AccessMode, actor: &str) -> ProjectService {
    ProjectService::open(directory.path().join("modelo.prumo"), access, actor).unwrap()
}

#[test]
fn persisted_model_reopens_with_the_same_identity_geometry_revision_and_actor() {
    let directory = tempfile::tempdir().unwrap();
    let ui = service(&directory, AccessMode::ReadWrite, "interface:operador");
    let initial = ui
        .create("Projeto São Pedro", LengthUnit::Centimetre)
        .unwrap();
    let model = ui
        .add_member(
            initial.revision,
            "Viga V1",
            [0.0, 0.0, 300.0],
            [500.0, 0.0, 300.0],
        )
        .unwrap();
    assert_eq!(model.revision, 2);
    assert_eq!(model.id, initial.id);
    assert_eq!(model.objects[0].start_m, [0.0, 0.0, 3.0]);
    assert_eq!(model.objects[0].end_m, [5.0, 0.0, 3.0]);
    assert_eq!(model.objects[0].length_m(), 5.0);
    assert_ne!(model.objects[0].id, model.objects[0].analytical_id);
    assert_eq!(model.objects[0].state, ModelState::Modelled);
    drop(ui);

    let mcp = service(&directory, AccessMode::ReadWrite, "mcp:agente");
    assert_eq!(mcp.read().unwrap(), model);
    let changed = mcp
        .rename_project(model.revision, "Revisão coordenada")
        .unwrap();
    drop(mcp);
    let reopened = service(&directory, AccessMode::ReadOnly, "interface:operador")
        .read()
        .unwrap();
    assert_eq!(reopened, changed);
    assert_eq!(reopened.objects, model.objects);
    assert_eq!(reopened.id, initial.id);
    assert_eq!(reopened.history.last().unwrap().actor, "mcp:agente");
    assert_eq!(reopened.revision, 3);
}

#[test]
fn commands_are_the_same_for_every_frontend_and_preserve_environments() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite, "interface");
    let create: Command =
        serde_json::from_value(json!({"operation":"create", "name":"Casa", "units":"millimetre"}))
            .unwrap();
    let project = app.execute(create).unwrap();
    let with_room = app
        .execute(Command::AddEnvironment {
            expected_revision: project.revision,
            name: "Sala 01".into(),
        })
        .unwrap();
    assert_eq!(with_room.environments.len(), 1);
    assert_eq!(app.read().unwrap().environments, with_room.environments);
    assert!(
        serde_json::from_value::<Command>(
            json!({"operation":"create","name":"Outro", "units":"metre", "path":"/tmp/other.prumo"})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<Command>(json!({"operation":"shell", "command":"ls"})).is_err()
    );
}

#[test]
fn read_only_authorization_applies_from_the_first_operation_and_to_every_write() {
    let directory = tempfile::tempdir().unwrap();
    let read_only = service(&directory, AccessMode::ReadOnly, "consulta");
    assert_eq!(
        read_only
            .create("Casa", LengthUnit::Metre)
            .unwrap_err()
            .code(),
        "permission_denied"
    );
    assert!(!read_only.project_path().exists());
    let writer = service(&directory, AccessMode::ReadWrite, "operador");
    let initial = writer.create("Casa", LengthUnit::Metre).unwrap();
    assert_eq!(read_only.read().unwrap(), initial);
    let before = fs::read(writer.project_path()).unwrap();
    let commands = [
        Command::Create {
            name: "Outra".into(),
            units: LengthUnit::Metre,
        },
        Command::AddMember {
            expected_revision: 1,
            name: "V1".into(),
            start: [0.0; 3],
            end: [5.0, 0.0, 0.0],
        },
        Command::RenameProject {
            expected_revision: 1,
            name: "Outra".into(),
        },
        Command::AddEnvironment {
            expected_revision: 1,
            name: "Sala".into(),
        },
    ];
    for command in commands {
        assert!(matches!(
            read_only.execute(command),
            Err(AppError::PermissionDenied)
        ));
    }
    assert_eq!(fs::read(writer.project_path()).unwrap(), before);
}

#[test]
fn creation_never_overwrites_an_existing_project() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite, "operador");
    let first = app.create("Primeiro", LengthUnit::Metre).unwrap();
    assert!(matches!(
        app.create("Segundo", LengthUnit::Millimetre),
        Err(AppError::AlreadyExists)
    ));
    assert_eq!(app.read().unwrap(), first);
}

#[test]
fn invalid_inputs_leave_exactly_the_last_saved_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite, "operador");
    app.create("Casa", LengthUnit::Metre).unwrap();
    let before = fs::read(app.project_path()).unwrap();
    let invalid = [
        ("V1", [0.0; 3], [0.0; 3]),
        ("V1", [f64::NAN, 0.0, 0.0], [1.0; 3]),
        ("V1", [0.0; 3], [f64::INFINITY, 0.0, 0.0]),
        ("", [0.0; 3], [5.0, 0.0, 0.0]),
    ];
    for (name, start, end) in invalid {
        assert_eq!(
            app.add_member(1, name, start, end).unwrap_err().code(),
            "invalid_input"
        );
        assert_eq!(fs::read(app.project_path()).unwrap(), before);
    }
    assert!(app.rename_project(1, "  ").is_err());
    assert!(app.add_environment(1, "\0").is_err());
    assert_eq!(fs::read(app.project_path()).unwrap(), before);
}

#[test]
fn stale_revision_does_not_replace_another_frontends_edit() {
    let directory = tempfile::tempdir().unwrap();
    let ui = service(&directory, AccessMode::ReadWrite, "interface");
    let mcp = service(&directory, AccessMode::ReadWrite, "mcp");
    ui.create("Casa", LengthUnit::Metre).unwrap();
    let from_mcp = mcp.add_member(1, "V1", [0.0; 3], [5.0, 0.0, 0.0]).unwrap();
    assert!(matches!(
        ui.rename_project(1, "Nome desatualizado"),
        Err(AppError::Conflict {
            expected: 1,
            actual: 2
        })
    ));
    assert_eq!(ui.read().unwrap(), from_mcp);
}

#[test]
fn independently_opened_sessions_serialize_writes_and_reject_the_loser() {
    let directory = tempfile::tempdir().unwrap();
    service(&directory, AccessMode::ReadWrite, "interface")
        .create("Casa", LengthUnit::Metre)
        .unwrap();
    let first = service(&directory, AccessMode::ReadWrite, "interface");
    let second = service(&directory, AccessMode::ReadWrite, "mcp");
    let barrier = Arc::new(Barrier::new(2));
    let barrier_b = barrier.clone();
    let thread_a = std::thread::spawn(move || {
        barrier.wait();
        first.rename_project(1, "A")
    });
    let thread_b = std::thread::spawn(move || {
        barrier_b.wait();
        second.rename_project(1, "B")
    });
    let results = [thread_a.join().unwrap(), thread_b.join().unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(AppError::Conflict { .. })))
            .count(),
        1
    );
    let project = service(&directory, AccessMode::ReadOnly, "revisor")
        .read()
        .unwrap();
    assert_eq!(project.revision, 2);
    assert_eq!(project.history.len(), 2);
}

#[test]
fn corrupted_unsupported_and_semantically_invalid_files_are_never_mutated() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite, "operador");
    let project = app.create("Casa", LengthUnit::Metre).unwrap();
    let mut unsupported = serde_json::to_value(&project).unwrap();
    unsupported["schema_version"] = json!(999);
    let mut invalid_identity = serde_json::to_value(&project).unwrap();
    invalid_identity["id"] = json!(Uuid::nil());
    let examples = [
        (b"{broken json".to_vec(), "invalid_project"),
        (
            serde_json::to_vec(&unsupported).unwrap(),
            "unsupported_schema",
        ),
        (
            serde_json::to_vec(&invalid_identity).unwrap(),
            "invalid_project",
        ),
    ];
    for (bytes, code) in examples {
        fs::write(app.project_path(), &bytes).unwrap();
        assert_eq!(app.read().unwrap_err().code(), code);
        assert!(app.rename_project(1, "Outro").is_err());
        assert_eq!(fs::read(app.project_path()).unwrap(), bytes);
    }
}

#[test]
fn extension_contracts_roundtrip_without_implying_a_calculation_or_automatic_connection() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite, "fixture:modelagem");
    app.create("Contrato de extensões", LengthUnit::Metre)
        .unwrap();
    let mut fixture = app
        .add_member(1, "Trecho", [0.0; 3], [1.0, 0.0, 0.0])
        .unwrap();
    fixture.objects[0].discipline = Discipline::WaterSanitation;
    let object_id = fixture.objects[0].id;
    fixture.ports = [0.0, 1.0]
        .into_iter()
        .map(|x| Port {
            id: Uuid::new_v4(),
            object_id,
            discipline: Discipline::WaterSanitation,
            kind: PortKind::Hydraulic,
            position_m: [x, 0.0, 0.0],
            extensions: BTreeMap::new(),
        })
        .collect();
    fixture.networks.push(Network {
        id: Uuid::new_v4(),
        name: "Rede de ensaio sem cálculo".into(),
        discipline: Discipline::WaterSanitation,
        environment_id: None,
        port_ids: fixture.ports.iter().map(|port| port.id).collect(),
        extensions: BTreeMap::new(),
    });
    fixture.extensions.insert(
        "fixture.future_catalog".into(),
        json!({"schema":7,"unknown":{"values":[1,"x",true]}}),
    );
    fixture.validate().unwrap();
    assert!(fixture.connections.is_empty());
    fixture.connections.push(Connection {
        id: Uuid::new_v4(),
        network_id: fixture.networks[0].id,
        from_port: fixture.ports[0].id,
        to_port: fixture.ports[1].id,
        extensions: BTreeMap::new(),
    });
    fixture.validate().unwrap();
    fs::write(app.project_path(), serde_json::to_vec(&fixture).unwrap()).unwrap();
    let saved = app.rename_project(2, "Extensões preservadas").unwrap();
    let reopened = app.read().unwrap();
    assert_eq!(saved, reopened);
    assert_eq!(reopened.extensions, fixture.extensions);
    assert_eq!(reopened.networks, fixture.networks);
    assert_eq!(reopened.ports, fixture.ports);
    assert_eq!(reopened.connections, fixture.connections);
    assert_eq!(reopened.objects[0].state, ModelState::Modelled);
    let mut orphan: Project = reopened;
    orphan.ports[0].object_id = Uuid::new_v4();
    assert!(orphan.validate().is_err());
}

#[test]
fn invalid_typed_references_fail_closed() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite, "fixture");
    app.create("Casa", LengthUnit::Metre).unwrap();
    let mut project = app.add_member(1, "V1", [0.0; 3], [5.0, 0.0, 0.0]).unwrap();
    project.ports.push(Port {
        id: Uuid::new_v4(),
        object_id: project.objects[0].id,
        discipline: Discipline::Structure,
        kind: PortKind::Hydraulic,
        position_m: [0.0; 3],
        extensions: BTreeMap::new(),
    });
    assert!(project.validate().is_err());
    project.ports[0].kind = PortKind::Structural;
    project.validate().unwrap();
    project.objects[0].analytical_id = project.objects[0].id;
    assert!(project.validate().is_err());
}

#[test]
fn project_size_is_bounded_before_parsing() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite, "operador");
    fs::File::create(app.project_path())
        .unwrap()
        .set_len(prumo_app::MAX_PROJECT_BYTES + 1)
        .unwrap();
    assert_eq!(app.read().unwrap_err().code(), "invalid_project");
}

#[cfg(unix)]
#[test]
fn read_only_project_opens_without_creating_a_lock_or_writing_its_directory() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let writer = service(&directory, AccessMode::ReadWrite, "operador");
    let expected = writer
        .create("Arquivo para consulta", LengthUnit::Metre)
        .unwrap();
    let path = writer.project_path().to_path_buf();
    drop(writer);
    let lock_path = directory.path().join("modelo.prumo.lock");
    fs::remove_file(&lock_path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o555)).unwrap();
    let result =
        ProjectService::open(&path, AccessMode::ReadOnly, "leitor").and_then(|app| app.read());
    let lock_created = lock_path.exists();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(result.unwrap(), expected);
    assert!(!lock_created);
}

#[cfg(unix)]
#[test]
fn selected_project_and_lock_cannot_be_symbolic_links() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("outro.prumo");
    fs::write(&target, "preservar").unwrap();
    let path = directory.path().join("modelo.prumo");
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert!(ProjectService::open(&path, AccessMode::ReadWrite, "operador").is_err());
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(&target, directory.path().join("modelo.prumo.lock")).unwrap();
    assert!(ProjectService::open(&path, AccessMode::ReadWrite, "operador").is_err());
    assert_eq!(fs::read_to_string(target).unwrap(), "preservar");
}
