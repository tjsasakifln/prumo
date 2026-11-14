use prumo_components::process::{self, ProcessError, WorkerRequest};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

fn worker() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_prumo-component-worker"))
}

fn fault_binary(mode: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let filename = if cfg!(windows) {
        format!("{mode}.exe")
    } else {
        mode.to_owned()
    };
    let target = dir.path().join(filename);
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/fault_worker.rs");
    let status = Command::new(rustc)
        .args(["--edition", "2024"])
        .arg(source)
        .arg("-o")
        .arg(&target)
        .status()
        .unwrap();
    assert!(status.success());
    (dir, target)
}

#[test]
fn real_worker_runs_complete_battery_with_immutable_identity_and_no_engineering_release() {
    let request = WorkerRequest::new(None);
    let report = process::run_worker(
        &worker(),
        &request,
        &AtomicBool::new(false),
        Duration::from_secs(15),
    )
    .unwrap();
    assert!(
        report.passed(),
        "{:?}",
        report
            .experiments
            .iter()
            .filter(|item| !item.passed())
            .collect::<Vec<_>>()
    );
    assert_eq!(report.experiments.len(), 6);
    assert_eq!(report.request_id, request.request_id);
    assert!(!report.engineering_release);
    let (hash, bytes) = process::executable_identity(&worker()).unwrap();
    assert_eq!(report.executable_sha256, hash);
    assert_eq!(report.executable_bytes, bytes);
}

#[test]
fn deadline_reaps_a_hung_process_and_a_subsequent_run_succeeds() {
    let (directory, path) = fault_binary("hang");
    let request = WorkerRequest::new(None);
    let started = Instant::now();
    assert!(matches!(
        process::run_worker(
            &path,
            &request,
            &AtomicBool::new(false),
            Duration::from_millis(60)
        ),
        Err(ProcessError::Timeout)
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
    directory.close().unwrap();
    assert!(
        process::run_worker(
            &worker(),
            &request,
            &AtomicBool::new(false),
            Duration::from_secs(15)
        )
        .unwrap()
        .passed()
    );
}

#[test]
fn cancellation_reaps_the_process_and_closes_pipe_resources() {
    let (directory, path) = fault_binary("cancel");
    let cancel = Arc::new(AtomicBool::new(false));
    let trigger = cancel.clone();
    let interrupt = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        trigger.store(true, Ordering::Release);
    });
    assert!(matches!(
        process::run_worker(
            &path,
            &WorkerRequest::new(None),
            &cancel,
            Duration::from_secs(5)
        ),
        Err(ProcessError::Cancelled)
    ));
    interrupt.join().unwrap();
    directory.close().unwrap();
}

#[test]
fn abnormal_exit_malformed_output_and_excess_output_fail_closed() {
    for (mode, expected) in [
        ("crash", "exit"),
        ("invalid", "protocol"),
        ("oversized", "size"),
    ] {
        let (directory, path) = fault_binary(mode);
        let result = process::run_worker(
            &path,
            &WorkerRequest::new(None),
            &AtomicBool::new(false),
            Duration::from_secs(5),
        );
        assert!(
            matches!(
                (&result, expected),
                (Err(ProcessError::WorkerFailed), "exit")
                    | (Err(ProcessError::Protocol(_)), "protocol")
                    | (Err(ProcessError::MessageTooLarge), "size")
            ),
            "{mode}: {result:?}"
        );
        directory.close().unwrap();
    }
}

#[test]
fn worker_rejects_wrong_version_units_extra_fields_and_input_over_budget() {
    for input in [
        br#"{"protocol_version":9,"request_id":"a1a6d4cb-2406-46a8-a764-bd3bd720ba66","basis":null,"units":"SI","suite_id":"PRU-003.1"}"#.to_vec(),
        br#"{"protocol_version":1,"request_id":"a1a6d4cb-2406-46a8-a764-bd3bd720ba66","basis":null,"units":"mm","suite_id":"PRU-003.1"}"#.to_vec(),
        br#"{"protocol_version":1,"request_id":"a1a6d4cb-2406-46a8-a764-bd3bd720ba66","basis":null,"units":"SI","suite_id":"PRU-003.1","path":"arbitrary"}"#.to_vec(),
        vec![b'x';process::MAX_INPUT_BYTES+1],
    ] {
        let mut child=Command::new(worker()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        let mut stdin=child.stdin.take().unwrap();
        let _=stdin.write_all(&input);drop(stdin);
        let output=child.wait_with_output().unwrap();
        assert!(!output.status.success());assert!(output.stdout.is_empty());
    }
}
