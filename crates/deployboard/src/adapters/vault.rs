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
    let output = std::process::Command::new("vault")
        .arg("login")
        .arg("-format=json")
        .arg("-method=oidc")
        .arg(format!("-address={}", endpoint))
        .output()
        .map_err(|e| e.to_string())?;
    let response =
        serde_json::from_slice::<GetTokenResponse>(&output.stdout).map_err(|e| e.to_string())?;
    return Ok(response.auth.client_token);
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
