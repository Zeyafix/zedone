#![forbid(unsafe_code)]
include!("../../scripts/comment-policy.rs");

fn main() {
    check_source_comments();
    #[cfg(feature = "android-shell")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        tauri_build::build();
    }
}
