//! Closed-form targets are stored separately from the matrix experiment.
//! Nothing here assembles or solves a user's project.
use crate::*;
use prumo_components::solver::{LinearSolverAdapter, LinearSystem, NalgebraCholesky, SolverError};
use std::collections::{BTreeMap, BTreeSet};

pub fn reference_protocols() -> Result<Vec<PreparedProtocol>, VerificationError> {
    let drafts: Vec<CaseProtocol> =
        serde_json::from_str(include_str!("../fixtures/reference-protocols.json"))?;
    drafts.into_iter().map(|protocol| {
        let review = ProtocolReview {
            protocol_sha256: protocol.sha256()?,
            reviewer: "prumo-verification/protocol-validator".into(),
            competence_and_scope: "Verificação automática de schema, unidades, finitude e congelamento do protocolo de software; não é revisão profissional.".into(),
            record: "crates/prumo-verification/fixtures/reference-protocols.json; derivadas e revisão independente em docs/evidencias/PRU-008.md".into(),
            kind: ReviewKind::Software,
            independence_confirmed: true,
        };
        PreparedProtocol::new(protocol, review)
    }).collect()
}

pub fn run_reference_suite(context: RunContext) -> Result<SuiteReport, VerificationError> {
    // Every target and tolerance is parsed, validated, hashed and reviewed before any solver call.
    let protocols = reference_protocols()?;
    run_suite(
        "pru-008-analytical-fixtures-v1",
        &protocols,
        context,
        execute,
    )
}

fn engine() -> EngineIdentity {
    EngineIdentity {
        name: "Prumo fixture matrix adapter / nalgebra Cholesky".into(),
        version: "prumo-components 0.1.0; nalgebra 0.35.0".into(),
        implementation_lineage: BTreeSet::from([
            "prumo-fixture-matrix-assembly-v1".into(),
            "nalgebra-cholesky".into(),
        ]),
    }
}

fn execute(prepared: &PreparedProtocol) -> Result<CaseExecution, String> {
    let protocol = prepared.protocol();
    let input = |id: &str, unit: Unit| -> Result<f64, String> {
        protocol
            .inputs
            .get(id)
            .filter(|value| value.unit == unit)
            .map(|value| value.value)
            .ok_or_else(|| format!("Entrada {id} ausente ou em outra unidade."))
    };
    let outcome = match protocol.id.as_str() {
        "beam-cantilever-v1" => {
            let length = input("length", Unit::Metre)?;
            let e = input("young_modulus", Unit::Pascal)?;
            let inertia = input("inertia", Unit::MetreToFourth)?;
            let force = input("tip_force_down", Unit::Newton)?;
            let stiffness = e * inertia / length.powi(3);
            // q=[vertical displacement, L*clockwise rotation] in m;
            // generalized forces=[P, M/L] in N, so every matrix coefficient is N/m.
            let solution = NalgebraCholesky
                .solve(&LinearSystem {
                    matrix_n_per_m: vec![
                        vec![12.0 * stiffness, -6.0 * stiffness],
                        vec![-6.0 * stiffness, 4.0 * stiffness],
                    ],
                    load_n: vec![force, 0.0],
                })
                .map_err(|error| error.to_string())?;
            results([
                ("tip_down", solution.displacements_m[0], Unit::Metre),
                (
                    "tip_clockwise_rotation",
                    solution.displacements_m[1] / length,
                    Unit::Radian,
                ),
            ])
        }
        "truss-v-v1" => {
            let half_span = input("half_span", Unit::Metre)?;
            let height = input("height", Unit::Metre)?;
            let e = input("young_modulus", Unit::Pascal)?;
            let area = input("area", Unit::SquareMetre)?;
            let force = input("apex_force_down", Unit::Newton)?;
            let length = half_span.hypot(height);
            let k = e * area / length;
            let c = half_span / length;
            let s = height / length;
            let solution = NalgebraCholesky
                .solve(&LinearSystem {
                    matrix_n_per_m: vec![vec![2.0 * k * c * c, 0.0], vec![0.0, 2.0 * k * s * s]],
                    load_n: vec![0.0, force],
                })
                .map_err(|error| error.to_string())?;
            results([
                ("apex_horizontal", solution.displacements_m[0], Unit::Metre),
                ("apex_down", solution.displacements_m[1], Unit::Metre),
            ])
        }
        "frame-l-v1" => {
            let height = input("column_height", Unit::Metre)?;
            let span = input("beam_span", Unit::Metre)?;
            let e = input("young_modulus", Unit::Pascal)?;
            let area = input("area", Unit::SquareMetre)?;
            let inertia = input("inertia", Unit::MetreToFourth)?;
            let force = input("tip_force_y", Unit::Newton)?;
            let nodes = [[0.0, 0.0], [0.0, height], [span, height]];
            let mut matrix = vec![vec![0.0; 9]; 9];
            for (start, end) in [(0, 1), (1, 2)] {
                assemble_frame_member(&mut matrix, &nodes, start, end, e * area, e * inertia);
            }
            // Rotations are represented by q_theta=(1 m)*theta and conjugate M/(1 m).
            // This keeps the algebra adapter's input contract in N/m, N and m.
            let reduced = matrix
                .iter()
                .skip(3)
                .map(|row| row.iter().skip(3).copied().collect())
                .collect();
            let solution = NalgebraCholesky
                .solve(&LinearSystem {
                    matrix_n_per_m: reduced,
                    load_n: vec![0.0, 0.0, 0.0, 0.0, force, 0.0],
                })
                .map_err(|error| error.to_string())?;
            results([
                ("tip_x", solution.displacements_m[3], Unit::Metre),
                ("tip_y", solution.displacements_m[4], Unit::Metre),
                ("tip_rotation", solution.displacements_m[5], Unit::Radian),
            ])
        }
        "mechanism-v1" => {
            let stiffness = input("dimensionless_coefficient", Unit::One)?;
            match NalgebraCholesky.solve(&LinearSystem {
                matrix_n_per_m: vec![vec![stiffness, stiffness], vec![stiffness, stiffness]],
                load_n: vec![1.0, 0.0],
            }) {
                Err(SolverError::NotPositiveDefinite) => ObservedOutcome::Rejection {
                    code: "not_positive_definite".into(),
                    detail: "Sistema singular recusado sem inserir apoio ou rigidez artificial."
                        .into(),
                },
                Err(error) => return Err(format!("Diagnóstico inesperado do mecanismo: {error}")),
                Ok(solution) => results([(
                    "unexpected_displacement",
                    solution.displacements_m[0],
                    Unit::Metre,
                )]),
            }
        }
        _ => {
            return Err("O executor de fixtures não executa casos arbitrários ou projetos.".into());
        }
    };
    Ok(CaseExecution {
        protocol_sha256: prepared.sha256().into(),
        engine: engine(),
        outcome,
    })
}

