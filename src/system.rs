use std::error;
use std::fmt;
use std::process::{Command, Stdio};
use sysinfo::{Process as SysProc, ProcessExt, RefreshKind, System, SystemExt};

#[derive(Debug)]
pub struct Process {
    pub cmd: String,
    pub pid: i32,
}

fn sysproc_to_process(proc: SysProc) -> Process {
    Process {
        cmd: join(proc.cmd().to_vec()),
        pid: proc.pid(),
    }
}

fn get_ext_by_pid(pid: i32) -> Option<SysProc> {
    let mut sys = System::new_with_specifics(RefreshKind::new());
    sys.refresh_process(pid);
    sys.get_process(pid).map(|proc| proc.clone())
}

fn join(parts: Vec<String>) -> String {
    let mut res = String::new();
    for part in parts {
        if res.len() > 0 {
            res.push_str(&" ");
        }
        res.push_str(&part)
    }
    return res;
}

pub fn get_by_pid(pid: i32) -> Option<Process> {
    get_ext_by_pid(pid)
        /*
            Zombie processes come up in our case when a process is started during the
            same program execution when killed For simplicity we are just acting like
            those weren't there and those resources will be cleaned once our program
            exits.
        */
        .filter(|proc| proc.status().to_string() != "Zombie")
        .map(|proc| Process {
            cmd: join(proc.cmd().to_vec()),
            pid: proc.pid(),
        })
}

pub fn kill_by_pid(pid: i32) -> bool {
    get_ext_by_pid(pid).map_or(false, |proc| proc.kill(sysinfo::Signal::Kill))
}

fn get_ext_by_cmd(cmd: &String) -> Option<SysProc> {
    let mut system = sysinfo::System::new();

    // First we update all information of our system struct.
    system.refresh_all();

    for (_pid, proc) in system.get_process_list() {
        if *cmd == join(proc.cmd().to_vec()) {
            return Some(proc.clone());
        }
    }

    None
}

pub fn get_by_cmd(cmd: &String) -> Option<Process> {
    get_ext_by_cmd(cmd)
        .map(|proc| Process {
            cmd: join(proc.cmd().to_vec()),
            pid: proc.pid(),
        })
}

#[derive(Debug)]
struct SysError {
    msg: String,
}

impl SysError {
    fn new(msg: &str) -> Self {
        SysError {
            msg: msg.to_string(),
        }
    }

    fn new_with_invalid_command(cmd: &str) -> Self {
        Self::new(&format!("Invalid command: {}", cmd))
    }
}

impl fmt::Display for SysError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl error::Error for SysError {}

pub fn run_from_string(input: &String) -> std::result::Result<i32, Box<dyn error::Error>> {
    let mut parts = input.trim().split_whitespace();
    let command = parts
        .next()
        .ok_or_else(|| SysError::new_with_invalid_command(input))?;
    let args = parts;

    let child = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    std::result::Result::Ok(child.id() as i32)
}
