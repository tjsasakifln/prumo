use prumo_app::{AccessMode, ProjectService};
use prumo_domain::LengthUnit;
use serde_json::json;

#[test]
fn trial_preserves_model_snapshot_and_records_input_hash() {
    let temp=tempfile::tempdir().unwrap();
    let path=temp.path().join("experiment.prumo");
    let service=ProjectService::open(path,AccessMode::ReadWrite,"trial").unwrap();
    let before=service.create("Modelo imutável",LengthUnit::Metre).unwrap();
    let input=json!({"kind":"linear","system":{
        "dimension":2,
        "terms":[
            {"row":0,"column":0,"stiffness_n_per_m":4.0},
            {"row":0,"column":1,"stiffness_n_per_m":1.0},
            {"row":1,"column":0,"stiffness_n_per_m":1.0},
            {"row":1,"column":1,"stiffness_n_per_m":3.0}
        ],
        "forces_n":[1.0,2.0],"prescribed":[]
    }});
    let trial=service.analysis_trial(input,None).unwrap();
    assert_eq!(trial["revision"],before.revision);
    assert_eq!(trial["project_id"],before.id.to_string());
    assert_eq!(trial["engineering_release"],false);
    assert!(trial["input_sha256"].as_str().is_some_and(|x|x.len()==64));
    assert!(trial["snapshot_sha256"].as_str().is_some_and(|x|x.len()==64));
    assert_eq!(service.read().unwrap(),before);
}
#[test]
fn trial_fails_closed_for_malformed_and_singular_inputs() {
    let temp=tempfile::tempdir().unwrap();
    let path=temp.path().join("reject.prumo");
    let service=ProjectService::open(path,AccessMode::ReadWrite,"trial").unwrap();
    let before=service.create("Rejeição",LengthUnit::Metre).unwrap();
    assert!(service.analysis_trial(json!({"kind":"linear","system":{
        "dimension":2,"terms":[{"row":0,"column":0,"stiffness_n_per_m":1.0}],
        "forces_n":[1.0,1.0],"prescribed":[]}}),None).is_err());
    assert!(service.analysis_trial(json!({"kind":"truss","unknown":true}),None).is_err());
    assert_eq!(service.read().unwrap(),before);
}
