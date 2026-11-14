use crate::{
    CONTAINER_VERSION, DATABASE_VERSION, DecodedPackage, EncodedPackage, EntryKind,
    MAX_COMPRESSION_RATIO, MAX_ENTRIES, MAX_ENTRY_BYTES, MAX_EXPANDED_BYTES, MAX_MANIFEST_BYTES,
    MAX_PACKAGE_BYTES, ManifestEntry, OpaqueEntry, PackageManifest, PackageOrigin, Result,
    SQLiteWorkingStore, StorageError, check_hash, check_text, check_version,
    store::{StoreContents, check_size, read_database_contents},
};
use prumo_domain::{PROJECT_SCHEMA_VERSION, canonical::hash_bytes};
use serde::{
    Deserializer,
    de::{Error as _, MapAccess, Visitor},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read, Write},
};
use zip::{
    CompressionMethod, ZipArchive, ZipWriter, read::ZipReadOptions, write::SimpleFileOptions,
};

const MANIFEST: &str = "manifest.json";
const DATABASE: &str = "state.sqlite";
const PROJECT: &str = "project.json";
const COMMANDS: &str = "commands.json";

/// Verify the complete uncompressed native container before the database
/// transaction commits. Content duplicated for auditing counts each time.
pub(crate) fn check_stored_export_size(contents: &StoreContents, database_size: u64) -> Result<()> {
    let mut entries = BTreeMap::new();
    let mut add = |name: String,
                   size: u64,
                   hash: String,
                   kind: EntryKind,
                   media_type: &str,
                   version: u32|
     -> Result<()> {
        check_size(size, MAX_ENTRY_BYTES, "entrada do pacote produzido")?;
        entries.insert(
            name,
            ManifestEntry {
                sha256: hash,
                size,
                kind,
                media_type: media_type.into(),
                version,
            },
        );
        Ok(())
    };
    add(
        DATABASE.into(),
        database_size,
        "0".repeat(64),
        EntryKind::Database,
        "application/vnd.sqlite3",
        DATABASE_VERSION,
    )?;
    add(
        PROJECT.into(),
        contents.project_bytes.len() as u64,
        hash_bytes(&contents.project_bytes),
        EntryKind::ProjectProjection,
        "application/json",
        PROJECT_SCHEMA_VERSION,
    )?;
    let commands = serde_json::to_vec(&contents.journal)?;
    add(
        COMMANDS.into(),
        commands.len() as u64,
        hash_bytes(&commands),
        EntryKind::CommandLog,
        "application/json",
        DATABASE_VERSION,
    )?;
    for (hash, content) in &contents.objects {
        add(
            format!("objects/{hash}.blob"),
            content.len() as u64,
            hash.clone(),
            EntryKind::ContentObject,
            "application/octet-stream",
            DATABASE_VERSION,
        )?;
    }
    for (name, entry) in &contents.extensions {
        add(
            name.clone(),
            entry.bytes.len() as u64,
            hash_bytes(&entry.bytes),
            EntryKind::Extension,
            &entry.media_type,
            entry.version,
        )?;
    }
    // Every allowed origin string has at most 160 UTF-8 bytes. Quotes consume
    // the maximum JSON-escaped size, so a real host origin cannot exceed this.
    let largest_text = "\"".repeat(160);
    let manifest = PackageManifest {
        format_version: CONTAINER_VERSION,
        database_version: DATABASE_VERSION,
        model_version: contents.project.schema_version,
        project_id: contents.project.id,
        revision: contents.project.revision,
        created_unix_ms: contents
            .journal
            .last()
            .ok_or(StorageError::Empty)?
            .record
            .timestamp_unix_ms,
        origin: PackageOrigin {
            application: largest_text.clone(),
            version: largest_text.clone(),
            platform: largest_text,
        },
        project_sha256: hash_bytes(&contents.project_bytes),
        head_sha256: contents.head_sha256.clone(),
        entries,
    };
    let manifest_size = serde_json::to_vec(&manifest)?.len() as u64;
    check_size(manifest_size, MAX_MANIFEST_BYTES, "manifesto produzido")?;
    let mut sizes: BTreeMap<String, u64> = manifest
        .entries
        .into_iter()
        .map(|(name, entry)| (name, entry.size))
        .collect();
    sizes.insert(MANIFEST.into(), manifest_size);
    validate_stored_sizes(&sizes)
}

