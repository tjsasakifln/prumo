use crate::{
    CommitPhase, CommitRecord, DATABASE_VERSION, MAX_COMMAND_BYTES, MAX_DATABASE_BYTES,
    MAX_ENTRIES, MAX_ENTRY_BYTES, MAX_REVISIONS, MAX_VALIDATION_BYTES, OpaqueEntry, Result,
    StorageError, WorkingDatabaseSnapshot, check_hash, check_text, check_version,
};
use prumo_domain::{PROJECT_SCHEMA_VERSION, Project, canonical::hash_bytes};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, TransactionBehavior,
    backup::{Backup, StepResult},
    config::DbConfig,
    hooks::{AuthAction, AuthContext, Authorization},
    limits::Limit,
    params,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tempfile::NamedTempFile;
use uuid::Uuid;

const APPLICATION_ID: u32 = 0x5052554d;
const STATEMENT_BUDGET: u64 = 20_000_000;
const OBJECT_TABLE: &str = "CREATE TABLE content_objects (sha256 TEXT PRIMARY KEY NOT NULL, size INTEGER NOT NULL CHECK(size >= 0), bytes BLOB NOT NULL CHECK(length(bytes) = size)) STRICT";
const REVISION_TABLE: &str = "CREATE TABLE revisions (ordinal INTEGER PRIMARY KEY CHECK(ordinal > 0), revision TEXT NOT NULL, root_sha256 TEXT NOT NULL REFERENCES content_objects(sha256), command_sha256 TEXT NOT NULL REFERENCES content_objects(sha256)) STRICT";
const HEAD_TABLE: &str = "CREATE TABLE head (singleton INTEGER PRIMARY KEY CHECK(singleton = 1), project_id TEXT NOT NULL, revision TEXT NOT NULL, root_sha256 TEXT NOT NULL REFERENCES content_objects(sha256)) STRICT";
const EXTENSION_TABLE: &str = "CREATE TABLE extensions (path TEXT PRIMARY KEY NOT NULL, media_type TEXT NOT NULL, version INTEGER NOT NULL CHECK(version > 0), sha256 TEXT NOT NULL REFERENCES content_objects(sha256)) STRICT";

