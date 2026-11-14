use prumo_app::{AccessMode, Command, ProjectService};
use prumo_domain::canonical::{hash_bytes, model_snapshot, source_index_unchecked};
use prumo_domain::dependencies::{
    Dependency, InspectionPayload, ResultState, SourceKey, SourceKind,
};
use prumo_domain::model::{ModelEdit, PhysicalRole, PointInput};
use prumo_domain::{Discipline, LengthUnit, Project};
use serde_json::json;
use std::fs;
use uuid::Uuid;

fn point(values: [f64; 3]) -> PointInput {
    PointInput {
        values,
        unit: LengthUnit::Metre,
    }
}
fn service(directory: &tempfile::TempDir, mode: AccessMode) -> ProjectService {
    ProjectService::open(
        directory.path().join("semantic.prumo"),
        mode,
        "fixture:service",
    )
    .unwrap()
}
fn create(directory: &tempfile::TempDir) -> ProjectService {
    let app = service(directory, AccessMode::ReadWrite);
    app.create("Modelo de referência", LengthUnit::Metre)
        .unwrap();
    app
}
fn edit(app: &ProjectService, edit: ModelEdit) -> Project {
    app.execute(Command::EditModel {
        expected_revision: app.read().unwrap().revision,
        edit,
    })
    .unwrap()
}
fn physical(app: &ProjectService, name: &str, end: [f64; 3]) -> Project {
    edit(
        app,
        ModelEdit::AddPhysical {
            name: name.into(),
            discipline: Discipline::Structure,
            start: point([0.0; 3]),
            end: point(end),
            role: PhysicalRole::VisualOnly,
            material_id: None,
            environment_id: None,
        },
    )
}
fn inspect(app: &ProjectService) -> Project {
    app.inspect_model(app.read().unwrap().revision).unwrap()
}

#[test]
fn nontrivial_float_bits_and_canonical_evidence_survive_real_file_roundtrip() {
    let directory = tempfile::tempdir().unwrap();
    let app = create(&directory);
    let coordinate = 1.910_905_980_455_653_9_f64;
    physical(&app, "Precisão preservada", [coordinate, 0.0, 0.0]);
    let measured = inspect(&app);
    let reopened = service(&directory, AccessMode::ReadOnly).read().unwrap();
    assert_eq!(reopened.objects[0].end_m[0].to_bits(), coordinate.to_bits());
    assert_eq!(reopened.dependencies, measured.dependencies);
    assert_eq!(
        model_snapshot(&reopened).unwrap().sha256,
        model_snapshot(&measured).unwrap().sha256
    );
    reopened.validate().unwrap();
}

