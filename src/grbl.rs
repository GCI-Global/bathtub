extern crate serial;
use itertools::Itertools;
use regex::Regex;
use serial::prelude::*;
use serial::SystemPort;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{str, thread};

use chrono::prelude::*;
use std::io::BufRead;
use std::io::BufReader;

// used to clean up code when this file is imporded into another
#[derive(Debug, Clone)]
pub struct Grbl {
    pub command_buffer: Arc<Mutex<Vec<Command>>>,
    pub response_buffer: Arc<Mutex<Vec<Command>>>,
    pub mutex_status: Arc<Mutex<Option<Status>>>,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub response_time: Option<chrono::DateTime<chrono::Local>>,
    pub command: String,
    pub result: Option<String>,
}

impl Command {
    pub fn new(command: String) -> Command {
        Command {
            response_time: None,
            command: command.replace("\n", "").replace(" ", ""),
            result: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Status {
    pub status: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Grbl {
    pub fn push_command(&self, command: Command) {
        let mut cb = self.command_buffer.lock().unwrap();
        if command.command == "\u{85}".to_string() {
            *cb = Vec::new();
        }
        cb.insert(0, command)
    }
    pub fn pop_command(&self) -> Option<Command> {
        let mut rb = self.response_buffer.lock().unwrap();
        rb.pop()
    }
    /*
    pub fn clear_queue(&self) {
        let mut cb = self.command_buffer.lock().unwrap();
        *cb = Vec::new();
    }
    */
    pub fn get_status(&self) -> Option<Status> {
        self.mutex_status.lock().unwrap().clone()
    }
    pub fn clear_responses(&self) -> Vec<Command> {
        let mut rb = self.response_buffer.lock().unwrap();
        let rb_c = rb.clone();
        *rb = Vec::new();
        rb_c
    }
    pub fn queue_len(&self) -> usize {
        let cb = self.command_buffer.lock().unwrap();
        cb.len()
    }
}

// Create new thread that, locks usb serial connection + used to send+recv gcode
pub fn new() -> Grbl {
    let command_buffer: Arc<Mutex<Vec<Command>>> = Arc::new(Mutex::new(Vec::new()));
    let response_buffer = Arc::new(Mutex::new(Vec::new()));
    let cb_c = Arc::clone(&command_buffer);
    let rb_c = Arc::clone(&response_buffer);
    let status = Arc::new(Mutex::new(None));
    let mutex_status = Arc::clone(&status);
    thread::spawn(move || {
        let mut port = get_port();
        let mut now = Instant::now();
        let r =
            Regex::new(r"(?P<status>[A-Za-z]+).{6}(?P<X>[-\d.]+),(?P<Y>[-\d.]+),(?P<Z>[-\d.]+)")
                .unwrap();
        let mut current_status = Command::new("?".to_string());
        loop {
            if now.elapsed().as_millis() >= 100 {
                now = Instant::now();
                port.flush().unwrap();
                send(&mut port, &mut current_status);
                if let Some(caps) = r.captures(&current_status.result.as_ref().unwrap()[..]) {
                    let loc = Status {
                        status: caps["status"].to_string(),
                        x: caps["X"].parse::<f32>().unwrap(),
                        y: caps["Y"].parse::<f32>().unwrap(),
                        z: caps["Z"].parse::<f32>().unwrap(),
                    };
                    current_status.response_time = None;
                    current_status.result = None;
                    let mut lctn = status.lock().unwrap();
                    *lctn = Some(loc);
                }
            } else {
                let mut cb = cb_c.lock().unwrap();
                //while cb.len() > 0 {
                if let Some(mut cmd) = cb.pop() {
                    let mut rb = rb_c.lock().unwrap();
                    send(&mut port, &mut cmd);
                    rb.push(cmd);
                }
            }
        }
    });
    Grbl {
        command_buffer,
        response_buffer,
        mutex_status,
    }
}

// used by new() to get the usb serial connection
fn get_port() -> SystemPort {
    let mut try_port = serial::open("/dev/ttyUSB0");
    if try_port.is_err() {
        let mut i = 0;
        while try_port.is_err() && i < 1000 {
            try_port = serial::open(&format!("/dev/ttyUSB{}", i));
            i += 1;
        }
        if i == 1000 {
            panic!("unable to find USB port");
        }
    }
    let mut port = try_port.expect("port error");
    // default port settings for grbl, maybe should be configurable?
    port.reconfigure(&|settings| {
        settings.set_baud_rate(serial::Baud115200).unwrap();
        settings.set_char_size(serial::Bits8);
        settings.set_parity(serial::ParityNone);
        settings.set_stop_bits(serial::Stop1);
        settings.set_flow_control(serial::FlowNone);

        Ok(())
    })
    .unwrap();
    port.set_timeout(Duration::from_secs(60)).unwrap();
    port
}

// used by the new() thread to send to grbl and parse response
pub fn send(port: &mut SystemPort, command: &mut Command) {
    port.flush().unwrap();
    let mut buf = format!("{}\n", command.command).as_bytes().to_owned();
    port.write(&buf[..]).unwrap();
    let mut reader = BufReader::new(port);
    let mut line: String;
    let mut output: Vec<String> = Vec::new();
    buf = Vec::new();
    if command.command == "$$".to_string() {
        loop {
            reader.read_until(0xD, &mut buf).unwrap();
            line = str::from_utf8(&buf).unwrap().to_string();
            if line.contains("$132=") {
                output.push(line);
                command.response_time = Some(Local::now());
                // update result, requires filtering because I can't figure out how to read the
                // serial output correctly
                command.result = Some(
                    output
                        .into_iter()
                        .fold(String::new(), |s, part| format!("{}{}{}", s, part, "\n\r"))
                        .split("\r")
                        .unique()
                        .collect::<String>(),
                );
                break;
            }
            output.push(line);
        }
    } else {
        loop {
            // read until caridge return kek from grbl
            match reader.read_until(0xD, &mut buf) {
                Ok(_reader) => {
                    line = str::from_utf8(&buf).unwrap().to_string();
                    // the first reponse from grbl initializing the connection is a bit weird, it has multiple
                    // caridge returns, lockily it is the only one with a unicode 'null' char. GRBL doesnt do
                    // anything with this first command, so we can mostly ignore it.
                    if line.contains("\u{0}\r") {
                        command.response_time = Some(Local::now());
                        command.result = Some("init".to_string());
                        break;
                    }
                    if line.contains("ok") {println!("{}", line);line = "ok\r".to_string()}
                    if command.command != "?".to_string() && line.contains("<") {
                        line = "".to_string();
                    }
                    if line.contains("\r") {
                        output.push(line.replace("\r", "").replace("\n", ""));
                        command.response_time = Some(Local::now());
                        command.result =
                            Some(output.into_iter().fold(String::new(), |mut string, part| {
                                string.push_str(&part[..]);
                                string
                            }));
                        break;
                    }
                    if command.command == "?".to_string() && line.contains("<") {
                        output.push(line);
                    } else if command.command != "?".to_string() && !line.contains("<") {
                        output.push(line);
                    }
                }
                Err(err) => println!("{:?}", err),
            }
        }
    }
}
