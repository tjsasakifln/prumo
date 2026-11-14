//! Replaceable, bounded experiments. No module qualifies an engineering design.
#![forbid(unsafe_code)]

pub use prumo_domain::components::*;
pub mod bim;
pub mod document;
pub mod geometry;
pub mod process;
pub mod sections;
pub mod solver;
pub mod wasm;

pub fn inventory() -> Vec<ComponentDescriptor> {
    vec![
        bim::descriptor(),
        geometry::descriptor(),
        solver::descriptor(),
        sections::descriptor(),
        document::descriptor(),
        wasm::descriptor(),
    ]
}

pub fn run_suite(
    request: process::WorkerRequest,
) -> Result<QualificationReport, process::ProcessError> {
    request.validate()?;
    let started = std::time::Instant::now();
    let (executable_sha256, executable_bytes) =
        process::executable_identity(&std::env::current_exe()?)?;
    let experiments = vec![
        bim::run_experiment(),
        geometry::run_experiment(),
        solver::run_experiment(),
        sections::run_experiment(),
        document::run_experiment(),
        wasm::run_experiment(),
    ];
    Ok(QualificationReport {
        protocol_version: process::PROTOCOL_VERSION,
        suite_id: process::SUITE_ID.into(),
        request_id: request.request_id,
        basis: request.basis,
        platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        executable_sha256,
        executable_bytes,
        elapsed_ns: started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
        experiments,
        engineering_release: false,
    })
}
