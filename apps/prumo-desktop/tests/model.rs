//! Real accesskit events drive typed forms; persistence is reopened independently.
use eframe::egui::{Key, Modifiers, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use prumo_desktop::Desktop;
use prumo_domain::dependencies::ResultState;
use prumo_domain::model::PhysicalRole;
use std::path::Path;

fn harness(path: &Path) -> Harness<'static, Desktop> {
    Harness::builder()
        .with_size(vec2(1400.0, 2400.0))
        .build_ui_state(
            |ui, desktop| desktop.render(ui),
            Desktop::new(path.to_path_buf()),
        )
}

fn click(harness: &mut Harness<'_, Desktop>, label: &str) {
    harness.get_by_label(label).scroll_to_me();
    harness.run();
    harness.get_by_label(label).click();
    harness.run();
    assert!(
        !harness.state().view.is_error,
        "{}",
        harness.state().view.notice
    );
}

fn text(harness: &mut Harness<'_, Desktop>, label: &str, value: &str) {
    harness.get_by_label(label).click();
    harness.run();
    harness.key_press_modifiers(
        Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
        Key::A,
    );
    harness.run();
    harness.get_by_label(label).type_text(value);
    harness.run();
}

fn number(harness: &mut Harness<'_, Desktop>, label: &str, value: f64) {
    text(harness, label, &value.to_string());
}

fn choose(harness: &mut Harness<'_, Desktop>, label: &str, option: &str) {
    click(harness, label);
    click(harness, option);
}

fn named(name: &str, id: impl std::fmt::Display) -> String {
    format!(
        "{name} ({})",
        id.to_string().chars().take(8).collect::<String>()
    )
}

fn open_editor(harness: &mut Harness<'_, Desktop>) {
    click(harness, "Criar projeto");
    click(harness, "Modelo físico e analítico");
}

#[test]
fn real_forms_preserve_visual_role_links_materials_and_typed_actions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("semantic-ui.prumo");
    let mut ui = harness(&path);
    open_editor(&mut ui);
    text(&mut ui, "Nome do objeto físico", "Referência visual");
    click(&mut ui, "Criar objeto físico");
    let visual = ui.state().project.as_ref().unwrap().objects[0].clone();
    assert_eq!(visual.role, PhysicalRole::VisualOnly);
    assert!(
        ui.state()
            .project
            .as_ref()
            .unwrap()
            .model
            .analytical_members
            .is_empty()
    );
    assert!(ui.state().project.as_ref().unwrap().model.links.is_empty());

    choose(&mut ui, "Peça para editar", &named(&visual.name, visual.id));
    choose(&mut ui, "Função do objeto", "Origem para análise");
    click(&mut ui, "Salvar função da peça");
    number(&mut ui, "Fim físico X", 5.0);
    click(&mut ui, "Salvar geometria da peça");
    assert_eq!(
        ui.state().project.as_ref().unwrap().objects[0].length_m(),
        5.0
    );

    click(&mut ui, "Materiais");
    text(&mut ui, "Nome do material", "Material UI");
    choose(&mut ui, "Unidade do módulo", "GPa");
    number(&mut ui, "Módulo de elasticidade", 25.0);
    click(&mut ui, "Cadastrar material");
    let material = ui.state().project.as_ref().unwrap().model.materials[0].clone();
    assert_eq!(material.elastic_modulus.si(), 25.0e9);
    choose(
        &mut ui,
        "Material para revisar",
        &named(&material.name, material.id),
    );
    number(&mut ui, "Coeficiente de Poisson", 0.25);
    click(&mut ui, "Revisar material selecionado");
    click(&mut ui, "Peças");
    choose(
        &mut ui,
        "Material da peça",
        &named(&material.name, material.id),
    );
    click(&mut ui, "Aplicar material à peça");

    click(&mut ui, "Analítico");
    text(&mut ui, "Nome do nó", "Nó A");
    click(&mut ui, "Criar nó analítico");
    text(&mut ui, "Nome do nó", "Nó B");
    choose(&mut ui, "Unidade do nó", "cm");
    number(&mut ui, "Posição do nó X", 500.0);
    click(&mut ui, "Criar nó analítico");
    let nodes = ui
        .state()
        .project
        .as_ref()
        .unwrap()
        .model
        .analytical_nodes
        .clone();
    assert_eq!(nodes[1].position_m.as_array(), [5.0, 0.0, 0.0]);
    choose(&mut ui, "Nó inicial", &named("Nó A", nodes[0].id));
    choose(&mut ui, "Nó final", &named("Nó B", nodes[1].id));
    click(&mut ui, "Criar barra analítica");
    let member = ui
        .state()
        .project
        .as_ref()
        .unwrap()
        .model
        .analytical_members[0]
        .clone();
    choose(&mut ui, "Peça do vínculo", &named(&visual.name, visual.id));
    choose(&mut ui, "Barra do vínculo", &named(&member.name, member.id));
    click(&mut ui, "Vincular peça e barra");

    click(&mut ui, "Ações");
    text(&mut ui, "Nome da ação", "Força e momento UI");
    choose(&mut ui, "Nó de aplicação", &named("Nó B", nodes[1].id));
    number(&mut ui, "Força Z", -2.0);
    number(&mut ui, "Momento Y", 3.0);
    click(&mut ui, "Adicionar ação nodal");
    let action = ui.state().project.as_ref().unwrap().model.actions[0].clone();
    assert_eq!(action.force.si(), [0.0, 0.0, -2000.0]);
    assert_eq!(action.couple.si(), [0.0, 3000.0, 0.0]);
    choose(
        &mut ui,
        "Ação para revisar",
        &named(&action.name, action.id),
    );
    number(&mut ui, "Força Z", -2500.0);
    click(&mut ui, "Revisar ação selecionada");
    let before = ui.state().project.as_ref().unwrap().clone();
    drop(ui);
    let reopened = Desktop::new(path);
    let after = reopened.project.unwrap();
    assert_eq!(before, after);
    assert_eq!(after.model.links.len(), 1);
    assert_eq!(after.objects[0].material_id, Some(material.id));
    assert_eq!(after.model.actions[0].force.si()[2], -2500.0);
    assert!(after.model.identities[&visual.id].ifc_guid.is_some());
}