fn results<const N: usize>(items: [(&str, f64, Unit); N]) -> ObservedOutcome {
    ObservedOutcome::Results {
        quantities: items
            .into_iter()
            .map(|(id, value, unit)| (id.into(), QuantityValue { value, unit }))
            .collect(),
        classifications: BTreeMap::new(),
    }
}

fn assemble_frame_member(
    matrix: &mut [Vec<f64>],
    nodes: &[[f64; 2]; 3],
    start: usize,
    end: usize,
    ea: f64,
    ei: f64,
) {
    let dx = nodes[end][0] - nodes[start][0];
    let dy = nodes[end][1] - nodes[start][1];
    let length = dx.hypot(dy);
    let c = dx / length;
    let s = dy / length;
    let a = ea / length;
    let b = 12.0 * ei / length.powi(3);
    let f = 6.0 * ei / length.powi(2);
    let d = 4.0 * ei / length;
    let g = 2.0 * ei / length;
    let local = [
        [a, 0.0, 0.0, -a, 0.0, 0.0],
        [0.0, b, f, 0.0, -b, f],
        [0.0, f, d, 0.0, -f, g],
        [-a, 0.0, 0.0, a, 0.0, 0.0],
        [0.0, -b, -f, 0.0, b, -f],
        [0.0, f, g, 0.0, -f, d],
    ];
    let transform = [
        [c, s, 0.0, 0.0, 0.0, 0.0],
        [-s, c, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, c, s, 0.0],
        [0.0, 0.0, 0.0, -s, c, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    ];
    let indices = [
        3 * start,
        3 * start + 1,
        3 * start + 2,
        3 * end,
        3 * end + 1,
        3 * end + 2,
    ];
    for (i, &row) in indices.iter().enumerate() {
        for (j, &column) in indices.iter().enumerate() {
            let mut value = 0.0;
            for (k, transform_row) in transform.iter().enumerate() {
                for (l, other_row) in transform.iter().enumerate() {
                    value += transform_row[i] * local[k][l] * other_row[j];
                }
            }
            matrix[row][column] += value;
        }
    }
}
