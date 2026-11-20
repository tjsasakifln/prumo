use prumo_app::{AccessMode, ProjectService};
use prumo_domain::{LengthUnit, geometry::*, model::ModelEdit};

fn setup() -> (tempfile::TempDir, ProjectService) {
    let d = tempfile::tempdir().unwrap();
    let service = ProjectService::open(
        d.path().join("geometry.prumo"),
        AccessMode::ReadWrite,
        "geometry-review",
    )
    .unwrap();
    service.create("Geometry", LengthUnit::Metre).unwrap();
    (d, service)
}

#[test]
fn close_visual_contacts_never_create_links_and_order_does_not_change_report() {
    let (_dir, s) = setup();
    let p = s.add_member(1, "A", [0., 0., 0.], [1., 0., 0.]).unwrap();
    let p = s
        .add_member(p.revision, "B", [1. + 0.5e-6, 0., 0.], [2., 0., 0.])
        .unwrap();
    let original = p.clone();
    let report = audit_project(&p, &GeometryFilter::default()).unwrap();
    assert!(report.findings.iter().any(|f| matches!(
        f,
        GeometryFinding::NearContact {
            explicitly_connected: false,
            ..
        }
    )));
    assert_eq!(p, original);
    let mut permuted = p;
    permuted.objects.reverse();
    assert_eq!(
        report,
        audit_project(&permuted, &GeometryFilter::default()).unwrap()
    );
    assert!(!report.engineering_calculation_available);
}

#[test]
fn repairs_bind_the_reviewed_revision_show_dimension_changes_and_are_audited() {
    let (_dir, s) = setup();
    let p = s.add_member(1, "A", [0., 0., 0.], [1., 0., 0.]).unwrap();
    let id = p.objects[0].id;
    let repairs = vec![GeometryRepair::SnapEndpoint {
        object_id: id,
        endpoint: Endpoint::End,
        target_m: [1. + 0.5e-6, 0., 0.],
        reason: "Ajuste conferido".into(),
    }];
    let preview = preview_repairs(&p, &repairs, "geometry-review").unwrap();
    assert_eq!(preview.dimensions_changed.len(), 1);
    assert_eq!(preview.explicit_connections_added, 0);
    let changed = s
        .edit_model(
            p.revision,
            ModelEdit::ApplyGeometryRepair {
                plan_hash: preview.plan_hash.clone(),
                repairs: repairs.clone(),
            },
        )
        .unwrap();
    assert_eq!(changed.objects[0].id, id);
    assert!(changed.objects[0].length_m() > 1.);
    assert_eq!(changed.history.last().unwrap().actor, "geometry-review");
    assert!(
        s.edit_model(
            changed.revision,
            ModelEdit::ApplyGeometryRepair {
                plan_hash: preview.plan_hash,
                repairs
            }
        )
        .is_err()
    );
    assert_eq!(s.read().unwrap(), changed);
}

#[test]
fn an_invalid_repair_inside_a_plan_does_not_partially_move_the_project() {
    let (_dir, s) = setup();
    let p = s.add_member(1, "A", [0., 0., 0.], [1., 0., 0.]).unwrap();
    let repairs = vec![
        GeometryRepair::SnapEndpoint {
            object_id: p.objects[0].id,
            endpoint: Endpoint::End,
            target_m: [1. + 0.5e-6, 0., 0.],
            reason: "Primeiro ajuste".into(),
        },
        GeometryRepair::SnapEndpoint {
            object_id: p.objects[0].id,
            endpoint: Endpoint::Start,
            target_m: [100., 0., 0.],
            reason: "Excede tolerância".into(),
        },
    ];
    assert!(preview_repairs(&p, &repairs, "geometry-review").is_err());
    assert_eq!(s.read().unwrap(), p);
}

#[test]
fn selection_tolerance_cannot_change_topological_contact_rules() {
    let (_dir, s) = setup();
    let p = s.add_member(1, "A", [0., 0., 0.], [1., 0., 0.]).unwrap();
    let p = s
        .add_member(p.revision, "B", [1.005, 0., 0.], [2., 0., 0.])
        .unwrap();
    let p = s
        .edit_model(
            p.revision,
            ModelEdit::SetGeometryTolerances {
                tolerances: TolerancePolicy {
                    selection_m: 1.,
                    ..Default::default()
                },
            },
        )
        .unwrap();
    assert!(
        audit_project(&p, &GeometryFilter::default())
            .unwrap()
            .findings
            .is_empty()
    );
    assert_eq!(
        project_tolerances(&s.read().unwrap()).unwrap().topology_m,
        1e-6
    );
}
