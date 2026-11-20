//! Cross-cutting regression cases for modelling, durable previews and view state.
use prumo_app::{AccessMode, ProjectService};
use prumo_domain::{
    Command, Discipline, LengthUnit,
    dependencies::{ResultState, SourceKind},
    layout::{LayoutEdit, SnapRequest},
    model::{ModelEdit, PhysicalRole},
    transactions::TransactionRequest,
    units::PointInput,
    view::{CutAxis, ViewCommand, ViewCut},
};
use uuid::Uuid;

fn point(values: [f64; 3]) -> PointInput {
    PointInput {
        values,
        unit: LengthUnit::Metre,
    }
}

#[test]
fn level_preview_moves_dependents_atomically_and_invalidates_existing_measurements() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("levels.prumo");
    let service = ProjectService::open(&path, AccessMode::ReadWrite, "projetista").unwrap();
    let mut project = service
        .create("Níveis desnivelados", LengthUnit::Centimetre)
        .unwrap();
    let level = Uuid::new_v4();
    project = service
        .edit_model(
            project.revision,
            ModelEdit::EditLayout {
                command: LayoutEdit::PutLevel {
                    id: level,
                    name: "Pavimento 1".into(),
                    elevation: "300 cm".into(),
                },
            },
        )
        .unwrap();
    project = service
        .edit_model(
            project.revision,
            ModelEdit::AddPhysical {
                name: "Referência do pavimento".into(),
                discipline: Discipline::Structure,
                start: point([0., 0., 0.]),
                end: point([4., 0., 0.]),
                role: PhysicalRole::VisualOnly,
                material_id: None,
                environment_id: None,
            },
        )
        .unwrap();
    let object = project.objects[0].id;
    project = service
        .edit_model(
            project.revision,
            ModelEdit::EditLayout {
                command: LayoutEdit::BindObject {
                    object_id: object,
                    level_id: level,
                    offset: "-5 cm".into(),
                },
            },
        )
        .unwrap();
    assert_eq!(project.objects[0].start_m[2], 2.95);
    project = service.inspect_model(project.revision).unwrap();
    let measured_ids: Vec<_> = project.dependencies.results.iter().map(|r| r.id).collect();
    assert!(!measured_ids.is_empty());
    let before_bytes = std::fs::read(&path).unwrap();
    let request = TransactionRequest::new(
        &project,
        vec![Command::EditModel {
            expected_revision: project.revision,
            edit: ModelEdit::EditLayout {
                command: LayoutEdit::PutLevel {
                    id: level,
                    name: "Pavimento 1".into(),
                    elevation: "3 m + 25 cm".into(),
                },
            },
        }],
    );
    let preview = service.preview_transaction(request.clone()).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before_bytes);
    assert!(
        preview
            .effects
            .iter()
            .any(|effect| effect.source.kind == SourceKind::Layout && effect.source.id == level)
    );
    assert!(
        preview
            .effects
            .iter()
            .any(|effect| effect.source.kind == SourceKind::PhysicalGeometry
                && effect.source.id == object)
    );
    assert!(!preview.invalidated_result_ids.is_empty());
    drop(service);
    let reopened = ProjectService::open(&path, AccessMode::ReadWrite, "projetista").unwrap();
    let changed = reopened.apply_transaction(&preview.plan_sha256).unwrap();
    assert_eq!(changed.project.objects[0].id, object);
    assert_eq!(changed.project.objects[0].start_m[2], 3.2);
    assert_eq!(changed.project.objects[0].end_m[2], 3.2);
    assert_eq!(changed.project.units, LengthUnit::Centimetre);
    assert!(
        changed
            .project
            .dependencies
            .results
            .iter()
            .filter(|r| preview.invalidated_result_ids.contains(&r.id))
            .all(|r| r.state == ResultState::Stale)
    );
    assert_eq!(reopened.read().unwrap(), changed.project);
    assert!(reopened.execute_transaction(request).unwrap().replayed);
    assert_eq!(
        reopened.model_snapshot().unwrap().sha256,
        preview.after_snapshot_sha256
    );
}

