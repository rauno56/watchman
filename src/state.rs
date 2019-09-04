use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

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

type MayError = Result<(), Box<error::Error>>;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ProcessConfig {
    pub name: Option<String>,
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

    fn fix(&mut self) -> MayError {
        if self.is_enabled() {
            self.run()?
        }

        Result::Ok(())
    }

    pub fn run(&mut self) -> MayError {
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
        match self.name {
            Some(ref name) => write!(f, "{}", name),
            None => write!(f, "{}", self.cmd),
        }
    }
}

pub type ParseError = serde_json::error::Error;

pub trait StateTrait<DS = Self> {
    fn update_all(&mut self);
    fn fix_all(&mut self) -> MayError;
    fn from_file<P: AsRef<Path>>(file_path: P) -> Result<DS, ParseError>;
    fn to_file<P: AsRef<Path>>(&self, file_path: P) -> MayError;
    fn add(&mut self, cmd: String, name: Option<String>) -> MayError;
}

pub type State = Vec<ProcessConfig>;

impl StateTrait for State {
    fn add(&mut self, cmd: String, name: Option<String>) -> MayError {
        let mut pc = ProcessConfig {
            cmd,
            name,
            status: ProcessStatus::Disabled,
        };

        pc.run()?;

        self.push(pc);

        Result::Ok(())
    }

    fn update_all(&mut self) {
        self.iter_mut().for_each(|process| process.update());
    }

    fn fix_all(&mut self) -> MayError {
        self.iter_mut()
            .map(|process| process.fix())
            .collect::<MayError>()?;

        Result::Ok(())
    }

    fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, ParseError> {
        let contents =
            fs::read_to_string(file_path).expect("Something went wrong reading the file");

        serde_json::from_str(&contents)
    }

    fn to_file<P: AsRef<Path>>(&self, file_path: P) -> MayError {
        let mut buffer = File::create(file_path)?;

        let serialized = serde_json::to_string_pretty(&self)?;

        buffer.write_all(serialized.as_bytes())?;
        buffer.write_all("\n".as_bytes())?;

        return std::result::Result::Ok(());
    }
}
