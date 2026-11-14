//! Dense linear algebra probe in explicit SI units. This is not a structural solver.
//! Assembly, elements, materials, load combinations and normative checks are outside this domain.

use nalgebra::{DMatrix, DVector};
use prumo_domain::components::{ComponentDescriptor, ExperimentReport, QualificationCheck};
use serde::{Deserialize, Serialize};
use std::hint::black_box;
use std::time::Instant;
use thiserror::Error;

pub const MAX_UNKNOWNS: usize = 64;
pub const MAX_INPUT_MAGNITUDE: f64 = 1.0e20;
pub const MAX_DISPLACEMENT_M: f64 = 1.0e9;
pub const MAX_CONDITION_ESTIMATE: f64 = 1.0e10;
pub const SYMMETRY_RELATIVE_TOLERANCE: f64 = 1.0e-12;
pub const RELATIVE_RESIDUAL_TOLERANCE: f64 = 1.0e-12;
const SOLUTION_RELATIVE_TOLERANCE: f64 = 1.0e-10;
const SOLUTION_ABSOLUTE_TOLERANCE_M: f64 = 1.0e-14;
const ITERATIONS: u32 = 64;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinearSystem {
    /// Symmetric positive definite dense matrix, rows and columns in the same order.
    pub matrix_n_per_m: Vec<Vec<f64>>,
    pub load_n: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinearSolution {
    pub displacements_m: Vec<f64>,
    /// ||Ax-b||∞ / (||A||∞ ||x||∞ + ||b||∞), zero for an exactly zero denominator.
    pub relative_residual: f64,
    /// Floating point estimate ||A||∞ ||A⁻¹||∞, not the spectral condition number.
    pub condition_estimate: f64,
}

pub trait LinearSolverAdapter {
    fn solve(&self, system: &LinearSystem) -> Result<LinearSolution, SolverError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NalgebraCholesky;

impl LinearSolverAdapter for NalgebraCholesky {
    fn solve(&self, system: &LinearSystem) -> Result<LinearSolution, SolverError> {
        let size = system.load_n.len();
        if size == 0
            || size > MAX_UNKNOWNS
            || system.matrix_n_per_m.len() != size
            || system.matrix_n_per_m.iter().any(|row| row.len() != size)
        {
            return Err(SolverError::Dimension);
        }
        if system
            .matrix_n_per_m
            .iter()
            .flatten()
            .chain(&system.load_n)
            .any(|value| !value.is_finite() || value.abs() > MAX_INPUT_MAGNITUDE)
        {
            return Err(SolverError::InvalidInput);
        }
        let norm_a = matrix_infinity_norm(&system.matrix_n_per_m);
        for row in 0..size {
            for column in 0..row {
                if (system.matrix_n_per_m[row][column] - system.matrix_n_per_m[column][row]).abs()
                    > SYMMETRY_RELATIVE_TOLERANCE * norm_a
                {
                    return Err(SolverError::NotSymmetric);
                }
            }
        }
        let matrix = DMatrix::from_fn(size, size, |row, column| system.matrix_n_per_m[row][column]);
        // nalgebra reads the lower triangle for Cholesky. The explicit symmetry check above
        // prevents it from silently accepting an unrelated upper triangle.
        let factor = matrix.cholesky().ok_or(SolverError::NotPositiveDefinite)?;
        let inverse = factor.inverse();
        if inverse.iter().any(|value| !value.is_finite()) {
            return Err(SolverError::IllConditioned {
                estimate: f64::INFINITY,
            });
        }
        let inverse_norm = (0..size)
            .map(|row| {
                (0..size)
                    .map(|column| inverse[(row, column)].abs())
                    .sum::<f64>()
            })
            .fold(0.0, f64::max);
        let condition_estimate = norm_a * inverse_norm;
        if !condition_estimate.is_finite() || condition_estimate > MAX_CONDITION_ESTIMATE {
            return Err(SolverError::IllConditioned {
                estimate: condition_estimate,
            });
        }
        let values = factor.solve(&DVector::from_row_slice(&system.load_n));
        let displacements_m = values.as_slice().to_vec();
        if displacements_m
            .iter()
            .any(|value| !value.is_finite() || value.abs() > MAX_DISPLACEMENT_M)
        {
            return Err(SolverError::UnacceptableSolution);
        }
        let relative_residual = relative_residual(system, &displacements_m);
        if !relative_residual.is_finite() || relative_residual > RELATIVE_RESIDUAL_TOLERANCE {
            return Err(SolverError::Residual {
                relative: relative_residual,
            });
        }
        Ok(LinearSolution {
            displacements_m,
            relative_residual,
            condition_estimate,
        })
    }
}

fn matrix_infinity_norm(matrix: &[Vec<f64>]) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn vector_infinity_norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

/// Residual computed independently from nalgebra's factorization and solve routines.
pub fn relative_residual(system: &LinearSystem, solution_m: &[f64]) -> f64 {
    if solution_m.is_empty()
        || system.matrix_n_per_m.len() != system.load_n.len()
        || solution_m.len() != system.load_n.len()
        || system
            .matrix_n_per_m
            .iter()
            .any(|row| row.len() != solution_m.len())
        || system
            .matrix_n_per_m
            .iter()
            .flatten()
            .chain(&system.load_n)
            .chain(solution_m)
            .any(|value| !value.is_finite())
    {
        return f64::INFINITY;
    }
    let residual = system
        .matrix_n_per_m
        .iter()
        .zip(&system.load_n)
        .map(|(row, load)| {
            (row.iter().zip(solution_m).map(|(a, x)| a * x).sum::<f64>() - load).abs()
        })
        .fold(0.0, f64::max);
    let denominator = matrix_infinity_norm(&system.matrix_n_per_m)
        * vector_infinity_norm(solution_m)
        + vector_infinity_norm(&system.load_n);
    if denominator == 0.0 {
        residual
    } else {
        residual / denominator
    }
}

#[derive(Debug, Error)]
pub enum SolverError {
    #[error("A matriz deve ser quadrada, compatível com o vetor e ter de 1 a 64 incógnitas.")]
    Dimension,
    #[error(
        "Coeficientes e cargas devem ser finitos, com módulo até 1e20 nas unidades declaradas."
    )]
    InvalidInput,
    #[error("A matriz não é simétrica dentro da tolerância relativa de 1e-12.")]
    NotSymmetric,
    #[error("A matriz não é definida positiva; não se acrescenta rigidez artificial.")]
    NotPositiveDefinite,
    #[error("Estimativa de condicionamento na norma infinito fora do domínio: {estimate}.")]
    IllConditioned { estimate: f64 },
    #[error("A solução é não finita ou ultrapassa o limite de 1e9 m.")]
    UnacceptableSolution,
    #[error("Resíduo relativo {relative} excede 1e-12.")]
    Residual { relative: f64 },
}

