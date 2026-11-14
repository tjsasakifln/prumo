mod common;

use common::{origin, project, record};
use prumo_domain::canonical::hash_bytes;
use prumo_storage::{
    CONTAINER_VERSION, MAX_ENTRIES, MAX_ENTRY_BYTES, OpaqueEntry, SQLiteWorkingStore, StorageError,
    decode_package, encode_package,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};
use tempfile::tempdir;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

fn pack(files: &BTreeMap<String, Vec<u8>>, method: CompressionMethod) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in files {
        writer
            .start_file(
                name,
                SimpleFileOptions::default()
                    .compression_method(method)
                    .unix_permissions(0o600),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn unpack(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|index| {
            let mut file = archive.by_index(index).unwrap();
            let name = file.name().unwrap().into_owned();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (name, content)
        })
        .collect()
}

fn valid_files() -> BTreeMap<String, Vec<u8>> {
    let dir = tempdir().unwrap();
    let mut store = SQLiteWorkingStore::open(&dir.path().join("work.sqlite")).unwrap();
    let project = project();
    store.commit(None, &project, record(&project)).unwrap();
    unpack(&encode_package(&store, &BTreeMap::new(), origin()).unwrap())
}

fn alter_manifest(files: &mut BTreeMap<String, Vec<u8>>, edit: impl FnOnce(&mut Value)) {
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    edit(&mut manifest);
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
}

fn retag_content(files: &mut BTreeMap<String, Vec<u8>>, name: &str, bytes: Vec<u8>) {
    alter_manifest(files, |manifest| {
        manifest["entries"][name]["sha256"] = json!(hash_bytes(&bytes));
        manifest["entries"][name]["size"] = json!(bytes.len());
    });
    files.insert(name.into(), bytes);
}

fn tiny_manifest() -> Vec<u8> {
    br#"{"format_version":2,"future_fixture":true}"#.to_vec()
}

fn expect_integrity(bytes: &[u8], detail: &str) {
    assert!(
        matches!(decode_package(bytes),Err(StorageError::Integrity(message)) if message.contains(detail)),
        "esperava recusa estrutural contendo {detail}"
    );
}

#[test]
fn future_container_database_model_and_wide_versions_are_incompatible_before_dto_parsing() {
    let golden = include_bytes!("fixtures/future-format.prumo");
    assert!(
        matches!(decode_package(golden), Err(StorageError::UnsupportedVersion { component, found:2, supported:1 }) if component == "contêiner")
    );
    for (field, version) in [
        ("format_version", 2_u64),
        ("format_version", u64::from(u32::MAX) + 1),
        ("format_version", u64::MAX),
        ("database_version", 2),
        ("model_version", 3),
    ] {
        let mut files = valid_files();
        alter_manifest(&mut files, |manifest| {
            manifest[field] = json!(version);
            manifest["future_field"] = json!({"must_not_be_discarded":true});
        });
        assert!(
            matches!(decode_package(&pack(&files, CompressionMethod::Stored)), Err(StorageError::UnsupportedVersion { found, .. }) if found == version),
            "campo {field}"
        );
    }
    let huge = BTreeMap::from([(
        "manifest.json".into(),
        br#"{"format_version":184467440737095516160,"new":1}"#.to_vec(),
    )]);
    assert!(matches!(
        decode_package(&pack(&huge, CompressionMethod::Stored)),
        Err(StorageError::UnsupportedVersionValue { value, .. }) if value=="184467440737095516160"
    ));
    for (numeric, normalized) in [("1e9999", "1e+9999"), ("-1", "-1"), ("1.5", "1.5")] {
        let files = BTreeMap::from([(
            "manifest.json".into(),
            format!("{{\"format_version\":{numeric},\"future\":true}}").into_bytes(),
        )]);
        assert!(
            matches!(decode_package(&pack(&files, CompressionMethod::Stored)),Err(StorageError::UnsupportedVersionValue { value, .. }) if value==normalized)
        );
    }
}

#[test]
fn all_documentary_projections_are_checked_against_the_database_even_with_new_valid_hashes() {
    let initial = valid_files();
    for path in ["project.json", "commands.json"] {
        let mut files = initial.clone();
        let mut value: Value = serde_json::from_slice(&files[path]).unwrap();
        if path == "project.json" {
            value["name"] = json!("projeção falsificada");
        } else {
            value[0]["record"]["actor"] = json!("ator falso");
        }
        retag_content(&mut files, path, serde_json::to_vec(&value).unwrap());
        assert!(matches!(
            decode_package(&pack(&files, CompressionMethod::Stored)),
            Err(StorageError::Integrity(_))
        ));
    }
    let mut files = initial.clone();
    alter_manifest(&mut files, |manifest| {
        manifest["project_sha256"] = json!("0".repeat(64))
    });
    assert!(decode_package(&pack(&files, CompressionMethod::Stored)).is_err());
    let mut files = initial.clone();
    let object = files
        .keys()
        .find(|name| name.starts_with("objects/"))
        .unwrap()
        .clone();
    files.remove(&object);
    alter_manifest(&mut files, |manifest| {
        manifest["entries"].as_object_mut().unwrap().remove(&object);
    });
    assert!(decode_package(&pack(&files, CompressionMethod::Stored)).is_err());
    let mut files = initial;
    files.insert("unlisted.bin".into(), vec![1, 2, 3]);
    assert!(decode_package(&pack(&files, CompressionMethod::Stored)).is_err());
}

#[test]
fn opaque_extensions_remain_byte_exact_across_stored_and_deflate_containers() {
    let dir = tempdir().unwrap();
    let mut store = SQLiteWorkingStore::open(&dir.path().join("work.sqlite")).unwrap();
    let project = project();
    let entries = BTreeMap::from([(
        "extensions/hvac-v999/opaque.bin".into(),
        OpaqueEntry {
            media_type: "application/vnd.test.hvac".into(),
            version: 999,
            bytes: (0..8192_u32)
                .map(|n| ((n * 97 + n / 7) % 256) as u8)
                .collect(),
        },
    )]);
    store
        .commit_with_extensions(None, &project, record(&project), &entries)
        .unwrap();
    let stored = encode_package(&store, &entries, origin()).unwrap();
    let first = decode_package(&stored).unwrap();
    assert_eq!(first.extensions, entries);
    let deflated = pack(&unpack(&stored), CompressionMethod::Deflated);
    let second = decode_package(&deflated).unwrap();
    assert_eq!(second.project, project);
    assert_eq!(second.extensions, entries);
    assert_eq!(
        second.manifest.project_sha256,
        first.manifest.project_sha256
    );
    let mut tampered = unpack(&stored);
    let path = entries.keys().next().unwrap();
    retag_content(&mut tampered, path, vec![0, 1, 2]);
    assert!(decode_package(&pack(&tampered, CompressionMethod::Stored)).is_err());
}

#[test]
fn highly_compressible_native_content_is_stored_and_remains_readable() {
    let dir = tempdir().unwrap();
    let mut store = SQLiteWorkingStore::open(&dir.path().join("work.sqlite")).unwrap();
    let project = project();
    let entries = BTreeMap::from([(
        "extensions/zero-fixture.bin".into(),
        OpaqueEntry {
            media_type: "application/octet-stream".into(),
            version: 1,
            bytes: vec![0; 2 * 1024 * 1024],
        },
    )]);
    store
        .commit_with_extensions(None, &project, record(&project), &entries)
        .unwrap();
    let stored = encode_package(&store, &entries, origin()).unwrap();
    let decoded = decode_package(&stored).unwrap();
    assert_eq!(decoded.extensions, entries);
    let archive = ZipArchive::new(Cursor::new(&stored)).unwrap();
    for index in 0..archive.len() {
        assert_eq!(
            archive.metadata().entry(index).unwrap().compression(),
            CompressionMethod::Stored
        );
    }
    let deflated = pack(&unpack(&stored), CompressionMethod::Deflated);
    assert!(matches!(
        decode_package(&deflated),
        Err(StorageError::Limit(_))
    ));
}

#[test]
fn invalid_paths_windows_names_and_normalized_collisions_are_never_extracted_or_accepted() {
    let attacks = [
        "../outside",
        "a/../../outside",
        "/absolute",
        "C:/drive",
        "C:ads",
        "extensions/file:stream",
        "extensions\\outside",
        "extensions//empty",
        "extensions/./dot",
        "extensions/trailing.",
        "extensions/trailing ",
        "extensions/NUL.bin",
        "extensions/COM1",
        "extensions/Lpt9.txt",
        "extensions/\0nul",
        "extensions/á.bin",
    ];
    for path in attacks {
        let files = BTreeMap::from([
            ("manifest.json".into(), tiny_manifest()),
            (path.into(), vec![1]),
        ]);
        let bytes = pack(&files, CompressionMethod::Stored);
        assert!(
            matches!(decode_package(&bytes), Err(StorageError::Integrity(_))),
            "caminho {path:?}"
        );
    }
    for pair in [
        ["extensions/A.bin", "extensions/a.bin"],
        ["extensions/file", "extensions/file/child"],
    ] {
        let files = BTreeMap::from([
            ("manifest.json".into(), tiny_manifest()),
            (pair[0].into(), vec![1]),
            (pair[1].into(), vec![2]),
        ]);
        assert!(matches!(
            decode_package(&pack(&files, CompressionMethod::Stored)),
            Err(StorageError::Integrity(_))
        ));
    }
}

#[test]
fn declared_counts_sizes_and_decompression_ratios_are_checked_before_expansion() {
    let files = BTreeMap::from([("manifest.json".into(), tiny_manifest())]);
    let original = pack(&files, CompressionMethod::Stored);
    let mut count = original.clone();
    let end = count.len() - 22;
    count[end + 8..end + 10].copy_from_slice(&((MAX_ENTRIES + 1) as u16).to_le_bytes());
    count[end + 10..end + 12].copy_from_slice(&((MAX_ENTRIES + 1) as u16).to_le_bytes());
    assert!(matches!(
        decode_package(&count),
        Err(StorageError::Limit(_))
    ));
    let mut size = original.clone();
    let central = size
        .windows(4)
        .position(|part| part == b"PK\x01\x02")
        .unwrap();
    size[central + 24..central + 28].copy_from_slice(&((MAX_ENTRY_BYTES + 1) as u32).to_le_bytes());
    assert!(matches!(decode_package(&size), Err(StorageError::Limit(_))));
    let mut ratio = original.clone();
    ratio[central + 20..central + 24].copy_from_slice(&1_u32.to_le_bytes());
    ratio[central + 24..central + 28].copy_from_slice(&1_000_000_u32.to_le_bytes());
    assert!(matches!(
        decode_package(&ratio),
        Err(StorageError::Limit(_))
    ));
    let mut encrypted = original.clone();
    encrypted[central + 8..central + 10].copy_from_slice(&1_u16.to_le_bytes());
    expect_integrity(&encrypted, "recurso não permitido");
    let mut method = original.clone();
    method[central + 10..central + 12].copy_from_slice(&12_u16.to_le_bytes());
    expect_integrity(&method, "recurso não permitido");
    let mut zip64 = original.clone();
    zip64[end + 12..end + 16].copy_from_slice(&u32::MAX.to_le_bytes());
    expect_integrity(&zip64, "ZIP64");
}

#[test]
fn duplicate_central_names_overlaps_and_inconsistent_local_headers_are_rejected() {
    let files = BTreeMap::from([
        ("manifest.json".into(), tiny_manifest()),
        ("same-a.bin".into(), vec![1]),
        ("same-b.bin".into(), vec![2]),
    ]);
    let original = pack(&files, CompressionMethod::Stored);
    let mut duplicate = original.clone();
    for offset in (0..duplicate.len() - 10).collect::<Vec<_>>() {
        if &duplicate[offset..offset + 10] == b"same-b.bin" {
            duplicate[offset..offset + 10].copy_from_slice(b"same-a.bin");
        }
    }
    expect_integrity(&duplicate, "Colisão");
    let central_offsets: Vec<_> = original
        .windows(4)
        .enumerate()
        .filter_map(|(index, part)| (part == b"PK\x01\x02").then_some(index))
        .collect();
    let mut overlap = original.clone();
    let first_local = overlap[central_offsets[1] + 42..central_offsets[1] + 46].to_vec();
    overlap[central_offsets[2] + 42..central_offsets[2] + 46].copy_from_slice(&first_local);
    expect_integrity(&overlap, "sobrepostas");
    let mut wrong_local = original.clone();
    let local_name = wrong_local
        .windows(10)
        .position(|value| value == b"same-a.bin")
        .unwrap();
    wrong_local[local_name + 5] = b'x';
    assert!(matches!(
        decode_package(&wrong_local),
        Err(StorageError::Zip(_))
    ));
    let mut prefixed = b"MZ-prefix".to_vec();
    prefixed.extend_from_slice(&original);
    expect_integrity(&prefixed, "Diretório ZIP");
    // A self-extracting prefix with corrected absolute offsets fools
    // ZipArchive::offset()==0; the minimum local-header offset still exposes it.
    let prefix_length = 9_usize;
    let mut corrected = b"MZ-prefix".to_vec();
    corrected.extend_from_slice(&original);
    for offset in &central_offsets {
        let shifted = offset + prefix_length;
        let local = u32::from_le_bytes(corrected[shifted + 42..shifted + 46].try_into().unwrap());
        corrected[shifted + 42..shifted + 46]
            .copy_from_slice(&(local + prefix_length as u32).to_le_bytes());
    }
    let end = corrected.len() - 22;
    let directory = u32::from_le_bytes(corrected[end + 16..end + 20].try_into().unwrap());
    corrected[end + 16..end + 20]
        .copy_from_slice(&(directory + prefix_length as u32).to_le_bytes());
    let mut accepted_by_library = ZipArchive::new(Cursor::new(&corrected)).unwrap();
    assert_eq!(accepted_by_library.offset(), 0);
    accepted_by_library.validate_header_consistency().unwrap();
    expect_integrity(&corrected, "Prefixo ZIP");
    let mut trailing = original.clone();
    trailing.push(0);
    expect_integrity(&trailing, "EOCD");
    for length in [0, 10, original.len() - 1, original.len() / 2] {
        assert!(matches!(
            decode_package(&original[..length]),
            Err(StorageError::Integrity(_))
        ));
    }
}

#[test]
fn symlinks_directories_and_crc_tampering_are_rejected() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("manifest.json", options).unwrap();
    writer.write_all(&tiny_manifest()).unwrap();
    writer
        .add_symlink("extensions/link", "../../outside", options)
        .unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    // A future version is inspected before extension semantics; a current
    // manifest exercises entry metadata without relying on extraction.
    let mut files = unpack(&bytes);
    let valid = valid_files();
    files.extend(valid);
    alter_manifest(&mut files, |manifest| {
        manifest["entries"]["extensions/link"] = json!({"sha256":hash_bytes(b"../../outside"),"size":13,"kind":"extension","media_type":"application/octet-stream","version":1});
    });
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in files {
        if name == "extensions/link" {
            writer.add_symlink(&name, "../../outside", options).unwrap();
        } else {
            writer.start_file(name, options).unwrap();
            writer.write_all(&content).unwrap();
        }
    }
    expect_integrity(&writer.finish().unwrap().into_inner(), "não regular");
    let files = valid_files();
    let mut bytes = pack(&files, CompressionMethod::Stored);
    let file = ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let index = file.index_for_name("project.json").unwrap();
    let start = file.metadata().entry(index).unwrap().header_start() as usize;
    let name_len = u16::from_le_bytes([bytes[start + 26], bytes[start + 27]]) as usize;
    let extra_len = u16::from_le_bytes([bytes[start + 28], bytes[start + 29]]) as usize;
    bytes[start + 30 + name_len + extra_len] ^= 1;
    assert!(matches!(decode_package(&bytes), Err(StorageError::Io(_))));
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer.start_file("manifest.json", options).unwrap();
    writer.write_all(&tiny_manifest()).unwrap();
    writer.add_directory("extensions/folder/", options).unwrap();
    expect_integrity(&writer.finish().unwrap().into_inner(), "caminho inválido");
}