/// A bounded native working database. The host supplies and locks its path.
///
/// DELETE journal mode permits read-only inspection without creating WAL/SHM
/// sidecars. SQLite's rollback journal and EXTRA synchronization govern commits.
pub struct SQLiteWorkingStore {
    connection: Connection,
    read_only: bool,
    budget: Arc<AtomicU64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredProject {
    version: u32,
    model_version: u32,
    project_id: Uuid,
    revision: u64,
    fields: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JournalEntry {
    pub base_revision: Option<u64>,
    pub revision: u64,
    pub previous_root_sha256: Option<String>,
    pub root_sha256: String,
    pub project_sha256: String,
    pub record: CommitRecord,
}

pub(crate) struct StoreContents {
    pub project: Project,
    pub project_bytes: Vec<u8>,
    pub head_sha256: String,
    pub objects: BTreeMap<String, Vec<u8>>,
    pub journal: Vec<JournalEntry>,
    pub extensions: BTreeMap<String, OpaqueEntry>,
}

impl SQLiteWorkingStore {
    /// Create a new database or open a previously validated schema. An existing
    /// empty/corrupt/future file is never silently initialized or migrated.
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with_project(path).map(|(store, _)| store)
    }

    /// Open and return the project from that validation pass, without requiring
    /// an immediate second load. Commits still read fresh under their own lock.
    pub fn open_with_project(path: &Path) -> Result<(Self, Option<Project>)> {
        let created = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => {
                file.sync_all()?;
                true
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
            Err(error) => return Err(error.into()),
        };
        check_file(path, created)?;
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let mut connection = Connection::open_with_flags(path, flags)?;
        let budget = harden(&mut connection)?;
        if created {
            connection.pragma_update(None, "journal_mode", "DELETE")?;
            require_delete_journal(&connection)?;
            set_durability(&connection)?;
            connection.pragma_update(None, "page_size", 4096)?;
            connection.pragma_update(None, "max_page_count", (MAX_DATABASE_BYTES / 4096) as u32)?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
            transaction.pragma_update(None, "user_version", DATABASE_VERSION)?;
            transaction.execute_batch(OBJECT_TABLE)?;
            transaction.execute_batch(REVISION_TABLE)?;
            transaction.execute_batch(HEAD_TABLE)?;
            transaction.execute_batch(EXTENSION_TABLE)?;
            transaction.commit()?;
        } else {
            validate_schema(&connection)?;
            // These affect only an already recognized Prumo database.
            set_durability(&connection)?;
            let page_size: u32 = connection.pragma_query_value(None, "page_size", |r| r.get(0))?;
            connection.pragma_update(
                None,
                "max_page_count",
                (MAX_DATABASE_BYTES / u64::from(page_size)) as u32,
            )?;
        }
        install_authorizer(&mut connection, false)?;
        let store = Self {
            connection,
            read_only: false,
            budget,
        };
        let project = store.load()?;
        Ok((store, project))
    }

    /// Back up one read-only database view and validate that frozen image once.
    /// The returned project can never belong to another revision than its bytes.
    pub fn snapshot_read_only(path: &Path) -> Result<WorkingDatabaseSnapshot> {
        let database_bytes = Self::backup_read_only(path)?;
        let project = read_database_state(&database_bytes)?.map(|contents| contents.project);
        Ok(WorkingDatabaseSnapshot {
            project,
            database_bytes,
        })
    }

    /// Return one consistent SQLite backup image without validating its project,
    /// objects or command chain. These bytes are not a trusted/validated value.
    /// A host may reuse a project only after exact equality with database bytes
    /// fully validated earlier in that same operation; otherwise validate fully.
    pub fn backup_read_only(path: &Path) -> Result<Vec<u8>> {
        let store = Self::open_read_only_connection(path)?;
        store.backup_bytes()
    }

    /// Read a database without creating or modifying files in its directory.
    /// A hot rollback journal requires recovery through a writable host path.
    pub fn open_read_only(path: &Path) -> Result<Self> {
        let store = Self::open_read_only_connection(path)?;
        store.load()?;
        Ok(store)
    }

    // Private snapshot helpers validate the frozen image before returning any
    // content. This constructor alone is never exposed as a validated store.
    fn open_read_only_connection(path: &Path) -> Result<Self> {
        check_file(path, false)?;
        let mut connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let budget = harden(&mut connection)?;
        connection.pragma_update(None, "query_only", true)?;
        validate_schema(&connection)?;
        install_authorizer(&mut connection, true)?;
        let store = Self {
            connection,
            read_only: true,
            budget,
        };
        Ok(store)
    }

    pub fn load(&self) -> Result<Option<Project>> {
        self.reset_budget();
        self.with_read_transaction(|connection| {
            Ok(read_contents(connection)?.map(|contents| contents.project))
        })
    }

    pub fn commit(
        &mut self,
        expected_revision: Option<u64>,
        project: &Project,
        record: CommitRecord,
    ) -> Result<()> {
        self.commit_with_hook(expected_revision, project, record, |_| Ok(()))
    }

    /// Host-only crash/cancellation seam. No environment variable, CLI, SQL,
    /// filesystem path or MCP parameter activates this hook.
    pub fn commit_with_hook(
        &mut self,
        expected_revision: Option<u64>,
        project: &Project,
        record: CommitRecord,
        hook: impl FnMut(CommitPhase) -> Result<()>,
    ) -> Result<()> {
        self.commit_inner(expected_revision, project, record, None, hook)
    }

    /// Initialize opaque future-domain payloads atomically with a baseline.
    /// Later commits preserve the exact set; replacement is not a public edit.
    pub fn commit_with_extensions(
        &mut self,
        expected_revision: Option<u64>,
        project: &Project,
        record: CommitRecord,
        extensions: &BTreeMap<String, OpaqueEntry>,
    ) -> Result<()> {
        self.commit_inner(expected_revision, project, record, Some(extensions), |_| {
            Ok(())
        })
    }

    pub fn extensions(&self) -> Result<BTreeMap<String, OpaqueEntry>> {
        self.reset_budget();
        self.with_read_transaction(|connection| {
            Ok(read_contents(connection)?
                .map_or_else(BTreeMap::new, |contents| contents.extensions))
        })
    }

    fn commit_inner(
        &mut self,
        expected_revision: Option<u64>,
        project: &Project,
        record: CommitRecord,
        extensions: Option<&BTreeMap<String, OpaqueEntry>>,
        mut hook: impl FnMut(CommitPhase) -> Result<()>,
    ) -> Result<()> {
        if self.read_only {
            return Err(StorageError::ReadOnly);
        }
        project.validate()?;
        validate_record(&record)?;
        self.reset_budget();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous = read_contents(&transaction)?;
        let actual = previous.as_ref().map(|old| old.project.revision);
        if expected_revision != actual {
            return Err(StorageError::RevisionConflict {
                expected: expected_revision,
                actual,
            });
        }
        if let Some(entries) = extensions {
            if let Some(old) = &previous {
                if &old.extensions != entries {
                    return Err(StorageError::Integrity(
                        "Extensões opacas existentes não podem ser substituídas.".into(),
                    ));
                }
            } else {
                for (name, entry) in entries {
                    crate::package::validate_extension(name, &entry.media_type, entry.version)?;
                    let hash = insert_object(&transaction, &entry.bytes)?;
                    transaction.execute("INSERT INTO extensions (path, media_type, version, sha256) VALUES (?1, ?2, ?3, ?4)", params![name, entry.media_type, entry.version, hash])?;
                }
            }
        }
        let value = serde_json::to_value(project)?;
        let project_bytes = serde_json::to_vec(&value)?;
        check_size(project_bytes.len() as u64, MAX_ENTRY_BYTES, "modelo")?;
        let administrative = previous.as_ref().is_some_and(|old| {
            old.project.revision == project.revision
                && old.project_bytes == project_bytes
                && record.operation == "storage.recover"
                && record.command.is_some()
        });
        if let Some(old) = &previous {
            if old.project.id != project.id
                || (!administrative
                    && old.project.revision.checked_add(1) != Some(project.revision))
                || !project.history.starts_with(&old.project.history)
                || record.command.is_none()
            {
                return Err(StorageError::Integrity(
                    "Identidade, histórico ou sequência de revisão incompatível.".into(),
                ));
            }
            if old
                .journal
                .iter()
                .any(|entry| entry.record.command_id == record.command_id)
            {
                return Err(StorageError::Integrity("Comando já registrado.".into()));
            }
        }
        if record.command.is_some() && !administrative {
            let last = project.history.last().ok_or(StorageError::Empty)?;
            if last.actor != record.actor || last.timestamp_unix_ms != record.timestamp_unix_ms {
                return Err(StorageError::Integrity(
                    "Autoria/data do comando difere do histórico do projeto.".into(),
                ));
            }
        }
        let ordinal = previous.as_ref().map_or(1, |old| old.journal.len() + 1);
        if ordinal > MAX_REVISIONS {
            return Err(StorageError::Limit("quantidade de revisões".into()));
        }
        let project_sha256 = hash_bytes(&project_bytes);
        let fields = value
            .as_object()
            .ok_or_else(|| StorageError::Integrity("Projeto não é objeto.".into()))?;
        let mut references = BTreeMap::new();
        for (name, value) in fields {
            let bytes = serde_json::to_vec(value)?;
            references.insert(name.clone(), insert_object(&transaction, &bytes)?);
        }
        let envelope = StoredProject {
            version: DATABASE_VERSION,
            model_version: project.schema_version,
            project_id: project.id,
            revision: project.revision,
            fields: references,
        };
        let root_sha256 = insert_object(&transaction, &serde_json::to_vec(&envelope)?)?;
        let entry = JournalEntry {
            base_revision: actual,
            revision: project.revision,
            previous_root_sha256: previous.as_ref().map(|old| old.head_sha256.clone()),
            root_sha256: root_sha256.clone(),
            project_sha256,
            record,
        };
        let command_sha256 = insert_object(&transaction, &serde_json::to_vec(&entry)?)?;
        enforce_object_limits(&transaction)?;
        hook(CommitPhase::ObjectsInserted)?;
        transaction.execute(
            "INSERT INTO revisions (ordinal, revision, root_sha256, command_sha256) VALUES (?1, ?2, ?3, ?4)",
            params![ordinal as u32, project.revision.to_string(), root_sha256, command_sha256],
        )?;
        hook(CommitPhase::RevisionInserted)?;
        transaction.execute(
            "INSERT INTO head (singleton, project_id, revision, root_sha256) VALUES (1, ?1, ?2, ?3) ON CONFLICT(singleton) DO UPDATE SET project_id = excluded.project_id, revision = excluded.revision, root_sha256 = excluded.root_sha256",
            params![project.id.to_string(), project.revision.to_string(), root_sha256],
        )?;
        hook(CommitPhase::HeadUpdated)?;
        check_database_pages(&transaction)?;
        read_contents_with_validated_project(&transaction, Some(&project_bytes))?
            .ok_or(StorageError::Empty)?;
        hook(CommitPhase::BeforeCommit)?;
        transaction.commit()?;
        Ok(())
    }

    /// A consistent SQLite online-backup image, with no active journal and no
    /// unbounded Busy/Locked retry. It never copies the live database file.
    pub fn backup_bytes(&self) -> Result<Vec<u8>> {
        self.reset_budget();
        let temp = NamedTempFile::new()?;
        let mut destination = Connection::open(temp.path())?;
        harden(&mut destination)?;
        destination.pragma_update(None, "journal_mode", "DELETE")?;
        require_delete_journal(&destination)?;
        set_durability(&destination)?;
        check_database_pages(&self.connection)?;
        {
            let backup = Backup::new(&self.connection, &mut destination)?;
            match backup.step(-1)? {
                StepResult::Done => {}
                StepResult::Busy | StepResult::Locked | StepResult::More => {
                    return Err(StorageError::Interrupted(
                        "Banco ocupado durante cópia consistente.".into(),
                    ));
                }
                _ => {
                    return Err(StorageError::Interrupted(
                        "Cópia consistente não concluída.".into(),
                    ));
                }
            }
        }
        destination.pragma_update(None, "journal_mode", "DELETE")?;
        require_delete_journal(&destination)?;
        destination.close().map_err(|(_, error)| error)?;
        temp.as_file().sync_all()?;
        read_bounded_file(temp.path(), MAX_DATABASE_BYTES)
    }

    fn with_read_transaction<T>(&self, read: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let transaction = self.connection.unchecked_transaction()?;
        let result = read(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }

    fn reset_budget(&self) {
        self.budget.store(STATEMENT_BUDGET, Ordering::Relaxed);
    }
}

/// Validate an image in a private temporary directory, then atomically install
/// it. The host must hold its project lock and close prior connections. Active
/// or hot sidecars are refused; recovery may quarantine their whole directory.
pub fn restore_database(path: &Path, bytes: &[u8]) -> Result<()> {
    read_database_contents(bytes)?;
    if let Ok(metadata) = fs::symlink_metadata(path)
        && (!metadata.is_file() || metadata.file_type().is_symlink())
    {
        return Err(StorageError::Integrity(
            "Destino do banco não é arquivo regular.".into(),
        ));
    }
    for suffix in ["-journal", "-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        if Path::new(&sidecar).try_exists()? {
            return Err(StorageError::Integrity(
                "Banco possui arquivo auxiliar; feche ou recupere a sessão antes de substituí-lo."
                    .into(),
            ));
        }
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| StorageError::Io(error.error))?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

pub(crate) fn read_database_contents(bytes: &[u8]) -> Result<StoreContents> {
    read_database_state(bytes)?.ok_or(StorageError::Empty)
}

fn read_database_state(bytes: &[u8]) -> Result<Option<StoreContents>> {
    check_size(bytes.len() as u64, MAX_DATABASE_BYTES, "banco")?;
    if bytes.len() < 100 || &bytes[..16] != b"SQLite format 3\0" {
        return Err(StorageError::Integrity("Cabeçalho SQLite ausente.".into()));
    }
    // Frozen exchange images never rely on a WAL or rollback journal supplied
    // from another source. Version bytes 1/1 mean rollback-journal format.
    if bytes[18] != 1 || bytes[19] != 1 {
        return Err(StorageError::Integrity(
            "Imagem SQLite exige journal externo.".into(),
        ));
    }
    let mut temporary = NamedTempFile::new()?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    let store = SQLiteWorkingStore::open_read_only_connection(temporary.path())?;
    store.reset_budget();
    store.with_read_transaction(read_contents)
}

fn harden(connection: &mut Connection) -> Result<Arc<AtomicU64>> {
    for (setting, enabled) in [
        (DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true),
        (DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_VIEW, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true),
        (DbConfig::SQLITE_DBCONFIG_DQS_DML, false),
        (DbConfig::SQLITE_DBCONFIG_DQS_DDL, false),
    ] {
        connection.set_db_config(setting, enabled)?;
    }
    for (limit, value) in [
        (Limit::SQLITE_LIMIT_LENGTH, MAX_ENTRY_BYTES as i32 + 1024),
        (Limit::SQLITE_LIMIT_SQL_LENGTH, 16_384),
        (Limit::SQLITE_LIMIT_COLUMN, 32),
        (Limit::SQLITE_LIMIT_EXPR_DEPTH, 32),
        (Limit::SQLITE_LIMIT_COMPOUND_SELECT, 4),
        (Limit::SQLITE_LIMIT_VDBE_OP, 100_000),
        (Limit::SQLITE_LIMIT_FUNCTION_ARG, 8),
        (Limit::SQLITE_LIMIT_ATTACHED, 0),
        (Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 16),
        (Limit::SQLITE_LIMIT_TRIGGER_DEPTH, 0),
        (Limit::SQLITE_LIMIT_WORKER_THREADS, 0),
        (Limit::SQLITE_LIMIT_SCHEMA, 16),
        (Limit::SQLITE_LIMIT_TRIGGER_STEPS, 0),
        (Limit::SQLITE_LIMIT_PARSER_DEPTH, 64),
    ] {
        connection.set_limit(limit, value)?;
    }
    connection.busy_timeout(Duration::from_millis(500))?;
    let budget = Arc::new(AtomicU64::new(STATEMENT_BUDGET));
    let callback_budget = Arc::clone(&budget);
    connection.progress_handler(
        1000,
        Some(move || {
            callback_budget
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
                    left.checked_sub(1000)
                })
                .is_err()
        }),
    )?;
    Ok(budget)
}

