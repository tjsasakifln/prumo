//! A fixed installed worker, bounded messages and an explicit process lifetime.
//! This is process isolation for reviewed code, not a sandbox for arbitrary code.
use crate::{ExperimentBasis, QualificationReport};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;

struct ChildOwner(std::process::Child);
impl std::ops::Deref for ChildOwner {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ChildOwner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for ChildOwner {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

pub const PROTOCOL_VERSION: u32 = 1;
pub const SUITE_ID: &str = "PRU-003.1";
pub const MAX_INPUT_BYTES: usize = 8192;
pub const MAX_OUTPUT_BYTES: usize = 512 * 1024;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerRequest {
    pub protocol_version: u32,
    pub request_id: Uuid,
    pub basis: Option<ExperimentBasis>,
    pub units: String,
    pub suite_id: String,
}

impl WorkerRequest {
    pub fn new(basis: Option<ExperimentBasis>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            basis,
            units: "SI".into(),
            suite_id: SUITE_ID.into(),
        }
    }

    pub fn validate(&self) -> Result<(), ProcessError> {
        if self.protocol_version != PROTOCOL_VERSION
            || self.request_id.is_nil()
            || self.units != "SI"
            || self.suite_id != SUITE_ID
            || self.basis.as_ref().is_some_and(|basis| {
                basis.revision == 0
                    || basis.project_id.is_nil()
                    || basis.content_sha256.len() != 64
                    || !basis
                        .content_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        {
            return Err(ProcessError::Protocol(
                "Contrato, unidades ou identidade incompatíveis.".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("O executável de qualificação não está disponível na instalação.")]
    WorkerUnavailable,
    #[error("A qualificação foi cancelada e o processo foi encerrado.")]
    Cancelled,
    #[error("A qualificação excedeu o prazo e o processo foi encerrado.")]
    Timeout,
    #[error("A mensagem excedeu o limite permitido.")]
    MessageTooLarge,
    #[error("O processo de qualificação terminou com falha.")]
    WorkerFailed,
    #[error("Resposta de qualificação inválida: {0}")]
    Protocol(String),
    #[error("Falha de comunicação com o processo: {0}")]
    Io(#[from] std::io::Error),
}

pub fn installed_worker() -> Result<PathBuf, ProcessError> {
    let exe = std::env::current_exe()?;
    let directory = exe.parent().ok_or(ProcessError::WorkerUnavailable)?;
    let name = if cfg!(windows) {
        "prumo-component-worker.exe"
    } else {
        "prumo-component-worker"
    };
    let adjacent = directory.join(name);
    if adjacent.is_file() {
        return Ok(adjacent);
    }
    // Cargo test executables live one level below the development binaries.
    if directory.file_name().is_some_and(|name| name == "deps") {
        let development = directory
            .parent()
            .ok_or(ProcessError::WorkerUnavailable)?
            .join(name);
        if development.is_file() {
            return Ok(development);
        }
    }
    Err(ProcessError::WorkerUnavailable)
}

pub fn executable_identity(path: &Path) -> Result<(String, u64), ProcessError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ProcessError::WorkerUnavailable)?;
    if !metadata.file_type().is_file() || metadata.len() == 0 || metadata.len() > 512 * 1024 * 1024
    {
        return Err(ProcessError::WorkerUnavailable);
    }
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok((
        digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        metadata.len(),
    ))
}

pub fn run_worker(
    path: &Path,
    request: &WorkerRequest,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<QualificationReport, ProcessError> {
    request.validate()?;
    if cancel.load(Ordering::Acquire) {
        return Err(ProcessError::Cancelled);
    }
    let input =
        serde_json::to_vec(request).map_err(|error| ProcessError::Protocol(error.to_string()))?;
    if input.len() > MAX_INPUT_BYTES {
        return Err(ProcessError::MessageTooLarge);
    }
    let (expected_hash, expected_bytes) = executable_identity(path)?;
    let started = Instant::now();
    let mut child = ChildOwner(
        Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let mut stdin = child.stdin.take().ok_or(ProcessError::WorkerFailed)?;
    if let Err(error) = stdin.write_all(&input) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(ProcessError::Io(error));
    }
    drop(stdin);
    let stdout = child.stdout.take().ok_or(ProcessError::WorkerFailed)?;
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout
            .take((MAX_OUTPUT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });
    let mut output = None;
    let outcome = loop {
        if cancel.load(Ordering::Acquire) {
            break Err(ProcessError::Cancelled);
        }
        if started.elapsed() > timeout {
            break Err(ProcessError::Timeout);
        }
        if output.is_none() {
            match receiver.try_recv() {
                Ok(Ok(bytes)) => {
                    if bytes.len() > MAX_OUTPUT_BYTES {
                        break Err(ProcessError::MessageTooLarge);
                    }
                    output = Some(bytes);
                }
                Ok(Err(error)) => break Err(ProcessError::Io(error)),
                Err(mpsc::TryRecvError::Disconnected) => break Err(ProcessError::WorkerFailed),
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        match child.try_wait() {
            Ok(Some(status)) if !status.success() => break Err(ProcessError::WorkerFailed),
            Ok(Some(_)) if output.is_some() => break Ok(output.take().unwrap_or_default()),
            Ok(_) => {}
            Err(error) => break Err(ProcessError::Io(error)),
        }
        thread::sleep(Duration::from_millis(5));
    };
    // Always reap the child and close/join its pipe reader, including every error.
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();
    let output = outcome?;
    let report: QualificationReport = serde_json::from_slice(&output)
        .map_err(|error| ProcessError::Protocol(error.to_string()))?;
    if report.protocol_version != PROTOCOL_VERSION
        || report.request_id != request.request_id
        || report.basis != request.basis
        || report.suite_id != request.suite_id
        || report.engineering_release
        || report.executable_sha256 != expected_hash
        || report.executable_bytes != expected_bytes
    {
        return Err(ProcessError::Protocol(
            "Identidade, versão, projeto ou artefato da resposta não coincidem.".into(),
        ));
    }
    let expected = crate::inventory();
    if report.experiments.len() != expected.len()
        || expected.iter().any(|descriptor| {
            report
                .experiments
                .iter()
                .filter(|experiment| &experiment.component == descriptor)
                .count()
                != 1
        })
    {
        return Err(ProcessError::Protocol(
            "A resposta não contém a bateria completa dos componentes instalados.".into(),
        ));
    }
    Ok(report)
}
