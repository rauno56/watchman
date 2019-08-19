extern crate dialoguer;

use crate::state::State;
use crate::state::StateTrait;
use std::error;

mod state;
mod system;

fn main() -> std::result::Result<(), Box<error::Error>> {
    let file_name = "example.watchman.state.json";
    let mut state: State = State::from_file(file_name)?;

    println!("{:?}", state);

    state.update_all();

    let sleep = &mut state.get_mut("sleep20").unwrap();
    println!("before {:?}", sleep);
    if sleep.is_running() {
        println!("killing");
        sleep.kill();
    } else {
        println!("running");
        sleep.run()?;
    }
    println!("after {:?}", sleep);

    state.to_file(file_name)?;

    // let selections = Checkboxes::with_theme(&ColorfulTheme::default())
    //     .with_prompt("Pick your food")
    //     .items(&all_processes[..])
    //     // .clear(false)
    //     .defaults(&[true])
    //     .interact()
    //     .unwrap();

    // if selections.is_empty() {
    //     println!("You did not select anything :(");
    // } else {
    //     println!("You selected these things:");
    //     let mut new_state = State { enabled: vec![] };
    //     for selection in selections {
    //         println!("  {:?}", all_processes[selection]);
    //         new_state.enabled.push(all_processes[selection].clone());
    //     }
    //     write_state(file_name, new_state).unwrap();
    // }
    Ok(())
}
