//! Isolated qualification probes, not product code.
//! Copy to src/lib.rs of the temporary Cargo project documented in PRU-005.md.

#[cfg(test)]
mod tests {
    use rusqlite::{
        Connection, TransactionBehavior,
        backup::{Backup, StepResult},
        config::DbConfig,
    };
    use std::{
        fs,
        io::{Cursor, Read},
    };
    use zip::{ZipArchive, ZipReadOptions};

    fn u16le(output: &mut Vec<u8>, value: u16) {
        output.extend(value.to_le_bytes());
    }

    fn u32le(output: &mut Vec<u8>, value: u32) {
        output.extend(value.to_le_bytes());
    }

    // Independent fixture encoder: the two header records follow PKWARE APPNOTE
    // 6.3.10, sections 4.3.7, 4.3.12 and 4.3.16. No zip writer creates this input.
    // Empty Stored payloads have CRC-32 zero. Names deliberately can disagree.
    fn empty_zip(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut output = Vec::new();
        let mut offsets = Vec::new();
        for (local_name, _) in entries {
            offsets.push(u32::try_from(output.len()).unwrap());
            u32le(&mut output, 0x0403_4b50);
            for value in [20, 0, 0, 0, 33] {
                u16le(&mut output, value);
            }
            for _ in 0..3 {
                u32le(&mut output, 0);
            }
            u16le(&mut output, u16::try_from(local_name.len()).unwrap());
            u16le(&mut output, 0);
            output.extend(local_name.as_bytes());
        }
        let directory_start = u32::try_from(output.len()).unwrap();
        for ((_, central_name), offset) in entries.iter().zip(offsets) {
            u32le(&mut output, 0x0201_4b50);
            for value in [20, 20, 0, 0, 0, 33] {
                u16le(&mut output, value);
            }
            for _ in 0..3 {
                u32le(&mut output, 0);
            }
            u16le(&mut output, u16::try_from(central_name.len()).unwrap());
            for _ in 0..4 {
                u16le(&mut output, 0);
            }
            u32le(&mut output, 0);
            u32le(&mut output, offset);
            output.extend(central_name.as_bytes());
        }
        let directory_size = u32::try_from(output.len()).unwrap() - directory_start;
        u32le(&mut output, 0x0605_4b50);
        u16le(&mut output, 0);
        u16le(&mut output, 0);
        u16le(&mut output, u16::try_from(entries.len()).unwrap());
        u16le(&mut output, u16::try_from(entries.len()).unwrap());
        u32le(&mut output, directory_size);
        u32le(&mut output, directory_start);
        u16le(&mut output, 0);
        output
    }

    #[test]
    fn zip_duplicate_count_is_not_the_public_unique_entry_count() {
        let bytes = empty_zip(&[
            ("manifest.json", "manifest.json"),
            ("manifest.json", "manifest.json"),
        ]);
        let archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert_eq!(archive.len(), 1);
        assert!(archive.has_duplicate_names());
    }

    #[test]
    fn zip_local_central_name_consistency_must_be_enabled() {
        let bytes = empty_zip(&[("local.json", "other.json")]);
        let mut default = ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
        let mut output = Vec::new();
        default
            .by_index(0)
            .unwrap()
            .read_to_end(&mut output)
            .unwrap();
        assert!(output.is_empty());
        let mut strict = ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert!(
            strict
                .by_index_with_options(0, ZipReadOptions::new().enforce_header_consistency(true))
                .is_err()
        );
    }

    #[test]
    fn adjusted_prefix_is_not_the_zip_archive_offset() {
        let mut original = empty_zip(&[("manifest.json", "manifest.json")]);
        let end = original.len() - 22;
        let directory =
            u32::from_le_bytes(original[end + 16..end + 20].try_into().unwrap()) as usize;
        let prefix = b"PK-prefix";
        let local =
            u32::from_le_bytes(original[directory + 42..directory + 46].try_into().unwrap());
        original[directory + 42..directory + 46]
            .copy_from_slice(&(local + prefix.len() as u32).to_le_bytes());
        original[end + 16..end + 20]
            .copy_from_slice(&((directory + prefix.len()) as u32).to_le_bytes());
        let mut bytes = prefix.to_vec();
        bytes.extend(original);
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        // ZIP's offset reports a correction relative to its recorded offsets.
        // It cannot detect a prefix already included in those recorded offsets.
        assert_eq!(archive.offset(), 0);
        assert_eq!(
            archive.metadata().entry(0).unwrap().header_start(),
            prefix.len() as u64
        );
        archive.validate_header_consistency().unwrap();
    }

    #[test]
    fn sqlite_backup_includes_wal_and_excludes_rolled_back_changes() {
        assert_eq!(rusqlite::version(), "3.54.1");
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.sqlite");
        let raw_copy = directory.path().join("raw-copy.sqlite");
        let backup_path = directory.path().join("backup.sqlite");
        let mut connection = Connection::open(&source).unwrap();
        connection.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY, revision INTEGER NOT NULL); INSERT INTO state VALUES(1,1);").unwrap();
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();
        connection
            .pragma_update(None, "synchronous", "FULL")
            .unwrap();
        connection
            .pragma_update(None, "wal_autocheckpoint", 0)
            .unwrap();
        let mode: String = connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        let sync: i64 = connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        assert_eq!(sync, 2);
        {
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            transaction
                .execute("UPDATE state SET revision = 2 WHERE id = 1", [])
                .unwrap();
            transaction.commit().unwrap();
        }
        {
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            transaction
                .execute("UPDATE state SET revision = 3 WHERE id = 1", [])
                .unwrap();
            // Drop performs the rollback; revision 3 must never reach the backup.
        }
        let revision = |db: &Connection| {
            db.query_row("SELECT revision FROM state WHERE id = 1", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap()
        };
        assert_eq!(revision(&connection), 2);
        assert!(directory.path().join("source.sqlite-wal").exists());
        fs::copy(&source, &raw_copy).unwrap();
        let stale = Connection::open(&raw_copy).unwrap();
        assert_eq!(
            revision(&stale),
            1,
            "The raw file alone omits committed WAL data"
        );
        let mut destination = Connection::open(&backup_path).unwrap();
        {
            let backup = Backup::new(&connection, &mut destination).unwrap();
            assert_eq!(backup.step(-1).unwrap(), StepResult::Done);
        }
        assert_eq!(revision(&destination), 2);
        let integrity: String = destination
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        drop(destination);
        let reopened = Connection::open(&backup_path).unwrap();
        assert_eq!(revision(&reopened), 2);
    }

    #[test]
    fn sqlite_defensive_configuration_is_effective_on_each_connection() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE value(x INTEGER); INSERT INTO value VALUES(7); CREATE VIEW hidden AS SELECT x FROM value;").unwrap();
        connection
            .set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)
            .unwrap();
        connection
            .set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)
            .unwrap();
        connection
            .set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, false)
            .unwrap();
        connection
            .set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_VIEW, false)
            .unwrap();
        assert!(
            connection
                .db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE)
                .unwrap()
        );
        assert!(
            !connection
                .db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA)
                .unwrap()
        );
        assert!(
            !connection
                .db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER)
                .unwrap()
        );
        assert!(
            !connection
                .db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_VIEW)
                .unwrap()
        );
        assert_eq!(
            connection
                .query_row("SELECT x FROM value", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            7
        );
        assert!(
            connection
                .query_row("SELECT x FROM hidden", [], |row| row.get::<_, i64>(0))
                .is_err()
        );
    }
}
