use dirs::home_dir;
use std::env::current_dir;
use std::path::PathBuf;
pub const CONFIG_FILENAME: &str = "docat.yml";

pub fn cwd() -> PathBuf {
    match current_dir() {
        Ok(path) => path,
        Err(_) => panic!("Could not determine the cwd"),
    }
}

pub fn cached_config_path() -> PathBuf {
    match home_dir() {
        None => panic!("Could not load home directory"),
        Some(mut file) => {
            file.push(".docat");
            file
        }
    }
}

pub fn cached_config_file() -> PathBuf {
    let mut file = cached_config_path();
    file.push(CONFIG_FILENAME);
    file
}
