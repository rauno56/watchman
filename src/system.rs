use chrono::Local;
use std::io::{BufRead, BufReader, Read};
use std::marker::Send;
use std::thread::JoinHandle;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{channel, Receiver};
use std::cmp;
use std::error;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use std::time::Duration;
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

pub fn join(parts: Vec<String>) -> String {
    let mut res = String::new();
    for part in parts {
        if !res.is_empty() {
            res.push_str(&" ");
        }
        res.push_str(&part)
    }
    res
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
        .map(sysproc_to_process)
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
    get_ext_by_cmd(cmd).map(sysproc_to_process)
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

fn open_log_file(path: &PathBuf) -> File {
    let mut new_file_options = OpenOptions::new();
    let file_options = new_file_options.read(true).append(true).create(true);

    file_options.open(path).unwrap()
}

pub fn run_from_string(
    input: &String,
    output_to: Option<&PathBuf>,
) -> std::result::Result<i32, Box<dyn error::Error>> {
    let mut parts = input.trim().split_whitespace();
    let command = parts
        .next()
        .ok_or_else(|| SysError::new_with_invalid_command(input))?;
    let args = parts;

    let (out, err) = match output_to {
        Some(path) => {
            let file_out = open_log_file(path);
            let file_err = file_out.try_clone()?;
            (Stdio::from(file_out), Stdio::from(file_err))
        }
        None => (Stdio::piped(), Stdio::piped()),
    };

    let child = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .spawn()?;

    std::result::Result::Ok(child.id() as i32)
}

use std::thread;

use std::io::{self, Write};

pub fn keep_running_from_string(
    input: &String,
    _prefix: &String,
    output_to: Option<&PathBuf>,
) -> std::result::Result<i32, Box<dyn error::Error>> {
    let mut parts = input.trim().split_whitespace();
    let command = parts
        .next()
        .ok_or_else(|| SysError::new_with_invalid_command(input))?;
    let args = parts;

    let mut wait_time = 1;
    let mut file_out = output_to.map(|path| open_log_file(path));
    let mut log = move |line: LogLine| {
        let line = format!("{} {}", Local::now().format("%Y-%m-%d %H:%M:%S"), line);
        println!("{}", line);
        if let Some(file_out) = file_out.as_mut() {
            writeln!(file_out, "{}", line).expect("Failed to write to log file");
        }
    };

    loop {
        log(LogLine::Sys(format!("Restarting {:?}", input)));
        log(LogLine::Sys(format!("res output {:?}", output_to)));

        let run = run_command_with_output_handler(
            Command::new(command)
                .args(args.clone())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        );

        match run {
            Ok((mut child, rx)) => {
                println!("child running {:?}", child.id());
                loop {
                    match rx.recv() {
                        Ok(output) => {
                            log(output);
                        }
                        Err(err) => {
                            if let Ok(_exit_status) = child.try_wait() {
                                break;
                            }
                            log(LogLine::Sys(format!(
                                "Error receiving a log line: {:?}",
                                err
                            )));
                        }
                    }
                }
                match child.wait() {
                    Ok(exit_status) => match exit_status.code() {
                        Some(status_code) => {
                            if exit_status.success() {
                                wait_time = 1;
                            } else {
                                wait_time = cmp::min(wait_time * 2, 60);
                            }
                            log(LogLine::Sys(format!(
                                "process exited with status {:?}",
                                status_code
                            )));
                        }
                        None => {
                            wait_time = 1;
                            log(LogLine::Sys(format!("process exited with no status")));
                        }
                    },
                    Err(e) => {
                        wait_time *= 2;
                        log(LogLine::Sys(format!(
                            "attempt to run the command errored {:?}",
                            e
                        )));
                    }
                }
            }
            Err(e) => {
                log(LogLine::Sys(format!("process errored {:?}", e)));
            }
        }

        log(LogLine::Sys(format!("sleeping {:?}s", wait_time)));
        thread::sleep(Duration::new(wait_time, 0));
    }
}

#[derive(Debug)]
pub enum LogLine {
    StdOut(String),
    StdErr(String),
    Sys(String),
}

impl fmt::Display for LogLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLine::StdOut(line) => write!(f, "OUT {}", line),
            LogLine::StdErr(line) => write!(f, "ERR {}", line),
            LogLine::Sys(line) => write!(f, "SYS {}", line),
        }
    }
}

fn process_readable_lines<T: 'static, F: 'static>(readable: T, mut handler: F) -> JoinHandle<()>
where
    T: Read + Send,
    F: FnMut(String) -> () + Send,
{
    thread::spawn(move || {
        let f = BufReader::new(readable);
        for line in f.lines() {
            if let Ok(line) = line {
                handler(line);
            }
        }
    })
}

pub fn run_command_with_output_handler(
    command: &mut Command,
) -> io::Result<(std::process::Child, Receiver<LogLine>)> {
    match command.spawn() {
        Ok(mut child) => {
            let stdout = child
                .stdout
                .take()
                .ok_or(Error::new(ErrorKind::Other, "Could not take stdout"))?;
            let stderr = child
                .stderr
                .take()
                .ok_or(Error::new(ErrorKind::Other, "Could not take stderr"))?;
            let (tx_out, rx) = channel();
            let tx_err = tx_out.clone();

            let _out_handle = process_readable_lines(stdout, move |line| {
                tx_out.send(LogLine::StdOut(line)).expect("Failed to send a stdout log line");
            });
            let _err_handle = process_readable_lines(stderr, move |line| {
                tx_err.send(LogLine::StdErr(line)).expect("Failed to send a stderr log line");
            });

            Ok((child, rx))

            // out_handle.join().unwrap();
            // err_handle.join().unwrap();
        }
        Err(err) => {
            println!("command didn't start: {:?}", err);
            Err(err)
        }
    }
}
