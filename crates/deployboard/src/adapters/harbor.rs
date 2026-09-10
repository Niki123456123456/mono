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
    pub digest: String,
    pub push_time: chrono::DateTime<chrono::Utc>,
    #[serde(default, deserialize_with = "deserialize_tags")]
    pub tags: Vec<Tag>,
}

fn deserialize_tags<'de, D>(deserializer: D) -> Result<Vec<Tag>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(<Option<Vec<Tag>> as serde::Deserialize>::deserialize(deserializer)?.unwrap_or_default())
}

impl Artifact {
    pub fn preferred_tag(&self) -> Option<&str> {
        self.tags
            .iter()
            .find(|tag| tag.name.contains("release"))
            .or_else(|| self.tags.first())
            .map(|tag| tag.name.as_str())
    }
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
        "{}/api/v2.0/projects/{}/repositories/{}/artifacts?sort={}&page_size={}&with_tag=true&q=tags%3D*",
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

    if !(200..300).contains(&response.status) {
        return Err(format!("Harbor artifact list returned HTTP {} for {}/{}", response.status, project_name, repository_name));
    }
    let artifacts =
        serde_json::from_slice::<Vec<Artifact>>(&response.bytes)
            .map_err(|e| format!("Invalid Harbor artifact list for {}/{}: {}", project_name, repository_name, e))?;
    Ok(artifacts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_tagged_and_untagged_artifacts_deserialize() {
        let artifacts: Vec<Artifact> = serde_json::from_value(serde_json::json!([
            {"repository_name": "dxp/documentation-service", "digest": "sha256:abc", "push_time": "2026-01-01T00:00:00Z", "tags": null},
            {"repository_name": "dxp/documentation-service", "digest": "sha256:def", "push_time": "2026-01-01T00:00:00Z"},
            {"repository_name": "dxp/documentation-service", "digest": "sha256:123", "push_time": "2026-01-01T00:00:00Z", "tags": [{"name": "latest"}, {"name": "cb984ea1-88787696-release"}]}
        ])).unwrap();
        assert!(artifacts[0].tags.is_empty());
        assert!(artifacts[1].tags.is_empty());
        assert_eq!(artifacts[2].preferred_tag(), Some("cb984ea1-88787696-release"));
        assert_eq!(artifacts[2].digest, "sha256:123");
    }
}