#[test]
fn real_forms_copy_split_merge_move_and_preserve_engineering_world_coordinates() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("transform-ui.prumo");
    let mut ui = harness(&path);
    open_editor(&mut ui);
    text(&mut ui, "Nome do objeto físico", "Peça original");
    click(&mut ui, "Criar objeto físico");
    let original = ui.state().project.as_ref().unwrap().objects[0].clone();
    click(&mut ui, "Cópia e transformação");
    choose(
        &mut ui,
        "Peça de origem",
        &named(&original.name, original.id),
    );
    number(&mut ui, "Translação Y", 2.0);
    click(&mut ui, "Copiar peça física");
    let copied = ui.state().project.as_ref().unwrap().objects[1].clone();
    assert_eq!(copied.start_m.as_array(), [0.0, 2.0, 0.0]);
    assert_ne!(
        ui.state().project.as_ref().unwrap().model.identities[&copied.id].ifc_guid,
        ui.state().project.as_ref().unwrap().model.identities[&original.id].ifc_guid
    );
    click(&mut ui, "Dividir peça selecionada");
    let pieces = ui.state().project.as_ref().unwrap().objects.clone();
    let first = pieces
        .iter()
        .find(|piece| piece.name == "Trecho A")
        .unwrap();
    let second = pieces
        .iter()
        .find(|piece| piece.name == "Trecho B")
        .unwrap();
    click(
        &mut ui,
        &format!("Selecionar {}", named(&first.name, first.id)),
    );
    click(
        &mut ui,
        &format!("Selecionar {}", named(&second.name, second.id)),
    );
    click(&mut ui, "Fundir peças selecionadas");
    let merged = ui
        .state()
        .project
        .as_ref()
        .unwrap()
        .objects
        .iter()
        .find(|piece| piece.name == "Peça reunida")
        .unwrap()
        .clone();
    assert_eq!(merged.length_m(), 4.0);
    click(
        &mut ui,
        &format!("Selecionar {}", named(&merged.name, merged.id)),
    );
    number(&mut ui, "Rotação em graus Z", 90.0);
    number(&mut ui, "Translação X", 10.0);
    number(&mut ui, "Translação Y", 0.0);
    click(&mut ui, "Aplicar transformação rígida");
    let moved = ui
        .state()
        .project
        .as_ref()
        .unwrap()
        .objects
        .iter()
        .find(|piece| piece.id == merged.id)
        .unwrap();
    assert!((moved.end_m[0] - 10.0).abs() < 1e-12);
    assert!((moved.end_m[1] - 4.0).abs() < 1e-12);
    click(&mut ui, "Origem e unidades");
    choose(&mut ui, "Nova unidade de apresentação", "mm");
    click(&mut ui, "Alterar unidade de apresentação");
    number(&mut ui, "Nova origem mundial (m) X", 100.0);
    number(&mut ui, "Ângulos da referência (graus) Z", 90.0);
    click(&mut ui, "Preencher matriz com os ângulos");
    click(&mut ui, "Aplicar referência de engenharia");
    let project = ui.state().project.as_ref().unwrap();
    let frame = &project.model.engineering_frame;
    let moved = project
        .objects
        .iter()
        .find(|piece| piece.id == merged.id)
        .unwrap();
    let world_x = frame.origin_world_m[0]
        + frame.rotation_to_world[0]
            .iter()
            .zip(moved.end_m.as_array())
            .map(|(a, b)| a * b)
            .sum::<f64>();
    let world_y = frame.origin_world_m[1]
        + frame.rotation_to_world[1]
            .iter()
            .zip(moved.end_m.as_array())
            .map(|(a, b)| a * b)
            .sum::<f64>();
    assert!((world_x - 10.0).abs() < 1e-10);
    assert!((world_y - 4.0).abs() < 1e-10);
    text(&mut ui, "Identificador do sistema de mapa", "EPSG:31983");
    number(&mut ui, "Origem no mapa (m) X", 500_000.0);
    click(&mut ui, "Salvar georreferenciamento");
    choose(
        &mut ui,
        "Entidade de origem",
        &named(&merged.name, merged.id),
    );
    choose(&mut ui, "Procedência", "Importada");
    text(&mut ui, "Sistema de origem", "Fixture independente");
    text(&mut ui, "Referência de origem", "Modelo IFC de ensaio");
    text(&mut ui, "Identificador externo", "elemento-21");
    click(&mut ui, "Registrar procedência");
    let before = ui.state().project.as_ref().unwrap().clone();
    assert_eq!(before.model.lineage.len(), 3);
    drop(ui);
    assert_eq!(Desktop::new(path).project.unwrap(), before);
}