#[test]
fn navigation_is_read_only_session_scoped_and_invalid_commands_are_atomic() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("navigation.prumo");
    let writer = ProjectService::open(&path, AccessMode::ReadWrite, "autor").unwrap();
    let project = writer.create("Vistas", LengthUnit::Metre).unwrap();
    let project = writer
        .add_member(
            project.revision,
            "Térreo",
            [900_000_000., 20., 0.],
            [900_000_004., 20., 0.],
        )
        .unwrap();
    let project = writer
        .add_member(
            project.revision,
            "Pavimento superior",
            [900_000_000., 20., 3.],
            [900_000_004., 20., 3.],
        )
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    let reader = ProjectService::open(&path, AccessMode::ReadOnly, "revisor").unwrap();
    let id = project.objects[1].id;
    let selected = reader
        .view_command(ViewCommand::Select {
            object_id: id,
            reveal: true,
        })
        .unwrap();
    assert_eq!(selected.state.selected_id, Some(id));
    reader.view_command(ViewCommand::HideSelected {}).unwrap();
    assert!(!reader.view_report().unwrap().objects[1].visible);
    reader
        .view_command(ViewCommand::SetCut {
            cut: Some(ViewCut {
                axis: CutAxis::Z,
                min_m: -0.5,
                max_m: 0.5,
            }),
        })
        .unwrap();
    let reveal = reader.view_command(ViewCommand::LocateSelected {}).unwrap();
    assert!(reveal.objects[1].visible);
    assert_eq!(reveal.state.cut, None);
    assert_eq!(reveal.source_revision, project.revision);
    assert!(!reveal.changes_project);
    assert!(!reveal.calculation_available);
    let shared = reader.clone();
    assert_eq!(shared.view_report().unwrap().state, reveal.state);
    assert!(
        shared
            .view_command(ViewCommand::Zoom { factor: 0. })
            .is_err()
    );
    assert_eq!(shared.view_report().unwrap().state, reveal.state);
    let second = ProjectService::open(&path, AccessMode::ReadOnly, "outro-revisor").unwrap();
    assert_eq!(second.view_report().unwrap().state.selected_id, None);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(reader.read().unwrap(), project);
}

#[test]
fn snapping_preserves_explicit_connectivity_and_numeric_ambiguities_fail_without_writes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("snap.prumo");
    let service = ProjectService::open(&path, AccessMode::ReadWrite, "projetista").unwrap();
    let project = service.create("Encaixes", LengthUnit::Millimetre).unwrap();
    let project = service
        .add_member(project.revision, "Viga", [0., 0., 0.], [4000., 0., 0.])
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    let proposal = service
        .layout_snap(&SnapRequest::Endpoint {
            object_id: project.objects[0].id,
            end: true,
        })
        .unwrap();
    assert_eq!(proposal.point_m.as_array(), [4., 0., 0.]);
    assert!(!proposal.creates_structural_connection);
    assert_eq!(
        service.layout_measure("(2,5 m + 50 cm) / 2").unwrap()["value_m"],
        1.5
    );
    for expression in ["1.234,56 m", "3", "4 m / 0", "3 cm + 4", "2 in", "1e999 m"] {
        assert!(service.layout_measure(expression).is_err(), "{expression}");
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(service.read().unwrap(), project);
}

