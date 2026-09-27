fn main() {
    if std::env::var_os("CARGO_FEATURE_E2E").is_some() {
        tauri_build::build();
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

    tauri_build::build()
}
