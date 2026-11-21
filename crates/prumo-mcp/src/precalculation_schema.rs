//! MCP shape for precalculation commands. Domain validation remains authoritative.
use serde_json::{Value, json};

fn record(fields: Value, required: &[&str]) -> Value {
    json!({"type":"object","additionalProperties":false,"properties":fields,"required":required})
}
fn edit(kind: &str, mut fields: Value, required: &[&str]) -> Value {
    fields["operation"] = json!({"type":"string","const":kind});
    let mut all = vec!["operation"];
    all.extend(required);
    record(fields, &all)
}
pub(super) fn edit_schema() -> Value {
    let id = json!({"type":"string","format":"uuid"});
    let ids = json!({"type":"array","minItems":1,"maxItems":500,"uniqueItems":true,"items":id});
    let number = json!({"type":"number"});
    let vector = json!({"type":"array","minItems":3,"maxItems":3,"items":number});
    let hash = json!({"type":"string","pattern":"^[0-9a-f]{64}$"});
    let name = json!({"type":"string","minLength":1,"maxLength":160});
    let optional_id = json!({"oneOf":[id,{"type":"null"}]});
    let freedom = json!({"oneOf":[
        edit("free",json!({}),&[]),
        edit("fixed",json!({}),&[]),
        edit("spring",json!({"stiffness_si":number}),&["stiffness_si"]),
        edit("compression_only",json!({"stiffness_si":number}),&["stiffness_si"])
    ]});
    // Freedom is tagged "kind"; replace only the tag name for this nested family.
    let freedom = {
        let mut f = freedom;
        for item in f["oneOf"].as_array_mut().expect("objects") {
            let value = item["properties"]["operation"]["const"].take();
            item["properties"]
                .as_object_mut()
                .expect("properties")
                .remove("operation");
            item["properties"]["kind"] = json!({"type":"string","const":value});
            for requirement in item["required"].as_array_mut().expect("required") {
                if requirement.as_str() == Some("operation") {
                    *requirement = json!("kind");
                }
            }
        }
        f
    };
    let degrees = json!({"type":"array","minItems":6,"maxItems":6,"items":freedom});
    let rotation = json!({"type":"array","minItems":3,"maxItems":3,"items":vector});
    let support = record(
        json!({"id":id,"node_id":id,"rotation_to_engineering":rotation,
        "freedoms":degrees,"justification":name}),
        &[
            "id",
            "node_id",
            "rotation_to_engineering",
            "freedoms",
            "justification",
        ],
    );
    let release = record(
        json!({"id":id,"member_id":id,"end":{"type":"string","enum":["start","end"]},
        "freedoms":degrees,"justification":name}),
        &["id", "member_id", "end", "freedoms", "justification"],
    );
    let sample = record(
        json!({"measured_a":number,"actual_a_m":number,
        "measured_b":number,"actual_b_m":number}),
        &["measured_a", "actual_a_m", "measured_b", "actual_b_m"],
    );
    let trace = record(
        json!({"id":id,"layer":name,"a":vector,"b":vector,"visible":{"type":"boolean"}}),
        &["id", "layer", "a", "b", "visible"],
    );
    let drawing = record(
        json!({"id":id,"label":name,
        "format":{"type":"string","enum":["dxf","pdf_vector","pdf_raster","image"]},
        "sha256":hash,"calibration":sample,"origin_m":vector,
        "traces":{"type":"array","maxItems":2000,"items":trace}}),
        &[
            "id",
            "label",
            "format",
            "sha256",
            "calibration",
            "origin_m",
            "traces",
        ],
    );
    let candidate = record(
        json!({"guid":name,"name":name,"entity_type":name,
        "start_m":vector,"end_m":vector}),
        &["guid", "name", "entity_type", "start_m", "end_m"],
    );
    let ifc = record(
        json!({"id":id,"label":name,"sha256":hash,
        "schema":{"type":"string","enum":["ifc2x3","ifc4","ifc4x3"]},
        "candidates":{"type":"array","maxItems":2000,"items":candidate},
        "unknown_entity_count":{"type":"integer","minimum":0,"maximum":100000}}),
        &[
            "id",
            "label",
            "sha256",
            "schema",
            "candidates",
            "unknown_entity_count",
        ],
    );
    let point = record(
        json!({"values":vector,"unit":{"type":"string","enum":["metre","centimetre","millimetre"]}}),
        &["values", "unit"],
    );
    let optional_sha = json!({"oneOf":[hash,{"type":"null"}]});
    json!({"oneOf":[
        edit("put_support",json!({"support":support}),&["support"]),
        edit("remove_support",json!({"id":id}),&["id"]),
        edit("put_release",json!({"release":release}),&["release"]),
        edit("remove_release",json!({"id":id}),&["id"]),
        edit("put_drawing",json!({"reference":drawing,"expected_sha256":optional_sha}),&["reference"]),
        edit("put_ifc",json!({"reference":ifc,"expected_sha256":optional_sha}),&["reference"]),
        edit("promote_drawing",json!({"reference_id":id,"trace_id":id,"name":name,
            "material_id":optional_id,"confirm_dimensions":{"type":"boolean"}}),
            &["reference_id","trace_id","name","confirm_dimensions"]),
        edit("promote_ifc",json!({"reference_id":id,"guid":name,
            "material_id":optional_id,"confirm_hypotheses":{"type":"boolean"}}),
            &["reference_id","guid","confirm_hypotheses"]),
        edit("copy_batch",json!({"object_ids":ids,"translation":point,"linked":{"type":"boolean"}}),
            &["object_ids","translation","linked"]),
        edit("set_material_batch",json!({"object_ids":ids,"material_id":optional_id}),
            &["object_ids","material_id"])
    ]})
}