pub fn descriptor() -> ComponentDescriptor {
    ComponentDescriptor {
        id: "solver-nalgebra".into(), name: "Álgebra linear SPD com nalgebra".into(), version: "0.35.0".into(),
        origin: "https://github.com/dimforge/nalgebra".into(), license: "Apache-2.0".into(), language: "Rust".into(), boundary: "crate Rust, interface segura".into(),
        scope: "Sistema denso simétrico definido positivo, até 64 incógnitas; matriz em N/m, vetor em N e solução em m.".into(),
        limitations: vec!["Ensaio de álgebra linear; não constitui solver estrutural, análise normativa ou dimensionamento.".into(), "Sem montagem de elementos, análise não linear, matriz esparsa ou instabilidade estrutural.".into(), "Estimativa ||A||∞ ||A⁻¹||∞ limitada a 1e10; resíduo relativo limitado a 1e-12.".into(), "Não substitui termos de Cholesky nem regulariza matrizes singulares.".into()],
    }
}

/// Reusable closed-reference battery for this narrowly specified linear algebra interface.
pub fn qualification_checks(adapter: &dyn LinearSolverAdapter) -> Vec<QualificationCheck> {
    let mut checks = Vec::new();
    for scale in [0.001, 1.0, 1000.0] {
        let system = LinearSystem {
            matrix_n_per_m: vec![vec![4.0 * scale, scale], vec![scale, 3.0 * scale]],
            load_n: vec![scale, 2.0 * scale],
        };
        // Independent inverse of a 2x2 matrix: det=11, x=(3-2)/11, y=(8-1)/11.
        check_solution(
            adapter,
            &system,
            &[1.0 / 11.0, 7.0 / 11.0],
            &format!("Inversa fechada 2x2, escala {scale}"),
            &mut checks,
        );
        let springs = series_springs(1000.0 * scale, 2000.0 * scale, 4000.0 * scale, 10.0);
        // Series relation: each spring carries F and extends F/k. No FEM implementation
        // supplies the reference values for this algebraic benchmark.
        check_solution(
            adapter,
            &springs,
            &[0.01 / scale, 0.015 / scale, 0.0175 / scale],
            &format!("Três molas em série, escala {scale}"),
            &mut checks,
        );
    }
    let zero = LinearSystem {
        matrix_n_per_m: vec![vec![4.0, 1.0], vec![1.0, 3.0]],
        load_n: vec![0.0, 0.0],
    };
    check_solution(adapter, &zero, &[0.0, 0.0], "Carga zero", &mut checks);
    let scalar = LinearSystem {
        matrix_n_per_m: vec![vec![2000.0]],
        load_n: vec![-10.0],
    };
    check_solution(
        adapter,
        &scalar,
        &[-0.005],
        "Sinal negativo e sistema escalar",
        &mut checks,
    );
    let cases = [
        (
            "Matriz vazia recusada",
            LinearSystem {
                matrix_n_per_m: vec![],
                load_n: vec![],
            },
        ),
        (
            "Dimensões incompatíveis recusadas",
            LinearSystem {
                matrix_n_per_m: vec![vec![1.0, 0.0]],
                load_n: vec![1.0],
            },
        ),
        (
            "NaN recusado",
            LinearSystem {
                matrix_n_per_m: vec![vec![f64::NAN]],
                load_n: vec![1.0],
            },
        ),
        (
            "Triângulos assimétricos recusados",
            LinearSystem {
                matrix_n_per_m: vec![vec![4.0, 3.0], vec![1.0, 3.0]],
                load_n: vec![1.0, 2.0],
            },
        ),
        (
            "Matriz singular recusada",
            LinearSystem {
                matrix_n_per_m: vec![vec![1.0, 1.0], vec![1.0, 1.0]],
                load_n: vec![1.0, 1.0],
            },
        ),
        (
            "Matriz indefinida recusada",
            LinearSystem {
                matrix_n_per_m: vec![vec![1.0, 0.0], vec![0.0, -1.0]],
                load_n: vec![1.0, 1.0],
            },
        ),
        (
            "Condicionamento além do domínio recusado",
            LinearSystem {
                matrix_n_per_m: vec![vec![1.0, 0.0], vec![0.0, 1.0e-14]],
                load_n: vec![0.0, 0.0],
            },
        ),
    ];
    for (name, system) in cases {
        checks.push(boolean_check(
            name,
            adapter.solve(&system).is_err(),
            "Entrada fora do domínio deve produzir erro, sem correção automática.",
        ));
    }
    checks
}

