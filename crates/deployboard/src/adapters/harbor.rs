#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct ConnectionConfig {
    pub endpoint: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Config {
    pub connection: ConnectionConfig,
    pub project_name: String,
    pub project_id : u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub artifact_count: u64,
    pub pull_count: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Artifact {
    pub repository_name: String,
    pub push_time: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Tag {
    pub name: String,
}

pub fn get_repositories(config: &ConnectionConfig, project_name: &str) -> Result<Artifact, String> {
    let mut request = ehttp::Request::get(format!(
        "{}/api/v2.0/projects/{}/repositories?page=1&page_size=100",
        config.endpoint, project_name
    ));
    request.headers.insert(
        "authorization",
        format!(
            "Basic {}",
            config
                .token
                .clone()
                .unwrap_or_else(|| base64::encode(format!(
                    "{}:{}",
                    config.username.as_ref().unwrap(),
                    config.password.as_ref().unwrap()
                )))
        ),
    );

    let response = super::http::fetch_blocking(&request)?;
    let artifact =
        serde_json::from_slice::<Artifact>(&response.bytes).map_err(|e| e.to_string())?;
    Ok(artifact)
}

pub async fn get_artifact(
    config: &ConnectionConfig,
    project_name: &str,
    repository_name: &str,
    artifact_reference: &str,
) -> Result<Artifact, String> {
    let mut request = ehttp::Request::get(format!(
        "{}/api/v2.0/projects/{}/repositories/{}/artifacts/{}",
        config.endpoint,
        project_name,
        urlencoding::encode(&urlencoding::encode(repository_name)),
        artifact_reference
    ));
    request.headers.insert(
        "authorization",
        format!(
            "Basic {}",
            config
                .token
                .clone()
                .unwrap_or_else(|| base64::encode(format!(
                    "{}:{}",
                    config.username.as_ref().unwrap(),
                    config.password.as_ref().unwrap()
                )))
        ),
    );

    let response = super::http::fetch(&request, true).await?;

    println!("get artifact: {} {} {}: {}", project_name, repository_name, artifact_reference, response.status_text);

    let artifact =
        serde_json::from_slice::<Artifact>(&response.bytes).map_err(|e| e.to_string())?;
    Ok(artifact)
}

pub async fn get_artifacts(
    config: &ConnectionConfig,
    project_name: &str,
    repository_name: &str, sort : &str, page_size : usize
) -> Result<Vec<Artifact>, String> {
    let mut request = ehttp::Request::get(format!(
        "{}/api/v2.0/projects/{}/repositories/{}/artifacts?sort={}&page_size={}",
        config.endpoint,
        project_name,
        urlencoding::encode(&urlencoding::encode(repository_name)), sort, page_size,
    ));
    request.headers.insert(
        "authorization",
        format!(
            "Basic {}",
            config
                .token
                .clone()
                .unwrap_or_else(|| base64::encode(format!(
                    "{}:{}",
                    config.username.as_ref().unwrap(),
                    config.password.as_ref().unwrap()
                )))
        ),
    );

    let response = super::http::fetch(&request, true).await?;

    let artifacts =
        serde_json::from_slice::<Vec<Artifact>>(&response.bytes).map_err(|e| e.to_string())?;
    Ok(artifacts)
}