fn install_authorizer(connection: &mut Connection, read_only: bool) -> Result<()> {
    connection.authorizer(Some(move |context: AuthContext<'_>| {
        if context.accessor.is_some() || context.database_name.is_some_and(|name| name != "main") {
            return Authorization::Deny;
        }
        let permitted = match context.action {
            AuthAction::Select | AuthAction::Transaction { .. } | AuthAction::Savepoint { .. } => {
                true
            }
            AuthAction::Read { table_name, .. } => matches!(
                table_name,
                "content_objects"
                    | "revisions"
                    | "head"
                    | "extensions"
                    | "sqlite_master"
                    | "sqlite_schema"
            ),
            AuthAction::Insert { table_name } => {
                !read_only
                    && matches!(
                        table_name,
                        "content_objects" | "revisions" | "head" | "extensions"
                    )
            }
            AuthAction::Update {
                table_name: "head", ..
            } => !read_only,
            AuthAction::Function { function_name } => matches!(
                function_name,
                "length" | "count" | "sum" | "coalesce" | "max"
            ),
            AuthAction::Pragma {
                pragma_name,
                pragma_value: None,
            } => matches!(
                pragma_name,
                "application_id"
                    | "user_version"
                    | "page_count"
                    | "page_size"
                    | "journal_mode"
                    | "quick_check"
                    | "foreign_key_check"
            ),
            _ => false,
        };
        if permitted {
            Authorization::Allow
        } else {
            Authorization::Deny
        }
    }))?;
    Ok(())
}