#[test]
fn real_forms_require_explicit_network_assignment_and_connections() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("networks-ui.prumo");
    let mut ui = harness(&path);
    open_editor(&mut ui);
    text(&mut ui, "Nome do objeto físico", "Tubo A");
    choose(&mut ui, "Disciplina do objeto", "Hidrossanitário");
    click(&mut ui, "Criar objeto físico");
    text(&mut ui, "Nome do objeto físico", "Tubo B");
    click(&mut ui, "Criar objeto físico");
    let objects = ui.state().project.as_ref().unwrap().objects.clone();
    assert!(ui.state().project.as_ref().unwrap().connections.is_empty());
    click(&mut ui, "Redes e portas");
    text(&mut ui, "Nome da rede ou circuito", "Água de teste");
    click(&mut ui, "Criar rede ou circuito");
    choose(
        &mut ui,
        "Objeto da nova porta",
        &named(&objects[0].name, objects[0].id),
    );
    click(&mut ui, "Criar porta no objeto");
    choose(
        &mut ui,
        "Objeto da nova porta",
        &named(&objects[1].name, objects[1].id),
    );
    click(&mut ui, "Criar porta no objeto");
    let project = ui.state().project.as_ref().unwrap();
    let network = project.networks[0].clone();
    let ports = project.ports.clone();
    assert_eq!(ports[0].position_m, ports[1].position_m);
    assert!(project.connections.is_empty());
    choose(
        &mut ui,
        "Rede para conectar",
        &named(&network.name, network.id),
    );
    for (object, port) in objects.iter().zip(&ports) {
        choose(
            &mut ui,
            "Porta de origem",
            &format!(
                "{} | Hidráulica ({})",
                object.name,
                port.id.to_string().chars().take(8).collect::<String>()
            ),
        );
        click(&mut ui, "Atribuir porta à rede");
    }
    choose(
        &mut ui,
        "Porta de origem",
        &format!(
            "Tubo A | Hidráulica ({})",
            ports[0].id.to_string().chars().take(8).collect::<String>()
        ),
    );
    choose(
        &mut ui,
        "Porta de destino",
        &format!(
            "Tubo B | Hidráulica ({})",
            ports[1].id.to_string().chars().take(8).collect::<String>()
        ),
    );
    click(&mut ui, "Conectar portas explicitamente");
    let connection = ui.state().project.as_ref().unwrap().connections[0].clone();
    choose(
        &mut ui,
        "Conexão para desfazer",
        &format!(
            "Água de teste: {} com {} ({})",
            ports[0].id.to_string().chars().take(8).collect::<String>(),
            ports[1].id.to_string().chars().take(8).collect::<String>(),
            connection
                .id
                .to_string()
                .chars()
                .take(8)
                .collect::<String>()
        ),
    );
    click(&mut ui, "Desconectar portas");
    let before = ui.state().project.as_ref().unwrap().clone();
    assert!(before.connections.is_empty());
    assert_eq!(before.networks[0].port_ids.len(), 2);
    drop(ui);
    assert_eq!(Desktop::new(path).project.unwrap(), before);
}

