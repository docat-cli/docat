use crate::file::CONFIG_FILENAME;
use crate::{cwd, file};
use anyhow::Result;
use app::App;
use config::Config;
use std::path::PathBuf;
use std::{env, fs};

pub(crate) mod app;
mod app_config;
pub(crate) mod config;
mod project;

pub fn combine(app: &Option<String>) -> Result<App> {
    // try and load file from current directory
    let config = load_from(&cwd()).ok();
    let user_config_path = file::cached_config();
    let user_config_file = file::cached_config_file();
    let mut user_config = load_from(&user_config_path)?;
    let app_name = &get_app_name(app, &config, &user_config);
    let app = user_config.get(app_name);

    // find install directory
    let install_config = load_from(&app.config.install_dir)?;

    let mut merged_config = user_config.merge(&install_config);

    let project_configs = merged_config
        .get(app_name)
        .projects
        .iter()
        .filter(|(_, project)| !path_buf_is_new(&project.dir))
        .map(|(_, project)| &project.dir)
        .map(load_from)
        .filter_map(|result| result.ok())
        .collect::<Vec<Config>>();

    let mut all_configs = project_configs
        .iter()
        .fold(merged_config, |base_config, config| {
            base_config.merge(config)
        });

    fs::write(user_config_file, serde_yaml::to_string(&all_configs)?)?;

    Ok(all_configs.get(app_name).clone())
}

pub fn load_from(dir: &PathBuf) -> Result<Config> {
    let mut path = dir.clone();
    path.push(CONFIG_FILENAME);
    let yaml = fs::read_to_string(path)?;

    Ok(serde_yaml::from_str(&yaml)?)
}

fn get_app_name(
    app_name: &Option<String>,
    project_config: &Option<Config>,
    cached_config: &Config,
) -> String {
    app_name
        .clone()
        .or_else(|| {
            project_config
                .clone()
                .and_then(|config| config.apps.iter().next().map(|(key, _)| key.clone()))
        })
        .or_else(|| {
            let cwd = cwd();
            cached_config
                .apps
                .iter()
                .find(|(_, app)| app.projects.iter().any(|(_, project)| project.dir.eq(&cwd)))
                .map(|(app_name, _)| app_name.clone())
        })
        .or_else(|| env::var("DOCAT_APP").ok())
        .expect("Could not determine app name, try passing it in as a flag")
}

fn bool_is_false(bool: &bool) -> bool {
    bool.eq(&false)
}

fn path_buf_is_new(path_buf: &PathBuf) -> bool {
    path_buf.eq(&PathBuf::new())
}