pub(crate) fn deserialize_entries<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, ManifestEntry>, D::Error> {
    struct EntriesVisitor;
    impl<'de> Visitor<'de> for EntriesVisitor {
        type Value = BTreeMap<String, ManifestEntry>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("mapa de entradas sem repetição de nomes")
        }
        fn visit_map<M: MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut result = BTreeMap::new();
            while let Some(name) = map.next_key::<String>()? {
                if result.len() >= MAX_ENTRIES - 1 || result.contains_key(&name) {
                    return Err(M::Error::custom(
                        "Entrada repetida ou limite de manifesto excedido.",
                    ));
                }
                result.insert(name, map.next_value::<ManifestEntry>()?);
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(EntriesVisitor)
}

/// Produce an offline package from one consistent online-backup image. The
/// native writer uses Stored entries: repeated data can never cause its own
/// output to exceed the reader's compression-ratio limit.
pub fn encode_package(
    store: &SQLiteWorkingStore,
    extensions: &BTreeMap<String, OpaqueEntry>,
    origin: PackageOrigin,
) -> Result<Vec<u8>> {
    Ok(encode_validated_package(store, extensions, origin)?.into_bytes())
}

/// The only producer of `EncodedPackage`. The authority, all projected bytes,
/// versions, hashes, path set and size bounds are established in this call.
pub fn encode_validated_package(
    store: &SQLiteWorkingStore,
    extensions: &BTreeMap<String, OpaqueEntry>,
    origin: PackageOrigin,
) -> Result<EncodedPackage> {
    validate_origin(&origin)?;
    let database_bytes = store.backup_bytes()?;
    let contents = read_database_contents(&database_bytes)?;
    if !extensions.is_empty() && extensions != &contents.extensions {
        return Err(StorageError::Integrity(
            "Extensões fornecidas divergem do banco.".into(),
        ));
    }
    let mut entries = BTreeMap::new();
    let mut files = BTreeMap::new();
    add_file(
        &mut entries,
        &mut files,
        DATABASE.into(),
        database_bytes,
        EntryKind::Database,
        "application/vnd.sqlite3",
        DATABASE_VERSION,
    )?;
    add_file(
        &mut entries,
        &mut files,
        PROJECT.into(),
        contents.project_bytes.clone(),
        EntryKind::ProjectProjection,
        "application/json",
        PROJECT_SCHEMA_VERSION,
    )?;
    add_file(
        &mut entries,
        &mut files,
        COMMANDS.into(),
        serde_json::to_vec(&contents.journal)?,
        EntryKind::CommandLog,
        "application/json",
        DATABASE_VERSION,
    )?;
    for (hash, bytes) in contents.objects {
        add_file(
            &mut entries,
            &mut files,
            format!("objects/{hash}.blob"),
            bytes,
            EntryKind::ContentObject,
            "application/octet-stream",
            DATABASE_VERSION,
        )?;
    }
    for (path, entry) in &contents.extensions {
        validate_extension(path, &entry.media_type, entry.version)?;
        add_file(
            &mut entries,
            &mut files,
            path.clone(),
            entry.bytes.clone(),
            EntryKind::Extension,
            &entry.media_type,
            entry.version,
        )?;
    }
    let created_unix_ms = contents
        .journal
        .last()
        .ok_or(StorageError::Empty)?
        .record
        .timestamp_unix_ms;
    let manifest = PackageManifest {
        format_version: CONTAINER_VERSION,
        database_version: DATABASE_VERSION,
        model_version: contents.project.schema_version,
        project_id: contents.project.id,
        revision: contents.project.revision,
        created_unix_ms,
        origin,
        project_sha256: hash_bytes(&contents.project_bytes),
        head_sha256: contents.head_sha256,
        entries,
    };
    let manifest_bytes = serde_json::to_vec(&manifest)?;
    check_size(manifest_bytes.len() as u64, MAX_MANIFEST_BYTES, "manifesto")?;
    files.insert(MANIFEST.into(), manifest_bytes);
    validate_file_set(&files)?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o600);
    for (name, bytes) in &files {
        writer.start_file(name, options)?;
        writer.write_all(bytes)?;
    }
    let result = writer.finish()?.into_inner();
    check_size(result.len() as u64, MAX_PACKAGE_BYTES, "pacote")?;
    preflight_zip(&result)?;
    Ok(EncodedPackage { bytes: result })
}

