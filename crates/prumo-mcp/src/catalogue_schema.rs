//! Public metadata for the domain catalogue. The Rust domain remains authoritative.
use serde_json::{Value, json};

fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","additionalProperties":false,"properties":properties,"required":required})
}
fn tagged(kind: &str, mut properties: Value, required: &[&str]) -> Value {
    properties["kind"] = json!({"const":kind,"type":"string"});
    let mut fields = vec!["kind"];
    fields.extend_from_slice(required);
    object(properties, &fields)
}
fn quantity() -> Value {
    json!({"type":"string","enum":["dimensionless","length","area","second_moment_of_area","elastic_modulus","stress","strength","density","volume_flow","absolute_pressure","gauge_pressure","voltage","current","power","energy","absolute_temperature","temperature_difference","thermal_conductivity","heat_capacity"]})
}
fn value() -> Value {
    object(
        json!({
            "quantity":quantity(),"decimal":{"type":"string","minLength":1,"maxLength":128},
            "unit":{"type":"string","enum":["one","metre","millimetre","square_metre","square_millimetre","metre_to_fourth","millimetre_to_fourth","pascal","kilopascal","megapascal","gigapascal","kilogram_per_cubic_metre","cubic_metre_per_second","litre_per_second","volt","ampere","watt","kilowatt","joule","kilowatt_hour","kelvin","degree_celsius","kelvin_difference","watt_per_metre_kelvin","joule_per_kilogram_kelvin"]},
            "basis":{"type":"string","enum":["nominal","characteristic","design","measured"]},
            "design_basis":{"type":["string","null"]}
        }),
        &["quantity", "decimal", "unit", "basis"],
    )
}
pub(super) fn conditions_values() -> Value {
    json!({"type":"object","maxProperties":32,"propertyNames":quantity(),"additionalProperties":value()})
}
fn ranges() -> Value {
    json!({"type":"array","maxItems":32,"items":object(json!({"quantity":quantity(),"minimum":value(),"maximum":value()}),&["quantity","minimum","maximum"])})
}
fn values() -> Value {
    json!({"type":"object","maxProperties":256,"additionalProperties":value()})
}
fn triple(schema: Value) -> Value {
    json!({"type":"array","minItems":3,"maxItems":3,"items":schema})
}
fn elastic() -> Value {
    json!({"oneOf":[
        tagged("isotropic",json!({"young_modulus":value(),"poisson_ratio":{"type":"number","exclusiveMinimum":-1,"exclusiveMaximum":0.5}}),&["young_modulus","poisson_ratio"]),
        tagged("orthotropic",json!({"young_moduli":triple(value()),"shear_moduli":triple(value()),"poisson_12_23_31":triple(json!({"type":"number"}))}),&["young_moduli","shear_moduli","poisson_12_23_31"])
    ]})
}
fn material() -> Value {
    object(
        json!({"class":{"type":"string","minLength":1},"states":{"type":"array","minItems":1,"maxItems":128,"items":object(json!({
        "name":{"type":"string","minLength":1},"conditions":ranges(),"elastic":elastic(),"density":value(),"properties":values()
    }),&["name","conditions","elastic","density","properties"])}}),
        &["class", "states"],
    )
}
fn shape(depth: u32) -> Value {
    let length = json!({"type":"number","exclusiveMinimum":0});
    let pair = json!({"type":"array","minItems":2,"maxItems":2,"items":{"type":"number"}});
    let ring = json!({"type":"array","minItems":3,"maxItems":513,"items":pair});
    let mut variants = vec![
        tagged(
            "rectangle",
            json!({"width_m":length,"height_m":length}),
            &["width_m", "height_m"],
        ),
        tagged("circle", json!({"diameter_m":length}), &["diameter_m"]),
        tagged(
            "i_section",
            json!({"width_m":length,"height_m":length,"flange_m":length,"web_m":length}),
            &["width_m", "height_m", "flange_m", "web_m"],
        ),
        tagged(
            "box",
            json!({"width_m":length,"height_m":length,"wall_m":length}),
            &["width_m", "height_m", "wall_m"],
        ),
        tagged(
            "polygon",
            json!({"section":object(json!({"exterior_m":ring,"holes_m":{"type":"array","maxItems":64,"items":ring}}),&["exterior_m","holes_m"])}),
            &["section"],
        ),
    ];
    if depth > 0 {
        variants.push(tagged("compound",json!({"parts":{"type":"array","minItems":1,"maxItems":32,"items":object(json!({"shape":shape(depth-1),"translation_m":pair,"rotation_rad":{"type":"number"}}),&["shape","translation_m","rotation_rad"])}}),&["parts"]));
    }
    json!({"oneOf":variants})
}
pub(super) fn entry() -> Value {
    let text = json!({"type":"string","minLength":1,"maxLength":16384});
    let id = json!({"type":"string","format":"uuid"});
    object(
        json!({
            "id":id,"revision":{"type":"integer","minimum":1},"name":{"type":"string","minLength":1,"maxLength":prumo_domain::MAX_NAME_CHARACTERS},
            "source":object(json!({"publisher":text,"reference":text,"edition":text,"revision":text,"data_license":text,"original_record":{"type":["string","null"],"maxLength":1048576}}),&["publisher","reference","edition","revision","data_license"]),
            "applicability":ranges(),
            "customization":{"oneOf":[{"type":"null"},object(json!({"base_id":id,"base_revision":{"type":"integer","minimum":1},"base_sha256":{"type":"string","pattern":"^[a-fA-F0-9]{64}$"},"reason":text}),&["base_id","base_revision","base_sha256","reason"])]},
            "payload":{"oneOf":[
                tagged("material",json!({"material":material()}),&["material"]),
                tagged("section",json!({"shape":shape(4)}),&["shape"]),
                tagged("properties",json!({"values":values()}),&["values"])
            ]}
        }),
        &[
            "id",
            "revision",
            "name",
            "source",
            "applicability",
            "payload",
        ],
    )
}
