//! Replaceable vector document experiment with explicit units and scale.
//!
//! It produces an inspection sheet, not a technical report or approved drawing.

use std::time::Instant;

use prumo_domain::components::{
    ComponentDescriptor, ExperimentReport, ProbeArtifact, QualificationCheck,
};
use serde::{Deserialize, Serialize};
use svg::{
    Document,
    node::element::{Circle, Group, Line, Rectangle, Text},
};
use thiserror::Error;

pub const SHEET_WIDTH_MM: f64 = 297.0;
pub const SHEET_HEIGHT_MM: f64 = 210.0;
pub const MAX_SEGMENTS: usize = 8;
pub const PROJECTION_TOLERANCE_MM: f64 = 1e-9;
const ORIGIN_MM: [f64; 2] = [35.0, 145.0];
const MAX_SPAN_MM: [f64; 2] = [125.0, 75.0];

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum DocumentError {
    #[error("Título ou identificação de segmento inválido para a prancha de ensaio.")]
    InvalidText,
    #[error("O ensaio exige revisão positiva, escala de 1 a 5000 e de um a oito segmentos.")]
    InvalidProfile,
    #[error("Coordenadas devem ser finitas e segmentos devem ter comprimento positivo.")]
    InvalidGeometry,
    #[error("A geometria não cabe na folha com a escala solicitada; a escala não foi alterada.")]
    OutsideSheet,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DrawingSegment {
    pub label: String,
    pub start_m: [f64; 2],
    pub end_m: [f64; 2],
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentInput {
    pub title: String,
    pub source_revision: u64,
    pub scale_denominator: u32,
    pub segments: Vec<DrawingSegment>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectedSegment {
    pub label: String,
    pub start_mm: [f64; 2],
    pub end_mm: [f64; 2],
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentArtifact {
    pub media_type: String,
    pub width_mm: f64,
    pub height_mm: f64,
    pub scale_denominator: u32,
    pub projected_segments: Vec<ProjectedSegment>,
    pub content: String,
}

pub trait DocumentAdapter: Send + Sync {
    fn render(&self, input: &DocumentInput) -> Result<DocumentArtifact, DocumentError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SvgDocumentAdapter;

impl DocumentAdapter for SvgDocumentAdapter {
    fn render(&self, input: &DocumentInput) -> Result<DocumentArtifact, DocumentError> {
        validate_text(&input.title, 40)?;
        if input.source_revision == 0
            || !(1..=5000).contains(&input.scale_denominator)
            || !(1..=MAX_SEGMENTS).contains(&input.segments.len())
        {
            return Err(DocumentError::InvalidProfile);
        }
        let mut minimum = [f64::INFINITY; 2];
        let mut maximum = [f64::NEG_INFINITY; 2];
        for segment in &input.segments {
            validate_text(&segment.label, 24)?;
            if segment
                .start_m
                .iter()
                .chain(&segment.end_m)
                .any(|value| !value.is_finite() || value.abs() > 1e9)
                || (segment.end_m[0] - segment.start_m[0])
                    .hypot(segment.end_m[1] - segment.start_m[1])
                    <= 1e-9
            {
                return Err(DocumentError::InvalidGeometry);
            }
            for point in [segment.start_m, segment.end_m] {
                for axis in 0..2 {
                    minimum[axis] = minimum[axis].min(point[axis]);
                    maximum[axis] = maximum[axis].max(point[axis]);
                }
            }
        }
        let factor = 1000.0 / f64::from(input.scale_denominator);
        for axis in 0..2 {
            if (maximum[axis] - minimum[axis]) * factor > MAX_SPAN_MM[axis] {
                return Err(DocumentError::OutsideSheet);
            }
        }
        let project = |point: [f64; 2]| {
            [
                ORIGIN_MM[0] + (point[0] - minimum[0]) * factor,
                ORIGIN_MM[1] - (point[1] - minimum[1]) * factor,
            ]
        };
        let projected: Vec<_> = input
            .segments
            .iter()
            .map(|segment| ProjectedSegment {
                label: segment.label.clone(),
                start_mm: project(segment.start_m),
                end_mm: project(segment.end_m),
            })
            .collect();

        // Only fixed SVG element/attribute names are produced. User text enters
        // text nodes, escaped by the upstream composer. No links or scripts exist.
        let mut document = Document::new()
            .set("viewBox", (0, 0, SHEET_WIDTH_MM, SHEET_HEIGHT_MM))
            .set("width", "297mm").set("height", "210mm")
            .set("role", "img").set("aria-labelledby", "sheet-title sheet-description")
            .add(svg::node::element::Title::new(input.title.clone()).set("id", "sheet-title"))
            .add(svg::node::element::Description::new().add(svg::node::Text::new("Prancha de ensaio de componentes. Sem cálculo, verificação normativa ou assinatura técnica.")).set("id", "sheet-description"))
            .add(Rectangle::new().set("width", SHEET_WIDTH_MM).set("height", SHEET_HEIGHT_MM).set("fill", "#ffffff"))
            .add(Rectangle::new().set("x", 12).set("y", 12).set("width", 273).set("height", 186).set("rx", 2).set("fill", "none").set("stroke", "#b8c4d4").set("stroke-width", 0.35))
            .add(text(22.0, 29.0, 8.0, "PRUMO", "#162b48").set("font-weight", "700"))
            .add(text(22.0, 40.0, 5.3, input.title.clone(), "#162b48"))
            .add(text(22.0, 49.0, 3.4, "Qualificação de componente | geometria de controle", "#55657a"))
            .add(Line::new().set("x1", 22).set("y1", 56).set("x2", 275).set("y2", 56).set("stroke", "#b8c4d4").set("stroke-width", 0.35));
        let mut drawing = Group::new().set("id", "drawing");
        for x in 0..=12 {
            drawing = drawing.add(
                Line::new()
                    .set("x1", ORIGIN_MM[0] + f64::from(x) * 10.0)
                    .set("y1", 70)
                    .set("x2", ORIGIN_MM[0] + f64::from(x) * 10.0)
                    .set("y2", ORIGIN_MM[1])
                    .set("stroke", "#e8edf3")
                    .set("stroke-width", 0.2),
            );
        }
        for y in 0..=7 {
            drawing = drawing.add(
                Line::new()
                    .set("x1", ORIGIN_MM[0])
                    .set("y1", ORIGIN_MM[1] - f64::from(y) * 10.0)
                    .set("x2", 160)
                    .set("y2", ORIGIN_MM[1] - f64::from(y) * 10.0)
                    .set("stroke", "#e8edf3")
                    .set("stroke-width", 0.2),
            );
        }
        for (index, segment) in projected.iter().enumerate() {
            let dx = segment.end_mm[0] - segment.start_mm[0];
            let dy = segment.end_mm[1] - segment.start_mm[1];
            let length = dx.hypot(dy);
            let label = [
                (segment.start_mm[0] + segment.end_mm[0]) / 2.0 - 4.0 * dy / length,
                (segment.start_mm[1] + segment.end_mm[1]) / 2.0 + 4.0 * dx / length,
            ];
            drawing = drawing
                .add(
                    Line::new()
                        .set("id", format!("segment-{index}"))
                        .set("x1", segment.start_mm[0])
                        .set("y1", segment.start_mm[1])
                        .set("x2", segment.end_mm[0])
                        .set("y2", segment.end_mm[1])
                        .set("stroke", "#146e8c")
                        .set("stroke-width", 0.9)
                        .set("stroke-linecap", "round"),
                )
                .add(
                    Circle::new()
                        .set("cx", label[0])
                        .set("cy", label[1])
                        .set("r", 2.4)
                        .set("fill", "#ffffff"),
                )
                .add(
                    text(
                        label[0],
                        label[1] + 1.1,
                        3.2,
                        format!("{}", index + 1),
                        "#146e8c",
                    )
                    .set("text-anchor", "middle"),
                );
            for point in [segment.start_mm, segment.end_mm] {
                drawing = drawing.add(
                    Circle::new()
                        .set("cx", point[0])
                        .set("cy", point[1])
                        .set("r", 1.0)
                        .set("fill", "#ffffff")
                        .set("stroke", "#146e8c")
                        .set("stroke-width", 0.5),
                );
            }
        }
        document = document
            .add(drawing)
            .add(text(
                30.0,
                162.0,
                3.2,
                "Coordenadas locais em metros; folha em milímetros",
                "#55657a",
            ))
            .add(text(183.0, 72.0, 4.3, "CONDIÇÕES DO ENSAIO", "#162b48").set("font-weight", "700"))
            .add(text(
                183.0,
                83.0,
                3.5,
                format!("Escala 1:{}", input.scale_denominator),
                "#334760",
            ))
            .add(text(
                183.0,
                92.0,
                3.5,
                format!("Revisão da entrada: {}", input.source_revision),
                "#334760",
            ))
            .add(text(
                183.0,
                103.0,
                3.5,
                "Segmentos de referência",
                "#334760",
            ));
        for (index, segment) in input.segments.iter().enumerate() {
            document = document.add(text(
                183.0,
                111.0 + index as f64 * 6.0,
                3.1,
                format!("{}. {}", index + 1, segment.label),
                "#55657a",
            ));
        }
        document = document
            .add(
                Rectangle::new()
                    .set("x", 22)
                    .set("y", 173)
                    .set("width", 253)
                    .set("height", 15)
                    .set("rx", 1)
                    .set("fill", "#f0f5f9"),
            )
            .add(
                text(27.0, 179.5, 3.4, "ARTEFATO DE QUALIFICAÇÃO", "#162b48")
                    .set("font-weight", "700"),
            )
            .add(text(
                27.0,
                185.0,
                3.0,
                "Sem cálculo de engenharia, aprovação normativa ou assinatura técnica.",
                "#55657a",
            ));
        Ok(DocumentArtifact {
            media_type: "image/svg+xml".into(),
            width_mm: SHEET_WIDTH_MM,
            height_mm: SHEET_HEIGHT_MM,
            scale_denominator: input.scale_denominator,
            projected_segments: projected,
            content: document.to_string(),
        })
    }
}

fn text(x: f64, y: f64, size: f64, value: impl Into<String>, color: &str) -> Text {
    Text::new(value)
        .set("x", x)
        .set("y", y)
        .set("font-size", size)
        .set("font-family", "DejaVu Sans, sans-serif")
        .set("fill", color)
}

fn validate_text(value: &str, limit: usize) -> Result<(), DocumentError> {
    if value.trim().is_empty()
        || value.chars().count() > limit
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{fffe}' | '\u{ffff}'))
    {
        return Err(DocumentError::InvalidText);
    }
    Ok(())
}

pub fn fixture_input() -> DocumentInput {
    DocumentInput {
        title: "Geometria de controle 3-4-5".into(),
        source_revision: 1,
        scale_denominator: 100,
        segments: vec![
            DrawingSegment {
                label: "Trecho A: 5,000 m".into(),
                start_m: [0.0, 0.0],
                end_m: [3.0, 4.0],
            },
            DrawingSegment {
                label: "Trecho B: 3,000 m".into(),
                start_m: [3.0, 4.0],
                end_m: [6.0, 4.0],
            },
        ],
    }
}

pub fn descriptor() -> ComponentDescriptor {
    ComponentDescriptor {
        id: "documents-svg".into(), name: "svg".into(), version: "0.18.0".into(),
        origin: "https://github.com/bodoni/svg/tree/859f49844757616bb5d40be8b5149325d7c9f383".into(),
        license: "Apache-2.0 OR MIT".into(), language: "Rust".into(), boundary: "biblioteca Rust em worker isolado".into(),
        scope: "Prancha SVG experimental de oito segmentos no máximo, A4 horizontal, escala e unidades explícitas".into(),
        limitations: vec![
            "Não produz memorial, PDF, prancha executiva ou documento aprovado".into(),
            "Fontes não incorporadas; a geometria vetorial é conferida separadamente da tipografia".into(),
            "Recusa geometria fora da folha sem alterar escala silenciosamente".into(),
            "Saída sem scripts, imagens, links, CSS externo ou acesso a arquivos".into(),
        ],
    }
}

fn independent_reference_error(artifact: &DocumentArtifact) -> Option<f64> {
    if artifact.projected_segments.len() != 2
        || artifact.scale_denominator != 100
        || artifact.width_mm != 297.0
        || artifact.height_mm != 210.0
        || !artifact.content.contains("width=\"297mm\"")
        || !artifact.content.contains("height=\"210mm\"")
    {
        return None;
    }
    // Independently specified paper coordinates: 1 m at 1:100 = 10 mm.
    let expected = [
        ([35.0, 145.0], [65.0, 105.0]),
        ([65.0, 105.0], [95.0, 105.0]),
    ];
    let mut error = 0.0_f64;
    for (segment, expected) in artifact.projected_segments.iter().zip(expected) {
        for (actual, expected) in segment
            .start_mm
            .into_iter()
            .chain(segment.end_mm)
            .zip(expected.0.into_iter().chain(expected.1))
        {
            if !actual.is_finite() {
                return None;
            }
            error = error.max((actual - expected).abs());
        }
    }
    Some(error)
}

pub fn run_experiment() -> ExperimentReport {
    const ITERATIONS: u32 = 100;
    let adapter = SvgDocumentAdapter;
    let input = fixture_input();
    let mut invalid = input.clone();
    invalid.segments[0].end_m[0] = f64::NAN;
    let started = Instant::now();
    let mut repeated_ok = true;
    for _ in 0..ITERATIONS {
        repeated_ok &= adapter.render(&input).is_ok_and(|artifact| {
            independent_reference_error(&artifact)
                .is_some_and(|error| error <= PROJECTION_TOLERANCE_MM)
        });
        repeated_ok &= adapter.render(&invalid) == Err(DocumentError::InvalidGeometry);
    }
    let elapsed_ns = started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
    let mut checks = vec![QualificationCheck {
        name: "bateria repetida de composição e entrada não finita".into(),
        passed: repeated_ok,
        detail: "100 pares da mesma fixture; entrada inválida não produz documento".into(),
        absolute_error: None,
        tolerance: None,
        unit: None,
    }];
    let artifact = match adapter.render(&input) {
        Ok(artifact) => {
            let error = independent_reference_error(&artifact);
            checks.push(QualificationCheck {
                name: "dimensões da folha e projeção independente".into(),
                passed: error.is_some_and(|error| error <= PROJECTION_TOLERANCE_MM),
                detail: "Folha 297 x 210 mm, escala 1:100, comprimentos de papel 50 e 30 mm".into(),
                absolute_error: error,
                tolerance: Some(PROJECTION_TOLERANCE_MM),
                unit: Some("mm".into()),
            });
            Some(ProbeArtifact {
                media_type: artifact.media_type,
                content: artifact.content,
            })
        }
        Err(error) => {
            checks.push(QualificationCheck {
                name: "composição da fixture".into(),
                passed: false,
                detail: error.to_string(),
                absolute_error: None,
                tolerance: None,
                unit: None,
            });
            None
        }
    };
    let mut escaped = input.clone();
    escaped.title = "Controle <script>x</script> & BIM".into();
    let escaped_ok = adapter.render(&escaped).is_ok_and(|artifact| {
        artifact
            .content
            .contains("&lt;script&gt;x&lt;/script&gt; &amp; BIM")
            && !artifact.content.contains("<script>")
            && !artifact.content.contains("href=")
    });
    let mut outside = input;
    outside.scale_denominator = 1;
    checks.push(QualificationCheck {
        name: "texto escapado e escala preservada na recusa".into(),
        passed: escaped_ok && adapter.render(&outside) == Err(DocumentError::OutsideSheet),
        detail: "Conteúdo inserido é texto; a geometria que excede a folha é recusada sem ajuste automático".into(),
        absolute_error: None, tolerance: None, unit: None,
    });
    ExperimentReport {
        component: descriptor(),
        checks,
        iterations: ITERATIONS,
        elapsed_ns,
        artifact,
    }
}
