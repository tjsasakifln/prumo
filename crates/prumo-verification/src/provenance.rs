use crate::{VerificationError, hash, sha_ok, text_ok};
use prumo_domain::components::ExperimentBasis;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildProvenance {
    pub prumo_commit: String,
    pub source_dirty: bool,
    pub prumo_artifact_sha256: String,
    pub artifact_bytes: u64,
    pub platform: String,
    pub compiler: String,
    pub target: String,
    pub build_profile: String,
    pub dependencies_sha256: String,
    pub configuration: BTreeMap<String, String>,
}

impl BuildProvenance {
    /// Captures the running binary and the source/lock/toolchain captured at its build.
    /// No paths supplied by a client are opened.
    pub fn capture() -> Result<Self, VerificationError> {
        let artifact = std::fs::read(std::env::current_exe()?)?;
        let result = Self {
            prumo_commit: env!("PRUMO_VERIFY_COMMIT").into(),
            source_dirty: env!("PRUMO_VERIFY_DIRTY") != "false",
            prumo_artifact_sha256: prumo_domain::canonical::hash_bytes(&artifact),
            artifact_bytes: artifact.len() as u64,
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            compiler: env!("PRUMO_VERIFY_COMPILER").into(),
            target: env!("PRUMO_VERIFY_TARGET").into(),
            build_profile: env!("PRUMO_VERIFY_PROFILE").into(),
            dependencies_sha256: prumo_domain::canonical::hash_bytes(include_bytes!(
                "../../../Cargo.lock"
            )),
            configuration: BTreeMap::from([
                (
                    "target_features".into(),
                    env!("PRUMO_VERIFY_FEATURES").into(),
                ),
                ("rustflags".into(), env!("PRUMO_VERIFY_FLAGS").into()),
                // Cargo DEBUG reports whether any debug information is emitted,
                // not its detail level or whether debug assertions are enabled.
                ("debug_info".into(), env!("PRUMO_VERIFY_DEBUG").into()),
                ("opt_level".into(), env!("PRUMO_VERIFY_OPT_LEVEL").into()),
                (
                    "float_model".into(),
                    "IEEE-754 binary64; no fast-math requested by this crate".into(),
                ),
            ]),
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), VerificationError> {
        if !sha_ok(&self.prumo_commit, 40)
            || !sha_ok(&self.prumo_artifact_sha256, 64)
            || !sha_ok(&self.dependencies_sha256, 64)
            || self.artifact_bytes == 0
            || [
                &self.platform,
                &self.compiler,
                &self.target,
                &self.build_profile,
            ]
            .into_iter()
            .any(|value| !text_ok(value) || value == "unavailable")
            || self.configuration.len() > 256
        {
            return Err(VerificationError::InvalidProvenance(
                "commit, artefato, plataforma, compilador, build ou lock ausentes".into(),
            ));
        }
        Ok(())
    }

    pub fn sha256(&self) -> Result<String, VerificationError> {
        self.validate()?;
        hash(self)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunContext {
    pub build: BuildProvenance,
    pub basis: Option<ExperimentBasis>,
    pub executor: String,
    /// Caller-supplied timestamp is captured once; comparison order adds no clock noise.
    pub execution_timestamp_unix_ms: u64,
}

impl RunContext {
    pub fn capture(basis: Option<ExperimentBasis>) -> Result<Self, VerificationError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| {
                VerificationError::InvalidProvenance("relógio anterior ao epoch".into())
            })?;
        let result = Self {
            build: BuildProvenance::capture()?,
            basis,
            executor: "prumo-verification/0.1.0".into(),
            execution_timestamp_unix_ms: u64::try_from(now.as_millis()).map_err(|_| {
                VerificationError::InvalidProvenance("relógio fora do intervalo".into())
            })?,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), VerificationError> {
        self.build.validate()?;
        if !text_ok(&self.executor)
            || self.execution_timestamp_unix_ms == 0
            || self.basis.as_ref().is_some_and(|basis| {
                basis.project_id.is_nil()
                    || basis.revision == 0
                    || !sha_ok(&basis.content_sha256, 64)
            })
        {
            return Err(VerificationError::InvalidProvenance(
                "executor, data ou identidade/revisão/hash do snapshot inválidos".into(),
            ));
        }
        Ok(())
    }
}
