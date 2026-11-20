use prumo_app::{AccessMode, ProjectService};
use prumo_domain::{LengthUnit, geometry::*, model::ModelEdit};

fn setup() -> (tempfile::TempDir, ProjectService) {
    let directory = tempfile::tempdir().unwrap();
    let service = ProjectService::open(
        directory.path().join("geometry-review.prumo"),
        AccessMode::ReadWrite,
        "geometry-review",
    )
    .unwrap();
    service
        .create("Revisão independente", LengthUnit::Metre)
        .unwrap();
    (directory, service)
}

#[test]
fn an_endpoint_touching_another_members_interior_is_reported_without_connecting_it() {
    let (_directory, service) = setup();
    let first = service
        .add_member(1, "A", [0.0; 3], [2.0, 0.0, 0.0])
        .unwrap();
    let project = service
        .add_member(first.revision, "B", [1.0, 0.0, 0.0], [1.0, 1.0, 0.0])
        .unwrap();
    let report = service.geometry_report(&GeometryFilter::default()).unwrap();
    assert!(report.findings.iter().any(|finding| matches!(finding,
        GeometryFinding::NearContact { distance_m, explicitly_connected: false, .. } if *distance_m == 0.0
    )), "{report:?}");
    assert_eq!(service.read().unwrap(), project);
    assert!(project.connections.is_empty());
}

#[test]
fn a_geometry_preview_cannot_be_reused_with_another_actor_payload_or_revision() {
    let (directory, service) = setup();
    let project = service
        .add_member(1, "A", [0.0; 3], [1.0, 0.0, 0.0])
        .unwrap();
    let repairs = vec![GeometryRepair::SnapEndpoint {
        object_id: project.objects[0].id,
        endpoint: Endpoint::End,
        target_m: [1.0 + 0.5e-6, 0.0, 0.0],
        reason: "Correção de ponta conferida".into(),
    }];
    let preview = service.geometry_repair_preview(&repairs).unwrap();
    let other = ProjectService::open(
        directory.path().join("geometry-review.prumo"),
        AccessMode::ReadWrite,
        "outro-operador",
    )
    .unwrap();
    assert!(
        other
            .edit_model(
                project.revision,
                ModelEdit::ApplyGeometryRepair {
                    plan_hash: preview.plan_hash.clone(),
                    repairs: repairs.clone(),
                }
            )
            .is_err()
    );
    let mut changed_repairs = repairs.clone();
    let GeometryRepair::SnapEndpoint { target_m, .. } = &mut changed_repairs[0] else {
        unreachable!()
    };
    target_m[0] = 1.0 + 0.75e-6;
    assert!(
        service
            .edit_model(
                project.revision,
                ModelEdit::ApplyGeometryRepair {
                    plan_hash: preview.plan_hash.clone(),
                    repairs: changed_repairs,
                }
            )
            .is_err()
    );
    assert_eq!(service.read().unwrap(), project);
    let current = service
        .rename_project(project.revision, "Revisão posterior")
        .unwrap();
    assert!(
        service
            .edit_model(
                current.revision,
                ModelEdit::ApplyGeometryRepair {
                    plan_hash: preview.plan_hash,
                    repairs,
                }
            )
            .is_err()
    );
    assert_eq!(service.read().unwrap(), current);
}

#[test]
fn a_short_valid_member_is_flagged_by_analysis_tolerance_without_becoming_a_connection() {
    let (_directory, service) = setup();
    let project = service
        .add_member(1, "Vão muito curto", [0.0; 3], [5e-9, 0.0, 0.0])
        .unwrap();
    let report = service.geometry_report(&GeometryFilter::default()).unwrap();
    assert!(
        matches!(report.findings.as_slice(), [GeometryFinding::ShortElement { object_id, length_m }]
        if *object_id == project.objects[0].id && *length_m == 5e-9)
    );
    assert_eq!(service.read().unwrap(), project);
}

#[test]
fn a_reviewed_tolerance_transaction_survives_serialization_and_reopening() {
    use prumo_domain::{Command, transactions::TransactionRequest};
    let (directory, service) = setup();
    let project = service.read().unwrap();
    let tolerances = TolerancePolicy {
        topology_m: 5e-7,
        analysis_m: 2e-8,
        ..Default::default()
    };
    let request = TransactionRequest::new(
        &project,
        vec![Command::EditModel {
            expected_revision: project.revision,
            edit: ModelEdit::SetGeometryTolerances { tolerances },
        }],
    );
    let preview = service.preview_transaction(request).unwrap();
    drop(service);
    let reopened = ProjectService::open(
        directory.path().join("geometry-review.prumo"),
        AccessMode::ReadWrite,
        "geometry-review",
    )
    .unwrap();
    let applied = reopened.apply_transaction(&preview.plan_sha256).unwrap();
    assert_eq!(project_tolerances(&applied.project).unwrap(), tolerances);
    assert_eq!(reopened.read().unwrap(), applied.project);
}
