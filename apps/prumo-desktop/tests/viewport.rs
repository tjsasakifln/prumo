use egui_kittest::{Harness, kittest::Queryable};
use prumo_desktop::Desktop;
use prumo_domain::{
    Command, LengthUnit,
    view::{ViewCommand, ViewKind},
};
use prumo_ui::UiAction;

fn fixture(path: std::path::PathBuf) -> Desktop {
    let mut desktop = Desktop::new(path);
    desktop.dispatch(UiAction::command(Command::Create {
        name: "Fixture de vistas, sem cálculo".into(),
        units: LengthUnit::Metre,
    }));
    for (name, z) in [("Peça térrea", 0.), ("Peça elevada", 3.)] {
        desktop.dispatch(UiAction::command(Command::AddMember {
            expected_revision: desktop.project.as_ref().unwrap().revision,
            name: name.into(),
            start: [0., 0., z],
            end: [4., 0., z],
        }));
    }
    desktop
}

#[test]
fn keyboard_and_accessibility_tree_select_reveal_and_locate_same_identity() {
    let directory = tempfile::tempdir().unwrap();
    let desktop = fixture(directory.path().join("navigation.prumo"));
    let expected = desktop.project.as_ref().unwrap().objects[1].id;
    let initial = serde_json::to_value(&desktop.project).unwrap();
    let mut harness = Harness::builder()
        .with_size(eframe::egui::vec2(1600., 2600.))
        .build_ui_state(|ui, desktop| desktop.render(ui), desktop);
    harness.get_by_label("Árvore de objetos e seleção").focus();
    harness.run();
    harness.key_press(eframe::egui::Key::Enter);
    harness.run();
    harness.get_by_label("Próximo objeto").focus();
    harness.run();
    assert!(harness.get_by_label("Próximo objeto").is_focused());
    harness.key_press(eframe::egui::Key::Enter);
    harness.run();
    harness.key_press(eframe::egui::Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().view.viewport.state.selected_id,
        Some(expected)
    );
    harness.get_by_label("Ocultar seleção").focus();
    harness.run();
    harness.key_press(eframe::egui::Key::Enter);
    harness.run();
    assert!(
        harness
            .state()
            .view
            .viewport
            .state
            .hidden_ids
            .contains(&expected)
    );
    harness.get_by_label("Localizar seleção").focus();
    harness.run();
    harness.key_press(eframe::egui::Key::Enter);
    harness.run();
    assert!(
        !harness
            .state()
            .view
            .viewport
            .state
            .hidden_ids
            .contains(&expected)
    );
    for label in ["Planta XY", "Corte XZ", "Modelo 3D"] {
        harness.get_by_label(label).click();
        harness.run();
        assert_eq!(
            harness.state().view.viewport.state.selected_id,
            Some(expected)
        );
        assert!(
            harness
                .query_by_label("Selecionado: Peça elevada")
                .is_some()
        );
    }
    assert_eq!(
        serde_json::to_value(&harness.state().project).unwrap(),
        initial
    );
}

#[test]
fn canvas_focus_has_keyboard_pan_and_textual_alternative() {
    let directory = tempfile::tempdir().unwrap();
    let mut desktop = fixture(directory.path().join("focus.prumo"));
    desktop.dispatch(UiAction::View(ViewCommand::SetView {
        view: ViewKind::Plan,
    }));
    let mut harness = Harness::builder()
        .with_size(eframe::egui::vec2(1600., 2400.))
        .build_ui_state(|ui, desktop| desktop.render(ui), desktop);
    let label = "Vista Planta XY. Seleção e navegação também disponíveis na lista de objetos.";
    harness.get_by_label(label).focus();
    harness.run();
    assert!(harness.get_by_label(label).is_focused());
    let before = harness.state().view.viewport.state.center_m;
    harness.key_press(eframe::egui::Key::ArrowRight);
    harness.run();
    assert_eq!(
        harness.state().view.viewport.state.center_m[0],
        before[0] + 1.
    );
    harness.key_press(eframe::egui::Key::Tab);
    harness.run();
    assert!(!harness.get_by_label(label).is_focused());
    assert!(
        harness
            .query_by_label("Árvore de objetos e seleção")
            .is_some()
    );
}

#[test]
fn keyboard_pan_uses_focused_section_when_three_views_are_shown() {
    let directory = tempfile::tempdir().unwrap();
    let mut desktop = fixture(directory.path().join("three-views.prumo"));
    desktop.view.viewport.three_views = true;
    desktop.dispatch(UiAction::View(ViewCommand::SetView {
        view: ViewKind::ThreeD,
    }));
    let mut harness = Harness::builder()
        .with_size(eframe::egui::vec2(1600., 2800.))
        .build_ui_state(|ui, desktop| desktop.render(ui), desktop);
    harness
        .get_by_label("Vista Corte XZ. Seleção e navegação também disponíveis na lista de objetos.")
        .focus();
    harness.run();
    let before = harness.state().view.viewport.state.center_m;
    harness.key_press(eframe::egui::Key::ArrowUp);
    harness.run();
    assert_eq!(
        harness.state().view.viewport.state.center_m,
        [before[0], before[1], before[2] + 1.0]
    );
}

#[test]
fn high_contrast_scaling_and_graphics_recovery_keep_project_operable() {
    let directory = tempfile::tempdir().unwrap();
    let mut desktop = fixture(directory.path().join("recovery.prumo"));
    let id = desktop.project.as_ref().unwrap().objects[0].id;
    desktop.dispatch(UiAction::View(ViewCommand::Select {
        object_id: id,
        reveal: true,
    }));
    desktop.dispatch(UiAction::View(ViewCommand::SetPreferences {
        scale: 1.5,
        high_contrast: true,
    }));
    desktop.dispatch(UiAction::View(ViewCommand::GraphicsUnavailable {
        reason: "Interrupção de renderização injetada pelo ensaio".into(),
    }));
    let before = serde_json::to_value(&desktop.project).unwrap();
    let mut harness = Harness::builder()
        .with_size(eframe::egui::vec2(2400., 3000.))
        .build_ui_state(|ui, desktop| desktop.render(ui), desktop);
    assert!(
        harness
            .query_by_label("Vista gráfica indisponível")
            .is_some()
    );
    harness.get_by_label("Restaurar vista").click();
    harness.run();
    assert!(
        harness
            .state()
            .view
            .viewport
            .state
            .graphics_failure
            .is_none()
    );
    assert_eq!(harness.state().view.viewport.state.selected_id, Some(id));
    assert_eq!(
        serde_json::to_value(&harness.state().project).unwrap(),
        before
    );
    let frame = harness
        .state()
        .view
        .viewport
        .state
        .frame(
            harness.state().project.as_ref().unwrap(),
            ViewKind::ThreeD,
            700.,
            400.,
        )
        .unwrap();
    assert!(
        frame
            .segments
            .iter()
            .any(|s| s.selected && s.object_id == id)
    );
    assert!(harness.output().shapes.len() > 20);
}
