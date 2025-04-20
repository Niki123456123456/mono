pub struct DeployProject {
    pub deployments_by_env: std::collections::BTreeMap<String, Deployment>,
    pub details_open: bool,
    pub name: String,
}
pub struct Deployment {
    pub name: String,
    pub env: String,
    pub path: String,
    pub source: crate::config::Source,
    pub content: Option<poll_promise::Promise<DeploymentContent>>,
    pub git_project: Option<crate::adapters::gitlab::Project>,
}

impl Deployment {
    pub fn vault_path(&self) -> Option<String> {
        self.source
            .vault_path
            .as_ref()
            .or(self
                .source
                .vault_paths
                .as_ref()
                .and_then(|x| x.get(&self.env)))
            .cloned()
    }

    pub fn argocd_endpoint(&self) -> Option<String> {
        self.source
            .argocd_endpoint
            .as_ref()
            .or(self
                .source
                .argocd_endpoints
                .as_ref()
                .and_then(|x| x.get(&self.env)))
            .cloned()
    }

    pub fn argocd_prefix(&self) -> Option<String> {
        if let Some(prefix) = &self.source.argocd_prefix {
            return Some(format!("{}-", prefix));
        }
        return None;
    }

}

#[derive(Clone)]
pub struct DeploymentContent {
    pub raw: String,
    pub secrets: Vec<Secret>,
    pub images: Vec<Image>,
}
#[derive(Clone)]
pub struct Secret {
    pub source_path: crate::yaml::Path,
    pub vault_name: String,
    pub secrets: EditorContext<std::collections::BTreeMap<String, String>>,
}

#[derive(Clone)]
pub struct EditorContext<T: 'static> {
    pub orignal_data: T,
    pub orignal_text: String,
    pub text: String,
    pub always_saveable : bool
}

impl<T :  serde::Serialize> EditorContext<T> {
    pub fn new(data: T) -> Self {
        let text = serde_json::to_string_pretty(&data).unwrap();
        let orignal_text = text.clone();
        Self {
            orignal_data: data,
            text,
            orignal_text,
            always_saveable: false,
        }
    }
    pub fn always_saveable(mut self) -> Self {
        self.always_saveable = true;
        self
    }

    pub fn reset(&mut self) {
        self.text = self.orignal_text.clone();
    }

    pub fn has_changes(&self) -> bool {
        self.text != self.orignal_text
    }

    pub fn get_changed_data(&self, f: impl Fn(&String) -> Result<T, String>) -> Result<T, String> {
        (f)(&self.text)
    }
}

impl<T> std::ops::Deref for EditorContext<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.orignal_data
    }
}

impl<T> std::ops::DerefMut for EditorContext<T> {
    
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.orignal_data
    }

   
}

#[derive(Debug, Clone)]
pub struct EnvVar {
    pub source_path: crate::yaml::Path,
    pub name: String,
    pub value: String,
}

#[derive( Clone)]
pub struct Image {
    pub source_path: crate::yaml::Path,
    pub artifact: crate::adapters::harbor::Artifact,
    pub artifacts: Vec<crate::adapters::harbor::Artifact>,
    pub identifier: ArtifactIdentifier,
    pub envs: Vec<EnvVar>,
    pub envs_json: Option<EditorContext<std::collections::BTreeMap<String, String>>>,
}

#[derive(Debug, Clone)]
pub struct ArtifactIdentifier {
    pub domain: String,
    pub project: String,
    pub path: String,
    pub tag: String,
}

impl ArtifactIdentifier {
    pub fn to_string_with_tag(&self, tag: &str) -> String {
        format!("{}/{}/{}:{}", self.domain, self.project, self.path, tag)
    }
}

pub struct Modal {
    pub id: String,
    pub ui: Box<dyn FnMut(&mut egui::Ui, &mut ModalContext)>,
}

pub struct ModalContext<'a> {
    pub close: bool,
    pub reload: bool,
    pub toasts: &'a mut egui_notify::Toasts,
}

impl Modal {
    pub fn new(
        id: impl Into<String>,
        ui: impl FnMut(&mut egui::Ui, &mut ModalContext) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            ui: Box::new(ui),
        }
    }
}
