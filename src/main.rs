extern crate dialoguer;

use crate::state::read_state;
use crate::state::write_state;
use crate::state::ProcessConfig;
use crate::state::ProcessStatus;
use crate::state::State;
use crate::state::StateTrait;
use std::error;

mod state;
mod system;

fn main() -> std::result::Result<(), Box<error::Error>> {
    let _all_processes = vec![
        ProcessConfig {
            name: "all processes".to_string(),
            cmd: "sleep 10".to_string(),
            status: ProcessStatus::Disabled,
        },
        ProcessConfig {
            name: "link to prod".to_string(),
            cmd: "sleep 20".to_string(),
            status: ProcessStatus::Disabled,
        },
        ProcessConfig {
            name: "forward sql".to_string(),
            cmd: "sleep 40".to_string(),
            status: ProcessStatus::Disabled,
        },
    ];
    let file_name = "example.watchman.state.json";
    let mut state: State = read_state(file_name)?;

    println!("{:?}", state);

    let proc = ProcessConfig {
        name: "blha".to_string(),
        cmd: "sleep     32".to_string(),
        status: ProcessStatus::Disabled,
    };
    println!("{:?}", proc);
    // proc.run();
    println!("{:?}", proc);

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

    write_state(file_name, &state)?;

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
