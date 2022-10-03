use crate::config::{bool_is_false, path_buf_is_new};
use crate::ProjectDirName;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::path::PathBuf;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Project {
    pub name: Option<String>,

    #[serde(default = "String::new", skip_serializing_if = "String::is_empty")]
    pub git: String,

    #[serde(default = "String::new", skip_serializing_if = "String::is_empty")]
    pub dir_name: ProjectDirName,

    #[serde(default = "PathBuf::new", skip_serializing_if = "path_buf_is_new")]
    pub dir: PathBuf,

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<String>,

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<String>,

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub on_install: Vec<String>,

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub on_up: Vec<String>,

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub after_up: Vec<String>,

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub compose_files: Vec<String>,

    #[serde(default = "bool::default", skip_serializing_if = "bool_is_false")]
    pub is_install: bool,
}

impl Project {
    pub fn new() -> Self {
        Project {
            name: None,
            git: "".to_string(),
            dir: PathBuf::new(),
            dir_name: "".to_string(),
            networks: Vec::new(),
            volumes: Vec::new(),
            on_install: Vec::new(),
            on_up: Vec::new(),
            after_up: Vec::new(),
            compose_files: Vec::new(),
            is_install: false,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone().unwrap_or(self.dir_name.clone())
    }

    pub fn merge(&self, provided_project: &Project) -> Project {
        let mut new_project = self.clone();

        let mut project = provided_project.clone();

        if project.name.is_some() {
            new_project.name = project.name.take();
        }

        if !project.git.is_empty() {
            new_project.git = project.git;
        }

        if !project.dir_name.is_empty() {
            new_project.dir_name = project.dir_name;
        }

        if !project.networks.is_empty() {
            new_project.networks = project.networks;
        }

        if !project.volumes.is_empty() {
            new_project.volumes = project.volumes;
        }

        if !project.on_install.is_empty() {
            new_project.on_install = project.on_install;
        }

        if !project.on_up.is_empty() {
            new_project.on_up = project.on_up;
        }

        if !project.after_up.is_empty() {
            new_project.after_up = project.after_up;
        }

        if !project.compose_files.is_empty() {
            new_project.compose_files = project.compose_files;
        }

        new_project.is_install = project.is_install;

        new_project
    }

    pub fn reset(&self) -> Project {
        let mut project = self.clone();

        project.git = String::new();
        project.networks = Vec::new();
        project.volumes = Vec::new();
        project.on_install = Vec::new();
        project.on_up = Vec::new();
        project.after_up = Vec::new();
        project.compose_files = Vec::new();

        project
    }
}
