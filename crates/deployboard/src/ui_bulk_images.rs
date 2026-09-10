use std::collections::{BTreeMap, BTreeSet};

use crate::{
    adapters::gitlab::FileUpdate,
    models::{DeployProject, Modal},
};

#[derive(Clone)]
struct Target {
    project: String,
    file: String,
    raw: String,
    field: crate::yaml::Path,
    env: String,
    current: String,
}

#[derive(Default)]
struct Group {
    targets: Vec<Target>,
    artifacts: BTreeMap<String, crate::adapters::harbor::Artifact>,
}

fn prepare_files(
    targets: &[Target],
    new_image: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let mut files = BTreeMap::new();
    for target in targets {
        let key = (target.project.clone(), target.file.clone());
        if !files.contains_key(&key) {
            let yaml = serde_yaml::from_str::<serde_yaml::Value>(&target.raw)
                .map_err(|err| format!("{}: {}", target.file, err))?;
            files.insert(key.clone(), yaml);
        }
        crate::yaml::set_field(
            files.get_mut(&key).unwrap(),
            &target.field,
            &serde_yaml::Value::String(new_image.to_owned()),
            false,
        );
    }
    files
        .into_iter()
        .map(|((project, file), yaml)| {
            serde_yaml::to_string(&yaml)
                .map(|raw| (project, file, raw))
                .map_err(|err| err.to_string())
        })
        .collect()
}

pub fn show(
    project: &mut DeployProject,
    config: &crate::config::Config,
    ui: &mut egui::Ui,
    modals: &mut Vec<Modal>,
) {
    let mut loading = false;
    for deployment in project.deployments_by_env.values_mut() {
        if deployment.content.is_none() {
            crate::core::fill_deployment(deployment, config, ui.ctx().clone());
        }
        loading |= deployment
            .content
            .as_ref()
            .and_then(|content| content.ready())
            .is_none();
    }
    if loading {
        ui.label("Loading images across deployments…");
        return;
    }

    let mut groups: BTreeMap<String, Group> = BTreeMap::new();
    for deployment in project.deployments_by_env.values() {
        let Some(content) = deployment
            .content
            .as_ref()
            .and_then(|content| content.ready())
        else {
            continue;
        };
        for image in &content.images {
            let id = &image.identifier;
            let repository = format!("{}/{}/{}", id.domain, id.project, id.path);
            let group = groups.entry(repository).or_default();
            group.targets.push(Target {
                project: deployment.source.gitlab_project.clone(),
                file: deployment.path.clone(),
                raw: content.raw.clone(),
                field: image.source_path.clone(),
                env: deployment.env.clone(),
                current: id.to_string_with_tag(&id.tag),
            });
            for artifact in &image.artifacts {
                if artifact.preferred_tag().is_some() {
                    group
                        .artifacts
                        .insert(artifact.digest.clone(), artifact.clone());
                }
            }
        }
    }
    for (repository, group) in groups {
        let environments: BTreeSet<_> = group.targets.iter().map(|target| &target.env).collect();
        if environments.len() < 2 {
            continue;
        }
        ui.menu_button(
            format!(
                "Change all images: {} ({} deployments)",
                repository,
                environments.len()
            ),
            |ui| {
                if group.artifacts.is_empty() {
                    ui.label("No image choices available");
                }
                let mut artifacts: Vec<_> = group.artifacts.values().collect();
                artifacts.sort_by(|a, b| b.push_time.cmp(&a.push_time));
                for artifact in artifacts {
                    let tag = artifact.preferred_tag().unwrap();
                    if ui.button(tag).clicked() {
                        let new_image = format!("{}:{}@{}", repository, tag, artifact.digest);
                        let targets = group.targets.clone();
                        let gitlab = config.gitlab.clone();
                        let mut commit_message =
                            format!("{}: update all deployments to {}", project.name, tag);
                        modals.push(Modal::new(
                            format!("bulk:{}:{}", project.name, new_image),
                            move |ui, ctx| {
                                ui.set_width(750.0);
                                ui.heading("Change image across deployments");
                                ui.label(&new_image);
                                for target in &targets {
                                    ui.label(format!("{}: {}", target.env, target.current));
                                }
                                ui.label("Commit message");
                                ui.text_edit_singleline(&mut commit_message);
                                if ui.button("Save all").clicked() {
                                    match prepare_files(&targets, &new_image) {
                                        Err(err) => {
                                            ctx.toasts.error(err);
                                        }
                                        Ok(files) => {
                                            for (project, file, content) in files {
                                                let result = crate::adapters::gitlab::update_file(
                                                    &gitlab.connection,
                                                    &project,
                                                    &file,
                                                    &FileUpdate {
                                                        branch: "main".to_owned(),
                                                        commit_message: commit_message.clone(),
                                                        content,
                                                        author_email: gitlab.author.email.clone(),
                                                        author_name: gitlab.author.name.clone(),
                                                    },
                                                );
                                                match result {
                                                    Ok(()) => {
                                                        ctx.toasts
                                                            .success(format!("Updated {}", file));
                                                        ctx.reload = true;
                                                    }
                                                    Err(err) => {
                                                        ctx.toasts.error(format!(
                                                            "Failed to update {}: {}",
                                                            file, err
                                                        ));
                                                    }
                                                }
                                            }
                                            ctx.close = true;
                                        }
                                    }
                                }
                                if ui.button("Cancel").clicked() {
                                    ctx.close = true;
                                }
                            },
                        ));
                    }
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_matching_fields_in_same_file_and_preserves_other_values() {
        let target = Target {
            project: "1".into(),
            file: "values.yaml".into(),
            raw: "first: old\nsecond: old\nother: keep\n".into(),
            field: vec![crate::yaml::PathEntry::Field("first".into())],
            env: "int".into(),
            current: "old".into(),
        };
        let mut second = target.clone();
        second.field = vec![crate::yaml::PathEntry::Field("second".into())];
        let files = prepare_files(&[target, second], "repo:release@sha256:abc").unwrap();
        assert_eq!(files.len(), 1);
        let yaml: serde_yaml::Value = serde_yaml::from_str(&files[0].2).unwrap();
        assert_eq!(yaml["first"].as_str(), Some("repo:release@sha256:abc"));
        assert_eq!(yaml["second"], yaml["first"]);
        assert_eq!(yaml["other"].as_str(), Some("keep"));
    }
}