/// Parse untrusted bytes without extracting archive paths. The database and
/// every documentary projection must agree before the project is returned.
pub fn decode_package(bytes: &[u8]) -> Result<DecodedPackage> {
    preflight_zip(bytes)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    if archive.offset() != 0
        || archive.has_duplicate_names()
        || archive.has_overlapping_entries()?
    {
        return Err(StorageError::Integrity(
            "ZIP com prefixo, nomes duplicados ou entradas sobrepostas.".into(),
        ));
    }
    archive.validate_header_consistency()?;
    let manifest_bytes = read_entry(&mut archive, MANIFEST, MAX_MANIFEST_BYTES)?;
    prumo_domain::canonical::validate_json_representation(&manifest_bytes)?;
    let value: Value = serde_json::from_slice(&manifest_bytes)?;
    check_json_version(&value, "format_version", "contêiner", CONTAINER_VERSION)?;
    check_json_version(&value, "database_version", "banco", DATABASE_VERSION)?;
    check_json_version(&value, "model_version", "modelo", PROJECT_SCHEMA_VERSION)?;
    let manifest: PackageManifest = serde_json::from_slice(&manifest_bytes)?;
    validate_origin(&manifest.origin)?;
    check_hash(&manifest.project_sha256)?;
    check_hash(&manifest.head_sha256)?;
    if manifest.project_id.is_nil() || manifest.revision == 0 {
        return Err(StorageError::Integrity(
            "Manifesto sem identidade/revisão.".into(),
        ));
    }
    if manifest.entries.len() + 1 != archive.len() || manifest.entries.contains_key(MANIFEST) {
        return Err(StorageError::Integrity(
            "Manifesto omite ou duplica entradas.".into(),
        ));
    }
    let mut expanded = manifest_bytes.len() as u64;
    let mut files = BTreeMap::new();
    let mut extensions = BTreeMap::new();
    for (name, entry) in &manifest.entries {
        validate_path(name)?;
        check_hash(&entry.sha256)?;
        check_size(entry.size, MAX_ENTRY_BYTES, "entrada declarada")?;
        validate_manifest_entry(name, entry)?;
        let content = read_entry(&mut archive, name, entry.size)?;
        expanded = expanded
            .checked_add(content.len() as u64)
            .ok_or_else(|| StorageError::Limit("expansão total".into()))?;
        check_size(expanded, MAX_EXPANDED_BYTES, "expansão total")?;
        if content.len() as u64 != entry.size || hash_bytes(&content) != entry.sha256 {
            return Err(StorageError::Integrity(
                "Hash ou tamanho de entrada diverge do manifesto.".into(),
            ));
        }
        if entry.kind == EntryKind::Extension {
            extensions.insert(
                name.clone(),
                OpaqueEntry {
                    media_type: entry.media_type.clone(),
                    version: entry.version,
                    bytes: content,
                },
            );
        } else {
            files.insert(name.clone(), content);
        }
    }
    let database_bytes = files
        .remove(DATABASE)
        .ok_or_else(|| StorageError::Integrity("Banco ausente do pacote.".into()))?;
    let contents = read_database_contents(&database_bytes)?;
    if contents.project.id != manifest.project_id
        || contents.project.revision != manifest.revision
        || contents.project.schema_version != manifest.model_version
        || contents.head_sha256 != manifest.head_sha256
        || hash_bytes(&contents.project_bytes) != manifest.project_sha256
        || contents
            .journal
            .last()
            .map(|entry| entry.record.timestamp_unix_ms)
            != Some(manifest.created_unix_ms)
        || contents.extensions != extensions
    {
        return Err(StorageError::Integrity(
            "Manifesto diverge da autoridade do banco.".into(),
        ));
    }
    require_projection(&mut files, PROJECT, &contents.project_bytes)?;
    require_projection(
        &mut files,
        COMMANDS,
        &serde_json::to_vec(&contents.journal)?,
    )?;
    for (hash, content) in contents.objects {
        require_projection(&mut files, &format!("objects/{hash}.blob"), &content)?;
    }
    if !files.is_empty() {
        return Err(StorageError::Integrity(
            "Projeção não corresponde a objeto do banco.".into(),
        ));
    }
    Ok(DecodedPackage {
        project: contents.project,
        manifest,
        database_bytes,
        extensions,
    })
}

