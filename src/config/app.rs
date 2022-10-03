use crate::config::app_config::AppConfig;
use crate::config::path_buf_is_new;
use crate::config::project::Project;
use crate::{cwd, ProjectDirName};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct App {
    #[serde(default = "BTreeMap::new", skip_serializing_if = "BTreeMap::is_empty")]
    pub projects: BTreeMap<ProjectDirName, Project>,

    #[serde(
        default = "AppConfig::new",
        skip_serializing_if = "AppConfig::is_empty"
    )]
    pub config: AppConfig,
}

impl App {
    pub fn new() -> Self {
        App {
            projects: BTreeMap::new(),
            config: AppConfig::new(),
        }
    }

    pub fn add_project(&mut self, name: &str) -> &mut Project {
        self.projects.insert(name.to_string(), Project::new());

        self.projects.get_mut(name).unwrap()
    }

    pub fn merge(&self, provided_app: &App) -> Self {
        let mut new_app = self.clone();

        new_app.config = new_app.config.merge(&provided_app.config);

        // if we don't have an install directory, construct it based on the cwd
        if path_buf_is_new(&new_app.config.install_dir) {
            new_app.config.install_dir = cwd();
            let mut shared_dir = new_app.config.install_dir.clone();
            shared_dir.pop();
            new_app.config.shared_dir = shared_dir;
        }

        let merged_projects = &mut provided_app
            .clone()
            .projects
            .iter()
            .map(|(dir_name, project)| {
                (
                    dir_name.clone(),
                    self.projects
                        .get(dir_name)
                        .cloned()
                        .map(|original_project| original_project.merge(project))
                        .or(Some(project.clone()))
                        .map(|mut project| {
                            let shared_dir = new_app.config.shared_dir.clone();
                            let mut project_dir = shared_dir.clone();
                            project_dir.push(dir_name);

                            project.dir_name = dir_name.clone();
                            project.dir = project_dir.clone();

                            project
                        })
                        .unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        new_app.projects.append(merged_projects);

        new_app
    }
}
