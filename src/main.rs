extern crate dialoguer;

use json5;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// Expecting the process to be running with pid.
    Running(i32),
    /// Expected process is not running, but there is another one with pid.
    Invalid(i32),
    /// Expected process is not running, there is nothing with pid.
    Stopped(i32),
    /// Process is not expected to run.
    Disabled,
}

impl Default for ProcessStatus {
    fn default() -> Self {
        ProcessStatus::Disabled
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct ProcessConfig {
    name: String,
    cmd: String,
    #[serde(default)]
    status: ProcessStatus,
}

impl ProcessConfig {
    fn get_pid(&self) -> Option<i32> {
        match self.status {
            ProcessStatus::Running(proc)
            | ProcessStatus::Invalid(proc)
            | ProcessStatus::Stopped(proc) => Some(proc),
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
            get_by_pid(pid).map_or(ProcessStatus::Stopped(pid), |proc| {
                if self.cmd == proc.cmd {
                    ProcessStatus::Running(proc.pid)
                } else {
                    ProcessStatus::Invalid(proc.pid)
                }
            })
        })
    }

    fn update(&mut self) {
        self.status = self.check_status();
    }

    fn run(&mut self) -> Result<(), Box<error::Error>> {
        self.update();

        if !self.is_running() {
            let res = run_from_string(&self.cmd)?;
            self.status = ProcessStatus::Running(res);

            get_by_pid(res).map(|proc| {
                self.cmd = proc.cmd;
                // self.name = proc.name;
            });
        }

        Result::Ok(())
    }

    fn kill(&mut self) -> bool {
        self.update();

        match self.status {
            ProcessStatus::Running(pid) => {
                let res = system::kill_by_pid(pid);
                self.update();
                match self.status {
                    ProcessStatus::Stopped(_) => self.status = ProcessStatus::Disabled,
                    _ => {}
                }
                res
            }
            _ => false,
        }
    }

    fn is_running(&self) -> bool {
        match self.status {
            ProcessStatus::Running(_pid) => true,
            _ => false,
        }
    }
}

impl fmt::Display for ProcessConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

trait StateTrait {
    fn update_all(&mut self) {}
}

type State = HashMap<String, ProcessConfig>;

impl StateTrait for State {
    fn update_all(&mut self) {
        self.values_mut().for_each(|process| process.update());
    }
}

fn read_state(file_path: &str) -> Result<State, json5::Error> {
    println!("Reading from {:?}", file_path);
    let contents = fs::read_to_string(file_path).expect("Something went wrong reading the file");

    json5::from_str(&contents)
}

fn write_state(file_path: &str, state: &State) -> std::result::Result<(), Box<error::Error>> {
    println!("Writing Map {:?} to {:?}", state, file_path);

    let mut buffer = File::create(format!("{}", file_path))?;

    let serialized = json5::to_string(&state)?;
    buffer.write_all(serialized.as_bytes())?;

    return std::result::Result::Ok(());
}

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
    let file_name = ".watchman.state.json";
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

    let sleep = &mut state.get_mut("sleep").unwrap();
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
