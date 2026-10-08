fn main() {
    if std::env::var_os("CARGO_FEATURE_E2E").is_some() {
        println!("cargo:rustc-env=TG_ID=1");
        println!("cargo:rustc-env=TG_HASH=e2e");
        build_app();
        return;
    }

    let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.env");
    println!("cargo:rerun-if-changed={}", env_path.display());
    for name in ["TG_ID", "TG_HASH"] {
        println!("cargo:rerun-if-env-changed={name}");
    }

    if env_path.exists() {
        dotenvy::from_path(&env_path).expect("Could not read the repository .env file");
    }

    for name in ["TG_ID", "TG_HASH"] {
        if let Ok(value) = std::env::var(name) {
            println!("cargo:rustc-env={name}={value}");
        }
    }

    build_app();
}

fn build_app() {
    let mut attributes = tauri_build::Attributes::new();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows.manifest");

        println!("cargo:rerun-if-changed={}", manifest.display());
        // Tauri's resource manifest only reaches the app; test harnesses also need Common Controls v6.
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    }

    tauri_build::try_build(attributes).expect("Could not build Tauri application resources");
}
