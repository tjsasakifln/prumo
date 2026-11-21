//! Public metadata for the closed, shared `ModelEdit` command.
//! Validation is performed by the domain enum and service, never by this catalogue.
use prumo_domain::MAX_NAME_CHARACTERS;
use serde_json::{Value, json};

fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","additionalProperties":false,"properties":properties,"required":required})
}

fn tagged(tag: &str, value: &str, mut properties: Value, required: &[&str]) -> Value {
    properties[tag] = json!({"type":"string","const":value});
    let mut fields = vec![tag];
    fields.extend_from_slice(required);
    object(properties, &fields)
}

fn edit(name: &str, properties: Value, required: &[&str]) -> Value {
    tagged("edit", name, properties, required)
}

fn optional(schema: &Value) -> Value {
    json!({"oneOf":[{"type":"null"},schema]})
}

pub(super) fn edit_schema() -> Value {
    let name = json!({"type":"string","minLength":1,"maxLength":MAX_NAME_CHARACTERS});
    let id = json!({"type":"string","format":"uuid"});
    let vector = json!({"type":"array","minItems":3,"maxItems":3,"items":{"type":"number"}});
    let length_unit = json!({"type":"string","enum":["metre","centimetre","millimetre"]});
    let point = object(
        json!({"values":vector,"unit":length_unit}),
        &["values", "unit"],
    );
    let force = object(
        json!({"values":vector,"unit":{"type":"string","enum":["newton","kilonewton"]}}),
        &["values", "unit"],
    );
    let moment = object(
        json!({"values":vector,"unit":{"type":"string","enum":["newton_metre","kilonewton_metre"]}}),
        &["values", "unit"],
    );
    let pressure = object(
        json!({"value":{"type":"number","exclusiveMinimum":0},"unit":{"type":"string","enum":["pascal","kilopascal","megapascal","gigapascal"]}}),
        &["value", "unit"],
    );
    let density = object(
        json!({"value":{"type":"number","exclusiveMinimum":0},"unit":{"type":"string","const":"kilogram_per_cubic_metre"}}),
        &["value", "unit"],
    );
    let discipline = json!({"type":"string","enum":["structure","water_sanitation","gas","fire_protection","electrical","photovoltaic","lightning_protection","cabling","hvac"]});
    let role = json!({"type":"string","enum":["visual_only","analytical_source"]});
    let ids = json!({"type":"array","items":id,"minItems":1,"maxItems":prumo_domain::MAX_OBJECTS,"uniqueItems":true});
    let material = object(
        json!({
            "name":name,
            "family":{"type":"string","enum":["concrete","steel","timber","generic"]},
            "elastic_modulus":pressure,"density":density,
            "poisson_ratio":{"type":"number","exclusiveMinimum":-1,"exclusiveMaximum":0.5}
        }),
        &[
            "name",
            "family",
            "elastic_modulus",
            "density",
            "poisson_ratio",
        ],
    );
    let action_frame = json!({"oneOf":[
        tagged("kind","engineering",json!({}),&[]),
        tagged("kind","member_local",json!({"member_id":id}),&["member_id"])
    ]});
    let action = object(
        json!({"name":name,"application_node_id":id,"force":force,"couple":moment,"frame":action_frame}),
        &["name", "application_node_id", "force", "couple", "frame"],
    );
    let source = json!({"oneOf":[
        tagged("kind","native",json!({}),&[]),
        tagged("kind","imported",json!({"system":name,"reference":name,"external_id":name}),&["system","reference","external_id"]),
        tagged("kind","derived",json!({"operation":name}),&["operation"])
    ]});
    let rotation = json!({"type":"array","items":vector,"minItems":3,"maxItems":3});
    let engineering_frame = object(
        json!({"origin_world_m":vector,"rotation_to_world":rotation}),
        &["origin_world_m", "rotation_to_world"],
    );
    let georeference = object(
        json!({"crs_identifier":name,"map_origin_m":vector,"rotation_radians":{"type":"number"},"scale":{"type":"number","exclusiveMinimum":0}}),
        &[
            "crs_identifier",
            "map_origin_m",
            "rotation_radians",
            "scale",
        ],
    );
    let variants = vec![
        edit(
            "edit_precalculation",
            json!({"command":super::precalculation_schema::edit_schema()}),
            &["command"],
        ),
        edit(
            "edit_surfaces",
            json!({"command":super::surface_assembly_schema::surface_edit()}),
            &["command"],
        ),
        edit(
            "edit_assemblies",
            json!({"command":super::surface_assembly_schema::assembly_edit()}),
            &["command"],
        ),
        edit(
            "edit_framing",
            json!({"command":super::authoring_schema::framing_edit()}),
            &["command"],
        ),
        edit(
            "edit_layout",
            json!({"command":super::authoring_schema::layout_edit()}),
            &["command"],
        ),
        edit(
            "put_catalog_entry",
            json!({"entry":super::catalogue_schema::entry()}),
            &["entry"],
        ),
        edit(
            "assign_catalog_entry",
            json!({"object_id":id,"entry_id":id}),
            &["object_id", "entry_id"],
        ),
        edit(
            "clear_catalog_assignment",
            json!({"object_id":id,"kind":{"type":"string","enum":["material","section","properties"]}}),
            &["object_id", "kind"],
        ),
        edit(
            "set_geometry_tolerances",
            json!({"tolerances":tolerances_schema()}),
            &["tolerances"],
        ),
        edit(
            "apply_geometry_repair",
            json!({"plan_hash":{"type":"string","pattern":"^[a-f0-9]{64}$"},"repairs":repairs_schema()}),
            &["plan_hash", "repairs"],
        ),
        edit(
            "add_physical",
            json!({"name":name,"discipline":discipline,"start":point,"end":point,"role":role,"material_id":optional(&id),"environment_id":optional(&id)}),
            &["name", "discipline", "start", "end", "role"],
        ),
        edit(
            "set_physical_geometry",
            json!({"object_id":id,"start":point,"end":point}),
            &["object_id", "start", "end"],
        ),
        edit(
            "set_physical_role",
            json!({"object_id":id,"role":role}),
            &["object_id", "role"],
        ),
        edit(
            "set_physical_material",
            json!({"object_id":id,"material_id":optional(&id)}),
            &["object_id", "material_id"],
        ),
        edit(
            "add_analytical_node",
            json!({"name":name,"position":point}),
            &["name", "position"],
        ),
        edit(
            "add_analytical_member",
            json!({"name":name,"start_node_id":id,"end_node_id":id,"local_y":vector}),
            &["name", "start_node_id", "end_node_id", "local_y"],
        ),
        edit(
            "link_physical_analytical",
            json!({"physical_id":id,"analytical_id":id}),
            &["physical_id", "analytical_id"],
        ),
        edit(
            "unlink_physical_analytical",
            json!({"link_id":id}),
            &["link_id"],
        ),
        edit("add_material", json!({"material":material}), &["material"]),
        edit(
            "set_material",
            json!({"material_id":id,"material":material}),
            &["material_id", "material"],
        ),
        edit("add_nodal_action", json!({"action":action}), &["action"]),
        edit(
            "set_nodal_action",
            json!({"action_id":id,"action":action}),
            &["action_id", "action"],
        ),
        edit(
            "add_network",
            json!({"name":name,"discipline":discipline,"kind":{"type":"string","enum":["network","circuit"]},"environment_id":optional(&id)}),
            &["name", "discipline", "kind"],
        ),
        edit(
            "add_port",
            json!({"object_id":id,"discipline":discipline,"kind":{"type":"string","enum":["structural","hydraulic","gas","electrical","signal","thermal"]},"position":point}),
            &["object_id", "discipline", "kind", "position"],
        ),
        edit(
            "assign_port",
            json!({"network_id":id,"port_id":id}),
            &["network_id", "port_id"],
        ),
        edit(
            "connect_ports",
            json!({"network_id":id,"from_port":id,"to_port":id}),
            &["network_id", "from_port", "to_port"],
        ),
        edit(
            "disconnect_ports",
            json!({"connection_id":id}),
            &["connection_id"],
        ),
        edit(
            "copy_physical",
            json!({"source_id":id,"name":name,"translation":point}),
            &["source_id", "name", "translation"],
        ),
        edit(
            "merge_physical",
            json!({"source_ids":ids,"name":name}),
            &["source_ids", "name"],
        ),
        edit(
            "split_physical",
            json!({"object_id":id,"fraction":{"type":"number","exclusiveMinimum":0,"exclusiveMaximum":1},"names":{"type":"array","items":name,"minItems":2,"maxItems":2}}),
            &["object_id", "fraction", "names"],
        ),
        edit(
            "rigid_move",
            json!({"object_ids":ids,"rotation":rotation,"translation":point}),
            &["object_ids", "rotation", "translation"],
        ),
        edit("set_display_unit", json!({"unit":length_unit}), &["unit"]),
        edit(
            "set_source_origin",
            json!({"entity_id":id,"origin":source}),
            &["entity_id", "origin"],
        ),
        edit(
            "change_engineering_frame",
            json!({"frame":engineering_frame}),
            &["frame"],
        ),
        edit(
            "set_georeference",
            json!({"georeference":optional(&georeference)}),
            &["georeference"],
        ),
    ];
    json!({"oneOf":variants})
}

pub(super) fn tolerances_schema() -> Value {
    let length = json!({"type":"number","minimum":1e-12,"maximum":10});
    object(
        json!({"selection_m":length,"topology_m":length,"mesh_m":length,"analysis_m":length}),
        &["selection_m", "topology_m", "mesh_m", "analysis_m"],
    )
}
pub(super) fn repairs_schema() -> Value {
    let id = json!({"type":"string","format":"uuid"});
    let reason = json!({"type":"string","minLength":1,"maxLength":160});
    let point = json!({"type":"array","minItems":3,"maxItems":3,"items":{"type":"number"}});
    json!({"type":"array","minItems":1,"maxItems":128,"items":{"oneOf":[
        tagged("repair","snap_endpoint",json!({"object_id":id,"endpoint":{"type":"string","enum":["start","end"]},"target_m":point,"reason":reason}),&["object_id","endpoint","target_m","reason"]),
        tagged("repair","connect_ports",json!({"network_id":id,"from_port":id,"to_port":id,"reason":reason}),&["network_id","from_port","to_port","reason"])
    ]}})
}
