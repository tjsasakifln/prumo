//! Bounded Wasm execution of a reviewed Rust fixture, without host capabilities.
use crate::{ComponentDescriptor, ExperimentReport, QualificationCheck};
use std::time::Instant;
use thiserror::Error;
use wasmi::{Config, Engine, Instance, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

#[rustfmt::skip]
mod bytes { include!("wasm_bytes.rs"); }
pub use bytes::RUST_BOUNDARY_MODULE;

const MAX_MODULE_BYTES: usize = 256 * 1024;
const MEMORY_BYTES: usize = 64 * 1024;
const FUEL: u64 = 20_000;

#[derive(Debug, Error)]
pub enum WasmBoundaryError {
    #[error("Entrada de comprimento fora do domínio do ensaio.")]
    InvalidInput,
    #[error("Módulo WASM excede o limite de tamanho.")]
    TooLarge,
    #[error("Este contrato não autoriza imports ou capacidades do host.")]
    ImportsDenied,
    #[error("Versão do contrato WASM incompatível.")]
    VersionMismatch,
    #[error("Execução WASM recusada: {0}")]
    Execution(String),
}

fn execution(error: impl std::fmt::Display) -> WasmBoundaryError {
    WasmBoundaryError::Execution(error.to_string())
}

fn instantiate(bytes: &[u8]) -> Result<(Store<StoreLimits>, Instance), WasmBoundaryError> {
    if bytes.len() > MAX_MODULE_BYTES {
        return Err(WasmBoundaryError::TooLarge);
    }
    let mut config = Config::default();
    config.consume_fuel(true);
    config
        .set_max_recursion_depth(64)
        .set_min_stack_height(64)
        .set_max_stack_height(4096)
        .set_max_cached_stacks(1);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, bytes).map_err(execution)?;
    if module.imports().next().is_some() {
        return Err(WasmBoundaryError::ImportsDenied);
    }
    let limits = StoreLimitsBuilder::new()
        .memory_size(MEMORY_BYTES)
        .memories(1)
        .instances(1)
        .tables(0)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(&engine, limits);
    store.limiter(|limits| limits);
    store.set_fuel(FUEL).map_err(execution)?;
    let linker = Linker::<StoreLimits>::new(&engine);
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(execution)?;
    let version = instance
        .get_typed_func::<(), u32>(&store, "prumo_contract_version")
        .map_err(execution)?
        .call(&mut store, ())
        .map_err(execution)?;
    if version != 1 {
        return Err(WasmBoundaryError::VersionMismatch);
    }
    Ok((store, instance))
}

/// Scalar ABI: input millimetres, output metres; no pointer or ownership crosses.
pub fn convert_millimetres(bytes: &[u8], value_mm: f64) -> Result<f64, WasmBoundaryError> {
    if !value_mm.is_finite() || !(0.0..=1.0e12).contains(&value_mm) {
        return Err(WasmBoundaryError::InvalidInput);
    }
    let (mut store, instance) = instantiate(bytes)?;
    let result = instance
        .get_typed_func::<f64, f64>(&store, "millimetres_to_metres")
        .map_err(execution)?
        .call(&mut store, value_mm)
        .map_err(execution)?;
    if !result.is_finite() || !(0.0..=1.0e9).contains(&result) {
        return Err(WasmBoundaryError::InvalidInput);
    }
    Ok(result)
}

pub fn descriptor() -> ComponentDescriptor {
    ComponentDescriptor {
        id: "wasmi-boundary".into(),
        name: "Wasmi".into(),
        version: "2.0.0".into(),
        origin: "https://github.com/wasmi-labs/wasmi".into(),
        license: "MIT OR Apache-2.0".into(),
        language: "Rust".into(),
        boundary: "WebAssembly sem imports, memória de 64 KiB e 20.000 unidades de execução".into(),
        scope:
            "Conversão escalar de unidades e contrato de falha com módulo de ensaio escrito em Rust"
                .into(),
        limitations: vec![
            "Não é host de extensões nem motor de cálculo.".into(),
            "Nenhum arquivo, rede, relógio ou função do host é importado.".into(),
            "O limite de memória linear não representa toda a memória do interpretador.".into(),
        ],
    }
}