fn validate_schema(connection: &Connection) -> Result<()> {
    let application: u32 =
        connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    if application != APPLICATION_ID {
        return Err(StorageError::Integrity(
            "Banco não pertence ao Prumo.".into(),
        ));
    }
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    check_version("banco", version, DATABASE_VERSION)?;
    require_delete_journal(connection)?;
    check_database_pages(connection)?;
    let mut statement =
        connection.prepare("SELECT type, name, tbl_name, sql FROM sqlite_schema ORDER BY name")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    let expected = BTreeMap::from([
        ("content_objects", OBJECT_TABLE),
        ("revisions", REVISION_TABLE),
        ("head", HEAD_TABLE),
        ("extensions", EXTENSION_TABLE),
    ]);
    let mut found = BTreeSet::new();
    let mut indexes = BTreeSet::new();
    for row in rows {
        let (kind, name, table, sql) = row?;
        if kind == "table"
            && name == table
            && expected.get(name.as_str()).copied() == sql.as_deref()
        {
            found.insert(name);
        } else if kind == "index"
            && sql.is_none()
            && ((name == "sqlite_autoindex_content_objects_1" && table == "content_objects")
                || (name == "sqlite_autoindex_extensions_1" && table == "extensions"))
        {
            indexes.insert(name);
        } else {
            return Err(StorageError::Integrity("Esquema SQLite não reconhecido; consultas, gatilhos e extensões de SQL não são permitidos.".into()));
        }
    }
    if found.len() != expected.len() || indexes.len() != 2 {
        return Err(StorageError::Integrity("Esquema SQLite incompleto.".into()));
    }
    let quick: String = connection.pragma_query_value(None, "quick_check", |row| row.get(0))?;
    if quick != "ok" {
        return Err(StorageError::Integrity(
            "SQLite detectou corrupção interna.".into(),
        ));
    }
    let mut foreign = connection.prepare("PRAGMA foreign_key_check")?;
    if foreign.query([])?.next()?.is_some() {
        return Err(StorageError::Integrity("Referência órfã no banco.".into()));
    }
    Ok(())
}

