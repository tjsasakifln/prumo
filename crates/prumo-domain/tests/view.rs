use prumo_domain::{
    ChangeRecord, Discipline, LengthUnit, PROJECT_SCHEMA_VERSION, Project,
    model::{ModelEdit, PhysicalRole, apply_model_edit},
    units::PointInput,
    view::{CutAxis, ViewCommand, ViewCut, ViewKind, ViewState},
};
use uuid::Uuid;

fn project() -> Project {
    Project {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name: "Fixture de navegação, sem cálculo".into(),
        revision: 1,
        units: LengthUnit::Metre,
        objects: vec![],
        environments: vec![],
        networks: vec![],
        ports: vec![],
        connections: vec![],
        history: vec![ChangeRecord {
            revision: 1,
            actor: "teste".into(),
            operation: "create".into(),
            timestamp_unix_ms: 1,
        }],
        normative: Default::default(),
        model: Default::default(),
        dependencies: Default::default(),
        extensions: Default::default(),
    }
}

fn add(project: &mut Project, name: &str, a: [f64; 3], b: [f64; 3]) -> Uuid {
    let change = apply_model_edit(
        project,
        ModelEdit::AddPhysical {
            name: name.into(),
            discipline: Discipline::Structure,
            start: PointInput {
                values: a,
                unit: LengthUnit::Metre,
            },
            end: PointInput {
                values: b,
                unit: LengthUnit::Metre,
            },
            role: PhysicalRole::VisualOnly,
            material_id: None,
            environment_id: None,
        },
        "teste",
    )
    .unwrap();
    project.revision += 1;
    project.history.push(ChangeRecord {
        revision: project.revision,
        actor: "teste".into(),
        operation: "edit_model".into(),
        timestamp_unix_ms: project.revision,
    });
    change.created_ids[0]
}

#[test]
fn selection_is_shared_by_views_and_reveals_hidden_other_floor_without_project_changes() {
    let mut project = project();
    let a = add(&mut project, "Térreo", [0., 0., 0.], [4., 0., 0.]);
    let b = add(
        &mut project,
        "Pavimento superior",
        [0., 0., 6.],
        [4., 0., 6.],
    );
    let original = serde_json::to_value(&project).unwrap();
    let mut state = ViewState::default();
    state
        .apply(
            &project,
            ViewCommand::Select {
                object_id: b,
                reveal: true,
            },
        )
        .unwrap();
    state.apply(&project, ViewCommand::HideSelected {}).unwrap();
    state
        .apply(
            &project,
            ViewCommand::SetCut {
                cut: Some(ViewCut {
                    axis: CutAxis::Z,
                    min_m: -0.2,
                    max_m: 1.0,
                }),
            },
        )
        .unwrap();
    assert!(!state.report(&project).unwrap().objects[1].visible);
    state
        .apply(
            &project,
            ViewCommand::Select {
                object_id: b,
                reveal: true,
            },
        )
        .unwrap();
    assert!(state.cut.is_none());
    assert!(!state.hidden_ids.contains(&b));
    for kind in [ViewKind::Plan, ViewKind::Section, ViewKind::ThreeD] {
        let frame = state.frame(&project, kind, 600., 400.).unwrap();
        assert!(
            frame
                .segments
                .iter()
                .any(|s| s.object_id == b && s.selected)
        );
        assert!(
            frame
                .segments
                .iter()
                .any(|s| s.object_id == a && !s.selected)
        );
    }
    assert_eq!(serde_json::to_value(&project).unwrap(), original);
}

#[test]
fn overlapping_objects_remain_distinct_candidates_and_keyboard_visits_hidden_objects() {
    let mut project = project();
    let lower = add(&mut project, "Baixo", [0., 0., 0.], [4., 0., 0.]);
    let upper = add(&mut project, "Alto", [0., 0., 3.], [4., 0., 3.]);
    let mut state = ViewState::default();
    state.apply(&project, ViewCommand::Fit {}).unwrap();
    let frame = state.frame(&project, ViewKind::Plan, 600., 400.).unwrap();
    let candidates = frame.pick_candidates([300., 200.], 7.).unwrap();
    assert_eq!(candidates, vec![upper, lower]);
    state
        .apply(
            &project,
            ViewCommand::Select {
                object_id: upper,
                reveal: true,
            },
        )
        .unwrap();
    state.apply(&project, ViewCommand::HideSelected {}).unwrap();
    state
        .apply(&project, ViewCommand::SelectNext { reverse: true })
        .unwrap();
    assert_eq!(state.selected_id, Some(lower));
    state
        .apply(&project, ViewCommand::SelectNext { reverse: false })
        .unwrap();
    assert_eq!(state.selected_id, Some(upper));
    assert!(state.report(&project).unwrap().objects[1].visible);
}

