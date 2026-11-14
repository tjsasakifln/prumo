//! Bounded STEP syntax and local IFC geometry experiment.
//!
//! This is an inspection profile, not a general IFC importer. Product placement,
//! B-rep, schema conformance, global GUIDs and analytical semantics are not inferred.

use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
    time::Instant,
};

use prumo_domain::components::{ComponentDescriptor, ExperimentReport, QualificationCheck};
use ruststep::ast::{EntityInstance, Exchange, Name, Parameter, Record};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MAX_INPUT_BYTES: usize = 256 * 1024;
pub const MAX_NESTING: usize = 32;
pub const MAX_ENTITIES: usize = 2048;
pub const LENGTH_TOLERANCE_M: f64 = 1e-9;
pub const FIXTURE: &str = include_str!("../fixtures/bim-local-ifc4.ifc");

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum BimError {
    #[error("O arquivo excede o limite de tamanho, entidades ou profundidade do ensaio.")]
    ResourceLimit,
    #[error("Sintaxe STEP inválida ou conteúdo após o encerramento do arquivo.")]
    Syntax,
    #[error("O perfil aceita somente IFC4 ASCII, uma seção DATA e referências locais simples.")]
    UnsupportedProfile,
    #[error("Escapes STEP de texto ainda não são aceitos pelo parser neste perfil de ensaio.")]
    UnsupportedTextEncoding,
    #[error("Identidade STEP inválida ou repetida: #{0}.")]
    DuplicateIdentity(u64),
    #[error("Referência STEP sem entidade correspondente: #{0}.")]
    DanglingReference(u64),
    #[error("Unidade de comprimento ausente, ambígua ou fora do perfil SI do ensaio.")]
    UnsupportedUnit,
    #[error("Entidade #{id} inválida: {reason}.")]
    InvalidEntity { id: u64, reason: &'static str },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BimPoint {
    pub step_id: u64,
    pub coordinates_m: [f64; 3],
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BimPolyline {
    pub step_id: u64,
    pub point_ids: Vec<u64>,
    pub length_m: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BimInspection {
    pub schema: String,
    pub source_length_unit: String,
    pub coordinate_space: String,
    pub record_count: usize,
    pub points: Vec<BimPoint>,
    pub polylines: Vec<BimPolyline>,
    pub uninterpreted_entity_types: Vec<String>,
    pub complete_ifc_validation: bool,
}

pub trait BimAdapter: Send + Sync {
    fn inspect(&self, source: &str) -> Result<BimInspection, BimError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RuststepAdapter;

impl BimAdapter for RuststepAdapter {
    fn inspect(&self, source: &str) -> Result<BimInspection, BimError> {
        preflight(source)?;
        // Unlike parser::parse, FromStr requires the entire input to be consumed.
        let exchange = Exchange::from_str(source).map_err(|_| BimError::Syntax)?;
        if exchange.data.len() != 1
            || !exchange.anchor.is_empty()
            || !exchange.reference.is_empty()
            || !exchange.signature.is_empty()
        {
            return Err(BimError::UnsupportedProfile);
        }
        for name in ["FILE_DESCRIPTION", "FILE_NAME", "FILE_SCHEMA"] {
            if exchange
                .header
                .iter()
                .filter(|record| record.name == name)
                .count()
                != 1
            {
                return Err(BimError::UnsupportedProfile);
            }
        }
        let schema = exchange
            .header
            .iter()
            .find(|record| record.name == "FILE_SCHEMA")
            .ok_or(BimError::UnsupportedProfile)?;
        let schema_fields = fields(schema)?;
        let supported = matches!(schema_fields, [Parameter::List(names)]
            if matches!(names.as_slice(), [Parameter::String(name)] if name == "IFC4"));
        if !supported {
            return Err(BimError::UnsupportedProfile);
        }

        let entities = &exchange.data[0].entities;
        if entities.len() > MAX_ENTITIES {
            return Err(BimError::ResourceLimit);
        }
        let mut records = BTreeMap::new();
        for entity in entities {
            let EntityInstance::Simple { id, record } = entity else {
                return Err(BimError::UnsupportedProfile);
            };
            if *id == 0 || records.insert(*id, record).is_some() {
                return Err(BimError::DuplicateIdentity(*id));
            }
            if !record.name.starts_with("IFC") {
                return Err(BimError::UnsupportedProfile);
            }
        }
        for record in records.values() {
            validate_references(&record.parameter, &records)?;
        }
        let (source_length_unit, factor) = length_unit(&records)?;
        let mut points = BTreeMap::new();
        for (id, record) in &records {
            if record.name != "IFCCARTESIANPOINT" {
                continue;
            }
            let [Parameter::List(coordinates)] = fields(record)? else {
                return Err(invalid(*id, "coordenadas devem ser uma lista"));
            };
            if coordinates.len() != 3 {
                return Err(invalid(*id, "o perfil exige três coordenadas"));
            }
            let mut converted = [0.0; 3];
            for (index, coordinate) in coordinates.iter().enumerate() {
                let value = match coordinate {
                    Parameter::Real(value) => *value,
                    Parameter::Integer(value) => *value as f64,
                    _ => return Err(invalid(*id, "coordenada não numérica")),
                } * factor;
                if !value.is_finite() || value.abs() > 1e9 {
                    return Err(invalid(
                        *id,
                        "coordenada não finita ou fora do limite de 1e9 m",
                    ));
                }
                converted[index] = value;
            }
            points.insert(
                *id,
                BimPoint {
                    step_id: *id,
                    coordinates_m: converted,
                },
            );
        }
        let mut polylines = Vec::new();
        for (id, record) in &records {
            if record.name != "IFCPOLYLINE" {
                continue;
            }
            let [Parameter::List(references)] = fields(record)? else {
                return Err(invalid(*id, "a polilinha deve conter uma lista de pontos"));
            };
            if !(2..=256).contains(&references.len()) {
                return Err(invalid(*id, "a polilinha exige de dois a 256 pontos"));
            }
            let mut point_ids = Vec::new();
            for reference in references {
                let Parameter::Ref(Name::Entity(point_id)) = reference else {
                    return Err(invalid(*id, "ponto deve ser uma referência local"));
                };
                if !points.contains_key(point_id) {
                    return Err(invalid(
                        *id,
                        "referência não identifica um ponto cartesiano tridimensional",
                    ));
                }
                point_ids.push(*point_id);
            }
            let mut length_m = 0.0;
            for pair in point_ids.windows(2) {
                let start = points[&pair[0]].coordinates_m;
                let end = points[&pair[1]].coordinates_m;
                let length = (end[0] - start[0])
                    .hypot(end[1] - start[1])
                    .hypot(end[2] - start[2]);
                if length <= LENGTH_TOLERANCE_M {
                    return Err(invalid(*id, "segmento nulo ou abaixo da tolerância"));
                }
                length_m += length;
            }
            polylines.push(BimPolyline {
                step_id: *id,
                point_ids,
                length_m,
            });
        }
        if points.is_empty() || polylines.is_empty() {
            return Err(BimError::UnsupportedProfile);
        }
        let uninterpreted: BTreeSet<_> = records
            .values()
            .filter(|record| {
                !matches!(
                    record.name.as_str(),
                    "IFCCARTESIANPOINT" | "IFCPOLYLINE" | "IFCSIUNIT" | "IFCUNITASSIGNMENT"
                )
            })
            .map(|record| record.name.clone())
            .collect();
        Ok(BimInspection {
            schema: "IFC4".into(),
            source_length_unit,
            coordinate_space: "entity_local".into(),
            record_count: records.len(),
            points: points.into_values().collect(),
            polylines,
            uninterpreted_entity_types: uninterpreted.into_iter().collect(),
            complete_ifc_validation: false,
        })
    }
}

fn fields(record: &Record) -> Result<&[Parameter], BimError> {
    match &record.parameter {
        Parameter::List(fields) => Ok(fields),
        _ => Err(BimError::Syntax),
    }
}

fn invalid(id: u64, reason: &'static str) -> BimError {
    BimError::InvalidEntity { id, reason }
}

fn length_unit(records: &BTreeMap<u64, &Record>) -> Result<(String, f64), BimError> {
    let assignments: Vec<_> = records
        .iter()
        .filter(|(_, record)| record.name == "IFCUNITASSIGNMENT")
        .collect();
    let [assignment] = assignments.as_slice() else {
        return Err(BimError::UnsupportedUnit);
    };
    let assignment_id = *assignment.0;
    let assignment = *assignment.1;
    let projects: Vec<_> = records
        .values()
        .filter(|record| record.name == "IFCPROJECT")
        .collect();
    let [project] = projects.as_slice() else {
        return Err(BimError::UnsupportedUnit);
    };
    let [
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        Parameter::Ref(Name::Entity(project_unit_id)),
    ] = fields(project)?
    else {
        return Err(BimError::UnsupportedUnit);
    };
    if *project_unit_id != assignment_id {
        return Err(BimError::UnsupportedUnit);
    }
    let [Parameter::List(units)] = fields(assignment)? else {
        return Err(BimError::UnsupportedUnit);
    };
    let [Parameter::Ref(Name::Entity(id))] = units.as_slice() else {
        return Err(BimError::UnsupportedUnit);
    };
    let unit = records.get(id).ok_or(BimError::DanglingReference(*id))?;
    if unit.name != "IFCSIUNIT" {
        return Err(BimError::UnsupportedUnit);
    }
    let [
        Parameter::Omitted,
        Parameter::Enumeration(kind),
        prefix,
        Parameter::Enumeration(name),
    ] = fields(unit)?
    else {
        return Err(BimError::UnsupportedUnit);
    };
    if kind != "LENGTHUNIT" || name != "METRE" {
        return Err(BimError::UnsupportedUnit);
    }
    match prefix {
        Parameter::NotProvided => Ok(("metre".into(), 1.0)),
        Parameter::Enumeration(prefix) if prefix == "MILLI" => Ok(("millimetre".into(), 0.001)),
        Parameter::Enumeration(prefix) if prefix == "CENTI" => Ok(("centimetre".into(), 0.01)),
        _ => Err(BimError::UnsupportedUnit),
    }
}

fn validate_references(
    parameter: &Parameter,
    records: &BTreeMap<u64, &Record>,
) -> Result<(), BimError> {
    match parameter {
        Parameter::Ref(Name::Entity(id)) => {
            if !records.contains_key(id) {
                return Err(BimError::DanglingReference(*id));
            }
        }
        Parameter::Ref(_) => return Err(BimError::UnsupportedProfile),
        Parameter::List(parameters) => {
            for parameter in parameters {
                validate_references(parameter, records)?;
            }
        }
        Parameter::Typed { parameter, .. } => validate_references(parameter, records)?,
        _ => {}
    }
    Ok(())
}

/// Resource guard only; syntax is parsed by the upstream STEP implementation.
fn preflight(source: &str) -> Result<(), BimError> {
    if source.len() > MAX_INPUT_BYTES {
        return Err(BimError::ResourceLimit);
    }
    if !source.is_ascii() {
        return Err(BimError::UnsupportedProfile);
    }
    if source
        .bytes()
        .any(|byte| byte.is_ascii_control() && !matches!(byte, b'\t' | b'\r' | b'\n'))
    {
        return Err(BimError::Syntax);
    }
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut depth: usize = 0;
    let mut quoted = false;
    let mut comment = false;
    while index < bytes.len() {
        let pair = bytes.get(index..index + 2);
        if comment {
            if pair == Some(b"*/") {
                comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if quoted {
            if bytes[index].is_ascii_control() {
                return Err(BimError::Syntax);
            }
            if pair == Some(b"''") {
                return Err(BimError::UnsupportedTextEncoding);
            }
            if bytes[index] == b'\\' {
                return Err(BimError::UnsupportedTextEncoding);
            }
            if bytes[index] == b'\'' {
                quoted = false;
            }
            index += 1;
            continue;
        }
        if pair == Some(b"/*") {
            comment = true;
            index += 2;
            continue;
        }
        match bytes[index] {
            b'\'' => quoted = true,
            b'(' => {
                depth += 1;
                if depth > MAX_NESTING {
                    return Err(BimError::ResourceLimit);
                }
            }
            b')' => depth = depth.checked_sub(1).ok_or(BimError::Syntax)?,
            _ => {}
        }
        index += 1;
    }
    if quoted || comment || depth != 0 {
        return Err(BimError::Syntax);
    }
    Ok(())
}

pub fn descriptor() -> ComponentDescriptor {
    ComponentDescriptor {
        id: "bim-ruststep".into(), name: "ruststep".into(), version: "0.4.0".into(),
        origin: "https://github.com/ricosjp/ruststep/tree/697aacdcbbd1027d9628ea15c1af95c16a7490ba/ruststep".into(),
        license: "Apache-2.0".into(), language: "Rust".into(), boundary: "biblioteca Rust em worker isolado".into(),
        scope: "Inspeção STEP de pontos e polilinhas locais IFC4; identidade de entidade e unidade SI explícitas".into(),
        limitations: vec![
            "Ensaio experimental, sem importação de projetos ou validação completa do schema IFC".into(),
            "Somente ASCII, uma seção DATA, entidades simples e uma unidade de comprimento SI".into(),
            "Escapes STEP de texto, inclusive apóstrofo duplicado, não são suportados pelo parser avaliado".into(),
            "Não resolve placements, GUIDs, B-rep, propriedades, topologia BIM ou modelo analítico".into(),
            format!("Entrada limitada a {MAX_INPUT_BYTES} bytes, {MAX_ENTITIES} entidades e profundidade {MAX_NESTING}"),
        ],
    }
}

fn independent_reference_error(result: &BimInspection) -> Option<f64> {
    if result.points.len() != 3
        || result.polylines.len() != 2
        || result.source_length_unit != "millimetre"
        || result.coordinate_space != "entity_local"
        || result.complete_ifc_validation
    {
        return None;
    }
    let mut error = 0.0_f64;
    for (point, (id, coordinates)) in result.points.iter().zip([
        (11, [0.0, 0.0, 0.0]),
        (12, [3.0, 4.0, 0.0]),
        (13, [6.0, 4.0, 0.0]),
    ]) {
        if point.step_id != id {
            return None;
        }
        for (actual, expected) in point.coordinates_m.into_iter().zip(coordinates) {
            if !actual.is_finite() {
                return None;
            }
            error = error.max((actual - expected).abs());
        }
    }
    for (line, (id, point_ids, length)) in result
        .polylines
        .iter()
        .zip([(20, [11, 12], 5.0), (21, [12, 13], 3.0)])
    {
        if line.step_id != id || line.point_ids != point_ids || !line.length_m.is_finite() {
            return None;
        }
        error = error.max((line.length_m - length).abs());
    }
    Some(error)
}

pub fn run_experiment() -> ExperimentReport {
    const ITERATIONS: u32 = 20;
    let adapter = RuststepAdapter;
    let invalid = FIXTURE.replace("(#11,#12)", "(#11,#999)");
    let started = Instant::now();
    let mut repeated_ok = true;
    for _ in 0..ITERATIONS {
        repeated_ok &= adapter.inspect(FIXTURE).is_ok_and(|result| {
            independent_reference_error(&result).is_some_and(|error| error <= LENGTH_TOLERANCE_M)
        });
        repeated_ok &= matches!(
            adapter.inspect(&invalid),
            Err(BimError::DanglingReference(999))
        );
    }
    let elapsed_ns = started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
    let inspection = adapter.inspect(FIXTURE);
    let mut checks = vec![QualificationCheck {
        name: "bateria repetida com entrada válida e referência órfã".into(), passed: repeated_ok,
        detail: "20 pares executados pelo mesmo adaptador; referência independente definida antes da medição".into(),
        absolute_error: None, tolerance: None, unit: None,
    }];
    match inspection {
        Ok(result) => {
            let error = independent_reference_error(&result);
            checks.push(QualificationCheck {
                name: "unidades, identidades e comprimento independente".into(),
                passed: error.is_some_and(|error| error <= LENGTH_TOLERANCE_M)
                    && result.points.iter().map(|point| point.step_id).collect::<Vec<_>>() == [11, 12, 13]
                    && result.source_length_unit == "millimetre" && !result.complete_ifc_validation,
                detail: "Fixture manual: triângulo 3000/4000/5000 mm e trecho adicional 3000 mm; coordenadas locais em metros".into(),
                absolute_error: error, tolerance: Some(LENGTH_TOLERANCE_M), unit: Some("m".into()),
            });
        }
        Err(error) => checks.push(QualificationCheck {
            name: "leitura da fixture".into(),
            passed: false,
            detail: error.to_string(),
            absolute_error: None,
            tolerance: None,
            unit: None,
        }),
    }
    checks.push(QualificationCheck {
        name: "recusa de truncamento, unidade desconhecida e conteúdo posterior".into(),
        passed: adapter.inspect("ISO-10303-21;HEADER;").is_err()
            && matches!(
                adapter.inspect(&FIXTURE.replace(".MILLI.", ".KILO.")),
                Err(BimError::UnsupportedUnit)
            )
            && matches!(
                adapter.inspect(&(FIXTURE.to_owned() + "extraneous")),
                Err(BimError::Syntax)
            ),
        detail:
            "Sem valor padrão de unidade nem aceitação silenciosa de bytes após END-ISO-10303-21"
                .into(),
        absolute_error: None,
        tolerance: None,
        unit: None,
    });
    ExperimentReport {
        component: descriptor(),
        checks,
        iterations: ITERATIONS,
        elapsed_ns,
        artifact: None,
    }
}