fn boolean_check(name: &str, passed: bool, detail: &str) -> QualificationCheck {
    QualificationCheck {
        name: name.into(),
        passed,
        detail: detail.into(),
        absolute_error: None,
        tolerance: None,
        unit: None,
    }
}

pub fn run_experiment() -> ExperimentReport {
    let started = Instant::now();
    let mut checks = Vec::new();
    // Independent dimensional identities, fixed before execution.
    for (input, expected) in [(0.0, 0.0), (1.0, 0.001), (8250.0, 8.25), (1.0e9, 1.0e6)] {
        let result = convert_millimetres(RUST_BOUNDARY_MODULE, input);
        let error = result
            .as_ref()
            .map(|value| (value - expected).abs())
            .unwrap_or(f64::MAX);
        checks.push(QualificationCheck {
            name: format!("{input} mm em metros"),
            passed: result.is_ok() && error <= 1.0e-12,
            detail: "Referência independente: fator SI exato 1/1000.".into(),
            absolute_error: Some(error),
            tolerance: Some(1.0e-12),
            unit: Some("m".into()),
        });
    }
    checks.push(boolean_check(
        "Entrada inválida",
        [f64::NAN, f64::INFINITY, -1.0, 1.0e13]
            .into_iter()
            .all(|value| convert_millimetres(RUST_BOUNDARY_MODULE, value).is_err()),
        "Valores não finitos, negativos e fora do domínio são recusados antes da execução.",
    ));
    let memory_blocked = instantiate(RUST_BOUNDARY_MODULE).is_ok_and(|(mut store, instance)| {
        instance
            .get_typed_func::<u32, u32>(&store, "request_memory")
            .is_ok_and(|function| function.call(&mut store, 1).is_err())
    });
    checks.push(boolean_check(
        "Limite de memória",
        memory_blocked,
        "Pedido de crescimento de uma página excede 64 KiB e é interrompido.",
    ));
    let execution_stopped = instantiate(RUST_BOUNDARY_MODULE).is_ok_and(|(mut store, instance)| {
        instance
            .get_typed_func::<(), ()>(&store, "exhaust_fuel")
            .is_ok_and(|function| function.call(&mut store, ()).is_err())
    });
    checks.push(boolean_check(
        "Cancelamento por orçamento",
        execution_stopped,
        "Laço sem término é interrompido pelo orçamento de execução.",
    ));
    checks.push(boolean_check("Reinício após falha",convert_millimetres(RUST_BOUNDARY_MODULE,1000.0).is_ok_and(|value|value==1.0),"Nova instância funciona após os dois testes de falha; os recursos anteriores foram descartados."));
    ExperimentReport {
        component: descriptor(),
        checks,
        iterations: 1,
        elapsed_ns: started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
        artifact: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_module_obeys_units_memory_fuel_and_restart_contracts() {
        let report = run_experiment();
        assert!(report.passed(), "{:?}", report.checks);
    }

    #[test]
    fn malformed_oversized_and_wrong_version_modules_are_rejected() {
        assert!(convert_millimetres(b"not Wasm", 1000.0).is_err());
        assert!(matches!(
            convert_millimetres(&vec![0; MAX_MODULE_BYTES + 1], 1000.0),
            Err(WasmBoundaryError::TooLarge)
        ));
        let mut bytes = RUST_BOUNDARY_MODULE.to_vec();
        // Bytecode mutation of the isolated version function, never a product module.
        let at = bytes
            .windows(5)
            .position(|part| part == [4, 0, 65, 1, 11])
            .unwrap();
        bytes[at + 3] = 2;
        assert!(matches!(
            convert_millimetres(&bytes, 1000.0),
            Err(WasmBoundaryError::VersionMismatch)
        ));
        // Minimal valid Wasm with one import named env.x. No implementation is granted.
        let imports = [
            0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0, 2, 9, 1, 3, 101, 110, 118, 1, 120, 0, 0,
        ];
        assert!(matches!(
            convert_millimetres(&imports, 1000.0),
            Err(WasmBoundaryError::ImportsDenied)
        ));
    }
}
