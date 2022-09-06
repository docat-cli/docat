use crate::config::app_config::AppConfig;
use crate::config::project::Project;
use crate::ProjectDirName;
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

    pub fn filter_projects(
        &self,
        projects: Vec<String>,
        include_install: bool,
    ) -> BTreeMap<ProjectDirName, Project> {
        if projects.is_empty() {
            return self.projects.clone();
        }

        let mut cloned_projects = self.projects.clone();
        let mut projects = projects
            .into_iter()
            .filter_map(|dir_or_project_name| {
                cloned_projects
                    .remove(&dir_or_project_name)
                    .or_else(|| {
                        self.projects
                            .clone()
                            .into_iter()
                            .find(|(_, project)| {
                                project
                                    .name
                                    .clone()
                                    .unwrap_or_default()
                                    .eq(&dir_or_project_name)
                            })
                            .and_then(|(dir_name, _)| cloned_projects.remove(&*dir_name))
                    })
                    .map(|project| (project.dir_name.clone(), project))
            })
            .collect::<BTreeMap<_, _>>();

        // if an install project still exists in the projects, add it
        if include_install {
            cloned_projects
                .into_iter()
                .find(|(_, project)| project.is_install)
                .map(|(dir_name, project)| projects.insert(dir_name, project));
        }

        return projects;
    }

    pub fn merge(&self, provided_app: &App) -> Self {
        let mut new_app = self.clone();

        new_app.config = new_app.config.merge(&provided_app.config);

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
                        .map(|mut merged_project| {
                            let shared_dir = new_app.config.shared_dir.clone();
                            let mut project_dir = shared_dir.clone();
                            project_dir.push(dir_name);
                            merged_project.dir_name = dir_name.clone();
                            merged_project.dir = project_dir.clone();

                            if merged_project.dir.eq(&new_app.config.install_dir) {
                                merged_project.is_install = true;
                            }

                            merged_project
                        })
                        .unwrap_or(project.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();

        new_app.projects.append(merged_projects);

        new_app
    }
}
