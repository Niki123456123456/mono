use std::collections::HashMap;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Config {
    pub gitlab: crate::adapters::gitlab::Config,
    pub vault: crate::adapters::vault::Config,
    pub harbor: crate::adapters::harbor::Config,
    pub envs: Vec<String>,
    pub sources: Vec<Source>,
}
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Source {
    pub gitlab_project: String,
    pub vault_path: Option<String>,
    pub vault_paths: Option<HashMap<String, String>>,
    pub argocd_endpoint: Option<String>,
    pub argocd_endpoints: Option<HashMap<String, String>>,
    pub argocd_prefix: Option<String>,
    pub env: Option<String>,
}