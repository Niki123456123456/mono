#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct LinkTemplate {
    /// URL containing {panes}, replaced with URL-encoded Grafana Explore JSON.
    pub url_template: String,
    pub datasource: String,
    /// Supports {deployment}, {component}, and {env}.
    pub app_template: String,
}

impl LinkTemplate {
    pub fn url(&self, deployment: &str, component: &str, env: &str) -> String {
        let app = self.app_template
            .replace("{deployment}", deployment)
            .replace("{component}", component)
            .replace("{env}", env);
        let expr = format!("{{app={}}}", serde_json::to_string(&app).unwrap());
        let panes = serde_json::json!({
            "nru": {
                "datasource": self.datasource,
                "queries": [{
                    "refId": "A", "expr": expr, "queryType": "range",
                    "datasource": {"type": "loki", "uid": self.datasource},
                    "editorMode": "builder", "direction": "backward"
                }],
                "range": {"from": "now-24h", "to": "now"},
                "panelsState": {"logs": {"sortOrder": "Descending"}},
                "compact": false
            }
        });
        self.url_template.replace("{panes}", &urlencoding::encode(&panes.to_string()))
    }
}

