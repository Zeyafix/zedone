fn check_source_comments() {
    let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest
        .ancestors()
        .find(|parent| parent.join("scripts/comment-policy.json").is_file())
        .expect("Build from a complete ZeDone checkout (scripts/comment-policy.json is required)");
    let output = std::process::Command::new("bun")
        .arg(root.join("scripts/source-comments.mjs"))
        .arg("--cargo")
        .current_dir(root)
        .output()
        .expect(
            "Comment policy requires Bun and bun install --frozen-lockfile at the repository root",
        );
    if !output.status.success() {
        panic!(
            "Source comment policy failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    print!("{}", String::from_utf8(output.stdout).unwrap());
}