#[test]
fn immutable_measurements_invalidate_selectively_and_propagate_to_the_summary() {
    let directory = tempfile::tempdir().unwrap();
    let app = create(&directory);
    physical(&app, "Triângulo 3-4-5", [3.0, 4.0, 0.0]);
    physical(&app, "Outra peça", [0.0, 0.0, 2.0]);
    let original = inspect(&app);
    assert_eq!(original.dependencies.results.len(), 3);
    assert!(
        original
            .dependencies
            .results
            .iter()
            .all(|r| r.state == ResultState::Current && !r.engineering_release)
    );
    let first_id = original.objects[0].id;
    let (changed_id, unchanged_id) = original
        .dependencies
        .results
        .iter()
        .filter_map(|r| match r.payload {
            InspectionPayload::PhysicalLength { object_id, .. } => Some((object_id, r.id)),
            _ => None,
        })
        .fold(
            (Uuid::nil(), Uuid::nil()),
            |mut ids, (object_id, result_id)| {
                if object_id == first_id {
                    ids.0 = result_id
                } else {
                    ids.1 = result_id
                }
                ids
            },
        );
    let original_payloads: Vec<_> = original
        .dependencies
        .results
        .iter()
        .map(|r| (r.id, r.payload.clone(), r.payload_sha256.clone()))
        .collect();
    let changed = edit(
        &app,
        ModelEdit::SetPhysicalGeometry {
            object_id: first_id,
            start: point([0.0; 3]),
            end: point([6.0, 8.0, 0.0]),
        },
    );
    assert_eq!(
        changed
            .dependencies
            .results
            .iter()
            .find(|r| r.id == changed_id)
            .unwrap()
            .state,
        ResultState::Stale
    );
    assert_eq!(
        changed
            .dependencies
            .results
            .iter()
            .find(|r| r.id == unchanged_id)
            .unwrap()
            .state,
        ResultState::Current
    );
    assert!(changed.dependencies.results.iter().any(|r| matches!(
        r.payload,
        InspectionPayload::ModelSummary {
            total_length_m: 7.0,
            ..
        }
    ) && r.state == ResultState::Stale));
    assert_eq!(changed.dependencies.invalidations.len(), 1);
    assert_eq!(changed.dependencies.invalidations[0].result_ids.len(), 2);
    assert_eq!(
        changed.dependencies.invalidations[0].actor,
        "fixture:service"
    );
    assert_eq!(
        changed.dependencies.invalidations[0].reason,
        "model.set_physical_geometry"
    );
    for (id, payload, hash) in original_payloads {
        let retained = changed
            .dependencies
            .results
            .iter()
            .find(|r| r.id == id)
            .unwrap();
        assert_eq!(retained.payload, payload);
        assert_eq!(retained.payload_sha256, hash);
    }
    assert_eq!(
        changed.dependencies.snapshots,
        original.dependencies.snapshots
    );
    let fresh = inspect(&app);
    assert!(fresh.dependencies.results.iter().any(|r| matches!(
        r.payload,
        InspectionPayload::ModelSummary {
            total_length_m: 12.0,
            ..
        }
    ) && r.state == ResultState::Current));
    assert_eq!(
        service(&directory, AccessMode::ReadOnly).read().unwrap(),
        fresh
    );
}

#[test]
fn additions_invalidate_collection_results_without_invalidating_unrelated_lengths() {
    let directory = tempfile::tempdir().unwrap();
    let app = create(&directory);
    physical(&app, "A", [1.0, 0.0, 0.0]);
    let measured = inspect(&app);
    let changed = physical(&app, "B", [0.0, 2.0, 0.0]);
    for original in &measured.dependencies.results {
        let actual = changed
            .dependencies
            .results
            .iter()
            .find(|r| r.id == original.id)
            .unwrap();
        let expected = if matches!(original.payload, InspectionPayload::ModelSummary { .. }) {
            ResultState::Stale
        } else {
            ResultState::Current
        };
        assert_eq!(actual.state, expected);
    }
    assert!(
        changed.dependencies.invalidations[0]
            .changes
            .iter()
            .any(|c| c.source.kind == SourceKind::Membership)
    );
}

#[test]
fn metadata_and_display_unit_changes_preserve_the_physical_measurements() {
    let directory = tempfile::tempdir().unwrap();
    let app = create(&directory);
    physical(&app, "A", [3.0, 4.0, 0.0]);
    let measured = inspect(&app);
    let changed = edit(
        &app,
        ModelEdit::SetDisplayUnit {
            unit: LengthUnit::Millimetre,
        },
    );
    assert_eq!(changed.objects, measured.objects);
    assert_eq!(changed.dependencies.results, measured.dependencies.results);
    let renamed = app.rename_project(changed.revision, "Outro nome").unwrap();
    assert_eq!(renamed.dependencies.results, measured.dependencies.results);
    assert!(renamed.dependencies.invalidations.is_empty());
    assert_ne!(
        app.model_snapshot().unwrap().sha256,
        model_snapshot(&measured).unwrap().sha256
    );
}

