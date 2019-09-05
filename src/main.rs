use colored::*;
use dialoguer::{theme::ColorfulTheme, Checkboxes};
use directories::ProjectDirs;
use std::error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::state::ProcessConfig;
use crate::state::ProcessStatus;
use crate::state::State;
use crate::state::StateTrait;

mod state;
mod system;

#[derive(Debug, StructOpt)]
enum SubCommand {
    #[structopt(name = "add")]
    Add {
        command: String,
        #[structopt(long = "name")]
        name: Option<String>,
    },
    #[structopt(name = "config")]
    Config,
    #[structopt(name = "fix")]
    Fix,
    #[structopt(name = "show")]
    Show,
}

#[derive(Debug, StructOpt)]
#[structopt()]
struct Cli {
    #[structopt(long = "state", short = "s", parse(from_os_str))]
    state_path: Option<PathBuf>,
    #[structopt(subcommand)]
    cmd: Option<SubCommand>,
}

fn update_from_user(mut state: State) -> State {
    let mut all_processes: Vec<&mut ProcessConfig> = state.iter_mut().collect();
    let defs: Vec<bool> = all_processes.iter().map(|proc| proc.is_enabled()).collect();

    let selections = Checkboxes::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick processes you want to be running")
        .items(&all_processes[..])
        .defaults(&defs[..])
        .interact()
        .unwrap();

    defs.iter().enumerate().for_each(|(i, old)| {
        let new = selections.contains(&i);
        match (*old, new) {
            (true, false) => {
                println!("Disabling {}", all_processes[i]);
                all_processes[i].kill();
            }
            (false, true) => {
                match all_processes[i].run() {
                    Result::Err(err) => {
                        println!("Enabling {} FAILED with {}", all_processes[i], err);
                    }
                    _ => {
                        println!("Enabling {}", all_processes[i]);
                    }
                };
            }
            _ => {}
        };
    });

    state
}

fn interactive(mut state: State) -> Result<State, Box<error::Error>> {
    if state.is_empty() {
        println!("No processes configured. See \"watchman --help\"");
        return Result::Ok(state);
    }
    state.fix_all()?;

    state = update_from_user(state);

    Result::Ok(state)
}

fn show(state: &State) {
    if state.is_empty() {
        return println!("No processes configured. See \"watchman --help\"");
    }
    //? Would like to implement Display for state, but I'd need to wrap then instead of aliasing?
    state.iter().for_each(|proc| {
        let status_symbol = match proc.status {
            ProcessStatus::Disabled => " ".normal(),
            ProcessStatus::Running(_) => "✔".green().bold(),
            ProcessStatus::Invalid(_) => "✘".red().bold(),
            ProcessStatus::Stopped(_) => "?".yellow().bold(),
        };
        println!(" {} {}", status_symbol, proc);
    })
}

fn get_state_path() -> Result<PathBuf, Box<error::Error>> {
    let mut default_state_path: PathBuf = ProjectDirs::from("", "rauno56", "watchman")
        .unwrap()
        .config_dir()
        .to_path_buf();

    if !default_state_path.is_dir() {
        println!("Creating config dir: {:?}", default_state_path);
        std::fs::create_dir_all(&default_state_path)?;
    }

    default_state_path.push(PathBuf::from("state.json"));

    if !default_state_path.is_file() {
        println!("Creating state file: {:?}", default_state_path);
        let mut file = File::create(&default_state_path)?;
        file.write_all(b"[]")?;
    }

    Result::Ok(default_state_path)
}

fn main() -> std::result::Result<(), Box<error::Error>> {
    let args = Cli::from_args();

    let file_input: PathBuf = args.state_path.unwrap_or(get_state_path()?);
    let state_path = fs::canonicalize(file_input)?;
    let mut state: State = State::from_file(&state_path)?;

    match args.cmd {
        Some(subcommand) => match subcommand {
            SubCommand::Add { command, name } => state.add(command, name)?,
            SubCommand::Config => println!("{}", state_path.to_str().unwrap()),
            SubCommand::Fix => {
                state.fix_all()?;
                show(&state);
            }
            SubCommand::Show => {
                state.update_all();
                show(&state);
            }
        },
        None => {
            state = interactive(state)?;
        }
    }

    state.to_file(&state_path)?;

    Ok(())
}
