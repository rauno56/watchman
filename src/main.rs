extern crate dialoguer;

use crate::state::ProcessConfig;
use dialoguer::{theme::ColorfulTheme, Checkboxes};

use crate::state::State;
use crate::state::StateTrait;
use std::error;

mod state;
mod system;

// fn select<T>(data: Vec<T>, ids: Vec<usize>) {}

fn update_from_user(mut state: State) -> State {
    let mut all_processes: Vec<&mut ProcessConfig> = state.values_mut().collect();
    let defs: Vec<bool> = all_processes.iter().map(|proc| proc.is_enabled()).collect();

    let selections = Checkboxes::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick processes you want to be running")
        .items(&all_processes[..])
        // .clear(false)
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
                println!("Enabling {}", all_processes[i]);
                all_processes[i].run();
            }
            _ => {}
        };
    });

    state
}

fn main() -> std::result::Result<(), Box<error::Error>> {
    let file_name = "example.watchman.state.json";
    let mut state: State = State::from_file(file_name)?;

    state.fix_all();

    // println!("{:?}", state);

    state = update_from_user(state);

    // println!("{:?}", state);

    state.to_file(file_name)?;

    Ok(())
}
