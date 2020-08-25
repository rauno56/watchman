#![allow(dead_code, unused_imports, unused_variables)]

use std::cmp;
use std::error;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
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
    prefix: &String,
    output_to: Option<&PathBuf>,
) -> std::result::Result<i32, Box<dyn error::Error>> {
    let mut parts = input.trim().split_whitespace();
    let command = parts
        .next()
        .ok_or_else(|| SysError::new_with_invalid_command(input))?;
    let args = parts;

    let mut wait_time = 1;

    loop {
        println!("Restarting {:?}", input);
        println!("Output {:?}", output_to);
        let (file_out, file_err) = match output_to {
            Some(path) => {
                let file_out = open_log_file(path);
                let file_err = file_out
                    .try_clone()
                    .ok()
                    .map(|file| Arc::new(Mutex::new(file)));
                (Some(Arc::new(Mutex::new(file_out))), file_err)
            }
            None => (None, None),
        };

        // .map(|path| {
        // });

        println!("res output {:?}", output_to);

        let output_handler = |source, line| {
            println!("[{:?}] {}", source, line);
            // match source {
            //     OutputSource::Out => {
            //         println!("{}", line);
            //     }
            //     OutputSource::Err => {
            //         eprintln!("{}", line);
            //     }
            // }

            if let Some(file_out) = file_out.clone() {
                writeln!(
                    file_out.lock().unwrap(),
                    "{} [{:?}] {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    source,
                    line
                )
                .unwrap();
            }
            if let Some(file_err) = file_err.clone() {
                writeln!(
                    file_err.lock().unwrap(),
                    "{} [{:?}] {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    source,
                    line
                )
                .unwrap();
            }
        };
        let run = run_command_with_output_handler(
            Command::new(command)
                .args(args.clone())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
            output_handler,
        );

        match run {
            // Ok(mut child) => {
            //     println!("child running {:?}", child.id());
            //     match child.wait() {
            Ok(exit_status) => match exit_status.code() {
                Some(status_code) => {
                    if exit_status.success() {
                        println!("success!");
                        wait_time = 1;
                    } else {
                        println!("failure!");
                        wait_time = cmp::min(wait_time * 2, 60);
                    }
                    println!("process exited with status {:?}", status_code);
                }
                None => {
                    wait_time = 1;
                    println!("process exited with no status");
                }
            },
            Err(e) => {
                println!("process errored {:?}", e);
            }
        }
        // }
        // Err(e) => {
        //     wait_time *= 2;
        //     println!("attempt to run the command errored {:?}", e);
        // }
        // }

        println!("sleeping {:?}s", wait_time);
        thread::sleep(Duration::new(wait_time, 0));
    }
}

#[derive(Debug)]
pub enum CommandResult {
    StdOut(String),
    StdErr(String),
    ExitStatus,
}

#[derive(Debug)]
pub enum OutputSource {
    Out,
    Err,
}

use chrono::Local;

use std::io::{BufRead, BufReader, Read};
use std::marker::Send;
use std::thread::JoinHandle;

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

use std::io::{Error, ErrorKind};

pub fn run_command_with_output_handler<Hn: 'static>(
    command: &mut Command,
    mut handler: Hn,
) -> io::Result<std::process::ExitStatus>
where
    Hn: FnMut(OutputSource, String) -> () + Send + Copy,
{
    match command.spawn() {
        Ok(mut child) => {
            println!("Child started: {:?}", child.stdout);
            let stdout = child
                .stdout
                .take()
                .ok_or(Error::new(ErrorKind::Other, "Could not take stdout"))?;
            let stderr = child
                .stderr
                .take()
                .ok_or(Error::new(ErrorKind::Other, "Could not take stderr"))?;

            let out_handle = process_readable_lines(stdout, move |line| {
                handler(OutputSource::Out, line);
            });
            let err_handle = process_readable_lines(stderr, move |line| {
                handler(OutputSource::Err, line);
            });
            println!("thread spawned");

            out_handle.join().unwrap();
            // err_handle.join().unwrap();

            child.wait()
        }
        Err(err) => {
            println!("command didn't start: {:?}", err);
            Err(err)
        }
    }
}
