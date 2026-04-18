fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".into(), |s| s.trim().to_owned());
    println!("cargo:rustc-env=GITLAB_CLI_TARGET={target}");
    println!("cargo:rustc-env=GITLAB_CLI_GIT_SHA={sha}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    // HEAD often points to a branch ref; we must watch that file too,
    // otherwise commits to the current branch don't trigger a rebuild.
    if let Ok(head) = std::fs::read_to_string("../../.git/HEAD") {
        if let Some(ref_path) = head.strip_prefix("ref: ").and_then(|s| s.lines().next()) {
            println!("cargo:rerun-if-changed=../../.git/{ref_path}");
        }
    }
    println!("cargo:rerun-if-changed=../../.git/packed-refs");
}