#[test]
fn web_export_is_a_verified_copy_and_cannot_overwrite_unrelated_content() {
    use prumo_domain::canonical::{hash_bytes, model_snapshot};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("project.prumo");
    let service = ProjectService::open(&path, AccessMode::ReadWrite, "autor").unwrap();
    let project = service
        .create("Cópia para visualização", LengthUnit::Metre)
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    let export = service.export_web_view().unwrap();
    let destination = directory.path().join(&export.file_name);
    let bytes = std::fs::read(&destination).unwrap();
    let copy: prumo_domain::Project = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(copy, project);
    assert_eq!(hash_bytes(&bytes), export.content_sha256);
    assert_eq!(
        model_snapshot(&copy).unwrap().sha256,
        export.snapshot_sha256
    );
    assert_eq!(
        service.export_web_view().unwrap().file_name,
        export.file_name
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let readonly = ProjectService::open(&path, AccessMode::ReadOnly, "revisor").unwrap();
    assert!(matches!(
        readonly.export_web_view(),
        Err(prumo_app::AppError::PermissionDenied)
    ));
    std::fs::write(&destination, b"unrelated existing content").unwrap();
    assert!(service.export_web_view().is_err());
    assert_eq!(
        std::fs::read(destination).unwrap(),
        b"unrelated existing content"
    );
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn navigation_and_export_refuse_a_file_changed_after_the_editor_read() {
    use prumo_app::AppError;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("view-basis.prumo");
    let service = ProjectService::open(&path, AccessMode::ReadWrite, "autor").unwrap();
    let project = service
        .create("Revisão exibida", LengthUnit::Metre)
        .unwrap();
    let (_, hash) = service.read_with_basis().unwrap();
    let displayed = service.with_write_basis(project.id, hash).unwrap();
    let before_state = displayed.view_report().unwrap().state;
    let changed = service
        .rename_project(project.revision, "Revisão externa")
        .unwrap();
    assert!(matches!(
        displayed.view_command(ViewCommand::Zoom { factor: 2. }),
        Err(AppError::StorageConflict)
    ));
    assert!(matches!(
        displayed.view_report(),
        Err(AppError::StorageConflict)
    ));
    assert!(matches!(
        displayed.export_web_view(),
        Err(AppError::StorageConflict)
    ));
    assert_eq!(service.view_report().unwrap().state, before_state);
    assert_eq!(service.read().unwrap(), changed);
    assert!(
        !std::fs::read_dir(directory.path())
            .unwrap()
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().ends_with(".view.json"))
    );
}

/// A future mesh and its dependent document are represented only as labelled
/// dependency fixtures. This does not manufacture a structural calculation.
fn dependent_fixtures(
    service: &ProjectService,
    source: prumo_domain::dependencies::SourceKey,
) -> [Uuid; 2] {
    use prumo_domain::{
        canonical::{model_snapshot, source_index_unchecked},
        dependencies::{Dependency, InspectionPayload},
    };
    let mut project = service.read().unwrap();
    let snapshot = model_snapshot(&project).unwrap();
    let index = source_index_unchecked(&project).unwrap();
    project
        .dependencies
        .snapshots
        .insert(snapshot.sha256.clone(), snapshot.clone());
    let first = project
        .dependencies
        .record(
            "Fixture de malha, sem cálculo".into(),
            "authoring-contract/1".into(),
            project.revision,
            project.revision,
            snapshot.sha256.clone(),
            vec![Dependency::source(source, &index).unwrap()],
            InspectionPayload::ExternalEvidence {
                artifact_sha256: "1".repeat(64),
                media_type: "application/x-prumo-fixture".into(),
            },
        )
        .unwrap();
    let payload = project
        .dependencies
        .results
        .last()
        .unwrap()
        .payload_sha256
        .clone();
    let second = project
        .dependencies
        .record(
            "Fixture de documento dependente, sem emissão".into(),
            "authoring-contract/1".into(),
            project.revision,
            project.revision,
            snapshot.sha256,
            vec![Dependency::Result {
                result_id: first,
                payload_sha256: payload,
            }],
            InspectionPayload::ExternalEvidence {
                artifact_sha256: "2".repeat(64),
                media_type: "application/x-prumo-fixture".into(),
            },
        )
        .unwrap();
    project.validate().unwrap();
    std::fs::write(
        service.project_path(),
        serde_json::to_vec(&project).unwrap(),
    )
    .unwrap();
    [first, second]
}

fn apply_json(service: &ProjectService, edit: serde_json::Value) -> prumo_domain::Project {
    service
        .edit_model(
            service.read().unwrap().revision,
            serde_json::from_value(edit).unwrap(),
        )
        .unwrap()
}
fn rectangle(w: f64, h: f64) -> serde_json::Value {
    serde_json::json!([{"x_m":0,"y_m":0},{"x_m":w,"y_m":0},{"x_m":w,"y_m":h},{"x_m":0,"y_m":h}])
}

#[test]
fn opening_and_landing_changes_invalidate_mesh_and_document_fixtures_transitively() {
    use prumo_domain::dependencies::SourceKey;
    use serde_json::json;
    let directory = tempfile::tempdir().unwrap();
    let service = ProjectService::open(
        directory.path().join("dependencies.prumo"),
        AccessMode::ReadWrite,
        "autor",
    )
    .unwrap();
    service
        .create("Dependências de geometria", LengthUnit::Metre)
        .unwrap();
    let project = apply_json(
        &service,
        json!({"edit":"add_material","material":{"name":"Concreto informado","family":"concrete","elastic_modulus":{"value":30,"unit":"gigapascal"},"density":{"value":2500,"unit":"kilogram_per_cubic_metre"},"poisson_ratio":0.2}}),
    );
    let material = project.model.materials[0].id;
    let surface = json!({"name":"Laje","role":"analytical_source","material_id":material,"frame":{"origin_m":[0,0,3],"u":[1,0,0],"v":[0,1,0]},
        "kind":{"kind":"solid_slab"},"outline":rectangle(4.,3.),"openings":[],"thickness_m":0.2,"regions":[],"supports":[]});
    let project = apply_json(
        &service,
        json!({"edit":"edit_surfaces","command":{"action":"create","surface":surface}}),
    );
    let slab = project.objects[0].id;
    let region = project.model.links[0].analytical_id;
    let fixtures = dependent_fixtures(
        &service,
        SourceKey {
            kind: SourceKind::AnalyticalRegion,
            id: region,
        },
    );
    let before = service.read().unwrap();
    let request=TransactionRequest::new(&before,vec![Command::EditModel{expected_revision:before.revision,edit:serde_json::from_value(json!({"edit":"edit_surfaces","command":{"action":"set_openings","object_id":slab,
        "openings":[{"name":"Vão","outline":[{"x_m":1,"y_m":1},{"x_m":2,"y_m":1},{"x_m":2,"y_m":2},{"x_m":1,"y_m":2}]}]}})).unwrap()}]);
    let preview = service.preview_transaction(request).unwrap();
    assert!(
        fixtures
            .iter()
            .all(|id| preview.invalidated_result_ids.contains(id))
    );
    assert_eq!(service.read().unwrap(), before);
    let changed = service
        .apply_transaction(&preview.plan_sha256)
        .unwrap()
        .project;
    for id in fixtures {
        assert_eq!(
            changed
                .dependencies
                .results
                .iter()
                .find(|r| r.id == id)
                .unwrap()
                .state,
            ResultState::Stale
        );
    }
    let inspected = service.surface_inspection(slab).unwrap();
    assert_eq!(inspected["preview"]["net_area_m2"], 11.0);
    let mass = inspected["preview"]["quantities"]["total_mass_kg"]
        .as_f64()
        .unwrap();
    assert!((mass - 5500.).abs() < 1e-9);
    assert_eq!(changed.model.links[0].analytical_id, region);
    let mut assembly = json!({"name":"Escada","role":"analytical_source","material_id":material,
        "shape":{"kind":"stair","form":"conventional","flights":[{"name":"Lance","start_m":[0,0,0],"end_m":[3,0,3],"width_m":1,"thickness_m":0.15,"steps":10}],
        "landings":[{"name":"Patamar","frame":{"origin_m":[3,0,3],"u":[1,0,0],"v":[0,1,0]},"outline":rectangle(1.,1.),"thickness_m":0.15}]},"supports":[],"continuities":[]});
    let project = apply_json(
        &service,
        json!({"edit":"edit_assemblies","command":{"action":"create","assembly":assembly}}),
    );
    let group = project
        .objects
        .iter()
        .find(|o| o.name == "Escada")
        .unwrap()
        .id;
    let inspected = prumo_domain::assemblies::inspect(&project, group).unwrap();
    let landing = inspected
        .parts
        .iter()
        .find(|part| part.key == "landing-0")
        .unwrap()
        .object_id;
    let landing_region = project
        .model
        .links
        .iter()
        .find(|link| link.physical_id == landing)
        .unwrap()
        .analytical_id;
    let fixtures = dependent_fixtures(
        &service,
        SourceKey {
            kind: SourceKind::AnalyticalRegion,
            id: landing_region,
        },
    );
    assembly["shape"]["landings"][0]["outline"] = rectangle(1.5, 1.);
    let changed = apply_json(
        &service,
        json!({"edit":"edit_assemblies","command":{"action":"update","object_id":group,"assembly":assembly}}),
    );
    for id in fixtures {
        assert_eq!(
            changed
                .dependencies
                .results
                .iter()
                .find(|r| r.id == id)
                .unwrap()
                .state,
            ResultState::Stale
        );
    }
    assert!(changed.objects.iter().any(|o| o.id == landing));
    assert_eq!(
        changed
            .model
            .links
            .iter()
            .find(|link| link.physical_id == landing)
            .unwrap()
            .analytical_id,
        landing_region
    );
    assert_eq!(
        prumo_domain::surfaces::inspect(&changed, landing)
            .unwrap()
            .preview
            .net_area_m2,
        1.5
    );
    assert_eq!(
        ProjectService::open(service.project_path(), AccessMode::ReadOnly, "revisor")
            .unwrap()
            .read()
            .unwrap(),
        changed
    );
}
