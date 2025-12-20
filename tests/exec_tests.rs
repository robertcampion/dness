use assert_cmd::Command;
use dotenvy::dotenv;
use std::path::PathBuf;
use std::str::from_utf8;

#[test]
fn resolve_wan_on_no_arguments() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    match cmd.ok() {
        Ok(output) => {
            let stdout = std::str::from_utf8(&output.stdout).unwrap();
            assert!(stdout.contains("resolved address to"));
        }
        Err(e) => {
            let output = e.as_output().unwrap();
            let stdout = std::str::from_utf8(&output.stdout).unwrap();
            assert!(stdout.contains("no records found for Query"));
        }
    }
}

fn config_dir() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base.join("assets").join("test-configs")
}

fn integration_test(config_name: &str) {
    dotenv().unwrap();
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.arg("--config").arg(config_dir().join(config_name));
    match cmd.ok() {
        Ok(output) => {
            println!("stdout:\n{}", from_utf8(&output.stdout).unwrap());
            eprintln!("stderr:\n{}", from_utf8(&output.stderr).unwrap());
        }
        Err(e) => {
            let output = e.as_output().unwrap();
            println!("stdout:\n{}", from_utf8(&output.stdout).unwrap());
            eprintln!("stderr:\n{}", from_utf8(&output.stderr).unwrap());
            panic!("failed with exit code {}", output.status);
        }
    }
}

#[test]
#[ignore = "requires API key"]
fn cloudflare_integration_test() {
    integration_test("cloudflare.toml")
}

#[test]
#[ignore = "requires API key"]
fn godaddy_integration_test() {
    integration_test("godaddy.toml")
}
