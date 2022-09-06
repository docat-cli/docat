use crate::config::app::App;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Config {
    #[serde(
        flatten,
        default = "BTreeMap::new",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub apps: BTreeMap<String, App>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            apps: BTreeMap::new(),
        }
    }

    pub fn add_app(&mut self, name: &String) -> &mut App {
        self.apps.insert(name.clone(), App::new());
        self.apps.get_mut(name).unwrap()
    }

    pub fn get(&mut self, name: &String) -> &mut App {
        self.apps.get_mut(name).unwrap()
    }

    pub fn merge(&self, config: &Config) -> Self {
        let mut new_config = self.clone();

        let merged_apps = &mut config
            .apps
            .iter()
            .map(|(key, app)| {
                (
                    key.clone(),
                    self.apps
                        .get(key)
                        .cloned()
                        .map(|original_app| original_app.merge(app))
                        .unwrap_or(app.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();

        new_config.apps.append(merged_apps);

        new_config
    }
}
