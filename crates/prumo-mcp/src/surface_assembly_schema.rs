//! Typed surface and assembly operations exposed through the common model editor.
use super::authoring_schema::{id, name, nullable, number, object, tagged, vector};
use serde_json::{Value, json};

fn array(items: Value, min: usize, max: usize) -> Value {
    json!({"type":"array","minItems":min,"maxItems":max,"items":items})
}
fn local_point() -> Value {
    object(json!({"x_m":number(),"y_m":number()}), &["x_m", "y_m"])
}
fn outline() -> Value {
    array(local_point(), 3, 512)
}
fn frame() -> Value {
    object(
        json!({"origin_m":vector(),"u":vector(),"v":vector()}),
        &["origin_m", "u", "v"],
    )
}
fn role() -> Value {
    json!({"type":"string","enum":["visual_only","analytical_source"]})
}
fn opening() -> Value {
    object(
        json!({"name":name(),"outline":outline()}),
        &["name", "outline"],
    )
}
fn surface_support() -> Value {
    object(
        json!({"object_id":id(),"point":local_point()}),
        &["object_id", "point"],
    )
}
fn edge_condition() -> Value {
    let behavior = json!({"oneOf":[
        tagged("kind","free",json!({}),&[]),
        tagged("kind","candidate_support",json!({"object_id":id()}),&["object_id"])
    ]});
    object(
        json!({"edge":{"type":"integer","minimum":0},"behavior":behavior}),
        &["edge", "behavior"],
    )
}

pub(super) fn surface_spec() -> Value {
    let kind = json!({"oneOf":[
        tagged("kind","solid_slab",json!({}),&[]),
        tagged("kind","panel",json!({}),&[]),
        tagged("kind","cantilever",json!({}),&[]),
        tagged("kind","ribbed",json!({"rib_width_m":number(),"spacing_x_m":number(),"spacing_y_m":nullable(number()),"extra_depth_m":number()}),&["rib_width_m","spacing_x_m","spacing_y_m","extra_depth_m"]),
        tagged("kind","grillage",json!({"rib_width_m":number(),"spacing_x_m":number(),"spacing_y_m":number(),"depth_m":number()}),&["rib_width_m","spacing_x_m","spacing_y_m","depth_m"])
    ]});
    let region = object(
        json!({"name":name(),"outline":outline(),"thickness_m":number(),"offset_m":number()}),
        &["name", "outline", "thickness_m", "offset_m"],
    );
    object(
        json!({"name":name(),"role":role(),"material_id":nullable(id()),"environment_id":nullable(id()),
        "frame":frame(),"kind":kind,"outline":outline(),"openings":array(opening(),0,256),
        "thickness_m":number(),"regions":array(region,0,256),"supports":array(surface_support(),0,256),"edge_conditions":array(edge_condition(),0,512)}),
        &[
            "name",
            "role",
            "frame",
            "kind",
            "outline",
            "openings",
            "thickness_m",
            "regions",
            "supports",
        ],
    )
}
pub(super) fn surface_edit() -> Value {
    json!({"oneOf":[
        tagged("action","create",json!({"surface":surface_spec()}),&["surface"]),
        tagged("action","update",json!({"object_id":id(),"surface":surface_spec()}),&["object_id","surface"]),
        tagged("action","set_openings",json!({"object_id":id(),"openings":array(opening(),0,256)}),&["object_id","openings"]),
        tagged("action","set_supports",json!({"object_id":id(),"supports":array(surface_support(),0,256)}),&["object_id","supports"])
    ]})
}

fn wall_panel() -> Value {
    object(
        json!({"name":name(),"frame":frame(),"outline":outline(),"openings":array(opening(),0,256),"thickness_m":number()}),
        &["name", "frame", "outline", "openings", "thickness_m"],
    )
}
fn flight() -> Value {
    object(
        json!({"name":name(),"start_m":vector(),"end_m":vector(),"width_m":number(),"thickness_m":number(),"steps":{"type":"integer","minimum":0,"maximum":128}}),
        &[
            "name",
            "start_m",
            "end_m",
            "width_m",
            "thickness_m",
            "steps",
        ],
    )
}
fn landing() -> Value {
    object(
        json!({"name":name(),"frame":frame(),"outline":outline(),"thickness_m":number()}),
        &["name", "frame", "outline", "thickness_m"],
    )
}
fn assembly_support() -> Value {
    object(
        json!({"part":{"type":"integer","minimum":0},"object_id":id(),"point":local_point(),"kind":{"type":"string","enum":["bearing","fixed","sliding"]}}),
        &["part", "object_id", "point", "kind"],
    )
}
fn continuity() -> Value {
    let index = json!({"type":"integer","minimum":0});
    object(
        json!({"from_part":index,"from_edge":index,"to_part":index,"to_edge":index}),
        &["from_part", "from_edge", "to_part", "to_edge"],
    )
}
pub(super) fn assembly_spec() -> Value {
    let coupling = object(
        json!({"name":name(),"start_m":vector(),"end_m":vector(),"height_m":number(),"thickness_m":number()}),
        &["name", "start_m", "end_m", "height_m", "thickness_m"],
    );
    let shape = json!({"oneOf":[
        tagged("kind","wall",json!({"panel":wall_panel()}),&["panel"]),
        tagged("kind","core",json!({"panels":array(wall_panel(),2,256),"coupling_beams":array(coupling,0,256)}),&["panels","coupling_beams"]),
        tagged("kind","stair",json!({"form":{"type":"string","enum":["conventional","folded","self_supporting"]},"flights":array(flight(),1,256),"landings":array(landing(),0,256)}),&["form","flights","landings"]),
        tagged("kind","ramp",json!({"flights":array(flight(),1,256),"landings":array(landing(),0,256)}),&["flights","landings"])
    ]});
    object(
        json!({"name":name(),"role":role(),"material_id":nullable(id()),"environment_id":nullable(id()),"shape":shape,
        "supports":array(assembly_support(),0,1024),"continuities":array(continuity(),0,1024)}),
        &["name", "role", "shape", "supports", "continuities"],
    )
}
pub(super) fn assembly_edit() -> Value {
    json!({"oneOf":[
        tagged("action","create",json!({"assembly":assembly_spec()}),&["assembly"]),
        tagged("action","update",json!({"object_id":id(),"assembly":assembly_spec()}),&["object_id","assembly"]),
        tagged("action","adopt",json!({"assembly":assembly_spec(),"part_ids":array(id(),1,256)}),&["assembly","part_ids"]),
        tagged("action","promote",json!({"object_id":id(),"material_id":id()}),&["object_id","material_id"]),
        tagged("action","set_supports",json!({"object_id":id(),"supports":array(assembly_support(),0,1024)}),&["object_id","supports"]),
        tagged("action","set_continuities",json!({"object_id":id(),"continuities":array(continuity(),0,1024)}),&["object_id","continuities"])
    ]})
}
