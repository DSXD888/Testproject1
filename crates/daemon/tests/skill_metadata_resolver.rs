use std::fs;
use std::path::PathBuf;

#[test]
fn l6_portal_form_model_profile_is_registered_in_daemon_resolver() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let main_rs_path = manifest_dir.join("src/main.rs");
    let main_rs = fs::read_to_string(&main_rs_path)
        .expect("daemon main.rs should be readable for resolver registration test");

    assert!(
        main_rs.contains("\"portal_form_high_reliability\" => Some(\"opus\".to_owned())"),
        "portal_form_high_reliability must resolve to the premium Claude model"
    );
    assert!(
        main_rs.contains("\"structured_extract_low_cost\" => Some(\"haiku\".to_owned())"),
        "existing low-cost extraction profile must remain pinned to haiku"
    );
    assert!(
        main_rs.contains("_ => None"),
        "unknown model profiles should retain the existing CLI-default fallback"
    );
}