fn read_contents(connection: &Connection) -> Result<Option<StoreContents>> {
    read_contents_with_validated_project(connection, None)
}

// The optional bytes exist only inside commit_inner after project.validate().
// No caller can supply a public "validated" flag or reuse one across requests.
fn read_contents_with_validated_project(
    connection: &Connection,
    validated_project_bytes: Option<&[u8]>,
) -> Result<Option<StoreContents>> {
    enforce_object_limits(connection)?;
    let mut objects = BTreeMap::new();
    let mut statement =
        connection.prepare("SELECT sha256, size, bytes FROM content_objects ORDER BY sha256")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let hash: String = row.get(0)?;
        let size = u64::from(row.get::<_, u32>(1)?);
        check_hash(&hash)?;
        check_size(size, MAX_ENTRY_BYTES, "objeto")?;
        let bytes: Vec<u8> = row.get(2)?;
        if bytes.len() as u64 != size || hash_bytes(&bytes) != hash {
            return Err(StorageError::Integrity(
                "Objeto imutável adulterado.".into(),
            ));
        }
        objects.insert(hash, bytes);
    }
    drop(rows);
    drop(statement);
    let mut extensions = BTreeMap::new();
    let mut extension_hashes = BTreeSet::new();
    let mut extension_paths = BTreeSet::new();
    let mut extension_bytes = 0_u64;
    let mut statement = connection
        .prepare("SELECT path, media_type, version, sha256 FROM extensions ORDER BY path")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if extensions.len() >= MAX_ENTRIES {
            return Err(StorageError::Limit("quantidade de extensões".into()));
        }
        let path: String = row.get(0)?;
        let media_type: String = row.get(1)?;
        let version: u32 = row.get(2)?;
        let hash: String = row.get(3)?;
        crate::package::validate_extension(&path, &media_type, version)?;
        crate::package::add_path(&mut extension_paths, &path)?;
        let content = objects
            .get(&hash)
            .ok_or_else(|| StorageError::Integrity("Extensão sem objeto imutável.".into()))?;
        extension_bytes = extension_bytes
            .checked_add(content.len() as u64)
            .ok_or_else(|| StorageError::Limit("extensões expandidas".into()))?;
        check_size(extension_bytes, MAX_ENTRY_BYTES, "extensões expandidas")?;
        extensions.insert(
            path,
            OpaqueEntry {
                media_type,
                version,
                bytes: content.clone(),
            },
        );
        extension_hashes.insert(hash);
    }
    drop(rows);
    drop(statement);
    crate::package::validate_extension_set(&extensions)?;
    let head: Option<(String, String, String)> = connection
        .query_row(
            "SELECT project_id, revision, root_sha256 FROM head WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let head_count: u32 =
        connection.query_row("SELECT count(*) FROM head", [], |row| row.get(0))?;
    let revision_count: u32 =
        connection.query_row("SELECT count(*) FROM revisions", [], |row| row.get(0))?;
    if revision_count as usize > MAX_REVISIONS {
        return Err(StorageError::Limit("quantidade de revisões".into()));
    }
    let Some((project_id, revision_text, head_sha256)) = head else {
        if head_count != 0 || revision_count != 0 || !objects.is_empty() || !extensions.is_empty() {
            return Err(StorageError::Integrity(
                "Banco sem cabeçalho contém estado parcial.".into(),
            ));
        }
        return Ok(None);
    };
    if head_count != 1 || revision_count == 0 {
        return Err(StorageError::Integrity(
            "Cabeçalho ou histórico incompleto.".into(),
        ));
    }
    let project = reconstruct(&head_sha256, &objects)?;
    if project.id.to_string() != project_id || project.revision.to_string() != revision_text {
        return Err(StorageError::Integrity(
            "Cabeçalho difere do objeto do projeto.".into(),
        ));
    }
    let project_bytes = serde_json::to_vec(&serde_json::to_value(&project)?)?;
    match validated_project_bytes {
        Some(expected) if expected == project_bytes => {}
        Some(_) => {
            return Err(StorageError::Integrity(
                "O modelo reconstruído difere dos bytes validados nesta transação.".into(),
            ));
        }
        None => project.validate()?,
    }
    check_size(
        project_bytes.len() as u64,
        MAX_ENTRY_BYTES,
        "modelo reconstruído",
    )?;
    let project_hash = hash_bytes(&project_bytes);
    let current_envelope = parse_envelope(&objects, &head_sha256)?;
    let mut validation_bytes = 0_u64;
    let mut journal: Vec<JournalEntry> = Vec::new();
    let mut used = extension_hashes;
    let mut command_ids = BTreeSet::new();
    let mut statement = connection.prepare(
        "SELECT ordinal, revision, root_sha256, command_sha256 FROM revisions ORDER BY ordinal",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let ordinal: u32 = row.get(0)?;
        let revision: String = row.get(1)?;
        let root: String = row.get(2)?;
        let command: String = row.get(3)?;
        let entry: JournalEntry = parse_object(&objects, &command)?;
        validate_record(&entry.record)?;
        let envelope = parse_envelope(&objects, &root)?;
        if ordinal as usize != journal.len() + 1
            || revision != entry.revision.to_string()
            || root != entry.root_sha256
            || envelope.project_id != project.id
            || envelope.revision != entry.revision
            || !command_ids.insert(entry.record.command_id)
        {
            return Err(StorageError::Integrity(
                "Registro de revisão inconsistente.".into(),
            ));
        }
        if !envelope.fields.keys().eq(current_envelope.fields.keys()) {
            return Err(StorageError::Integrity(
                "Campos da revisão histórica incompatíveis.".into(),
            ));
        }
        for (field, expected) in [
            ("id", serde_json::to_value(envelope.project_id)?),
            ("revision", Value::from(envelope.revision)),
            ("schema_version", Value::from(envelope.model_version)),
        ] {
            let hash = envelope
                .fields
                .get(field)
                .ok_or_else(|| StorageError::Integrity("Identidade histórica ausente.".into()))?;
            let value: Value = parse_object(&objects, hash)?;
            if value != expected {
                return Err(StorageError::Integrity(
                    "Identidade histórica divergente.".into(),
                ));
            }
        }
        for hash in envelope.fields.values() {
            let bytes = objects
                .get(hash)
                .ok_or_else(|| StorageError::Integrity("Campo histórico ausente.".into()))?;
            validation_bytes = validation_bytes
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| StorageError::Limit("volume da validação histórica".into()))?;
            check_size(
                validation_bytes,
                MAX_VALIDATION_BYTES,
                "volume da validação histórica",
            )?;
        }
        check_hash(&entry.project_sha256)?;
        let administrative = journal.last().is_some_and(|previous| {
            entry.revision == previous.revision
                && entry.root_sha256 == previous.root_sha256
                && entry.project_sha256 == previous.project_sha256
                && entry.record.operation == "storage.recover"
        });
        if let Some(previous) = journal.last() {
            if entry.base_revision != Some(previous.revision)
                || (!administrative && previous.revision.checked_add(1) != Some(entry.revision))
                || entry.previous_root_sha256.as_deref() != Some(previous.root_sha256.as_str())
                || entry.record.command.is_none()
            {
                return Err(StorageError::Integrity(
                    "Cadeia de comandos interrompida.".into(),
                ));
            }
        } else if entry.base_revision.is_some() || entry.previous_root_sha256.is_some() {
            return Err(StorageError::Integrity(
                "Baseline importada não está identificada.".into(),
            ));
        }
        if entry.record.command.is_some() && !administrative {
            let index = usize::try_from(entry.revision)
                .ok()
                .and_then(|n| n.checked_sub(1));
            let change = index
                .and_then(|n| project.history.get(n))
                .ok_or_else(|| StorageError::Integrity("Comando fora do histórico.".into()))?;
            if change.actor != entry.record.actor
                || change.timestamp_unix_ms != entry.record.timestamp_unix_ms
            {
                return Err(StorageError::Integrity(
                    "Autoria do comando diverge do histórico.".into(),
                ));
            }
        }
        // Every stored root is auditable and its whole-model digest is checked,
        // including imported baselines. No journal row can point at orphan data.
        if hash_fields(&envelope.fields, &objects)? != entry.project_sha256 {
            return Err(StorageError::Integrity(
                "Revisão histórica diverge do hash ou da proveniência.".into(),
            ));
        }
        let history_hash = envelope
            .fields
            .get("history")
            .ok_or_else(|| StorageError::Integrity("Revisão sem histórico.".into()))?;
        let history: Vec<prumo_domain::ChangeRecord> = parse_object(&objects, history_hash)?;
        if !project.history.starts_with(&history) || history.len() as u64 != entry.revision {
            return Err(StorageError::Integrity(
                "Revisão histórica perdeu a proveniência.".into(),
            ));
        }
        used.insert(root);
        used.insert(command);
        used.extend(envelope.fields.into_values());
        journal.push(entry);
    }
    let last = journal.last().ok_or(StorageError::Empty)?;
    if last.revision != project.revision
        || last.root_sha256 != head_sha256
        || last.project_sha256 != project_hash
        || used.len() != objects.len()
        || used.iter().any(|hash| !objects.contains_key(hash))
    {
        return Err(StorageError::Integrity(
            "Cabeçalho, objetos ou última revisão divergem.".into(),
        ));
    }
    let contents = StoreContents {
        project,
        project_bytes,
        head_sha256,
        objects,
        journal,
        extensions,
    };
    crate::package::check_stored_export_size(&contents, database_size(connection)?)?;
    Ok(Some(contents))
}

