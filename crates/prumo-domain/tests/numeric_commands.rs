use prumo_domain::Command;
use prumo_domain::units::{Angle, Point3, PointInput, PressureInput};

#[test]
fn every_existing_numeric_command_accepts_equivalent_scientific_json_spellings() {
    let commands = [
        r#"{"operation":"add_member","expected_revision":1,"name":"Viga μ","start":[1.00e-7,-2E-7,0.0],"end":[4.000e0,0,0]}"#,
        r#"{"operation":"edit_model","expected_revision":1,"edit":{"edit":"add_physical","name":"Peça","discipline":"structure","start":{"values":[1.00e-7,0,0],"unit":"metre"},"end":{"values":[4.00e0,0,0],"unit":"metre"},"role":"visual_only","material_id":null,"environment_id":null}}"#,
        r#"{"operation":"edit_model","expected_revision":1,"edit":{"edit":"add_analytical_member","name":"Barra","start_node_id":"08cb9713-a2d6-44fd-83a7-d33ef87877d5","end_node_id":"19cb9713-a2d6-44fd-83a7-d33ef87877d5","local_y":[0.0e0,1.00e0,0.00e0]}}"#,
        r#"{"operation":"edit_model","expected_revision":1,"edit":{"edit":"add_material","material":{"name":"Aço","family":"steel","elastic_modulus":{"value":2.00e11,"unit":"pascal"},"density":{"value":7.850e3,"unit":"kilogram_per_cubic_metre"},"poisson_ratio":3.00e-1}}}"#,
        r#"{"operation":"edit_model","expected_revision":1,"edit":{"edit":"add_nodal_action","action":{"name":"Ação","application_node_id":"08cb9713-a2d6-44fd-83a7-d33ef87877d5","force":{"values":[1.00e-7,0,0],"unit":"newton"},"couple":{"values":[0,2.00e-8,0],"unit":"newton_metre"},"frame":{"kind":"engineering"}}}}"#,
        r#"{"operation":"edit_model","expected_revision":1,"edit":{"edit":"rigid_move","object_ids":["08cb9713-a2d6-44fd-83a7-d33ef87877d5"],"rotation":[[1.00e0,0,0],[0,1.00e0,0],[0,0,1.00e0]],"translation":{"values":[1.00e-7,0,0],"unit":"metre"}}}"#,
        r#"{"operation":"edit_model","expected_revision":1,"edit":{"edit":"change_engineering_frame","frame":{"origin_world_m":[1.00e-7,0,0],"rotation_to_world":[[1.00e0,0,0],[0,1.00e0,0],[0,0,1.00e0]]}}}"#,
        r#"{"operation":"edit_model","expected_revision":1,"edit":{"edit":"set_georeference","georeference":{"crs_identifier":"EPSG:31983","map_origin_m":[1.00e7,2.00e7,0],"rotation_radians":1.00e-7,"scale":1.000e0}}}"#,
        r#"{"operation":"edit_model","expected_revision":1,"edit":{"edit":"split_physical","object_id":"08cb9713-a2d6-44fd-83a7-d33ef87877d5","fraction":5.00e-1,"names":["Parte α","Parte β"]}}"#,
    ];
    for raw in commands {
        let direct: Command =
            serde_json::from_str(raw).unwrap_or_else(|error| panic!("{error}: {raw}"));
        let buffered: Command = serde_json::from_value(serde_json::from_str(raw).unwrap()).unwrap();
        let encoded = serde_json::to_value(&direct).unwrap();
        assert_eq!(encoded, serde_json::to_value(&buffered).unwrap());
        let repeated: Command =
            serde_json::from_slice(&serde_json::to_vec(&direct).unwrap()).unwrap();
        assert_eq!(encoded, serde_json::to_value(repeated).unwrap());
    }
    let command: Command = serde_json::from_str(commands[0]).unwrap();
    let Command::AddMember {
        name, start, end, ..
    } = command
    else {
        unreachable!()
    };
    assert_eq!(name, "Viga μ");
    assert_eq!(start, [1e-7, -2e-7, 0.0]);
    assert_eq!(end, [4.0, 0.0, 0.0]);
}

#[test]
fn typed_numbers_reject_nonfinite_values_underflow_and_wrong_vector_lengths() {
    for value in ["1e999", "1e-999", "null", "\"1e-7\""] {
        assert!(serde_json::from_str::<Angle>(value).is_err());
        let raw = format!(r#"{{"value":{value},"unit":"pascal"}}"#);
        assert!(serde_json::from_str::<PressureInput>(&raw).is_err());
    }
    for values in ["[1,2]", "[1,2,3,4]", "[1,1e999,0]", "[1,1e-999,0]"] {
        assert!(serde_json::from_str::<Point3>(values).is_err());
        let raw = format!(r#"{{"values":{values},"unit":"metre"}}"#);
        assert!(serde_json::from_str::<PointInput>(&raw).is_err());
    }
    let zero: Angle = serde_json::from_str("0.00e-999").unwrap();
    assert_eq!(zero.si(), 0.0);
}