#[test]
fn real_forms_show_snapshot_and_trace_geometry_invalidation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("traces-ui.prumo");
    let mut ui = harness(&path);
    open_editor(&mut ui);
    click(&mut ui, "Criar objeto físico");
    let object = ui.state().project.as_ref().unwrap().objects[0].clone();
    click(&mut ui, "Medidas e dependências");
    click(&mut ui, "Atualizar medidas do modelo");
    click(&mut ui, "Conferir snapshot e dependências");
    let snapshot = ui
        .state()
        .view
        .model
        .trace
        .snapshot
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(
        snapshot.source_revision,
        ui.state().project.as_ref().unwrap().revision
    );
    assert!(
        ui.state()
            .project
            .as_ref()
            .unwrap()
            .dependencies
            .results
            .iter()
            .all(|result| result.state == ResultState::Current && !result.engineering_release)
    );
    click(&mut ui, "Peças");
    choose(&mut ui, "Peça para editar", &named(&object.name, object.id));
    number(&mut ui, "Fim físico X", 6.0);
    click(&mut ui, "Salvar geometria da peça");
    assert!(ui.state().view.model.trace.snapshot.is_none());
    click(&mut ui, "Medidas e dependências");
    click(&mut ui, "Conferir snapshot e dependências");
    assert_ne!(
        ui.state()
            .view
            .model
            .trace
            .snapshot
            .as_ref()
            .unwrap()
            .sha256,
        snapshot.sha256
    );
    let graph = &ui.state().project.as_ref().unwrap().dependencies;
    assert!(
        graph
            .results
            .iter()
            .any(|result| result.state == ResultState::Stale)
    );
    assert!(!graph.invalidations.is_empty());
    click(&mut ui, "Causas de invalidação");
    let before = ui.state().project.as_ref().unwrap().clone();
    drop(ui);
    assert_eq!(Desktop::new(path).project.unwrap(), before);
}

#[test]
fn actual_numeric_widgets_accept_decimal_comma_and_block_ambiguous_input() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("decimal-ui.prumo");
    let mut ui = harness(&path);
    open_editor(&mut ui);
    text(&mut ui, "Fim físico X", "1,5");
    click(&mut ui, "Criar objeto físico");
    assert_eq!(
        ui.state().project.as_ref().unwrap().objects[0].length_m(),
        1.5
    );
    let before = ui.state().project.as_ref().unwrap().clone();
    let bytes = std::fs::read(&path).unwrap();
    for invalid in ["1 2", "1.234,56", "NaN", "1e999", "1e-9999"] {
        text(&mut ui, "Fim físico X", invalid);
        click(&mut ui, "Criar objeto físico");
        assert_eq!(ui.state().project.as_ref().unwrap(), &before);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(
            ui.query_by_label("Corrija os campos numéricos antes de salvar: Fim físico X.")
                .is_some()
        );
    }
    text(&mut ui, "Fim físico X", "2.5");
    click(&mut ui, "Criar objeto físico");
    assert_eq!(
        ui.state().project.as_ref().unwrap().objects[1].length_m(),
        2.5
    );
    click(&mut ui, "Modelo físico e analítico");
    text(&mut ui, "Fim X", "3,5");
    click(&mut ui, "Adicionar peça e salvar");
    assert_eq!(
        ui.state().project.as_ref().unwrap().objects[2].length_m(),
        3.5
    );
    text(&mut ui, "Fim X", "1 2");
    ui.get_by_label("Adicionar peça e salvar").click();
    ui.run();
    assert_eq!(ui.state().project.as_ref().unwrap().objects.len(), 3);
}

