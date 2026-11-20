use prumo_app::{AccessMode, AccessScope, AppError, ProjectGrant, ProjectService};
use prumo_domain::{Discipline, LengthUnit};
use std::fs;
use uuid::Uuid;

#[test]
fn immediate_commands_report_zero_or_stale_revision_before_payload_validation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("revisions.prumo");
    let service = ProjectService::open(&path, AccessMode::ReadWrite, "operador").unwrap();
    let project = service.create("Projeto", LengthUnit::Metre).unwrap();
    let before = fs::read(&path).unwrap();

    for expected_revision in [0, project.revision + 1] {
        for name in ["Nome novo", " "] {
            assert!(matches!(
                service.rename_project(expected_revision, name),
                Err(AppError::Conflict { expected, actual })
                    if expected == expected_revision && actual == project.revision
            ));
        }
        assert!(matches!(
            service.add_member(expected_revision, "Degenerada", [0.0; 3], [0.0; 3]),
            Err(AppError::Conflict { expected, actual })
                if expected == expected_revision && actual == project.revision
        ));
    }

    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(service.read().unwrap(), project);
}

#[test]
fn authorization_precedes_revision_details_for_immediate_commands() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("authorization.prumo");
    let owner = ProjectService::open(&path, AccessMode::ReadWrite, "operador").unwrap();
    let project = owner.create("Projeto", LengthUnit::Metre).unwrap();
    let before = fs::read(&path).unwrap();
    let readonly = ProjectService::open(&path, AccessMode::ReadOnly, "leitor").unwrap();
    assert!(matches!(
        readonly.rename_project(0, "Nome novo"),
        Err(AppError::PermissionDenied)
    ));

    let foreign_project = ProjectService::open_scoped(
        &path,
        ProjectGrant {
            project_id: Some(Uuid::new_v4()),
            scopes: vec![AccessScope::Read, AccessScope::Model],
            disciplines: None,
        },
        "outro-projeto",
    )
    .unwrap();
    assert!(matches!(
        foreign_project.rename_project(0, "Nome novo"),
        Err(AppError::PermissionDenied)
    ));

    let electrical = ProjectService::open_scoped(
        &path,
        ProjectGrant {
            project_id: Some(project.id),
            scopes: vec![AccessScope::Read, AccessScope::Model],
            disciplines: Some(vec![Discipline::Electrical]),
        },
        "eletrica",
    )
    .unwrap();
    assert!(matches!(
        electrical.add_member(0, "Viga", [0.0; 3], [1.0, 0.0, 0.0]),
        Err(AppError::PermissionDenied)
    ));
    assert!(matches!(
        electrical.rename_project(0, "Nome novo"),
        Err(AppError::PermissionDenied)
    ));

    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(owner.read().unwrap(), project);
}
