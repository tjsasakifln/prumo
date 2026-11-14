//! Delimited qualification of rigid, right-handed transformations in metres.
//! This is not a B-rep kernel or a qualification of CAD/IFC conversion.

use nalgebra::{Matrix3, Vector3};
use prumo_domain::components::{ComponentDescriptor, ExperimentReport, QualificationCheck};
use serde::{Deserialize, Serialize};
use std::hint::black_box;
use std::time::Instant;
use thiserror::Error;

pub const MAX_POINTS: usize = 4096;
pub const MAX_COORDINATE_M: f64 = 1.0e9;
pub const ORTHONORMAL_TOLERANCE: f64 = 1.0e-12;
const ITERATIONS: u32 = 128;
const RELATIVE_TOLERANCE: f64 = 1.0e-12;
const ABSOLUTE_TOLERANCE_M: f64 = 1.0e-10;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RigidTransform {
    /// Row-major rotation, right-handed and orthonormal within the declared tolerance.
    pub rotation: [[f64; 3]; 3],
    pub translation_m: [f64; 3],
}

impl Default for RigidTransform {
    fn default() -> Self {
        Self {
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation_m: [0.0; 3],
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GeometryOutput {
    pub points_m: Vec<[f64; 3]>,
    pub minimum_m: [f64; 3],
    pub maximum_m: [f64; 3],
}

pub trait GeometryAdapter {
    fn transform(
        &self,
        points_m: &[[f64; 3]],
        transform: RigidTransform,
    ) -> Result<GeometryOutput, GeometryError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NalgebraGeometry;

impl GeometryAdapter for NalgebraGeometry {
    fn transform(
        &self,
        points_m: &[[f64; 3]],
        transform: RigidTransform,
    ) -> Result<GeometryOutput, GeometryError> {
        if points_m.is_empty() || points_m.len() > MAX_POINTS {
            return Err(GeometryError::PointCount);
        }
        if !valid_point(transform.translation_m)
            || points_m.iter().any(|point| !valid_point(*point))
            || transform
                .rotation
                .iter()
                .flatten()
                .any(|value| !value.is_finite())
        {
            return Err(GeometryError::InvalidCoordinate);
        }
        let rotation = Matrix3::from_row_slice(&transform.rotation.concat());
        let orthogonality = rotation.transpose() * rotation - Matrix3::identity();
        if orthogonality
            .iter()
            .any(|value| value.abs() > ORTHONORMAL_TOLERANCE)
            || (rotation.determinant() - 1.0).abs() > ORTHONORMAL_TOLERANCE
        {
            return Err(GeometryError::InvalidRotation);
        }
        let translation = Vector3::from(transform.translation_m);
        let mut output = GeometryOutput {
            points_m: Vec::with_capacity(points_m.len()),
            minimum_m: [f64::INFINITY; 3],
            maximum_m: [f64::NEG_INFINITY; 3],
        };
        for point in points_m {
            let transformed = rotation * Vector3::from(*point) + translation;
            let transformed = [transformed.x, transformed.y, transformed.z];
            if !valid_point(transformed) {
                return Err(GeometryError::InvalidCoordinate);
            }
            for (axis, value) in transformed.iter().copied().enumerate() {
                output.minimum_m[axis] = output.minimum_m[axis].min(value);
                output.maximum_m[axis] = output.maximum_m[axis].max(value);
            }
            output.points_m.push(transformed);
        }
        Ok(output)
    }
}

fn valid_point(point: [f64; 3]) -> bool {
    point
        .into_iter()
        .all(|value| value.is_finite() && value.abs() <= MAX_COORDINATE_M)
}

#[derive(Debug, Error)]
pub enum GeometryError {
    #[error("O ensaio aceita de 1 a 4096 pontos.")]
    PointCount,
    #[error("Coordenada não finita ou fora do limite de 1e9 m.")]
    InvalidCoordinate,
    #[error(
        "A matriz deve representar rotação ortonormal com orientação positiva; escala e reflexão não pertencem a este adaptador."
    )]
    InvalidRotation,
}

pub fn descriptor() -> ComponentDescriptor {
    ComponentDescriptor {
        id: "geometry-nalgebra".into(), name: "Geometria rígida com nalgebra".into(), version: "0.35.0".into(),
        origin: "https://github.com/dimforge/nalgebra".into(), license: "Apache-2.0".into(), language: "Rust".into(), boundary: "crate Rust, interface segura".into(),
        scope: "Transformação rígida de pontos em metros, f64, orientação positiva e limites explícitos.".into(),
        limitations: vec!["Experimento delimitado; não qualifica B-rep, interseções, malhas ou conversão IFC.".into(), "Não inclui escala, reflexão nem transformação geodésica.".into(), "Até 4096 pontos; coordenadas absolutas até 1e9 m.".into()],
    }
}

/// The same battery can exercise another implementation of GeometryAdapter.
pub fn qualification_checks(adapter: &dyn GeometryAdapter) -> Vec<QualificationCheck> {
    let mut checks = Vec::new();
    let quarter_turn = RigidTransform {
        rotation: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        translation_m: [10.0, -20.0, 30.0],
    };
    for scale in [0.001, 1.0, 1000.0] {
        let input = [[0.0; 3], [3.0 * scale, 4.0 * scale, 0.0]];
        match adapter.transform(&input, quarter_turn) {
            Ok(output) if output.points_m.len() == 2 => {
                // Independent closed reference: a positive quarter turn maps (x,y) to (-y,x).
                let expected = [10.0 - 4.0 * scale, -20.0 + 3.0 * scale, 30.0];
                for (axis, expected) in expected.into_iter().enumerate() {
                    checks.push(numeric_check(
                        &format!("Rotação e translação, escala {scale}, eixo {axis}"),
                        output.points_m[1][axis],
                        expected,
                        "m",
                    ));
                }
                let length = output.points_m[0]
                    .iter()
                    .zip(output.points_m[1])
                    .map(|(a, b)| (b - a).powi(2))
                    .sum::<f64>()
                    .sqrt();
                checks.push(numeric_check(
                    &format!("Conservação do comprimento 3-4-5, escala {scale}"),
                    length,
                    5.0 * scale,
                    "m",
                ));
                checks.push(numeric_check(
                    &format!("Limite mínimo X, escala {scale}"),
                    output.minimum_m[0],
                    expected[0],
                    "m",
                ));
                checks.push(numeric_check(
                    &format!("Limite máximo X, escala {scale}"),
                    output.maximum_m[0],
                    10.0,
                    "m",
                ));
            }
            result => checks.push(failed_check(
                "Transformação de referência",
                format!("Resposta inesperada: {result:?}"),
            )),
        }
    }
    let identity = adapter.transform(&[[0.0; 3], [1.0, -2.0, 3.0]], RigidTransform::default());
    checks.push(boolean_check(
        "Identidade preserva zero e sinais",
        identity
            .as_ref()
            .is_ok_and(|output| output.points_m == [[0.0; 3], [1.0, -2.0, 3.0]]),
        "Comparação direta com coordenadas originais.",
    ));
    let large = adapter.transform(
        &[[1.0, 2.0, 3.0]],
        RigidTransform {
            translation_m: [1.0e8, -1.0e8, 0.0],
            ..RigidTransform::default()
        },
    );
    checks.push(boolean_check(
        "Translação grande preserva deslocamentos locais inteiros",
        large.is_ok_and(|output| output.points_m == [[100000001.0, -99999998.0, 3.0]]),
        "Referência fechada em metros; nenhuma conversão para f32.",
    ));
    let invalid = [
        (
            "Conjunto vazio recusado",
            adapter.transform(&[], RigidTransform::default()).is_err(),
        ),
        (
            "NaN recusado",
            adapter
                .transform(&[[f64::NAN, 0.0, 0.0]], RigidTransform::default())
                .is_err(),
        ),
        (
            "Escala indevida recusada",
            adapter
                .transform(
                    &[[0.0; 3]],
                    RigidTransform {
                        rotation: [[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                        ..RigidTransform::default()
                    },
                )
                .is_err(),
        ),
        (
            "Reflexão recusada",
            adapter
                .transform(
                    &[[0.0; 3]],
                    RigidTransform {
                        rotation: [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                        ..RigidTransform::default()
                    },
                )
                .is_err(),
        ),
    ];
    for (name, passed) in invalid {
        checks.push(boolean_check(
            name,
            passed,
            "Entrada fora do domínio do ensaio deve produzir erro.",
        ));
    }
    checks
}

pub fn run_experiment() -> ExperimentReport {
    run_repeated(&NalgebraGeometry)
}

fn run_repeated(adapter: &dyn GeometryAdapter) -> ExperimentReport {
    let started = Instant::now();
    let mut checks = Vec::new();
    let mut first_failure = None;
    for _ in 0..ITERATIONS {
        let current = black_box(qualification_checks(adapter));
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

fn numeric_check(name: &str, actual: f64, expected: f64, unit: &str) -> QualificationCheck {
    let tolerance = ABSOLUTE_TOLERANCE_M + RELATIVE_TOLERANCE * expected.abs();
    let error = (actual - expected).abs();
    QualificationCheck {
        name: name.into(),
        passed: actual.is_finite() && error <= tolerance,
        detail: format!("Obtido {actual:.15e}; referência fechada {expected:.15e}."),
        absolute_error: Some(error),
        tolerance: Some(tolerance),
        unit: Some(unit.into()),
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

fn failed_check(name: &str, detail: String) -> QualificationCheck {
    boolean_check(name, false, &detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_passes_closed_reference_and_invalid_input_battery() {
        let checks = qualification_checks(&NalgebraGeometry);
        assert!(checks.len() >= 24);
        assert!(checks.iter().all(|check| check.passed), "{checks:#?}");
    }

    #[test]
    fn multiplication_order_is_rotation_then_translation_and_inverse_restores_points() {
        let original = [[2.0, 3.0, -4.0], [-1.0, 0.0, 0.0]];
        let forward = RigidTransform {
            rotation: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            translation_m: [10.0, -20.0, 30.0],
        };
        let transformed = NalgebraGeometry.transform(&original, forward).unwrap();
        assert_eq!(transformed.points_m[0], [7.0, -18.0, 26.0]);
        let inverse = RigidTransform {
            rotation: [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            translation_m: [20.0, 10.0, -30.0],
        };
        assert_eq!(
            NalgebraGeometry
                .transform(&transformed.points_m, inverse)
                .unwrap()
                .points_m,
            original
        );
    }

    #[test]
    fn domain_limits_and_non_finite_transform_fail_without_partial_results() {
        assert!(
            NalgebraGeometry
                .transform(&vec![[0.0; 3]; MAX_POINTS + 1], RigidTransform::default())
                .is_err()
        );
        assert!(
            NalgebraGeometry
                .transform(
                    &[[1.0e9, 0.0, 0.0]],
                    RigidTransform {
                        translation_m: [1.0, 0.0, 0.0],
                        ..RigidTransform::default()
                    }
                )
                .is_err()
        );
        assert!(
            NalgebraGeometry
                .transform(
                    &[[0.0; 3]],
                    RigidTransform {
                        translation_m: [f64::INFINITY, 0.0, 0.0],
                        ..RigidTransform::default()
                    }
                )
                .is_err()
        );
    }

    struct WrongAdapter;
    impl GeometryAdapter for WrongAdapter {
        fn transform(
            &self,
            points: &[[f64; 3]],
            _: RigidTransform,
        ) -> Result<GeometryOutput, GeometryError> {
            Ok(GeometryOutput {
                points_m: points.to_vec(),
                minimum_m: [0.0; 3],
                maximum_m: [0.0; 3],
            })
        }
    }

    #[test]
    fn the_same_qualification_battery_detects_a_wrong_replacement() {
        assert!(
            qualification_checks(&WrongAdapter)
                .iter()
                .any(|check| !check.passed)
        );
    }

    struct IntermittentAdapter(std::cell::Cell<bool>);
    impl GeometryAdapter for IntermittentAdapter {
        fn transform(
            &self,
            points: &[[f64; 3]],
            transform: RigidTransform,
        ) -> Result<GeometryOutput, GeometryError> {
            if !self.0.replace(true) {
                WrongAdapter.transform(points, transform)
            } else {
                NalgebraGeometry.transform(points, transform)
            }
        }
    }

    #[test]
    fn measurement_preserves_failure_even_when_all_later_iterations_pass() {
        let report = run_repeated(&IntermittentAdapter(std::cell::Cell::new(false)));
        assert_eq!(report.iterations, ITERATIONS);
        assert!(!report.passed());
        assert!(!report.checks[0].passed);
    }
}