#[test]
fn missing_duplicate_unknown_manifest_fields_and_role_mismatches_do_not_pass() {
    let mut files = valid_files();
    alter_manifest(&mut files, |manifest| {
        manifest.as_object_mut().unwrap().remove("project_id");
    });
    assert!(decode_package(&pack(&files, CompressionMethod::Stored)).is_err());
    let mut files = valid_files();
    alter_manifest(&mut files, |manifest| {
        manifest["unrecognized"] = json!(true)
    });
    assert!(decode_package(&pack(&files, CompressionMethod::Stored)).is_err());
    let mut files = valid_files();
    alter_manifest(&mut files, |manifest| {
        manifest["entries"]["state.sqlite"]["kind"] = json!("extension")
    });
    assert!(decode_package(&pack(&files, CompressionMethod::Stored)).is_err());
    let mut files = valid_files();
    let source = String::from_utf8(files["manifest.json"].clone()).unwrap();
    files.insert(
        "manifest.json".into(),
        format!("{{\"format_version\":{CONTAINER_VERSION},{}", &source[1..]).into_bytes(),
    );
    assert!(decode_package(&pack(&files, CompressionMethod::Stored)).is_err());
    let mut files = valid_files();
    let manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    let repeated = serde_json::to_string(&manifest["entries"]["project.json"]).unwrap();
    let source = String::from_utf8(files["manifest.json"].clone()).unwrap();
    let source = source.replacen(
        "\"entries\":{",
        &format!("\"entries\":{{\"project.json\":{repeated},"),
        1,
    );
    files.insert("manifest.json".into(), source.into_bytes());
    assert!(matches!(
        decode_package(&pack(&files, CompressionMethod::Stored)),
        Err(StorageError::Json(_))
    ));
}

