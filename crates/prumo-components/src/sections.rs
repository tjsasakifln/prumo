//! Qualified adapter for simple polygon sections. The shared catalogue kernel owns geometry.
//! The legacy adapter retains its original no-hole qualification boundary.

pub use prumo_domain::catalogue::sections::{
    MAX_COORDINATE_M, MAX_SPAN_M, MAX_VERTICES, MIN_EDGE_M, MIN_RELATIVE_AREA, SectionError,
    SectionGeometry as SectionProperties, SectionInput,
};
use prumo_domain::components::{ComponentDescriptor, ExperimentReport, QualificationCheck};
use std::hint::black_box;
use std::time::Instant;

const RELATIVE_TOLERANCE: f64 = 1.0e-10;
const AREA_ABSOLUTE_TOLERANCE_M2: f64 = 1.0e-20;
const CENTROID_ABSOLUTE_TOLERANCE_M: f64 = 1.0e-12;
const MOMENT_ABSOLUTE_TOLERANCE_M4: f64 = 1.0e-28;
const ITERATIONS: u32 = 64;

pub trait SectionAdapter {
    fn properties(&self, input: &SectionInput) -> Result<SectionProperties, SectionError>;
}
#[derive(Clone, Copy, Debug, Default)]
pub struct GeoSection;
impl SectionAdapter for GeoSection {
    fn properties(&self, input: &SectionInput) -> Result<SectionProperties, SectionError> {
        if !input.holes_m.is_empty() {
            return Err(SectionError::HolesUnsupported);
        }
        prumo_domain::catalogue::sections::polygon_properties(input)
    }
}

pub fn descriptor() -> ComponentDescriptor {
    ComponentDescriptor {
        id: "sections-geo".into(), name: "Seção poligonal com geo e integral central".into(), version: "0.33.1".into(),
        origin: "https://github.com/georust/geo".into(), license: "MIT OR Apache-2.0".into(), language: "Rust".into(), boundary: "crate Rust, interface segura".into(),
        scope: "Área, centroide e momentos centrais de área de um polígono simples, até 512 vértices, sem furos; coordenadas em m.".into(),
        limitations: vec!["geo valida topologia e calcula área/centroide; a integral dos momentos é código Rust próprio com contraprovas analíticas.".into(), "Experimento geométrico; não qualifica resistência, torção, empenamento, centro de cisalhamento ou plasticidade.".into(), "Sem furos, materiais compostos, curvas ou seções com área relativa menor ou igual a 1e-12.".into(), "Momentos em m⁴; Ixy usa integral positiva xy, com sinal definido pelas coordenadas.".into()],
    }
}

