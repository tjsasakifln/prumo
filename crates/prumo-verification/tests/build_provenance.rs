use std::{
    fs,
    path::Path,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct TemporaryRepository(std::path::PathBuf);

impl Drop for TemporaryRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(root: &Path, program: &str, args: &[&str]) -> String {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn executable_output(root: &Path, target: &Path, profile: Option<(&str, &str)>) -> String {
    let mut command = Command::new(env!("CARGO"));
    command
        .args([
            "run",
            "--offline",
            "--quiet",
            "--manifest-path",
            "crates/probe/Cargo.toml",
        ])
        .env("CARGO_TARGET_DIR", target)
        .current_dir(root);
    if let Some((debug_info, opt_level)) = profile {
        command
            .env("CARGO_PROFILE_DEV_DEBUG", debug_info)
            .env("CARGO_PROFILE_DEV_OPT_LEVEL", opt_level);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn executable_commit(root: &Path, target: &Path) -> String {
    executable_output(root, target, None)
        .split('|')
        .next()
        .unwrap()
        .to_owned()
}

fn commit_only(root: &Path, message: &str) {
    let index = root.join(run(root, "git", &["rev-parse", "--git-path", "index"]));
    let modified = fs::metadata(&index).unwrap().modified().unwrap();
    run(
        root,
        "git",
        &["commit", "--allow-empty", "--message", message],
    );
    // Git may refresh the index for an empty commit. Preserve its timestamp so
    // the old index watcher cannot mask a failure to observe the changed ref.
    fs::File::options()
        .write(true)
        .open(index)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
}

fn fixture_repository() -> TemporaryRepository {
    let unique = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
    let temp = TemporaryRepository(std::env::temp_dir().join(format!(
        "prumo-provenance-{}-{unique}-{sequence}",
        std::process::id()
    )));
    let root = temp.0.join("repository");
    fs::create_dir_all(root.join("crates/probe/src")).unwrap();
    fs::create_dir_all(root.join("apps")).unwrap();
    fs::write(root.join("apps/README.md"), "Provenance test fixture.\n").unwrap();
    let compiler_version = env!("PRUMO_VERIFY_COMPILER")
        .split_whitespace()
        .nth(1)
        .unwrap();
    fs::write(
        root.join("rust-toolchain.toml"),
        format!("[toolchain]\nchannel = \"{compiler_version}\"\nprofile = \"minimal\"\n"),
    )
    .unwrap();
    fs::write(
        root.join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"provenance-probe\"\nversion = \"0.0.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/probe\"]\nresolver = \"3\"\n",
    )
    .unwrap();
    fs::write(
        root.join("crates/probe/Cargo.toml"),
        "[package]\nname = \"provenance-probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(
        root.join("crates/probe/build.rs"),
        include_str!("../build.rs"),
    )
    .unwrap();
    fs::write(
        root.join("crates/probe/src/main.rs"),
        r#"fn main() {
    println!("{}|{}|{}|{}|{}",
        env!("PRUMO_VERIFY_COMMIT"),
        env!("PRUMO_VERIFY_PROFILE"),
        env!("PRUMO_VERIFY_DEBUG"),
        env!("PRUMO_VERIFY_OPT_LEVEL"),
        cfg!(debug_assertions));
}
"#,
    )
    .unwrap();
    run(&root, "git", &["init", "--initial-branch=main"]);
    run(
        &root,
        "git",
        &["config", "user.name", "Prumo provenance test"],
    );
    run(
        &root,
        "git",
        &["config", "user.email", "test@invalid.example"],
    );
    run(&root, "git", &["config", "commit.gpgsign", "false"]);
    run(&root, "git", &["add", "."]);
    run(&root, "git", &["commit", "--message", "fixture"]);
    run(&root, "git", &["pack-refs", "--all", "--no-prune"]);
    temp
}

#[test]
fn commit_only_changes_refresh_the_binary_in_regular_and_linked_worktrees() {
    let temp = fixture_repository();
    let root = temp.0.join("repository");
    let first = run(&root, "git", &["rev-parse", "HEAD"]);
    assert_eq!(executable_commit(&root, &temp.0.join("target-main")), first);
    commit_only(&root, "only commit changed");
    let second = run(&root, "git", &["rev-parse", "HEAD"]);
    assert_ne!(first, second);
    assert_eq!(
        executable_commit(&root, &temp.0.join("target-main")),
        second
    );

    let linked = temp.0.join("linked");
    run(
        &root,
        "git",
        &[
            "worktree",
            "add",
            "-b",
            "linked-probe",
            linked.to_str().unwrap(),
        ],
    );
    assert!(linked.join(".git").is_file());
    assert_eq!(
        executable_commit(&linked, &temp.0.join("target-linked")),
        second
    );
    commit_only(&linked, "linked commit changed");
    let third = run(&linked, "git", &["rev-parse", "HEAD"]);
    assert_ne!(second, third);
    assert_eq!(
        executable_commit(&linked, &temp.0.join("target-linked")),
        third
    );
}

#[test]
fn effective_profile_overrides_are_embedded_in_the_binary_without_source_changes() {
    let temp = fixture_repository();
    let root = temp.0.join("repository");
    let target = temp.0.join("target-profile");
    let commit = run(&root, "git", &["rev-parse", "HEAD"]);
    // Reuse one source tree and target: changing Cargo's effective settings must
    // refresh the embedded values even though PROFILE remains "debug". DEBUG is
    // a presence flag, so debug=1 yields "true", and debug=0 yields "false".
    for (debug_info, opt_level, expected_debug) in [
        ("0", "0", "false"),
        ("1", "0", "true"),
        ("0", "1", "false"),
        ("0", "0", "false"),
    ] {
        let output = executable_output(&root, &target, Some((debug_info, opt_level)));
        assert_eq!(
            output,
            format!("{commit}|debug|{expected_debug}|{opt_level}|true"),
            "the executable must report its effective profile and retain debug assertions"
        );
    }
}
