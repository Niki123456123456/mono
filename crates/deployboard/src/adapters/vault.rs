#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct ConnectionConfig {
    pub endpoint: String,
    pub token: String,
}
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Config {
    pub connection: ConnectionConfig,
}

pub fn get_token(endpoint: &str) -> Result<String, String> {
    let mut command = login_command(endpoint);
    #[cfg(target_os = "macos")]
    command.env("PATH", macos_cli_path(std::env::var_os("PATH"))?);
    let output = command.output().map_err(|e| {
        format!("Could not start Vault CLI: {e}. Make sure the Vault CLI is installed.")
    })?;
    token_from_output(output)
}

fn login_command(endpoint: &str) -> std::process::Command {
    let mut command = std::process::Command::new("vault");
    command
        .arg("login")
        .arg("-format=json")
        .arg("-method=oidc")
        .arg(format!("-address={}", endpoint));
    command
}

#[cfg(target_os = "macos")]
fn macos_cli_path(path: Option<std::ffi::OsString>) -> Result<std::ffi::OsString, String> {
    // Finder-launched apps do not inherit the terminal's Homebrew PATH.
    // Keep existing entries first and only change the child process environment.
    let mut paths: Vec<_> = path
        .as_deref()
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .collect();
    for directory in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
        let directory = std::path::PathBuf::from(directory);
        if !paths.contains(&directory) {
            paths.push(directory);
        }
    }
    std::env::join_paths(paths).map_err(|e| format!("Could not set Vault CLI PATH: {e}"))
}

fn token_from_output(output: std::process::Output) -> Result<String, String> {
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Vault login failed ({}): {}",
            output.status,
            error.trim()
        ));
    }
    let response = serde_json::from_slice::<GetTokenResponse>(&output.stdout)
        .map_err(|e| format!("Invalid Vault login response: {e}"))?;
    if response.auth.client_token.trim().is_empty() {
        return Err("Vault login returned an empty token".to_owned());
    }
    Ok(response.auth.client_token)
}

pub async fn get_secret(
    config: &ConnectionConfig,
    path: &str,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let mut request = ehttp::Request::get(format!("{}/v1/secret/data/{}", config.endpoint, path));
    request.headers.insert("X-Vault-Token", &config.token);

    let response = super::http::fetch(&request, true).await?;
    let response =
        serde_json::from_slice::<GetSecretResponse>(&response.bytes).map_err(|e| e.to_string())?;
    Ok(response.data.data)
}

pub fn update_secret(
    config: &ConnectionConfig,
    path: &str,
    data: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    let body = serde_json::to_vec(&Secret { data: data.clone() }).map_err(|e| e.to_string())?;
    let mut request =
        ehttp::Request::post(format!("{}/v1/secret/data/{}", config.endpoint, path), body);
    request.headers.insert("X-Vault-Token", &config.token);

    let response = super::http::fetch_blocking(&request)?;
    if !response.ok {
        return Err("Saving not succeed response was not ok".to_owned());
    }
    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GetSecretResponse {
    pub data: Secret,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Secret {
    pub data: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GetTokenResponse {
    pub auth: Auth,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Auth {
    pub client_token: String,
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn output(code: i32, stdout: &[u8], stderr: &[u8]) -> std::process::Output {
        std::process::Output {
            status: std::process::ExitStatus::from_raw(code << 8),
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
        }
    }

    #[test]
    fn login_passes_endpoint_as_a_single_argument() {
        let command = login_command("https://vault.example.com");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(
            args,
            [
                "login",
                "-format=json",
                "-method=oidc",
                "-address=https://vault.example.com"
            ]
        );
    }

    #[test]
    fn login_handles_success_failure_and_invalid_responses() {
        assert_eq!(
            token_from_output(output(0, br#"{"auth":{"client_token":"test-token"}}"#, b"")),
            Ok("test-token".into())
        );
        // A failed command must never supply a token, even if stdout contains one.
        let error = token_from_output(output(
            1,
            br#"{"auth":{"client_token":"test-token"}}"#,
            b"permission denied\n",
        ))
        .unwrap_err();
        assert!(error.contains("permission denied"));
        assert!(!error.contains("test-token"));
        assert!(token_from_output(output(0, b"not json", b""))
            .unwrap_err()
            .contains("Invalid Vault login response"));
        assert!(
            token_from_output(output(0, br#"{"auth":{"client_token":""}}"#, b""))
                .unwrap_err()
                .contains("empty token")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn finder_path_includes_homebrew_locations() {
        for path in [None, Some("/usr/bin:/bin".into())] {
            let path = macos_cli_path(path).unwrap();
            let paths: Vec<_> = std::env::split_paths(&path).collect();
            assert!(paths.contains(&"/opt/homebrew/bin".into()));
            assert!(paths.contains(&"/usr/local/bin".into()));
            assert!(paths.contains(&"/usr/bin".into()));
            assert!(paths.contains(&"/bin".into()));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn terminal_path_keeps_precedence_without_duplicates() {
        let path = macos_cli_path(Some("/custom/bin:/opt/homebrew/bin:/usr/bin".into())).unwrap();
        let paths: Vec<_> = std::env::split_paths(&path).collect();
        assert_eq!(paths[0], std::path::PathBuf::from("/custom/bin"));
        assert_eq!(
            paths
                .iter()
                .filter(|path| **path == std::path::PathBuf::from("/opt/homebrew/bin"))
                .count(),
            1
        );
    }
}
