use std::{env, path::Path, process::Command};

fn command(program: &str, args: &[&str], root: &Path) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest path");
    let root = Path::new(&manifest).join("../..");
    let commit =
        command("git", &["rev-parse", "HEAD"], &root).unwrap_or_else(|| "unavailable".into());
    let dirty = command(
        "git",
        &["status", "--porcelain", "--untracked-files=normal"],
        &root,
    )
    .map_or(
        "unknown",
        |status| if status.is_empty() { "false" } else { "true" },
    );
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let compiler = command(&rustc, &["--version"], &root).unwrap_or_else(|| "unavailable".into());
    for (key, value) in [
        ("PRUMO_VERIFY_COMMIT", commit),
        ("PRUMO_VERIFY_DIRTY", dirty.into()),
        ("PRUMO_VERIFY_COMPILER", compiler),
        (
            "PRUMO_VERIFY_TARGET",
            env::var("TARGET").unwrap_or_default(),
        ),
        (
            "PRUMO_VERIFY_PROFILE",
            env::var("PROFILE").unwrap_or_default(),
        ),
        (
            "PRUMO_VERIFY_FEATURES",
            env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default(),
        ),
        (
            "PRUMO_VERIFY_FLAGS",
            env::var("CARGO_ENCODED_RUSTFLAGS")
                .unwrap_or_default()
                .replace('\u{1f}', " "),
        ),
    ] {
        println!("cargo:rustc-env={key}={value}");
    }
    for path in [
        "Cargo.lock",
        "Cargo.toml",
        "rust-toolchain.toml",
        "crates",
        "apps",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    // HEAD is usually symbolic, and linked worktrees use a .git file. Observe
    // the actual metadata paths so a commit with no Rust change still refreshes
    // the identity embedded in the executable.
    let mut metadata = vec!["HEAD".to_owned(), "index".into(), "packed-refs".into()];
    if let Some(reference) = command("git", &["symbolic-ref", "--quiet", "HEAD"], &root) {
        metadata.push(reference);
    }
    for name in metadata {
        if let Some(path) = command("git", &["rev-parse", "--git-path", &name], &root) {
            println!("cargo:rerun-if-changed={}", root.join(path).display());
        }
    }
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
}