fn add_file(
    entries: &mut BTreeMap<String, ManifestEntry>,
    files: &mut BTreeMap<String, Vec<u8>>,
    name: String,
    bytes: Vec<u8>,
    kind: EntryKind,
    media_type: &str,
    version: u32,
) -> Result<()> {
    validate_path(&name)?;
    check_size(bytes.len() as u64, MAX_ENTRY_BYTES, "entrada")?;
    if entries.contains_key(&name) {
        return Err(StorageError::Integrity("Entrada duplicada.".into()));
    }
    entries.insert(
        name.clone(),
        ManifestEntry {
            sha256: hash_bytes(&bytes),
            size: bytes.len() as u64,
            kind,
            media_type: media_type.into(),
            version,
        },
    );
    files.insert(name, bytes);
    Ok(())
}

fn require_projection(
    files: &mut BTreeMap<String, Vec<u8>>,
    name: &str,
    expected: &[u8],
) -> Result<()> {
    let actual = files
        .remove(name)
        .ok_or_else(|| StorageError::Integrity("Projeção auditável ausente.".into()))?;
    if actual != expected {
        return Err(StorageError::Integrity(
            "Projeção auditável difere do banco.".into(),
        ));
    }
    Ok(())
}

fn validate_manifest_entry(name: &str, entry: &ManifestEntry) -> Result<()> {
    let version = match entry.kind {
        EntryKind::Database | EntryKind::CommandLog | EntryKind::ContentObject => {
            Some(DATABASE_VERSION)
        }
        EntryKind::ProjectProjection => Some(PROJECT_SCHEMA_VERSION),
        EntryKind::Extension => None,
    };
    if let Some(supported) = version {
        check_version(&format!("entrada {name}"), entry.version, supported)?;
    }
    let valid = match entry.kind {
        EntryKind::Database => {
            name == DATABASE
                && entry.version == DATABASE_VERSION
                && entry.media_type == "application/vnd.sqlite3"
        }
        EntryKind::ProjectProjection => {
            name == PROJECT
                && entry.version == PROJECT_SCHEMA_VERSION
                && entry.media_type == "application/json"
        }
        EntryKind::CommandLog => {
            name == COMMANDS
                && entry.version == DATABASE_VERSION
                && entry.media_type == "application/json"
        }
        EntryKind::ContentObject => {
            let hash = name
                .strip_prefix("objects/")
                .and_then(|value| value.strip_suffix(".blob"));
            hash.is_some_and(|hash| hash == entry.sha256 && check_hash(hash).is_ok())
                && entry.version == DATABASE_VERSION
                && entry.media_type == "application/octet-stream"
        }
        EntryKind::Extension => {
            validate_extension(name, &entry.media_type, entry.version)?;
            true
        }
    };
    if !valid {
        return Err(StorageError::Integrity(
            "Papel, caminho ou versão de entrada incompatível.".into(),
        ));
    }
    Ok(())
}

fn validate_origin(origin: &PackageOrigin) -> Result<()> {
    check_text(&origin.application, 160)?;
    check_text(&origin.version, 160)?;
    check_text(&origin.platform, 160)
}