#[test]
fn canonical_snapshot_is_invariant_to_collection_permutation_but_preserves_endpoint_order() {
    let directory = tempfile::tempdir().unwrap();
    let app = create(&directory);
    app.add_member(1, "A", [0.0; 3], [3.0, 4.0, 0.0]).unwrap();
    app.add_member(2, "B", [0.0; 3], [0.0, 0.0, 2.0]).unwrap();
    let original = app.read().unwrap();
    let expected = model_snapshot(&original).unwrap();
    let mut permuted = original.clone();
    permuted.objects.reverse();
    permuted.model.analytical_nodes.reverse();
    permuted.model.analytical_members.reverse();
    permuted.model.links.reverse();
    assert_eq!(model_snapshot(&permuted).unwrap(), expected);
    let member = &mut permuted.model.analytical_members[0];
    std::mem::swap(&mut member.start_node_id, &mut member.end_node_id);
    assert_ne!(model_snapshot(&permuted).unwrap().sha256, expected.sha256);
    assert_eq!(
        hash_bytes(expected.canonical_json.as_bytes()),
        expected.sha256
    );
}

fn external_fixture(app: &ProjectService, source: SourceKey) -> (Uuid, Uuid) {
    let mut project = app.read().unwrap();
    let snapshot = model_snapshot(&project).unwrap();
    let index = source_index_unchecked(&project).unwrap();
    let hash = snapshot.sha256.clone();
    project
        .dependencies
        .snapshots
        .insert(hash.clone(), snapshot);
    let first = project
        .dependencies
        .record(
            "Fixture de contrato, sem cálculo".into(),
            "fixture/1".into(),
            project.revision,
            project.revision,
            hash.clone(),
            vec![Dependency::source(source, &index).unwrap()],
            InspectionPayload::ExternalEvidence {
                artifact_sha256: hash_bytes(b"fixture only"),
                media_type: "application/x-prumo-fixture".into(),
            },
        )
        .unwrap();
    let payload_hash = project
        .dependencies
        .results
        .last()
        .unwrap()
        .payload_sha256
        .clone();
    let second = project
        .dependencies
        .record(
            "Fixture dependente, sem cálculo".into(),
            "fixture/1".into(),
            project.revision,
            project.revision,
            hash,
            vec![Dependency::Result {
                result_id: first,
                payload_sha256: payload_hash,
            }],
            InspectionPayload::ExternalEvidence {
                artifact_sha256: hash_bytes(b"derived fixture only"),
                media_type: "application/x-prumo-fixture".into(),
            },
        )
        .unwrap();
    project.validate().unwrap();
    fs::write(app.project_path(), serde_json::to_vec(&project).unwrap()).unwrap();
    (first, second)
}

#[test]
fn engineering_input_changes_reach_dependent_contracts_without_producing_engineering_results() {
    use prumo_domain::model::{MaterialFamily, MaterialInput};
    use prumo_domain::units::{DensityInput, DensityUnit, PressureInput, PressureUnit};
    let directory = tempfile::tempdir().unwrap();
    let app = create(&directory);
    let input = MaterialInput {
        name: "Dados informados de ensaio".into(),
        family: MaterialFamily::Generic,
        elastic_modulus: PressureInput {
            value: 200.0,
            unit: PressureUnit::Gigapascal,
        },
        density: DensityInput {
            value: 7850.0,
            unit: DensityUnit::KilogramPerCubicMetre,
        },
        poisson_ratio: 0.3,
    };
    let project = edit(
        &app,
        ModelEdit::AddMaterial {
            material: input.clone(),
        },
    );
    let material_id = project.model.materials[0].id;
    let ids = external_fixture(
        &app,
        SourceKey {
            kind: SourceKind::Material,
            id: material_id,
        },
    );
    let mut updated = input;
    updated.elastic_modulus.value = 190.0;
    let changed = edit(
        &app,
        ModelEdit::SetMaterial {
            material_id,
            material: updated,
        },
    );
    for id in [ids.0, ids.1] {
        let r = changed
            .dependencies
            .results
            .iter()
            .find(|r| r.id == id)
            .unwrap();
        assert_eq!(r.state, ResultState::Stale);
        assert!(!r.engineering_release);
    }
    assert!(
        changed.dependencies.invalidations[0]
            .changes
            .iter()
            .any(|c| c.source.kind == SourceKind::Material)
    );
    assert!(
        !app.final_result_gate("not-a-qualified-capability")
            .unwrap()
            .allowed
    );
}