fn reconstruct(hash: &str, objects: &BTreeMap<String, Vec<u8>>) -> Result<Project> {
    let envelope = parse_envelope(objects, hash)?;
    // A root contains only references to top-level Project fields. Opaque
    // extension contents are never treated as references or executable data.
    if envelope.fields.len() > 32 {
        return Err(StorageError::Limit("campos do modelo".into()));
    }
    let mut values = serde_json::Map::new();
    let mut accumulated = 0_u64;
    for (name, field_hash) in envelope.fields {
        let bytes = objects
            .get(&field_hash)
            .ok_or_else(|| StorageError::Integrity("Referência de conteúdo ausente.".into()))?;
        accumulated = accumulated
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| StorageError::Limit("modelo".into()))?;
        check_size(accumulated, MAX_ENTRY_BYTES, "modelo")?;
        prumo_domain::canonical::validate_json_representation(bytes)?;
        // Keep the Value boundary: Serde's buffered, internally tagged enums
        // otherwise cannot read f64 fields with arbitrary_precision enabled.
        // from_slice also requires exactly one complete value for each field.
        let value: Value = serde_json::from_slice(bytes)?;
        values.insert(name, value);
    }
    let project: Project = serde_json::from_value(Value::Object(values))?;
    if project.schema_version != envelope.model_version
        || project.id != envelope.project_id
        || project.revision != envelope.revision
    {
        return Err(StorageError::Integrity(
            "Envelope difere do projeto reconstruído.".into(),
        ));
    }
    Ok(project)
}