#[test]
fn projected_selection_retains_millimetres_at_remote_origin_and_display_scale() {
    let mut near = project();
    let id = add(&mut near, "Detalhe", [0., 0., 0.], [0.125, 0., 0.]);
    let mut far = near.clone();
    far.objects[0].start_m = [900_000_000., 900_000_000., 0.].try_into().unwrap();
    far.objects[0].end_m = [900_000_000.125, 900_000_000., 0.].try_into().unwrap();
    let mut local = ViewState::default();
    let mut remote = ViewState::default();
    local.apply(&near, ViewCommand::Fit {}).unwrap();
    remote.apply(&far, ViewCommand::Fit {}).unwrap();
    local
        .apply(&near, ViewCommand::Zoom { factor: 10. })
        .unwrap();
    remote
        .apply(&far, ViewCommand::Zoom { factor: 10. })
        .unwrap();
    for scale in [1., 1.25, 1.5, 2.] {
        remote
            .apply(
                &far,
                ViewCommand::SetPreferences {
                    scale,
                    high_contrast: true,
                },
            )
            .unwrap();
        for kind in [ViewKind::Plan, ViewKind::Section, ViewKind::ThreeD] {
            let a = local.frame(&near, kind, 600., 400.).unwrap();
            let b = remote.frame(&far, kind, 600., 400.).unwrap();
            for i in 0..2 {
                assert!((a.segments[0].start_px[i] - b.segments[0].start_px[i]).abs() < 1e-8);
                assert!((a.segments[0].end_px[i] - b.segments[0].end_px[i]).abs() < 1e-8);
            }
            assert_eq!(b.pick_candidates([300., 200.], 7.).unwrap(), vec![id]);
        }
    }
}

#[test]
fn cut_clips_crossing_element_and_failed_camera_commands_are_atomic() {
    let mut project = project();
    add(&mut project, "Pilar", [0., 0., 0.], [0., 0., 6.]);
    let mut state = ViewState::default();
    state.apply(&project, ViewCommand::Fit {}).unwrap();
    state
        .apply(
            &project,
            ViewCommand::SetCut {
                cut: Some(ViewCut {
                    axis: CutAxis::Z,
                    min_m: 2.,
                    max_m: 4.,
                }),
            },
        )
        .unwrap();
    let frame = state
        .frame(&project, ViewKind::Section, 400., 400.)
        .unwrap();
    let length = (frame.segments[0].end_px[1] - frame.segments[0].start_px[1]).abs()
        / frame.pixels_per_metre;
    assert!((length - 2.).abs() < 1e-12);
    let before = state.clone();
    for command in [
        ViewCommand::Zoom { factor: f64::NAN },
        ViewCommand::Pan {
            view: ViewKind::Plan,
            right_m: f64::INFINITY,
            up_m: 0.,
        },
        ViewCommand::SetPreferences {
            scale: 0.1,
            high_contrast: true,
        },
        ViewCommand::SetCamera {
            yaw_degrees: 900.,
            pitch_degrees: 0.,
        },
        ViewCommand::SetCut {
            cut: Some(ViewCut {
                axis: CutAxis::Z,
                min_m: 4.,
                max_m: 2.,
            }),
        },
    ] {
        assert!(state.apply(&project, command).is_err());
        assert_eq!(state, before);
    }
    assert!(
        state
            .frame(&project, ViewKind::Plan, f64::NAN, 100.)
            .is_err()
    );
}

#[test]
fn pan_axes_follow_each_view_without_rotating_or_mutating_the_project() {
    let project = project();
    let original = serde_json::to_value(&project).unwrap();
    for kind in [ViewKind::Plan, ViewKind::Section, ViewKind::ThreeD] {
        let mut state = ViewState::default();
        state.synchronize(&project);
        let before = state.project_point([2.0, 3.0, 4.0], kind);
        state
            .apply(
                &project,
                ViewCommand::Pan {
                    view: kind,
                    right_m: 1.25,
                    up_m: 2.5,
                },
            )
            .unwrap();
        let after = state.project_point([2.0, 3.0, 4.0], kind);
        assert!((after[0] - before[0] + 1.25).abs() < 1e-12);
        assert!((after[1] - before[1] + 2.5).abs() < 1e-12);
        assert!((after[2] - before[2]).abs() < 1e-12);
    }
    assert_eq!(serde_json::to_value(project).unwrap(), original);
}

#[test]
fn graphics_recovery_and_project_switch_preserve_data_and_do_not_leak_selection() {
    let mut project = project();
    let id = add(&mut project, "Viga", [0., 0., 0.], [3., 0., 0.]);
    let original = serde_json::to_value(&project).unwrap();
    let mut state = ViewState::default();
    state
        .apply(
            &project,
            ViewCommand::Select {
                object_id: id,
                reveal: true,
            },
        )
        .unwrap();
    state
        .apply(
            &project,
            ViewCommand::GraphicsUnavailable {
                reason: "Interrupção gráfica de teste".into(),
            },
        )
        .unwrap();
    assert_eq!(state.report(&project).unwrap().objects[0].id, id);
    state
        .apply(&project, ViewCommand::RestoreGraphics {})
        .unwrap();
    assert!(state.graphics_failure.is_none());
    assert_eq!(state.selected_id, Some(id));
    assert_eq!(serde_json::to_value(&project).unwrap(), original);
    let mut another = project.clone();
    another.id = Uuid::new_v4();
    assert!(state.report(&another).is_err());
    state.synchronize(&another);
    assert_eq!(state.selected_id, None);
    assert!(state.hidden_ids.is_empty());
}

#[test]
fn serialized_view_commands_keep_exponents_and_reject_unknown_fields() {
    let raw = r#"{"action":"zoom","factor":1.25e0}"#;
    let direct: ViewCommand = serde_json::from_str(raw).unwrap();
    let through_value: ViewCommand =
        serde_json::from_value(serde_json::from_str(raw).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(direct).unwrap(),
        serde_json::to_value(through_value).unwrap()
    );
    assert!(serde_json::from_str::<ViewCommand>(r#"{"action":"fit","shell":"x"}"#).is_err());
}