fn series_springs(k1: f64, k2: f64, k3: f64, force: f64) -> LinearSystem {
    LinearSystem {
        matrix_n_per_m: vec![
            vec![k1 + k2, -k2, 0.0],
            vec![-k2, k2 + k3, -k3],
            vec![0.0, -k3, k3],
        ],
        load_n: vec![0.0, 0.0, force],
    }
}

fn check_solution(
    adapter: &dyn LinearSolverAdapter,
    system: &LinearSystem,
    expected: &[f64],
    name: &str,
    checks: &mut Vec<QualificationCheck>,
) {
    match adapter.solve(system) {
        Ok(output) if output.displacements_m.len() == expected.len() => {
            for (index, (&actual, &reference)) in
                output.displacements_m.iter().zip(expected).enumerate()
            {
                let error = (actual - reference).abs();
                let tolerance =
                    SOLUTION_ABSOLUTE_TOLERANCE_M + SOLUTION_RELATIVE_TOLERANCE * reference.abs();
                checks.push(QualificationCheck {
                    name: format!("{name}, incógnita {index}"),
                    passed: actual.is_finite() && error <= tolerance,
                    detail: format!(
                        "Solução {actual:.15e} m; referência fechada {reference:.15e} m."
                    ),
                    absolute_error: Some(error),
                    tolerance: Some(tolerance),
                    unit: Some("m".into()),
                });
            }
            let residual = relative_residual(system, &output.displacements_m);
            checks.push(QualificationCheck {
                name: format!("{name}, resíduo independente"),
                passed: residual.is_finite() && residual <= RELATIVE_RESIDUAL_TOLERANCE,
                detail:
                    "||Ax-b||∞ / (||A||∞ ||x||∞ + ||b||∞), calculado a partir da matriz original."
                        .into(),
                absolute_error: Some(residual),
                tolerance: Some(RELATIVE_RESIDUAL_TOLERANCE),
                unit: Some("adimensional".into()),
            });
            checks.push(boolean_check(&format!("{name}, diagnósticos finitos"), output.condition_estimate.is_finite() && output.condition_estimate >= 1.0 - 1.0e-12 && output.condition_estimate <= MAX_CONDITION_ESTIMATE && output.relative_residual.is_finite() && (output.relative_residual - residual).abs() <= 1.0e-15,
                "Estimativa na norma infinito e resíduo devem respeitar o domínio e a verificação independente."));
        }
        result => checks.push(boolean_check(
            name,
            false,
            &format!("Solução inesperada: {result:?}"),
        )),
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
    let mut first_failure = None;
    for _ in 0..ITERATIONS {
        let current = black_box(qualification_checks(&NalgebraCholesky));
        if first_failure.is_none() && current.iter().any(|check| !check.passed) {
            first_failure = Some(current);
        } else {
            checks = current;
        }
    }
    ExperimentReport {
        component: descriptor(),
        checks: first_failure.unwrap_or(checks),
        iterations: ITERATIONS,
        elapsed_ns: started.elapsed().as_nanos().try_into().unwrap_or(u64::MAX),
        artifact: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_passes_closed_references_scales_signs_and_invalid_inputs() {
        let checks = qualification_checks(&NalgebraCholesky);
        assert!(checks.len() >= 40);
        assert!(checks.iter().all(|check| check.passed), "{checks:#?}");
    }

    #[test]
    fn permutation_preserves_the_independent_series_reference() {
        let system = series_springs(1000.0, 2000.0, 4000.0, 10.0);
        let order = [2, 0, 1];
        let permuted = LinearSystem {
            matrix_n_per_m: order
                .iter()
                .map(|&row| {
                    order
                        .iter()
                        .map(|&column| system.matrix_n_per_m[row][column])
                        .collect()
                })
                .collect(),
            load_n: order.iter().map(|&index| system.load_n[index]).collect(),
        };
        let solution = NalgebraCholesky.solve(&permuted).unwrap();
        for (actual, expected) in solution.displacements_m.iter().zip([0.0175, 0.01, 0.015]) {
            assert!((actual - expected).abs() < 1.0e-13);
        }
    }

    #[test]
    fn condition_estimate_is_the_infinity_norm_not_a_cholesky_diagonal_ratio() {
        let system = LinearSystem {
            matrix_n_per_m: vec![vec![4.0, 1.0], vec![1.0, 3.0]],
            load_n: vec![0.0; 2],
        };
        // ||A||∞=5; A⁻¹=(1/11)[[3,-1],[-1,4]], ||A⁻¹||∞=5/11.
        let solution = NalgebraCholesky.solve(&system).unwrap();
        assert!((solution.condition_estimate - 25.0 / 11.0).abs() < 1.0e-13);
        assert_eq!(solution.relative_residual, 0.0);
    }

    #[test]
    fn bounded_dimension_and_solution_are_enforced() {
        let mut diagonal = vec![vec![0.0; MAX_UNKNOWNS]; MAX_UNKNOWNS];
        let expected: Vec<_> = (0..MAX_UNKNOWNS).map(|index| index as f64 / 10.0).collect();
        let load_n: Vec<_> = expected
            .iter()
            .enumerate()
            .map(|(index, value)| {
                diagonal[index][index] = (index + 1) as f64;
                diagonal[index][index] * value
            })
            .collect();
        let solution = NalgebraCholesky
            .solve(&LinearSystem {
                matrix_n_per_m: diagonal,
                load_n,
            })
            .unwrap();
        for (actual, reference) in solution.displacements_m.iter().zip(expected) {
            assert!((actual - reference).abs() < 1.0e-13);
        }
        let size = MAX_UNKNOWNS + 1;
        assert!(matches!(
            NalgebraCholesky.solve(&LinearSystem {
                matrix_n_per_m: vec![vec![0.0; size]; size],
                load_n: vec![0.0; size]
            }),
            Err(SolverError::Dimension)
        ));
        assert!(matches!(
            NalgebraCholesky.solve(&LinearSystem {
                matrix_n_per_m: vec![vec![1.0e-10]],
                load_n: vec![1.0]
            }),
            Err(SolverError::UnacceptableSolution)
        ));
        assert!(
            NalgebraCholesky
                .solve(&LinearSystem {
                    matrix_n_per_m: vec![vec![1.0]],
                    load_n: vec![f64::INFINITY]
                })
                .is_err()
        );
    }

    struct WrongAdapter;
    impl LinearSolverAdapter for WrongAdapter {
        fn solve(&self, system: &LinearSystem) -> Result<LinearSolution, SolverError> {
            Ok(LinearSolution {
                displacements_m: vec![0.0; system.load_n.len()],
                relative_residual: 0.0,
                condition_estimate: 1.0,
            })
        }
    }

    #[test]
    fn common_battery_detects_inaccurate_or_dishonest_replacement() {
        assert!(
            qualification_checks(&WrongAdapter)
                .iter()
                .any(|check| !check.passed)
        );
    }
}
