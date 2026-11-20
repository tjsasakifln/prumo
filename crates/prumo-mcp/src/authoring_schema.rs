//! Closed discovery schemas for navigation and dimensional authoring.
//! Runtime decoding still uses the very same Rust domain types as the UI.
use serde_json::{Value, json};

pub(super) fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","additionalProperties":false,"properties":properties,"required":required})
}
pub(super) fn tagged(tag: &str, name: &str, mut properties: Value, required: &[&str]) -> Value {
    properties[tag] = json!({"const":name,"type":"string"});
    let mut fields = vec![tag];
    fields.extend_from_slice(required);
    object(properties, &fields)
}
pub(super) fn id() -> Value {
    json!({"type":"string","format":"uuid"})
}
pub(super) fn number() -> Value {
    json!({"type":"number"})
}
pub(super) fn name() -> Value {
    json!({"type":"string","minLength":1,"maxLength":160})
}
pub(super) fn measure() -> Value {
    json!({"type":"string","minLength":1,"maxLength":256,"description":"Medida com unidade explícita m, cm ou mm; vírgula ou ponto decimal, sem milhares. Aceita +, -, *, / e parênteses com dimensões coerentes."})
}
pub(super) fn nullable(value: Value) -> Value {
    json!({"oneOf":[{"type":"null"},value]})
}
pub(super) fn vector() -> Value {
    json!({"type":"array","minItems":3,"maxItems":3,"items":number()})
}
pub(super) fn point() -> Value {
    object(
        json!({"values":vector(),"unit":{"type":"string","enum":["metre","centimetre","millimetre"]}}),
        &["values", "unit"],
    )
}
pub(super) fn view_command() -> Value {
    let unit = json!({"type":"boolean"});
    let cut = object(
        json!({"axis":{"type":"string","enum":["x","y","z"]},"min_m":number(),"max_m":number()}),
        &["axis", "min_m", "max_m"],
    );
    let mut variants = vec![
        tagged(
            "action",
            "select",
            json!({"object_id":id(),"reveal":unit}),
            &["object_id", "reveal"],
        ),
        tagged(
            "action",
            "select_next",
            json!({"reverse":unit}),
            &["reverse"],
        ),
        tagged(
            "action",
            "set_view",
            json!({"view":{"type":"string","enum":["plan","section","three_d"]}}),
            &["view"],
        ),
        tagged("action", "set_cut", json!({"cut":nullable(cut)}), &["cut"]),
        tagged(
            "action",
            "pan",
            json!({"view":{"type":"string","enum":["plan","section","three_d"]},"right_m":number(),"up_m":number()}),
            &["right_m", "up_m"],
        ),
        tagged(
            "action",
            "zoom",
            json!({"factor":{"type":"number","minimum":0.01,"maximum":100}}),
            &["factor"],
        ),
        tagged(
            "action",
            "set_camera",
            json!({"yaw_degrees":{"type":"number","minimum":-360,"maximum":360},"pitch_degrees":{"type":"number","minimum":-89,"maximum":89}}),
            &["yaw_degrees", "pitch_degrees"],
        ),
        tagged(
            "action",
            "set_preferences",
            json!({"scale":{"type":"number","minimum":0.75,"maximum":2},"high_contrast":unit}),
            &["scale", "high_contrast"],
        ),
        tagged(
            "action",
            "graphics_unavailable",
            json!({"reason":name()}),
            &["reason"],
        ),
    ];
    for action in [
        "clear_selection",
        "locate_selected",
        "hide_selected",
        "isolate_selected",
        "show_all",
        "fit",
        "restore_graphics",
    ] {
        variants.push(tagged("action", action, json!({}), &[]));
    }
    json!({"oneOf":variants})
}
pub(super) fn layout_point() -> Value {
    json!({"oneOf":[
        tagged("point","coordinates",json!({"position":point()}),&["position"]),
        tagged("point","local",json!({"frame_id":id(),"position":point()}),&["frame_id","position"]),
        tagged("point","axis_station",json!({"axis_id":id(),"station":measure(),"level_id":id(),"offset":measure()}),&["axis_id","station","level_id","offset"]),
        tagged("point","grid_intersection",json!({"first_axis_id":id(),"second_axis_id":id(),"level_id":id(),"offset":measure()}),&["first_axis_id","second_axis_id","level_id","offset"]),
        tagged("point","node",json!({"node_id":id()}),&["node_id"])
    ]})
}
pub(super) fn layout_edit() -> Value {
    let axis = object(
        json!({"name":name(),"origin":point(),"angle_degrees":number(),"offset":measure(),"extent":measure(),"frame_id":nullable(id())}),
        &["name", "origin", "angle_degrees", "offset", "extent"],
    );
    json!({"oneOf":[
        tagged("operation","put_level",json!({"id":id(),"name":name(),"elevation":measure()}),&["id","name","elevation"]),
        tagged("operation","put_axis",json!({"id":id(),"input":axis}),&["id","input"]),
        tagged("operation","put_frame",json!({"id":id(),"name":name(),"origin":point(),"angle_degrees":number()}),&["id","name","origin","angle_degrees"]),
        tagged("operation","bind_object",json!({"object_id":id(),"level_id":id(),"offset":measure()}),&["object_id","level_id","offset"]),
        tagged("operation","unbind_object",json!({"object_id":id()}),&["object_id"])
    ]})
}
pub(super) fn snap_request() -> Value {
    json!({"oneOf":[
        tagged("kind","endpoint",json!({"object_id":id(),"end":{"type":"boolean"}}),&["object_id","end"]),
        tagged("kind","alignment",json!({"axis_id":id(),"position":point()}),&["axis_id","position"]),
        tagged("kind","intersection",json!({"first_axis_id":id(),"second_axis_id":id(),"level_id":id(),"offset":measure()}),&["first_axis_id","second_axis_id","level_id","offset"]),
        tagged("kind","distance",json!({"from":layout_point(),"towards":layout_point(),"distance":measure()}),&["from","towards","distance"])
    ]})
}

