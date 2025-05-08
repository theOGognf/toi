#![allow(dead_code)]

use reqwest::Response;
use std::path::PathBuf;
use std::process::{Command, Output};

pub async fn assert_ok_response(response: Response) -> Result<Response, String> {
    if response.status().is_success() {
        Ok(response)
    } else {
        let body = response.text().await.map_err(|err| format!("{err:?}"))?;
        Err(body)
    }
}

pub fn reset_database() -> Result<Output, String> {
    let migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");
    Command::new("diesel")
        .args([
            "database",
            "reset",
            "--migration-dir",
            migrations_dir.to_str().expect("invalid migration dir"),
        ])
        .output()
        .map_err(|err| format!("{err:?}"))
}