pub(crate) fn validate_extension(name: &str, media_type: &str, version: u32) -> Result<()> {
    validate_path(name)?;
    check_text(media_type, 160)?;
    if !name.starts_with("extensions/") || version == 0 || !media_type.contains('/') {
        return Err(StorageError::Integrity(
            "Extensão sem namespace, tipo ou versão.".into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_extension_set(extensions: &BTreeMap<String, OpaqueEntry>) -> Result<()> {
    let mut names = BTreeSet::new();
    let mut expanded = 0_u64;
    for (name, entry) in extensions {
        add_path(&mut names, name)?;
        validate_extension(name, &entry.media_type, entry.version)?;
        expanded = expanded
            .checked_add(entry.bytes.len() as u64)
            .ok_or_else(|| StorageError::Limit("extensões".into()))?;
        check_size(expanded, MAX_ENTRY_BYTES, "extensões")?;
    }
    Ok(())
}

pub(crate) fn check_json_version(
    value: &Value,
    field: &str,
    component: &str,
    supported: u32,
) -> Result<()> {
    let number = value.get(field).and_then(Value::as_number).ok_or_else(|| {
        StorageError::Integrity(format!("Versão de {component} ausente ou inválida."))
    })?;
    if let Some(found) = number.as_u64() {
        return check_version(component, found, supported);
    }
    // With arbitrary_precision, a positive exponent may be outside f64 too.
    // All unsupported numeric representations block migration/recovery; they
    // must never be downgraded to a generic, recoverable corruption diagnosis.
    Err(StorageError::UnsupportedVersionValue {
        component: component.into(),
        value: number.to_string(),
        supported,
    })
}

fn read_entry(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str, limit: u64) -> Result<Vec<u8>> {
    let mut file = archive
        .by_name_with_options(name, ZipReadOptions::new().enforce_header_consistency(true))?;
    if file.encrypted()
        || !file.is_file()
        || file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
        || !matches!(
            file.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        )
    {
        return Err(StorageError::Integrity(
            "Entrada não regular, cifrada ou compactação não permitida.".into(),
        ));
    }
    check_size(file.size(), limit.min(MAX_ENTRY_BYTES), "entrada expandida")?;
    check_ratio(file.size(), file.compressed_size())?;
    let mut content = Vec::new();
    // No capacity is allocated from an untrusted size. Reading through EOF
    // checks the CRC, and the actual-byte limit also catches lying metadata.
    (&mut file)
        .take(limit.min(MAX_ENTRY_BYTES) + 1)
        .read_to_end(&mut content)?;
    check_size(
        content.len() as u64,
        limit.min(MAX_ENTRY_BYTES),
        "bytes efetivamente expandidos",
    )?;
    if content.len() as u64 != file.size() {
        return Err(StorageError::Integrity(
            "Tamanho ZIP expandido inconsistente.".into(),
        ));
    }
    Ok(content)
}

fn validate_file_set(files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    validate_stored_sizes(
        &files
            .iter()
            .map(|(name, bytes)| (name.clone(), bytes.len() as u64))
            .collect(),
    )
}

fn validate_stored_sizes(files: &BTreeMap<String, u64>) -> Result<()> {
    if files.len() > MAX_ENTRIES {
        return Err(StorageError::Limit("quantidade de entradas".into()));
    }
    let mut total = 22_u64;
    let mut names = BTreeSet::new();
    for (name, size) in files {
        add_path(&mut names, name)?;
        total = total
            .checked_add(size + 76 + name.len() as u64 * 2)
            .ok_or_else(|| StorageError::Limit("pacote".into()))?;
    }
    // Exact Stored overhead without extras/comments; conservative room for
    // library metadata prevents a large intermediate allocation by the writer.
    check_size(
        total + files.len() as u64 * 64,
        MAX_PACKAGE_BYTES,
        "pacote produzido",
    )
}

fn validate_path(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 240
        || !name.is_ascii()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/_-.".contains(&byte))
    {
        return Err(StorageError::Integrity(
            "Caminho de pacote inválido.".into(),
        ));
    }
    for component in name.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.len() > 80
            || component.ends_with('.')
        {
            return Err(StorageError::Integrity(
                "Componente de caminho inválido.".into(),
            ));
        }
        let stem = component
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.len() == 4
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(StorageError::Integrity("Nome reservado de arquivo.".into()));
        }
    }
    Ok(())
}

pub(crate) fn add_path(names: &mut BTreeSet<String>, name: &str) -> Result<()> {
    validate_path(name)?;
    let folded = name.to_ascii_lowercase();
    let prefix = folded.clone() + "/";
    let has_child = names
        .range(prefix.clone()..)
        .next()
        .is_some_and(|other| other.starts_with(&prefix));
    let has_parent = folded
        .match_indices('/')
        .any(|(index, _)| names.contains(&folded[..index]));
    if names.contains(&folded) || has_child || has_parent {
        return Err(StorageError::Integrity(
            "Colisão de caminhos no pacote.".into(),
        ));
    }
    names.insert(folded);
    Ok(())
}

fn check_ratio(expanded: u64, compressed: u64) -> Result<()> {
    if expanded > compressed.saturating_mul(MAX_COMPRESSION_RATIO) {
        return Err(StorageError::Limit("razão de descompressão".into()));
    }
    Ok(())
}

/// Before ZipArchive allocates its directory, walk bounded classic-ZIP records.
/// Version 1 deliberately excludes ZIP64, multi-disk, comments and prefixes.
fn preflight_zip(bytes: &[u8]) -> Result<()> {
    check_size(bytes.len() as u64, MAX_PACKAGE_BYTES, "pacote recebido")?;
    if bytes.len() < 22 {
        return Err(StorageError::Integrity("ZIP truncado.".into()));
    }
    let end = bytes.len() - 22;
    if bytes.get(end..end + 4) != Some(b"PK\x05\x06")
        || u16_at(bytes, end + 20)? != 0
        || u16_at(bytes, end + 4)? != 0
        || u16_at(bytes, end + 6)? != 0
    {
        return Err(StorageError::Integrity(
            "EOCD ausente, multidisco, comentário ou cauda não permitida.".into(),
        ));
    }
    let count = usize::from(u16_at(bytes, end + 10)?);
    if count == 0 || count > MAX_ENTRIES || usize::from(u16_at(bytes, end + 8)?) != count {
        return Err(StorageError::Limit("quantidade de entradas ZIP".into()));
    }
    let directory_size = u32_at(bytes, end + 12)? as usize;
    let mut offset = u32_at(bytes, end + 16)? as usize;
    if offset.checked_add(directory_size) != Some(end) {
        return Err(StorageError::Integrity(
            "Diretório ZIP fora dos limites ou ZIP64.".into(),
        ));
    }
    let mut names = BTreeSet::new();
    let mut expanded_total = 0_u64;
    let mut first_local = usize::MAX;
    for _ in 0..count {
        if offset.checked_add(46).is_none_or(|next| next > end)
            || bytes.get(offset..offset + 4) != Some(b"PK\x01\x02")
        {
            return Err(StorageError::Integrity("Diretório ZIP truncado.".into()));
        }
        let flags = u16_at(bytes, offset + 8)?;
        let method = u16_at(bytes, offset + 10)?;
        let compressed = u64::from(u32_at(bytes, offset + 20)?);
        let expanded = u64::from(u32_at(bytes, offset + 24)?);
        let name_len = usize::from(u16_at(bytes, offset + 28)?);
        let extra_len = usize::from(u16_at(bytes, offset + 30)?);
        let comment_len = usize::from(u16_at(bytes, offset + 32)?);
        let local_offset = u32_at(bytes, offset + 42)? as usize;
        first_local = first_local.min(local_offset);
        if flags & (1 | 0x40 | 0x2000) != 0
            || !matches!(method, 0 | 8)
            || u16_at(bytes, offset + 34)? != 0
            || name_len > 240
            || extra_len > 1024
            || comment_len != 0
            || local_offset >= offset
        {
            return Err(StorageError::Integrity(
                "Entrada ZIP possui recurso não permitido.".into(),
            ));
        }
        check_size(expanded, MAX_ENTRY_BYTES, "tamanho declarado ZIP")?;
        check_ratio(expanded, compressed)?;
        expanded_total = expanded_total
            .checked_add(expanded)
            .ok_or_else(|| StorageError::Limit("expansão total ZIP".into()))?;
        check_size(expanded_total, MAX_EXPANDED_BYTES, "expansão total ZIP")?;
        let next = offset
            .checked_add(46 + name_len + extra_len + comment_len)
            .filter(|next| *next <= end)
            .ok_or_else(|| StorageError::Integrity("Metadado ZIP fora dos limites.".into()))?;
        let name = std::str::from_utf8(&bytes[offset + 46..offset + 46 + name_len])
            .map_err(|_| StorageError::Integrity("Nome ZIP não é UTF-8.".into()))?;
        add_path(&mut names, name)?;
        let mut extra = offset + 46 + name_len;
        let extra_end = extra + extra_len;
        while extra < extra_end {
            if extra + 4 > extra_end {
                return Err(StorageError::Integrity("Campo extra ZIP truncado.".into()));
            }
            let kind = u16_at(bytes, extra)?;
            let length = usize::from(u16_at(bytes, extra + 2)?);
            if kind == 1 || extra + 4 + length > extra_end {
                return Err(StorageError::Integrity(
                    "ZIP64 ou campo extra inválido.".into(),
                ));
            }
            extra += 4 + length;
        }
        offset = next;
    }
    if first_local != 0 {
        return Err(StorageError::Integrity("Prefixo ZIP não permitido.".into()));
    }
    if offset != end || !names.contains(MANIFEST) {
        return Err(StorageError::Integrity(
            "Diretório ZIP incoerente ou manifesto ausente.".into(),
        ));
    }
    Ok(())
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16> {
    let pair = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| StorageError::Integrity("ZIP truncado.".into()))?;
    Ok(u16::from_le_bytes([pair[0], pair[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    let part = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| StorageError::Integrity("ZIP truncado.".into()))?;
    Ok(u32::from_le_bytes([part[0], part[1], part[2], part[3]]))
}
