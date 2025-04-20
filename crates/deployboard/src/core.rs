pub async fn get_projects(
    config: &crate::config::Config,
    egui_ctx: egui::Context,
) -> std::collections::BTreeMap<String, crate::models::DeployProject> {
    let mut project_by_name =
        std::collections::BTreeMap::<String, crate::models::DeployProject>::default();
    for source in config.sources.iter() {
        egui_ctx.request_repaint();
        let paths = crate::adapters::gitlab::get_filepaths(
            &config.gitlab.connection,
            &source.gitlab_project,
            "main",
        )
        .await;
        egui_ctx.request_repaint();

        if let Err(err) = paths {
            println!("err :{}", err);
            return Default::default();
        }

        let paths = paths.unwrap_or_default();

        let git_project =
            crate::adapters::gitlab::get_project(&config.gitlab.connection, &source.gitlab_project).await
                .ok();

        let regex_for_name = regex::Regex::new(&config.gitlab.regex_for_name).unwrap();
        let regex_for_env = regex::Regex::new(
            &config
                .gitlab
                .regex_for_env
                .as_ref()
                .unwrap_or(&".*".to_string()),
        )
        .unwrap();
        for path in paths.iter() {
            let name = regex_for_name
                .captures(path)
                .and_then(|x| x.name("name"))
                .and_then(|x| Some(x.as_str()));
            let env = source
                .env
                .as_ref()
                .and_then(|x| Some(x.as_str()))
                .or(regex_for_env
                    .captures(path)
                    .and_then(|x| x.name("env"))
                    .and_then(|x| Some(x.as_str())));

            if let Some((name, env)) = name.zip(env) {
                let project = project_by_name.entry(name.to_string()).or_insert(
                    crate::models::DeployProject {
                        deployments_by_env: Default::default(),
                        details_open: false,
                        name: name.to_string(),
                    },
                );

                project.deployments_by_env.insert(
                    env.to_string(),
                    crate::models::Deployment {
                        name: name.to_string(),
                        env: env.to_string(),
                        path: path.to_string(),
                        source: source.clone(),
                        content: None,
                        git_project: git_project.clone(),
                    },
                );
            }
        }
    }

    return project_by_name;
}

pub fn fill_deployment(
    deployment: &mut crate::models::Deployment,
    config: &crate::config::Config,
    ctx: egui::Context,
) {
    let (sender, promise) = poll_promise::Promise::new();
    let vault_path = deployment.vault_path();
    let project = deployment.source.gitlab_project.clone();
    let path = deployment.path.clone();

    let config = config.clone();
    common::execute(async move {
        if let Ok(raw) = crate::adapters::gitlab::get_filecontent(
            &config.gitlab.connection,
            &project,
            &path,
            "main",
        ).await {
            let mut content = crate::models::DeploymentContent {
                raw,
                secrets: vec![],
                images: vec![],
            };
            if let Ok(parsed) = serde_yaml::from_str::<serde_yaml::Value>(&content.raw) {
                let mut secrets = vec![];
                for secret in  crate::yaml::get_fields(
                    &parsed,
                    config.gitlab.secret_path.as_str(),
                    Default::default(),
                )
                .into_iter() {
                    if let Some(secret) = get_secret(secret, &config, &vault_path).await {
                        secrets.push(secret);
                    }
                }
                content.secrets = secrets;

                let mut images = vec![];
                for field in crate::yaml::get_fields(
                    &parsed,
                    config.gitlab.image_path.as_str(),
                    Default::default(),
                )
                .into_iter(){
                    if let Some(image) = get_image(field, &config).await {
                        images.push(image);
                    }
                }
                content.images = images;

                let envs: Vec<_> = crate::yaml::get_fields(
                    &parsed,
                    config.gitlab.envs_path.as_str(),
                    Default::default(),
                )
                .into_iter()
                .map(crate::yaml::as_sequence)
                .flatten()
                .filter_map(|field| get_env(field, &config))
                .collect();

                for image in content.images.iter_mut() {
                    for env in envs.iter() {
                        if crate::yaml::starts_with_indexonly(&env.source_path, &image.source_path)
                        {
                            image.envs.push(env.clone());
                        }
                    }
                    let envs: std::collections::BTreeMap<_, _> = image
                        .envs
                        .iter()
                        .map(|e| (e.name.clone(), e.value.clone()))
                        .collect();
                    image.envs_json = Some(crate::models::EditorContext::new(envs))
                }
            }

            sender.send(content);
            ctx.request_repaint();
        }
    });
    deployment.content = Some(promise);
}

pub async fn get_secret<'a>(
    field: crate::yaml::YamlField<'a>,
    config: &crate::config::Config,
    vault_path: &Option<String>,
) -> Option<crate::models::Secret> {
    if let Some(vault_name) = crate::yaml::as_string(field.value) {
        if let Some(vault_path) = &vault_path {
            if let Ok(secrets) = crate::adapters::vault::get_secret(
                &config.vault.connection,
                &format!("{}/{}", vault_path, vault_name),
            ).await {
                let secret = crate::models::Secret {
                    source_path: field.path,
                    vault_name,
                    secrets: crate::models::EditorContext::new(secrets),
                };
                return Some(secret);
            }
        }
    }
    return None;
}

pub async fn get_image<'a>(
    field: crate::yaml::YamlField<'a>,
    config: &crate::config::Config,
) -> Option<crate::models::Image> {
    let image_regex =
        regex::Regex::new(r"(?<domain>[^:/]+)\/(?<project>[^:/]+)\/(?<path>[^:]+):(?<tag>[^:/@]+)")
            .unwrap();
    if let Some(image_name) = crate::yaml::as_string(field.value) {
        let identifier = image_regex
            .captures(&image_name)
            .and_then(|x| {
                x.name("domain")
                    .zip(x.name("project"))
                    .zip(x.name("path"))
                    .zip(x.name("tag"))
            })
            .and_then(|(((domain, project), path), tag)| {
                Some(crate::models::ArtifactIdentifier {
                    domain: domain.as_str().to_string(),
                    project: project.as_str().to_string(),
                    path: path.as_str().to_string(),
                    tag: tag.as_str().to_string(),
                })
            });

        if let Some(identifier) = identifier {
            let artifact = crate::adapters::harbor::get_artifact(
                &config.harbor.connection,
                &identifier.project,
                &identifier.path,
                &identifier.tag,
            ).await;
            if let Ok(artifact) = artifact {
                let artifacts = crate::adapters::harbor::get_artifacts(
                    &config.harbor.connection,
                    &identifier.project,
                    &identifier.path,
                    "-push_time",
                    20,
                ).await
                .unwrap_or_default();
                return Some(crate::models::Image {
                    source_path: field.path,
                    identifier,
                    artifact,
                    artifacts,
                    envs: vec![],
                    envs_json: Default::default(),
                });
            }
        }
    }
    return None;
}

pub fn get_env<'a>(
    field: crate::yaml::YamlField<'a>,
    config: &crate::config::Config,
) -> Option<crate::models::EnvVar> {
    let name = crate::yaml::get_field(
        field.value,
        config.gitlab.env_name_path.as_str(),
        Default::default(),
    )
    .and_then(|x| crate::yaml::as_string(x.value));
    let value = crate::yaml::get_field(
        field.value,
        config.gitlab.env_value_path.as_str(),
        Default::default(),
    )
    .and_then(|x| crate::yaml::as_string(x.value));
    if let Some((name, value)) = name.zip(value) {
        return Some(crate::models::EnvVar {
            source_path: field.path,
            name,
            value,
        });
    }

    return None;
}