fn hash_fields(
    fields: &BTreeMap<String, String>,
    objects: &BTreeMap<String, Vec<u8>>,
) -> Result<String> {
    if fields.len() > 32 {
        return Err(StorageError::Limit("campos da revisão".into()));
    }
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    hasher.update(b"{");
    for (index, (name, hash)) in fields.iter().enumerate() {
        check_hash(hash)?;
        let bytes = objects
            .get(hash)
            .ok_or_else(|| StorageError::Integrity("Campo histórico órfão.".into()))?;
        total = total
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| StorageError::Limit("revisão histórica".into()))?;
        check_size(total, MAX_ENTRY_BYTES, "revisão histórica")?;
        if index != 0 {
            hasher.update(b",");
        }
        hasher.update(serde_json::to_vec(name)?);
        hasher.update(b":");
        hasher.update(bytes);
    }
    hasher.update(b"}");
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn parse_object<T: serde::de::DeserializeOwned>(
    objects: &BTreeMap<String, Vec<u8>>,
    hash: &str,
) -> Result<T> {
    check_hash(hash)?;
    let bytes = objects
        .get(hash)
        .ok_or_else(|| StorageError::Integrity("Objeto por hash ausente.".into()))?;
    prumo_domain::canonical::validate_json_representation(bytes)?;
    Ok(serde_json::from_slice(bytes)?)
}

fn parse_envelope(objects: &BTreeMap<String, Vec<u8>>, hash: &str) -> Result<StoredProject> {
    let value: Value = parse_object(objects, hash)?;
    crate::package::check_json_version(&value, "version", "envelope do banco", DATABASE_VERSION)?;
    crate::package::check_json_version(&value, "model_version", "modelo", PROJECT_SCHEMA_VERSION)?;
    parse_object(objects, hash)
}

fn insert_object(connection: &Connection, bytes: &[u8]) -> Result<String> {
    check_size(bytes.len() as u64, MAX_ENTRY_BYTES, "objeto")?;
    let hash = hash_bytes(bytes);
    connection.execute(
        "INSERT OR IGNORE INTO content_objects (sha256, size, bytes) VALUES (?1, ?2, ?3)",
        params![hash, bytes.len() as u32, bytes],
    )?;
    let existing: Vec<u8> = connection.query_row(
        "SELECT bytes FROM content_objects WHERE sha256 = ?1",
        [&hash],
        |row| row.get(0),
    )?;
    if existing != bytes {
        return Err(StorageError::Integrity(
            "Colisão ou adulteração de conteúdo imutável.".into(),
        ));
    }
    Ok(hash)
}

fn validate_record(record: &CommitRecord) -> Result<()> {
    if record.command_id.is_nil() {
        return Err(StorageError::Integrity("Comando sem identidade.".into()));
    }
    check_text(&record.actor, 640)?;
    check_text(&record.operation, 640)?;
    if let Some(reason) = &record.reason {
        check_text(reason, 4096)?;
    }
    check_size(
        serde_json::to_vec(record)?.len() as u64,
        MAX_COMMAND_BYTES,
        "registro de comando",
    )
}

fn enforce_object_limits(connection: &Connection) -> Result<()> {
    let (count, total): (u32, u32) = connection.query_row(
        "SELECT count(*), coalesce(sum(size), 0) FROM content_objects",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let extensions: u32 =
        connection.query_row("SELECT count(*) FROM extensions", [], |row| row.get(0))?;
    if count as usize > MAX_ENTRIES - 4
        || extensions as usize > MAX_ENTRIES - 4 - count as usize
        || u64::from(total) > MAX_DATABASE_BYTES
    {
        return Err(StorageError::Limit(
            "objetos imutáveis ou tamanho do banco".into(),
        ));
    }
    Ok(())
}

fn check_database_pages(connection: &Connection) -> Result<()> {
    check_size(database_size(connection)?, MAX_DATABASE_BYTES, "banco")
}

fn database_size(connection: &Connection) -> Result<u64> {
    let pages =
        u64::from(connection.pragma_query_value(None, "page_count", |row| row.get::<_, u32>(0))?);
    let page_size =
        u64::from(connection.pragma_query_value(None, "page_size", |row| row.get::<_, u32>(0))?);
    if !(512..=65_536).contains(&page_size) || !page_size.is_power_of_two() {
        return Err(StorageError::Integrity("Página SQLite inválida.".into()));
    }
    let bytes = pages
        .checked_mul(page_size)
        .ok_or_else(|| StorageError::Limit("banco".into()))?;
    Ok(bytes)
}

fn require_delete_journal(connection: &Connection) -> Result<()> {
    let journal_mode: String =
        connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    if journal_mode != "delete" {
        return Err(StorageError::Integrity(
            "Modo de journal incompatível com banco versão 1.".into(),
        ));
    }
    Ok(())
}

fn set_durability(connection: &Connection) -> Result<()> {
    connection.pragma_update(None, "synchronous", "EXTRA")?;
    let setting: u32 = connection.pragma_query_value(None, "synchronous", |row| row.get(0))?;
    if setting != 3 {
        return Err(StorageError::Integrity(
            "SQLite não confirmou sincronização EXTRA.".into(),
        ));
    }
    Ok(())
}

fn check_file(path: &Path, allow_empty: bool) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || (!allow_empty && metadata.len() == 0)
    {
        return Err(StorageError::Integrity(
            "Banco não é arquivo regular inicializado.".into(),
        ));
    }
    check_size(metadata.len(), MAX_DATABASE_BYTES, "banco")?;
    if !allow_empty {
        let mut header = [0_u8; 100];
        File::open(path)?.read_exact(&mut header)?;
        if &header[..16] != b"SQLite format 3\0" {
            return Err(StorageError::Integrity("Cabeçalho SQLite ausente.".into()));
        }
        let application = u32::from_be_bytes(
            header[68..72]
                .try_into()
                .map_err(|_| StorageError::Integrity("Cabeçalho SQLite truncado.".into()))?,
        );
        if application != APPLICATION_ID {
            return Err(StorageError::Integrity(
                "Banco não pertence ao Prumo.".into(),
            ));
        }
        let version = u32::from_be_bytes(
            header[60..64]
                .try_into()
                .map_err(|_| StorageError::Integrity("Cabeçalho SQLite truncado.".into()))?,
        );
        check_version("banco", version, DATABASE_VERSION)?;
    }
    Ok(())
}

