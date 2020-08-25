use colored::*;
use dialoguer::{theme::ColorfulTheme, Checkboxes};
use std::error;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::state::ProcessConfig;
use crate::state::ProcessStatus;
use crate::state::State;
use crate::state::StateTrait;
use crate::utils::get_state_path;

mod state;
mod system;
mod utils;

// TODO: set cmd
// TODO: enable command by name
// TODO: disable command by name
// TODO: remove command
#[derive(Debug, StructOpt)]
enum SubCommand {
    #[structopt(name = "add")]
    /// Adds a command
    ///
    /// Example:
    /// watchman add "sleep 3" --name "short sleep"
    Add {
        command: String,
        #[structopt(long = "name")]
        name: Option<String>,
    },
    #[structopt(name = "run")]
    /// Runs a command handling restarts and logs
    ///
    /// Example:
    /// watchman run sleep 3
    Run {
        command: Vec<String>,
        #[structopt(long = "log")]
        output: Option<String>,
    },
    #[structopt(name = "config")]
    /// Shows configuration file location
    Config,
    #[structopt(name = "fix")]
    /// Ensures that all configured processes are running
    Fix,
    #[structopt(name = "show")]
    /// Updates and displays the state of all processes
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

fn interactive(mut state: State) -> Result<State, Box<dyn error::Error>> {
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
        println!(" {} {:+}", status_symbol, proc);
    })
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let args = Cli::from_args();

    let file_input: PathBuf = args.state_path.unwrap_or(get_state_path()?);
    let state_path = fs::canonicalize(file_input)?;

    match args.cmd {
        Some(subcommand) => match subcommand {
            SubCommand::Run { command, output } => {
                let s = dbg!(system::join(command));

                let output = output.and_then(|val| Some(PathBuf::from(val)));
                let output: Option<&PathBuf> = output.as_ref();
                system::keep_running_from_string(&s, &String::from("kala"), output)?;
            }
            SubCommand::Add { command, name } => {
                let mut state: State = State::from_file(&state_path)?;
                state.add(command, name, None)?;
                state.to_file(&state_path)?;
            }
            SubCommand::Config => println!("{}", state_path.to_str().unwrap()),
            SubCommand::Fix => {
                let mut state: State = State::from_file(&state_path)?;
                state.fix_all()?;
                show(&state);
                state.to_file(&state_path)?;
            }
            SubCommand::Show => {
                let mut state: State = State::from_file(&state_path)?;
                state.update_all();
                show(&state);
                state.to_file(&state_path)?;
            }
        },
        None => {
            let mut state: State = State::from_file(&state_path)?;
            state = interactive(state)?;
            state.to_file(&state_path)?;
        }
    }

    Ok(())
}
