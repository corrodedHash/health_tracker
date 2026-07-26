#![allow(clippy::unwrap_used, clippy::expect_used, unsafe_code)]

use std::env;

use crate::config::Config;

#[test]
fn env_overrides_file_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config");
    std::fs::create_dir_all(&config_path).unwrap();

    let toml_path = config_path.join("default.toml");
    std::fs::write(
        &toml_path,
        r#"
database_url = "postgres://file:5432/db"
cookie_key = "file-key"
listen_addr = "0.0.0.0:9999"
"#,
    )
    .unwrap();

    let original_cwd = env::current_dir().unwrap();
    env::set_current_dir(dir.path()).unwrap();

    // SAFETY: single-threaded test — no concurrent env access
    unsafe {
        env::set_var("HEALTH__DATABASE_URL", "postgres://env:5432/db");
        env::set_var("HEALTH__COOKIE_KEY", "env-key");
    }

    let result = Config::load();

    // SAFETY: single-threaded test — no concurrent env access
    unsafe {
        env::remove_var("HEALTH__DATABASE_URL");
        env::remove_var("HEALTH__COOKIE_KEY");
    }
    env::set_current_dir(original_cwd).unwrap();

    let config = result.unwrap();
    assert_eq!(config.database_url, "postgres://env:5432/db");
    assert_eq!(config.cookie_key, "env-key");
    assert_eq!(config.listen_addr, "0.0.0.0:9999");
}