pub(crate) fn check_size(size: u64, limit: u64, context: &str) -> Result<()> {
    if size > limit {
        return Err(StorageError::Limit(context.into()));
    }
    Ok(())
}

fn read_bounded_file(path: &Path, limit: u64) -> Result<Vec<u8>> {
    check_size(fs::metadata(path)?.len(), limit, "banco")?;
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    check_size(bytes.len() as u64, limit, "banco")?;
    Ok(bytes)
}

#[cfg(test)]
mod transaction_validation_tests {
    use super::*;
    use prumo_domain::{ChangeRecord, LengthUnit};

    fn project() -> Project {
        Project {
            schema_version: PROJECT_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            name: "Fixture de igualdade transacional".into(),
            revision: 1,
            units: LengthUnit::Metre,
            objects: vec![],
            environments: vec![],
            networks: vec![],
            ports: vec![],
            connections: vec![],
            history: vec![ChangeRecord {
                revision: 1,
                actor: "fixture".into(),
                operation: "create".into(),
                timestamp_unix_ms: 0,
            }],
            normative: Default::default(),
            model: Default::default(),
            dependencies: Default::default(),
            extensions: BTreeMap::new(),
        }
    }

    #[test]
    fn reused_validation_requires_exact_reconstructed_bytes_in_the_same_call() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = SQLiteWorkingStore::open(&directory.path().join("proof.sqlite")).unwrap();
        let project = project();
        let record = CommitRecord {
            command_id: Uuid::new_v4(),
            actor: "fixture".into(),
            operation: "create".into(),
            reason: None,
            timestamp_unix_ms: 0,
            command: Some(serde_json::json!({"operation":"create"})),
        };
        store.commit(None, &project, record).unwrap();
        let bytes = serde_json::to_vec(&serde_json::to_value(&project).unwrap()).unwrap();
        assert!(read_contents_with_validated_project(&store.connection, Some(&bytes)).is_ok());
        let mut other = project.clone();
        other.name = "Outro estado igualmente válido, mas diferente".into();
        other.validate().unwrap();
        let wrong_bytes = serde_json::to_vec(&serde_json::to_value(&other).unwrap()).unwrap();
        assert!(
            matches!(read_contents_with_validated_project(&store.connection,Some(&wrong_bytes)),Err(StorageError::Integrity(message)) if message.contains("bytes validados"))
        );
        assert_eq!(store.load().unwrap(), Some(project));
    }

    fn encoded_fields(project: &Project) -> (StoredProject, BTreeMap<String, Vec<u8>>) {
        let value = serde_json::to_value(project).unwrap();
        let mut objects = BTreeMap::new();
        let mut fields = BTreeMap::new();
        for (name, value) in value.as_object().unwrap() {
            let bytes = serde_json::to_vec(value).unwrap();
            let hash = hash_bytes(&bytes);
            fields.insert(name.clone(), hash.clone());
            objects.insert(hash, bytes);
        }
        (
            StoredProject {
                version: DATABASE_VERSION,
                model_version: PROJECT_SCHEMA_VERSION,
                project_id: project.id,
                revision: project.revision,
                fields,
            },
            objects,
        )
    }

    fn reconstruct_fields(
        envelope: &StoredProject,
        mut objects: BTreeMap<String, Vec<u8>>,
    ) -> Result<Project> {
        let bytes = serde_json::to_vec(envelope).unwrap();
        let hash = hash_bytes(&bytes);
        objects.insert(hash.clone(), bytes);
        reconstruct(&hash, &objects)
    }

    #[test]
    fn reconstruction_requires_one_complete_value_per_field() {
        let original = project();
        let (envelope, objects) = encoded_fields(&original);
        assert_eq!(
            reconstruct_fields(&envelope, objects.clone()).unwrap(),
            original
        );
        for fragment in [
            br#"null,"injected":true"#.as_slice(),
            br#""Fixture","injected":true"#.as_slice(),
            br#""Fixture" "second-value""#.as_slice(),
        ] {
            let mut modified_envelope = envelope.clone();
            let mut modified_objects = objects.clone();
            let hash = hash_bytes(fragment);
            modified_objects.insert(hash.clone(), fragment.to_vec());
            modified_envelope.fields.insert("name".into(), hash);
            assert!(matches!(
                reconstruct_fields(&modified_envelope, modified_objects),
                Err(StorageError::Json(error)) if error.to_string().contains("trailing characters")
            ));
        }
    }
}