#[test]
fn numeric_edit_buffer_follows_selected_entity_even_when_values_match() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("numeric-scope.prumo");
    let mut ui = harness(&path);
    open_editor(&mut ui);
    text(&mut ui, "Nome do objeto físico", "Peça A");
    click(&mut ui, "Criar objeto físico");
    text(&mut ui, "Nome do objeto físico", "Peça B");
    click(&mut ui, "Criar objeto físico");
    let objects = ui.state().project.as_ref().unwrap().objects.clone();
    choose(
        &mut ui,
        "Peça para editar",
        &named(&objects[0].name, objects[0].id),
    );
    text(&mut ui, "Fim físico X", "1 2");
    choose(
        &mut ui,
        "Peça para editar",
        &named(&objects[1].name, objects[1].id),
    );
    assert_eq!(
        ui.get_by_label("Fim físico X").value().as_deref(),
        Some("4")
    );
    assert!(
        ui.query_by_label("Corrija os campos numéricos antes de salvar: Fim físico X.")
            .is_none()
    );
    click(&mut ui, "Salvar geometria da peça");
    assert_eq!(
        ui.state().project.as_ref().unwrap().objects[1].length_m(),
        4.0
    );
}

#[test]
fn selected_forms_reload_new_revision_before_saving_or_after_transforming() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("reload-selected.prumo");
    let mut ui = harness(&path);
    open_editor(&mut ui);
    click(&mut ui, "Criar objeto físico");
    let object = ui.state().project.as_ref().unwrap().objects[0].clone();
    choose(&mut ui, "Peça para editar", &named(&object.name, object.id));
    assert_eq!(
        ui.get_by_label("Fim físico X").value().as_deref(),
        Some("4")
    );
    let service =
        prumo_app::ProjectService::open(&path, prumo_app::AccessMode::ReadWrite, "external-editor")
            .unwrap();
    service
        .execute(prumo_domain::Command::EditModel {
            expected_revision: ui.state().project.as_ref().unwrap().revision,
            edit: prumo_domain::model::ModelEdit::SetPhysicalGeometry {
                object_id: object.id,
                start: prumo_domain::model::PointInput {
                    values: [0.0; 3],
                    unit: prumo_domain::LengthUnit::Metre,
                },
                end: prumo_domain::model::PointInput {
                    values: [6.0, 0.0, 0.0],
                    unit: prumo_domain::LengthUnit::Metre,
                },
            },
        })
        .unwrap();
    click(&mut ui, "Reabrir / atualizar");
    assert_eq!(
        ui.get_by_label("Fim físico X").value().as_deref(),
        Some("6")
    );
    click(&mut ui, "Salvar geometria da peça");
    assert_eq!(service.read().unwrap().objects[0].length_m(), 6.0);
    click(&mut ui, "Cópia e transformação");
    click(
        &mut ui,
        &format!("Selecionar {}", named(&object.name, object.id)),
    );
    number(&mut ui, "Translação X", 2.0);
    number(&mut ui, "Translação Y", 0.0);
    click(&mut ui, "Aplicar transformação rígida");
    click(&mut ui, "Peças");
    assert_eq!(
        ui.get_by_label("Início físico X").value().as_deref(),
        Some("2")
    );
    assert_eq!(
        ui.get_by_label("Fim físico X").value().as_deref(),
        Some("8")
    );
    click(&mut ui, "Salvar geometria da peça");
    assert_eq!(
        service.read().unwrap().objects[0].start_m.as_array(),
        [2.0, 0.0, 0.0]
    );
    assert_eq!(
        service.read().unwrap().objects[0].end_m.as_array(),
        [8.0, 0.0, 0.0]
    );
}
