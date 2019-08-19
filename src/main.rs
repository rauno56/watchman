extern crate dialoguer;

use json5;
use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::Write;

mod system;

use crate::system::run_from_string;
use system::get_by_pid;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
enum ProcessStatus {
    Running(i32),
    Invalid(i32),
    Stopped,
    Disabled,
}

//TODO: refactor ProcessConfig into a Map instead to have name as an index more naturally
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct ProcessConfig {
    name: String,
    cmd: String,
    status: Option<ProcessStatus>,
}

impl ProcessConfig {
    //TODO: see if you can make status as non-optional and use something like "New" as a variant
    fn get_pid(&self) -> Option<i32> {
        match self.status {
            Some(ProcessStatus::Running(proc)) | Some(ProcessStatus::Invalid(proc)) => Some(proc),
            _ => None,
        }
    }

    fn check_status(&self) -> ProcessStatus {
        //? Is there a way to avoid nestedness here?
        /*
            My initial ambition was to somehow have chained calls instead of nested ones,
            each one unwrapping another layer of Option.
        */
        self.get_pid().map_or(ProcessStatus::Disabled, |pid| {
            get_by_pid(pid).map_or(ProcessStatus::Stopped, |proc| {
                if self.cmd == proc.cmd {
                    ProcessStatus::Running(proc.pid)
                } else {
                    ProcessStatus::Invalid(proc.pid)
                }
            })
        })
    }

    fn update(&mut self) {
        self.status = Some(self.check_status());
    }

    fn run(&mut self) -> Result<(), Box<error::Error>> {
        self.update();

        if !self.is_running() {
            let res = run_from_string(&self.cmd)?;
            self.status = Some(ProcessStatus::Running(res));

            get_by_pid(res).map(|proc| {
                self.cmd = proc.cmd;
                self.name = proc.name;
            });
        }

        Result::Ok(())
    }

    fn kill(&mut self) -> bool {
        self.update();

        match self.status {
            Some(ProcessStatus::Running(pid)) => {
                let res = system::kill_by_pid(pid);
                self.update();
                res
            }
            _ => false,
        }
    }

    fn is_running(&self) -> bool {
        match self.status {
            Some(ProcessStatus::Running(_pid)) => true,
            _ => false,
        }
    }
}

impl fmt::Display for ProcessConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct State {
    processes: Vec<ProcessConfig>,
}

impl State {
    fn update_all(&mut self) {
        //? How to turn that into a for_each call?
        for process in &mut self.processes {
            process.update();
        }
    }
}

fn read_state(file_path: &str) -> Result<State, json5::Error> {
    println!("Reading from {:?}", file_path);
    let contents = fs::read_to_string(file_path).expect("Something went wrong reading the file");

    json5::from_str(&contents)
}

fn write_state(file_path: &str, state: &State) -> std::result::Result<(), Box<error::Error>> {
    println!("Writing {:?} to {:?}", state, file_path);

    let mut buffer = File::create(file_path)?;

    let serialized = json5::to_string(&state)?;
    buffer.write_all(serialized.as_bytes())?;

    return std::result::Result::Ok(());
}

fn main() -> std::result::Result<(), Box<error::Error>> {
    let _all_processes = vec![
        ProcessConfig {
            name: "all processes".to_string(),
            cmd: "sleep 10".to_string(),
            status: None,
        },
        ProcessConfig {
            name: "link to prod".to_string(),
            cmd: "sleep 20".to_string(),
            status: None,
        },
        ProcessConfig {
            name: "forward sql".to_string(),
            cmd: "sleep 40".to_string(),
            status: None,
        },
    ];
    let file_name = ".watchman.state.json";
    let mut state: State = read_state(file_name).unwrap();

    println!("{:?}", state);
    // state.processes[0].toggle();

    // let res = run_from_string(&"sleep   20".to_string());
    // println!("{:?}", res);

    // for proc in &state.processes {
    //   println!("{} is {:?}", proc, proc.check());
    // }
    let proc = ProcessConfig {
        name: "blha".to_string(),
        cmd: "sleep     32".to_string(),
        status: None,
    };
    println!("{:?}", proc);
    // proc.run();
    println!("{:?}", proc);

    state.update_all();

    let sleep = &mut state.processes[1];
    println!("before {:?}", sleep);
    if sleep.is_running() {
        println!("killing");
        sleep.kill();
    } else {
        println!("running");
        sleep.run()?;
    }
    println!("after {:?}", sleep);

    // let sleep = &mut state.processes[0];
    // println!("before {:?}", sleep);
    // sleep.run();
    // println!("after {:?}", sleep);

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
