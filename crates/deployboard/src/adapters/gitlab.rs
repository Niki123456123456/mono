#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct ConnectionConfig {
    pub endpoint: String,
    pub token: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Author {
    pub email: String,
    pub name: String,
}
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Config {
    pub connection: ConnectionConfig,
    pub author: Author,
    pub regex_for_name: String,
    pub regex_for_env: Option<String>,
    pub secret_path: String,
    pub image_path: String,
    pub envs_path: String,
    pub env_name_path: String,
    pub env_value_path: String,
}

pub async  fn get_filecontent(
    config: &ConnectionConfig,
    project_id: &str,
    file_path: &str,
    branch: &str,
) -> Result<String, String> {
    let mut request = ehttp::Request::get(format!(
        "{}/api/v4/projects/{}/repository/files/{}/raw?ref={}",
        config.endpoint,
        project_id,
        urlencoding::encode(file_path),
        branch
    ));
    request.headers.insert("PRIVATE-TOKEN", &config.token);

    let response = super::http::fetch(&request, false).await?;
    let text = String::from_utf8(response.bytes).map_err(|e| e.to_string())?;
    Ok(text)
}

pub fn update_file(
    config: &ConnectionConfig,
    project_id: &str,
    file_path: &str,
    update: &FileUpdate,
) -> Result<(), String> {
    let body = serde_json::to_vec(update).map_err(|e| e.to_string())?;
    let mut request = ehttp::Request::post(
        format!(
            "{}/api/v4/projects/{}/repository/files/{}",
            config.endpoint,
            project_id,
            urlencoding::encode(file_path)
        ),
        body,
    );
    request.method = "PUT".to_owned();
    request.headers.insert("PRIVATE-TOKEN", &config.token);
    request.headers.insert("Content-Type", "application/json");

    let response = super::http::fetch_blocking(&request)?;
    if !response.ok {
        return Err(format!(
            "Update file not succeed,\n response was not ok: {}",
            response.status_text
        ));
    }
    Ok(())
}

pub async fn get_filepaths(
    config: &ConnectionConfig,
    project_id: &str,
    branch: &str,
) -> Result<Vec<String>, String> {
    let mut results = vec![];

    let mut page = 1;
    loop {
        let (mut filepaths,next_page) = get_filepaths2(config, project_id, branch, page).await?;
        results.append(&mut filepaths);
        if let Some(next_page) = next_page {
            page = next_page;
        } else {
            break;
        }
    }
    return Ok(results);
}

pub async fn get_filepaths2(
    config: &ConnectionConfig,
    project_id: &str,
    branch: &str,
    page : usize,
) -> Result<(Vec<String>, Option<usize>), String> {
    let mut request = ehttp::Request::get(format!(
        "{}/api/v4/projects/{}/repository/tree?recursive=true&per_page=100&page={}&ref={}",
        config.endpoint, project_id, page, branch
    ));
    request.headers.insert("PRIVATE-TOKEN", &config.token);

    let response = super::http::fetch(&request, false).await?;

    let next_page = response.headers.get("x-next-page").and_then(|s| s.parse::<usize>().ok());

    let entries = serde_json::from_slice::<Vec<RepositoryEntry>>(&response.bytes)
        .map_err(|e| e.to_string())?;
    Ok((entries
        .into_iter()
        .filter_map(|e| {
            if e.r#type == "blob" {
                Some(e.path)
            } else {
                None
            }
        })
        .collect(), next_page))
}


pub async fn get_project(config: &ConnectionConfig, project_id: &str) -> Result<Project, String> {
    let mut request = ehttp::Request::get(format!(
        "{}/api/v4/projects/{}",
        config.endpoint, project_id
    ));
    request.headers.insert("PRIVATE-TOKEN", &config.token);

    let response = super::http::fetch(&request, false).await?;
    let project = serde_json::from_slice::<Project>(&response.bytes).map_err(|e| e.to_string())?;

    Ok(project)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RepositoryEntry {
    pub r#type: String,
    pub path: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub web_url: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct FileUpdate {
    pub branch: String,
    pub commit_message: String,
    pub content: String,
    pub author_email: String,
    pub author_name: String,
}
