use crate::config::path_buf_is_new;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::path::PathBuf;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct AppConfig {
    #[serde(default = "String::new", skip_serializing_if = "String::is_empty")]
    pub shared_network: String,

    #[serde(default = "PathBuf::new", skip_serializing_if = "path_buf_is_new")]
    pub install_dir: PathBuf,

    #[serde(default = "PathBuf::new", skip_serializing_if = "path_buf_is_new")]
    pub shared_dir: PathBuf,
}

impl AppConfig {
    pub fn new() -> Self {
        AppConfig {
            shared_network: "".to_string(),
            install_dir: PathBuf::new(),
            shared_dir: PathBuf::new(),
        }
    }

    pub fn init(&mut self, dir: &PathBuf) {
        let install_dir = dir.clone();
        let mut shared_dir = dir.clone();
        shared_dir.pop();

        self.install_dir = install_dir;
        self.shared_dir = shared_dir;
    }

    pub fn is_empty(&self) -> bool {
        self.shared_network.is_empty()
            && path_buf_is_new(&self.install_dir)
            && path_buf_is_new(&self.shared_dir)
    }

    pub fn merge(&self, config: &AppConfig) -> AppConfig {
        let mut new_config = self.clone();

        if !config.shared_network.is_empty() {
            new_config.shared_network = config.shared_network.clone();
        }

        new_config
    }
}
