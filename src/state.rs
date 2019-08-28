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
        // println!("Checking from {:?} for {:?}", self.status, self);
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
                    eprintln!(
                        "Expected {:?}, saw {:?} at {}",
                        self.cmd, proc.cmd, proc.pid
                    );
                    ProcessStatus::Invalid(proc.pid)
                }
            })
        })
    }

    fn update(&mut self) {
        self.status = self.check_status();
    }

    fn fix(&mut self) {
        if self.is_enabled() {
            self.run();
        }
    }

    pub fn run(&mut self) -> Result<(), Box<error::Error>> {
        self.update();

        if !self.is_running() {
            let res = run_from_string(&self.cmd)?;
            self.status = ProcessStatus::Running(res);

            get_by_pid(res).map(|proc| {
                self.cmd = proc.cmd;
            });
        }

        Result::Ok(())
    }

    pub fn kill(&mut self) -> bool {
        // println!("Killing {:?}", self.name);
        self.update();

        match self.status {
            ProcessStatus::Running(pid) => {
                let res = system::kill_by_pid(pid);
                // set disabled or... retry?
                self.kill();
                res
            }
            ProcessStatus::Stopped(_) => {
                self.status = ProcessStatus::Disabled;
                false
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

    pub fn is_enabled(&self) -> bool {
        match self.status {
            ProcessStatus::Disabled => false,
            _ => true,
        }
    }
}

impl fmt::Display for ProcessConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub trait StateTrait<DS = Self> {
    fn update_all(&mut self);
    fn fix_all(&mut self);
    fn from_file(file_path: &str) -> Result<DS, json5::Error>;
    fn to_file(&self, file_path: &str) -> std::result::Result<(), Box<error::Error>>;
}

pub type State = Vec<ProcessConfig>;

impl StateTrait for State {
    fn update_all(&mut self) {
        self.iter_mut().for_each(|process| process.update());
    }

    fn fix_all(&mut self) {
        self.iter_mut().for_each(|process| process.fix());
    }

    fn from_file(file_path: &str) -> Result<Self, json5::Error> {
        let contents =
            fs::read_to_string(file_path).expect("Something went wrong reading the file");

        json5::from_str(&contents)
    }

    fn to_file(&self, file_path: &str) -> std::result::Result<(), Box<error::Error>> {
        let mut buffer = File::create(format!("{}", file_path))?;

        let serialized = json5::to_string(&self)?;
        buffer.write_all(serialized.as_bytes())?;

        return std::result::Result::Ok(());
    }
}
