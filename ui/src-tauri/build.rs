#![forbid(unsafe_code)]
fn main() {
    #[cfg(feature = "android-shell")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        tauri_build::build();
    }
}