#[test]
fn reserved_json_representation_is_unsupported_while_truncated_json_is_corrupt() {
    for version in [1, 2] {
        let text = format!(
            r#"{{"format_version":{version},"future":{{"\u0024serde_json::private::Number":"1"}}}}"#
        );
        let files = BTreeMap::from([("manifest.json".into(), text.into_bytes())]);
        assert!(matches!(
            decode_package(&pack(&files, CompressionMethod::Stored)),
            Err(StorageError::UnsupportedRepresentation(_))
        ));
    }
    let files = BTreeMap::from([(
        "manifest.json".into(),
        br#"{"format_version":1,"open":"unfinished"#.to_vec(),
    )]);
    assert!(matches!(
        decode_package(&pack(&files, CompressionMethod::Stored)),
        Err(StorageError::Json(_))
    ));
}

#[test]
fn encoded_owned_package_does_not_make_modified_received_bytes_trusted() {
    let directory = tempdir().unwrap();
    let mut store = SQLiteWorkingStore::open(&directory.path().join("encoded.sqlite")).unwrap();
    let project = project();
    store.commit(None, &project, record(&project)).unwrap();
    let package =
        prumo_storage::encode_validated_package(&store, &BTreeMap::new(), origin()).unwrap();
    assert_eq!(decode_package(package.as_bytes()).unwrap().project, project);
    let mut files = unpack(&package.into_bytes());
    let mut projection: Value = serde_json::from_slice(&files["project.json"]).unwrap();
    projection["name"] = json!("Bytes recebidos adulterados após consumo do tipo");
    retag_content(
        &mut files,
        "project.json",
        serde_json::to_vec(&projection).unwrap(),
    );
    assert!(
        matches!(decode_package(&pack(&files,CompressionMethod::Stored)),Err(StorageError::Integrity(message)) if message.contains("Projeção"))
    );
}
