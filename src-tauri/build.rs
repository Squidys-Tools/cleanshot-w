fn main() {
    // Skip the full Tauri build setup when the `app` feature is off (i.e.
    // during `cargo test --no-default-features`). This avoids linking
    // WebView2Loader.dll into the test binary.
    if std::env::var("CARGO_FEATURE_APP").is_err() {
        return;
    }
    tauri_build::build()
}