/// Analytic references are independent from the polygon integration implementation.
pub fn qualification_checks(adapter: &dyn SectionAdapter) -> Vec<QualificationCheck> {
    let mut checks = Vec::new();
    for scale in [0.001_f64, 1.0, 1000.0] {
        let rectangle = SectionInput {
            exterior_m: vec![
                [0.0, 0.0],
                [0.2 * scale, 0.0],
                [0.2 * scale, 0.6 * scale],
                [0.0, 0.6 * scale],
            ],
            holes_m: vec![],
        };
        // Closed rectangle expressions: A=bh, C=(b/2,h/2), Ix=bh^3/12, Iy=hb^3/12.
        let expected = SectionProperties {
            area_m2: 0.12 * scale.powi(2),
            centroid_m: [0.1 * scale, 0.3 * scale],
            ixx_m4: 0.0036 * scale.powi(4),
            iyy_m4: 0.0004 * scale.powi(4),
            ixy_m4: 0.0,
        };
        check_properties(
            adapter,
            &rectangle,
            &expected,
            &format!("Retângulo fechado, escala {scale}"),
            &mut checks,
        );
        let mut reversed = rectangle;
        reversed.exterior_m.reverse();
        check_properties(
            adapter,
            &reversed,
            &expected,
            &format!("Orientação reversa, escala {scale}"),
            &mut checks,
        );
    }
    let triangle = SectionInput {
        exterior_m: vec![[0.0, 0.0], [2.0, 0.0], [0.0, 3.0]],
        holes_m: vec![],
    };
    // Right triangle: A=bh/2, Ix=bh^3/36, Iy=hb^3/36, Ixy=-b^2 h^2/72.
    check_properties(
        adapter,
        &triangle,
        &SectionProperties {
            area_m2: 3.0,
            centroid_m: [2.0 / 3.0, 1.0],
            ixx_m4: 1.5,
            iyy_m4: 2.0 / 3.0,
            ixy_m4: -0.5,
        },
        "Triângulo com produto de área negativo",
        &mut checks,
    );
    let rotated = SectionInput {
        exterior_m: vec![[0.0, 0.0], [0.0, 2.0], [-3.0, 0.0]],
        holes_m: vec![],
    };
    check_properties(
        adapter,
        &rotated,
        &SectionProperties {
            area_m2: 3.0,
            centroid_m: [-1.0, 2.0 / 3.0],
            ixx_m4: 2.0 / 3.0,
            iyy_m4: 1.5,
            ixy_m4: 0.5,
        },
        "Triângulo girado 90 graus",
        &mut checks,
    );
    let concave = SectionInput {
        exterior_m: vec![
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 1.0],
            [1.0, 1.0],
            [1.0, 3.0],
            [0.0, 3.0],
        ],
        holes_m: vec![],
    };
    // Independent composition of disjoint rectangles 2x1 and 1x2, using parallel axes.
    check_properties(
        adapter,
        &concave,
        &SectionProperties {
            area_m2: 4.0,
            centroid_m: [0.75, 1.25],
            ixx_m4: 37.0 / 12.0,
            iyy_m4: 13.0 / 12.0,
            ixy_m4: -0.75,
        },
        "Seção L por composição de retângulos",
        &mut checks,
    );
    let large_origin = SectionInput {
        exterior_m: vec![
            [1.0e8, -1.0e8],
            [1.0e8 + 2.0, -1.0e8],
            [1.0e8 + 2.0, -1.0e8 + 4.0],
            [1.0e8, -1.0e8 + 4.0],
        ],
        holes_m: vec![],
    };
    check_properties(
        adapter,
        &large_origin,
        &SectionProperties {
            area_m2: 8.0,
            centroid_m: [1.0e8 + 1.0, -1.0e8 + 2.0],
            ixx_m4: 32.0 / 3.0,
            iyy_m4: 8.0 / 3.0,
            ixy_m4: 0.0,
        },
        "Origem grande com inércia local preservada",
        &mut checks,
    );
    let cases = [
        (
            "Furo explicitamente recusado",
            SectionInput {
                exterior_m: vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]],
                holes_m: vec![vec![[0.5, 0.5], [1.5, 0.5], [1.5, 1.5], [0.5, 1.5]]],
            },
        ),
        (
            "Autointerseção recusada",
            SectionInput {
                exterior_m: vec![[0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0]],
                holes_m: vec![],
            },
        ),
        (
            "Área quase degenerada recusada",
            SectionInput {
                exterior_m: vec![[0.0, 0.0], [1.0, 0.0], [2.0, 1.0e-13]],
                holes_m: vec![],
            },
        ),
        (
            "Vértice consecutivo repetido recusado",
            SectionInput {
                exterior_m: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
                holes_m: vec![],
            },
        ),
        (
            "Coordenada não finita recusada",
            SectionInput {
                exterior_m: vec![[0.0, 0.0], [f64::NAN, 0.0], [0.0, 1.0]],
                holes_m: vec![],
            },
        ),
        (
            "Dois pontos recusados",
            SectionInput {
                exterior_m: vec![[0.0, 0.0], [1.0, 0.0]],
                holes_m: vec![],
            },
        ),
    ];
    for (name, input) in cases {
        checks.push(boolean_check(
            name,
            adapter.properties(&input).is_err(),
            "Entrada fora do domínio deve produzir erro, sem preenchimento ou reparo silencioso.",
        ));
    }
    checks
}