pub(super) fn framing_edit() -> Value {
    let construction_point = layout_point();
    let path = json!({"oneOf":[
        tagged("kind","straight",json!({"start":construction_point,"end":construction_point}),&["start","end"]),
        tagged("kind","circular_arc",json!({"start":construction_point,"through":construction_point,"end":construction_point,"segments":{"type":"integer","minimum":2,"maximum":256}}),&["start","through","end","segments"])
    ]});
    let station = object(
        json!({"fraction":{"type":"number","minimum":0,"maximum":1},"section_id":id()}),
        &["fraction", "section_id"],
    );
    let input = object(
        json!({
            "name":name(),"kind":{"type":"string","enum":["column","beam","bar","truss_bar"]},
            "path":path,"material_id":id(),"sections":{"type":"array","minItems":1,"maxItems":64,"items":station},
            "variation":{"type":"string","enum":["constant","stepped","linear"]},
            "alignment":{"type":"string","enum":["centroid","section_origin"]},
            "eccentricity":point(),"roll_degrees":{"type":"number","minimum":-360,"maximum":360},
            "connections":{"type":"array","minItems":2,"maxItems":2,"items":nullable(id())}
        }),
        &[
            "name",
            "kind",
            "path",
            "material_id",
            "sections",
            "variation",
            "alignment",
            "eccentricity",
            "roll_degrees",
            "connections",
        ],
    );
    json!({"oneOf":[
        tagged("operation","put_member",json!({"id":id(),"input":input}),&["id","input"]),
        tagged("operation","create_truss",json!({"id":id(),"name":name(),
            "points":{"type":"array","minItems":2,"maxItems":512,"items":construction_point},
            "edges":{"type":"array","minItems":1,"maxItems":1024,"items":{"type":"array","minItems":2,"maxItems":2,"items":{"type":"integer","minimum":0}}},
            "material_id":id(),"section_id":id()}),&["id","name","points","edges","material_id","section_id"])
    ]})
}
