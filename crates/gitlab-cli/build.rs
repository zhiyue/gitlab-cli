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
}