fn check_properties(
    adapter: &dyn SectionAdapter,
    input: &SectionInput,
    expected: &SectionProperties,
    name: &str,
    checks: &mut Vec<QualificationCheck>,
) {
    match adapter.properties(input) {
        Ok(output) => {
            let inertia_scale = expected.ixx_m4.abs().max(expected.iyy_m4.abs());
            // Accuracy follows the local section, not its global project origin.
            // Otherwise a large translation would permit centimetres of centroid error.
            let centroid_local = [
                expected.centroid_m[0] - input.exterior_m[0][0],
                expected.centroid_m[1] - input.exterior_m[0][1],
            ];
            let values = [
                (
                    "área",
                    output.area_m2,
                    expected.area_m2,
                    AREA_ABSOLUTE_TOLERANCE_M2 + RELATIVE_TOLERANCE * expected.area_m2.abs(),
                    "m²",
                ),
                (
                    "centroide X",
                    output.centroid_m[0],
                    expected.centroid_m[0],
                    CENTROID_ABSOLUTE_TOLERANCE_M + RELATIVE_TOLERANCE * centroid_local[0].abs(),
                    "m",
                ),
                (
                    "centroide Y",
                    output.centroid_m[1],
                    expected.centroid_m[1],
                    CENTROID_ABSOLUTE_TOLERANCE_M + RELATIVE_TOLERANCE * centroid_local[1].abs(),
                    "m",
                ),
                (
                    "Ixx",
                    output.ixx_m4,
                    expected.ixx_m4,
                    MOMENT_ABSOLUTE_TOLERANCE_M4 + RELATIVE_TOLERANCE * expected.ixx_m4.abs(),
                    "m⁴",
                ),
                (
                    "Iyy",
                    output.iyy_m4,
                    expected.iyy_m4,
                    MOMENT_ABSOLUTE_TOLERANCE_M4 + RELATIVE_TOLERANCE * expected.iyy_m4.abs(),
                    "m⁴",
                ),
                (
                    "Ixy",
                    output.ixy_m4,
                    expected.ixy_m4,
                    MOMENT_ABSOLUTE_TOLERANCE_M4 + RELATIVE_TOLERANCE * inertia_scale,
                    "m⁴",
                ),
            ];
            for (property, actual, reference, tolerance, unit) in values {
                let error = (actual - reference).abs();
                checks.push(QualificationCheck {
                    name: format!("{name}, {property}"),
                    passed: actual.is_finite() && error <= tolerance,
                    detail: format!("Obtido {actual:.15e}; referência analítica {reference:.15e}."),
                    absolute_error: Some(error),
                    tolerance: Some(tolerance),
                    unit: Some(unit.into()),
                });
            }
        }
        result => checks.push(boolean_check(
            name,
            false,
            &format!("Propriedades inesperadas: {result:?}"),
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
        let current = black_box(qualification_checks(&GeoSection));
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
    fn adapter_passes_independent_shapes_orientation_scales_and_rejections() {
        let checks = qualification_checks(&GeoSection);
        assert!(checks.len() >= 60);
        assert!(checks.iter().all(|check| check.passed), "{checks:#?}");
    }

    #[test]
    fn optional_closing_vertex_is_preserved_semantically() {
        let mut input = SectionInput {
            exterior_m: vec![[0.0, 0.0], [2.0, 0.0], [0.0, 3.0]],
            holes_m: vec![],
        };
        let open = GeoSection.properties(&input).unwrap();
        input.exterior_m.push(input.exterior_m[0]);
        assert_eq!(GeoSection.properties(&input).unwrap(), open);
    }

    #[test]
    fn large_translation_does_not_change_local_inertia_or_local_centroid() {
        let input = SectionInput {
            exterior_m: vec![
                [1.0e8, -1.0e8],
                [1.0e8 + 2.0, -1.0e8],
                [1.0e8 + 2.0, -1.0e8 + 4.0],
                [1.0e8, -1.0e8 + 4.0],
            ],
            holes_m: vec![],
        };
        let result = GeoSection.properties(&input).unwrap();
        assert_eq!(result.centroid_m, [100000001.0, -99999998.0]);
        assert!((result.ixx_m4 - 32.0 / 3.0).abs() < 1.0e-12);
        assert!((result.iyy_m4 - 8.0 / 3.0).abs() < 1.0e-12);
    }

    #[test]
    fn bounds_and_degenerate_topology_are_checked_before_use() {
        assert!(matches!(
            GeoSection.properties(&SectionInput {
                exterior_m: vec![[0.0; 2]; MAX_VERTICES + 2],
                holes_m: vec![]
            }),
            Err(SectionError::VertexCount)
        ));
        assert!(matches!(
            GeoSection.properties(&SectionInput {
                exterior_m: vec![[0.0, 0.0], [2.0e6, 0.0], [0.0, 2.0e6]],
                holes_m: vec![]
            }),
            Err(SectionError::SpanExceeded)
        ));
        assert!(
            GeoSection
                .properties(&SectionInput {
                    exterior_m: vec![[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]],
                    holes_m: vec![]
                })
                .is_err()
        );
    }

    struct WrongAdapter;
    impl SectionAdapter for WrongAdapter {
        fn properties(&self, _: &SectionInput) -> Result<SectionProperties, SectionError> {
            Ok(SectionProperties {
                area_m2: 0.0,
                centroid_m: [0.0; 2],
                ixx_m4: 0.0,
                iyy_m4: 0.0,
                ixy_m4: 0.0,
            })
        }
    }

    #[test]
    fn same_battery_detects_an_inaccurate_replacement() {
        assert!(
            qualification_checks(&WrongAdapter)
                .iter()
                .any(|check| !check.passed)
        );
    }
}
