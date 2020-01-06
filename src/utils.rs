use crate::state::State;
use crate::state::StateTrait;
use directories::ProjectDirs;
use std::error;
use std::fs;
use std::path::PathBuf;

fn get_config_dir() -> Result<PathBuf, Box<dyn error::Error>> {
    let default_state_path: PathBuf = ProjectDirs::from("", "rauno56", "watchman")
        .unwrap()
        .config_dir()
        .to_path_buf();

    //? Existance of the config folder is checked every time PathBuf is aquired
    if !default_state_path.is_dir() {
        println!("Creating config dir: {:?}", default_state_path);
        fs::create_dir_all(&default_state_path)?;
    }

    Result::Ok(default_state_path)
}

pub fn get_state_path() -> Result<PathBuf, Box<dyn error::Error>> {
    let mut default_state_path: PathBuf = get_config_dir()?;

    default_state_path.push(PathBuf::from("state.json"));

    if !default_state_path.is_file() {
        println!("Creating state file: {:?}", default_state_path);
        State::new().to_file(&default_state_path)?;
    }

    Result::Ok(default_state_path)
}

pub fn get_output_path() -> Result<PathBuf, Box<dyn error::Error>> {
    let mut default_state_path: PathBuf = get_config_dir()?;

    default_state_path.push(PathBuf::from("logs"));

    Result::Ok(default_state_path)
}
