//! PRU-024: explicit in-plane rigid-diaphragm kinematic relations.
//! Positions and displacement coefficients are SI. Flexible and semirigid
//! diaphragms are never silently replaced with rigid constraints.
use crate::{
    AnalysisError,
    constraints::{Constraint, Master},
    ensure,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiaphragmMode {
    Rigid,
    Flexible,
    Semirigid,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiaphragmPoint {
    pub dofs: [usize; 3],
    pub position_m: [f64; 2],
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Diaphragm {
    pub master: DiaphragmPoint,
    pub slaves: Vec<DiaphragmPoint>,
    pub mode: DiaphragmMode,
    pub justification: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Proposal {
    pub constraints: Vec<Constraint>,
    pub warnings: Vec<String>,
    pub analysis_approved: bool,
}
pub fn propose(input: &Diaphragm) -> Result<Proposal, AnalysisError> {
    ensure(
        !input.justification.trim().is_empty() && input.slaves.len() <= 100,
        "Justificativa ou número de nós do diafragma inválidos.",
    )?;
    ensure(
        input.master.position_m.iter().all(|x| x.is_finite())
            && input
                .slaves
                .iter()
                .all(|x| x.position_m.iter().all(|p| p.is_finite())),
        "Coordenadas do diafragma inválidas.",
    )?;
    let mut distinct = std::collections::BTreeSet::new();
    for dof in input.master.dofs {
        ensure(distinct.insert(dof), "Graus de liberdade mestre repetidos.")?;
    }
    for s in &input.slaves {
        for dof in s.dofs {
            ensure(
                distinct.insert(dof),
                "O mesmo grau pertence a mais de um nó ou relação.",
            )?;
        }
    }
    if matches!(input.mode, DiaphragmMode::Semirigid) {
        return Err(AnalysisError::Invalid(
            "Diafragma semirrígido exige representação de rigidez de membrana e não recebe vínculos rígidos.".into()));
    }
    if matches!(input.mode, DiaphragmMode::Flexible) {
        return Ok(Proposal {
            constraints: vec![],
            warnings: vec![
                "Diafragma flexível: nenhum vínculo multiponto criado. Exige membrana qualificada."
                    .into(),
            ],
            analysis_approved: false,
        });
    }
    let mut constraints = Vec::new();
    for slave in &input.slaves {
        let dx = slave.position_m[0] - input.master.position_m[0];
        let dy = slave.position_m[1] - input.master.position_m[1];
        ensure(
            dx.is_finite() && dy.is_finite() && dx.abs() <= 1e8 && dy.abs() <= 1e8,
            "Excentricidade de diafragma excessiva.",
        )?;
        let mut mx = vec![Master {
            dof: input.master.dofs[0],
            coefficient: 1.0,
        }];
        if dy.abs() > 1e-12 {
            mx.push(Master {
                dof: input.master.dofs[2],
                coefficient: -dy,
            });
        }
        let mut my = vec![Master {
            dof: input.master.dofs[1],
            coefficient: 1.0,
        }];
        if dx.abs() > 1e-12 {
            my.push(Master {
                dof: input.master.dofs[2],
                coefficient: dx,
            });
        }
        for (axis, masters) in [
            mx,
            my,
            vec![Master {
                dof: input.master.dofs[2],
                coefficient: 1.0,
            }],
        ]
        .into_iter()
        .enumerate()
        {
            constraints.push(Constraint {
                slave_dof: slave.dofs[axis],
                masters,
                offset_m: 0.0,
                justification: input.justification.clone(),
            });
        }
    }
    Ok(Proposal{
        constraints,
        warnings:vec![
            "A hipótese de rigidez infinita não foi verificada contra deformação de membrana. Conferir diafragma flexível/semirrígido antes de cálculo final.".into(),
            "A compatibilidade de deslocamentos é imposta em plano XY com três graus de liberdade por nó.".into(),
        ],
        analysis_approved:false,
    })
}
