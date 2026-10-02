fn main() {
    println!("cargo:rerun-if-changed=../ui/dist");
    #[cfg(all(feature = "native-webdriver", target_os = "macos"))]
    tauri_build::try_build(
        tauri_build::Attributes::new().capabilities_path_pattern("native-webdriver/*.json"),
    )
    .expect("Native WebDriver test capability generation failed");
    #[cfg(not(all(feature = "native-webdriver", target_os = "macos")))]
    tauri_build::build();
}