#[test]
fn altered_snapshot_indices_payloads_and_graph_cycles_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let app = create(&directory);
    let project = physical(&app, "A", [1.0, 0.0, 0.0]);
    external_fixture(
        &app,
        SourceKey {
            kind: SourceKind::Physical,
            id: project.objects[0].id,
        },
    );
    let good = app.read().unwrap();
    let mut altered = good.clone();
    altered
        .dependencies
        .snapshots
        .values_mut()
        .next()
        .unwrap()
        .sources[0]
        .sha256 = hash_bytes(b"tampered index");
    assert!(altered.validate().is_err());
    let mut altered = good.clone();
    altered.dependencies.snapshots.clear();
    assert!(altered.validate().is_err());
    let mut altered = good.clone();
    altered.dependencies.results[0].engineering_release = true;
    assert!(altered.validate().is_err());
    let mut altered = good.clone();
    altered.dependencies.results[0].payload = InspectionPayload::PhysicalLength {
        object_id: project.objects[0].id,
        length_m: 9.0,
    };
    assert!(altered.validate().is_err());
    let mut altered = good.clone();
    let child = altered.dependencies.results[1].clone();
    altered.dependencies.results[0].dependencies = vec![Dependency::Result {
        result_id: child.id,
        payload_sha256: child.payload_sha256,
    }];
    assert!(altered.validate().is_err());
    let mut altered = good.clone();
    altered.dependencies.results[1].dependencies = vec![Dependency::Result {
        result_id: Uuid::new_v4(),
        payload_sha256: hash_bytes(b"orphan"),
    }];
    assert!(altered.validate().is_err());
    let mut altered = good;
    altered
        .dependencies
        .snapshots
        .values_mut()
        .next()
        .unwrap()
        .canonical_json
        .push(' ');
    assert!(altered.validate().is_err());
}

#[test]
fn removing_a_physical_identity_keeps_old_snapshots_and_the_invalidation_cause() {
    let directory = tempfile::tempdir().unwrap();
    let app = create(&directory);
    let project = physical(&app, "A", [10.0, 0.0, 0.0]);
    let id = project.objects[0].id;
    let measured = inspect(&app);
    let snapshots = measured.dependencies.snapshots.clone();
    let changed = edit(
        &app,
        ModelEdit::SplitPhysical {
            object_id: id,
            fraction: 0.4,
            names: ["A1".into(), "A2".into()],
        },
    );
    assert_eq!(changed.objects.len(), 2);
    assert!(!changed.objects.iter().any(|o| o.id == id));
    assert!(
        changed
            .dependencies
            .results
            .iter()
            .all(|r| r.state == ResultState::Stale)
    );
    assert_eq!(changed.dependencies.snapshots, snapshots);
    assert!(
        changed.dependencies.invalidations[0]
            .changes
            .iter()
            .any(|c| c.source.id == id && c.after_sha256.is_none())
    );
    assert_eq!(changed.model.lineage[0].retired_objects[0].id, id);
    assert_eq!(app.read().unwrap(), changed);
}

