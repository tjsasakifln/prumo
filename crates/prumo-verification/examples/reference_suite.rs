fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = prumo_verification::RunContext::capture(None)?;
    let report = prumo_verification::run_reference_suite(context)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.passed() {
        std::process::exit(1);
    }
    Ok(())
}
