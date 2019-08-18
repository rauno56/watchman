use std::error;
use std::process::{Command, Stdio};
use sysinfo::{Process as SysProc, ProcessExt, RefreshKind, System, SystemExt};

#[derive(Debug)]
pub struct Process {
    pub name: String,
    pub cmd: String,
    pub pid: i32,
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
    get_ext_by_pid(pid).map(|proc| Process {
        name: proc.name().to_string(),
        cmd: join(proc.cmd().to_vec()),
        pid: proc.pid(),
    })
}

pub fn kill_by_pid(pid: i32) -> bool {
    get_ext_by_pid(pid).map_or(false, |proc| proc.kill(sysinfo::Signal::Kill))
}

pub fn run_from_string(input: &String) -> std::result::Result<i32, Box<error::Error>> {
    let mut parts = input.trim().split_whitespace();
    let command = parts.next().unwrap();
    let args = parts;

    let child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    std::result::Result::Ok(child.id() as i32)
}
