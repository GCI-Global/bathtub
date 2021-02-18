extern crate serial;
use regex::Regex;
use serial::prelude::*;
use serial::SystemPort;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
            command,
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
        cb.insert(0, command)
    }
    pub fn pop_command(&self) -> Option<Command> {
        let mut rb = self.response_buffer.lock().unwrap();
        rb.pop()
    }
    pub fn get_status(&self) -> Option<Status> {
        self.mutex_status.lock().unwrap().clone()
    }
    pub fn clear_responses(&self) -> Vec<Command> {
        let mut rb = self.response_buffer.lock().unwrap();
        let rb_c = rb.clone();
        *rb = Vec::new();
        rb_c
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
        let r =
            Regex::new(r"(?P<status>[A-Za-z]+).{6}(?P<X>[-\d.]+),(?P<Y>[-\d.]+),(?P<Z>[-\d.]+)")
                .unwrap();
        loop {
            let mut current_status = Command::new("?".to_string());
            send(&mut port, &mut current_status);
            if let Some(caps) = r.captures(&current_status.result.unwrap()[..]) {
                let loc = Status {
                    status: caps["status"].to_string(),
                    x: caps["X"].parse::<f32>().unwrap(),
                    y: caps["Y"].parse::<f32>().unwrap(),
                    z: caps["Z"].parse::<f32>().unwrap(),
                };
                let mut lctn = status.lock().unwrap();
                *lctn = Some(loc);
            }
            let mut cb = cb_c.lock().unwrap();
            let mut rb = rb_c.lock().unwrap();
            while cb.len() > 0 {
                let mut cmd = cb.pop().unwrap();
                println!("{}", cmd.command.replace("\n", ""));
                send(&mut port, &mut cmd);
                rb.push(cmd)
            }
            thread::sleep(Duration::from_millis(40));
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
    let mut buf = format!("{}\n", command.command).as_bytes().to_owned();
    port.write(&buf[..]).unwrap();
    let mut reader = BufReader::new(port);
    let mut line: String;
    let mut output: Vec<String> = Vec::new();
    buf = Vec::new();
    loop {
        // read until caridge return kek from grbl
        reader.read_until(0xD, &mut buf).unwrap();
        line = str::from_utf8(&buf).unwrap().to_string();
        // the first reponse from grbl initializing the connection is a bit weird, it has multiple
        // caridge returns, lockily it is the only one with a unicode 'null' char. GRBL doesnt do
        // anything with this first command, so we can mostly ignore it.
        if line.contains("\u{0}\r") {
            command.response_time = Some(Local::now());
            command.result = Some("init".to_string());
            break;
        }
        if line.contains("\r") {
            output.push(line.replace("\r", ""));
            command.response_time = Some(Local::now());
            command.result = Some(output.into_iter().fold(String::new(), |mut string, part| {
                string.push_str(&part[..]);
                string
            }));
            break;
        }
        output.push(line);
    }
}
