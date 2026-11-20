use super::{base_tool_catalog, definition};
use prumo_domain::transactions::MAX_TRANSACTION_OPERATIONS;
use rmcp::model::Tool;
use serde_json::{Value, json};

pub(super) fn tools(writable: bool) -> Vec<Tool> {
    let mut tools = vec![definition(
        "prumo.model.history",
        "Consulta comandos aplicados, autoria e opções atuais de desfazer ou refazer. O histórico permanece preservado.",
        json!({}),
        &[],
        true,
    )];
    if !writable {
        return tools;
    }
    let uuid = json!({"type":"string","format":"uuid"});
    let hash = json!({"type":"string","pattern":"^[a-f0-9]{64}$"});
    let revision = json!({"type":"integer","minimum":1});
    let mut commands = Vec::new();
    for tool in base_tool_catalog(true) {
        let operation = match tool.name.as_ref() {
            "prumo.model.add_member" => "add_member",
            "prumo.project.rename" => "rename_project",
            "prumo.model.add_environment" => "add_environment",
            "prumo.norms.register" => "register_norm_reference",
            "prumo.norms.pin" => "pin_normative_package",
            "prumo.norms.migrate" => "migrate_normative_package",
            "prumo.model.edit" => "edit_model",
            "prumo.model.inspect" => "inspect_model",
            _ => continue,
        };
        let mut schema = Value::Object((*tool.input_schema).clone());
        schema["properties"]["operation"] = json!({"const":operation});
        schema["required"]
            .as_array_mut()
            .expect("required fields")
            .push(json!("operation"));
        commands.push(schema);
    }
    let source = json!({
        "type":"object","additionalProperties":false,
        "properties":{"kind":{"type":"string","enum":["project_metadata","membership","physical","physical_geometry","analytical_node","analytical_member","link","material","action","environment","port","network","connection","coordinates","georeference","normative","geometry_settings","catalog_entry","layout","analytical_region"]},"id":uuid},
        "required":["kind","id"]
    });
    let precondition = json!({
        "type":"object","additionalProperties":false,
        "properties":{"source":source,"sha256":{"anyOf":[hash,{"type":"null"}]}},
        "required":["source","sha256"]
    });
    tools.push(definition("prumo.model.preview_change", "Valida um lote atômico e prepara exatamente seus efeitos, sem aplicar a alteração. A revisão deve ser a mesma em todos os comandos; a identidade vem do host.",
        json!({"project_id":uuid,"expected_revision":revision,"idempotency_key":uuid,"preconditions":{"type":"array","items":precondition},"operations":{"type":"array","minItems":1,"maxItems":MAX_TRANSACTION_OPERATIONS,"items":{"oneOf":commands}}}),
        &["project_id","expected_revision","idempotency_key","operations"], false));
    let mut apply = definition(
        "prumo.model.apply_change",
        "Aplica exatamente o plano preparado identificado por SHA-256. Repetição retorna o recibo original sem duplicar objetos; revisão ou autorização alterada provoca recusa.",
        json!({"plan_sha256":hash}),
        &["plan_sha256"],
        false,
    );
    if let Some(annotations) = &mut apply.annotations {
        annotations.idempotent_hint = Some(true);
    }
    tools.push(apply);
    for (name, description) in [
        (
            "prumo.model.undo",
            "Desfaz o último lote reversível por um novo comando auditado. Preserva identidade, unidades e histórico; recusa revisão desatualizada.",
        ),
        (
            "prumo.model.redo",
            "Refaz o lote anunciado pelo histórico, com nova revisão auditada e chave idempotente. Não restaura evidência obsoleta como atual.",
        ),
    ] {
        let mut tool = definition(
            name,
            description,
            json!({"project_id":uuid,"expected_revision":revision,"idempotency_key":uuid,"target_command_id":uuid}),
            &[
                "project_id",
                "expected_revision",
                "idempotency_key",
                "target_command_id",
            ],
            false,
        );
        if let Some(annotations) = &mut tool.annotations {
            annotations.idempotent_hint = Some(true);
        }
        tools.push(tool);
    }
    tools
}
