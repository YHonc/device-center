fn main() {
    tauri_build::build();
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        // Tauri emits this resource for binaries; unit-test executables also import
        // common-controls v6 APIs and require the same activation manifest.
        println!(
            "cargo:rustc-link-arg={}/libresource.a",
            std::env::var("OUT_DIR").unwrap()
        );
    }
}
