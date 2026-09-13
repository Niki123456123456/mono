#[derive(Debug, serde::Deserialize)]
pub struct ResourceNode {
    pub name: String,
    pub namespace: String,
}

fn server(endpoint: &str) -> Result<&str, String> {
    let host = endpoint.trim().strip_prefix("https://").unwrap_or(endpoint.trim()).trim_end_matches('/');
    if host.is_empty() || host.starts_with('-') || host.contains(['/', '?', '#', '@']) || host.chars().any(char::is_whitespace) {
        return Err("Argo CD endpoint must be an HTTPS hostname with an optional port.".into());
    }
    Ok(host)
}

#[cfg(not(target_arch = "wasm32"))]
fn run(args: &[&str]) -> Result<Vec<u8>, String> {
    let output = std::process::Command::new("argocd")
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|err| format!("Could not run argocd. Ensure the CLI is installed and on PATH: {}", err))?;
    if !output.status.success() {
        return Err(format!("Argo CD command failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(output.stdout)
}

#[cfg(target_arch = "wasm32")]
fn run(_args: &[&str]) -> Result<Vec<u8>, String> {
    Err("Argo CD CLI login and pod discovery require the desktop app.".into())
}

pub fn login(endpoint: &str) -> Result<(), String> {
    let host = server(endpoint)?;
    run(&["login", host, "--sso", "--grpc-web", "--name", host]).map(|_| ())
}

pub fn get_pods(endpoint: &str, application: &str) -> Result<Vec<ResourceNode>, String> {
    let host = server(endpoint)?;
    let application = format!("argocd/{}", application);
    let output = run(&[
        "app", "get-resource", &application, "--kind", "Pod", "--output", "yaml",
        "--argocd-context", host, "--server", host, "--grpc-web",
    ])?;
    parse_pods(&output)
}

fn parse_pods(output: &[u8]) -> Result<Vec<ResourceNode>, String> {
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct Pod { kind: String, metadata: ResourceNode }
    let mut pods = Vec::new();
    for document in serde_yaml::Deserializer::from_slice(output) {
        let value = serde_yaml::Value::deserialize(document).map_err(|err| err.to_string())?;
        if value.is_null() { continue; }
        let pod: Pod = serde_yaml::from_value(value).map_err(|_| {
            "Unexpected Argo CD output. Upgrade the argocd CLI to a version supporting app get-resource (3.3 or newer).".to_owned()
        })?;
        if pod.kind == "Pod" { pods.push(pod.metadata); }
    }
    pods.sort_by(|a, b| (&a.namespace, &a.name).cmp(&(&b.namespace, &b.name)));
    Ok(pods)
}

pub fn logs_url(application_url: &str, pod: &ResourceNode) -> String {
    let node = format!("/Pod/{}/{}/0", pod.namespace, pod.name);
    format!("{}?node={}&tab=logs", application_url, urlencoding::encode(&node))
}
