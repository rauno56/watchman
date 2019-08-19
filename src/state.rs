use json5;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::Write;

use crate::system;
use crate::system::get_by_pid;
use crate::system::run_from_string;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ProcessStatus {
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
pub struct ProcessConfig {
    pub name: String,
    pub cmd: String,
    #[serde(default)]
    pub status: ProcessStatus,
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

    pub fn run(&mut self) -> Result<(), Box<error::Error>> {
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

    pub fn kill(&mut self) -> bool {
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

    pub fn is_running(&self) -> bool {
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

pub trait StateTrait {
    fn update_all(&mut self) {}
}

pub type State = HashMap<String, ProcessConfig>;

impl StateTrait for State {
    fn update_all(&mut self) {
        self.values_mut().for_each(|process| process.update());
    }
}

pub fn read_state(file_path: &str) -> Result<State, json5::Error> {
    println!("Reading from {:?}", file_path);
    let contents = fs::read_to_string(file_path).expect("Something went wrong reading the file");

    json5::from_str(&contents)
}

pub fn write_state(file_path: &str, state: &State) -> std::result::Result<(), Box<error::Error>> {
    println!("Writing Map {:?} to {:?}", state, file_path);

    let mut buffer = File::create(format!("{}", file_path))?;

    let serialized = json5::to_string(&state)?;
    buffer.write_all(serialized.as_bytes())?;

    return std::result::Result::Ok(());
}