fn legacy_bytes() -> (Vec<u8>, Uuid, Uuid) {
    let object = Uuid::new_v4();
    let analytical = Uuid::new_v4();
    let value = json!({"schema_version":1,"id":Uuid::new_v4(),"name":"Legado real de contrato","revision":2,"units":"centimetre",
        "objects":[{"id":object,"analytical_id":analytical,"name":"V1","discipline":"structure","start_m":[0.0,0.0,0.0],"end_m":[5.0,0.0,0.0],"state":"modelled","extensions":{"fixture.future":{"schema":9,"opaque":[1,"x",true]}}}],
        "environments":[],"ports":[],"networks":[],"connections":[],
        "history":[{"revision":1,"actor":"fixture-v1","operation":"create","timestamp_unix_ms":100},{"revision":2,"actor":"fixture-v1","operation":"add_member","timestamp_unix_ms":101}],
        "extensions":{"fixture.mep":{"version":7,"unknown":{"pressure_unit":"future"}}}});
    (
        serde_json::to_vec_pretty(&value).unwrap(),
        object,
        analytical,
    )
}

#[test]
fn legacy_reads_are_deterministic_and_first_write_preserves_the_exact_original() {
    let directory = tempfile::tempdir().unwrap();
    let app = service(&directory, AccessMode::ReadWrite);
    let (bytes, object, analytical) = legacy_bytes();
    fs::write(app.project_path(), &bytes).unwrap();
    let readonly = service(&directory, AccessMode::ReadOnly);
    let first = readonly.read().unwrap();
    assert_eq!(first, readonly.read().unwrap());
    assert_eq!(fs::read(app.project_path()).unwrap(), bytes);
    assert_eq!(first.schema_version, 2);
    assert_eq!(first.objects[0].id, object);
    assert_eq!(first.model.analytical_members[0].id, analytical);
    assert_eq!(first.objects[0].length_m(), 5.0);
    assert!(
        first
            .model
            .identities
            .values()
            .all(|identity| identity.created_by.is_none() && identity.created_revision.is_none())
    );
    let written = app.rename_project(2, "Legado preservado").unwrap();
    assert_eq!(written.objects, first.objects);
    assert_eq!(written.extensions, first.extensions);
    let backup = directory
        .path()
        .join(format!("prumo-v1-{}.backup", hash_bytes(&bytes)));
    assert_eq!(fs::read(&backup).unwrap(), bytes);
    assert_eq!(app.read().unwrap(), written);
    app.rename_project(3, "Segunda edição").unwrap();
    assert_eq!(fs::read(backup).unwrap(), bytes);
}

#[test]
fn invalid_edits_readonly_queries_and_bad_backup_never_replace_saved_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let app = create(&directory);
    let before = fs::read(app.project_path()).unwrap();
    assert!(
        app.edit_model(
            1,
            ModelEdit::AddAnalyticalMember {
                name: "Órfão".into(),
                start_node_id: Uuid::new_v4(),
                end_node_id: Uuid::new_v4(),
                local_y: [0.0, 1.0, 0.0]
            }
        )
        .is_err()
    );
    assert_eq!(fs::read(app.project_path()).unwrap(), before);
    let readonly = service(&directory, AccessMode::ReadOnly);
    assert_eq!(
        readonly.inspect_model(1).unwrap_err().code(),
        "permission_denied"
    );
    assert_eq!(
        readonly
            .edit_model(
                1,
                ModelEdit::SetDisplayUnit {
                    unit: LengthUnit::Millimetre
                }
            )
            .unwrap_err()
            .code(),
        "permission_denied"
    );
    assert!(readonly.model_snapshot().is_ok());
    assert!(readonly.dependency_report().is_ok());
    assert_eq!(fs::read(app.project_path()).unwrap(), before);
    let (legacy, _, _) = legacy_bytes();
    fs::write(app.project_path(), &legacy).unwrap();
    let backup = directory
        .path()
        .join(format!("prumo-v1-{}.backup", hash_bytes(&legacy)));
    fs::write(&backup, b"incomplete backup fixture").unwrap();
    assert!(app.rename_project(2, "Must not overwrite").is_err());
    assert_eq!(fs::read(app.project_path()).unwrap(), legacy);
    assert_eq!(fs::read(backup).unwrap(), b"incomplete backup fixture");
}
