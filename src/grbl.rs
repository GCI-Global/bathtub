use regex::Regex;
use std::io::Write;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use std::{str, thread};

use chrono::prelude::*;
use futures::future::result;
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::io::BufRead;
use std::io::BufReader;

// used to clean up code when this file is imporded into another
#[derive(Debug, Clone)]
pub struct Grbl {
    pub command_buffer: Arc<Mutex<Vec<Command>>>,
    pub response_buffer: Arc<Mutex<Vec<Command>>>,
    pub mutex_status: Arc<Mutex<Option<Status>>>,
    ok_tx: mpsc::Sender<()>,
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
    pub fn safe_pop(&self) -> Option<Command> {
        let mut rb = self.response_buffer.lock().unwrap();
        if rb.len() > 0 {
            rb.pop()
        } else {
            None
        }
    }
    pub fn clear_all(&mut self) {
        let mut rb = self.response_buffer.lock().unwrap();
        let mut cb = self.command_buffer.lock().unwrap();
        *rb = Vec::new();
        *cb = Vec::new();
    }
    pub fn is_ok(&self) -> bool {
        self.ok_tx.send(()).is_ok()
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
    let (ok_tx, ok_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut port = get_port();
        let mut now = Instant::now();
        let r =
            Regex::new(r"(?P<status>[A-Za-z]+).{6}(?P<X>[-\d.]+),(?P<Y>[-\d.]+),(?P<Z>[-\d.]+)")
                .unwrap();
        let mut current_status = Command::new("?".to_string());
        loop {
            // does nothing in this thread used to test if died in other threads
            match ok_rx.try_recv() {
                _ => {}
            };
            if now.elapsed().as_millis() >= 100 && cb_c.lock().unwrap().len() == 0 {
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
                    port.flush().unwrap();
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
        ok_tx,
    }
}

// used by new() to get the usb serial connection
fn get_port() -> Box<dyn SerialPort> {
    let ports = serialport::available_ports().expect("no ports available");
    for p in ports {
        if let Ok(mut port) = serialport::new(p.port_name, 115_200)
            .parity(Parity::None)
            .data_bits(DataBits::Eight)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(if cfg!(windows) {
                Duration::from_millis(50)
            } else {
                Duration::from_secs(60)
            })
            .open()
        {
            return port;
        }
    }
    panic!("unable to get port!");
}

// used by the new() thread to send to grbl and parse response
pub fn send(port: &mut Box<dyn SerialPort>, command: &mut Command) {
    let buf = format!("{}\n", command.command).as_bytes().to_owned();
    port.write(&buf[..]).unwrap();
    loop {
        match read_until(0xA, port) {
            Ok(line) => {
                command.response_time = Some(Local::now());
                match &command.command[..] {
                    "$$" => command.result = Some(line),
                    "$N" => command.result = Some(line),
                    _ => command.result = Some(line.replace("\n", "").replace("\r", "")),
                }
                break;
            }
            Err(err) => panic!("{}", err),
        }
    }
}
cfg_if::cfg_if! {
if #[cfg(windows)] {
fn read_until(c: u8, port: &mut Box<dyn SerialPort>) -> Result<String, std::io::Error> {
    //let mut reader = BufReader::new(port);
    let mut buf: Vec<u8> = vec![0;32];
    let mut result_buf: Vec<u8> = Vec::new();
    let mut cont = true;
    while cont {
        match port.read(&mut buf) {
            Ok(num) => {
                result_buf.extend_from_slice(&buf[..]);
                if result_buf[0] != 0x24 {
                    result_buf = result_buf.into_iter().take_while(|u| *u != 0xA).collect();
                } else {
                    let len1 = result_buf.len();
                    result_buf = result_buf.into_iter().take_while(|u| *u != 0x6F).collect();
                    let len2 = result_buf.len();
                    if len1 != len2 {result_buf.pop();}
                }
            }
            Err(err) => {
                if **result_buf.last().as_ref().unwrap_or(&&0x1) == 0xD ||
                    **result_buf.last().as_ref().unwrap_or(&&0x1) == 0x47 ||
                    **result_buf.first().as_ref().unwrap_or(&&0x1) == 0x0 {cont = false}
            },
        }
    }
    Ok(str::from_utf8(&result_buf[..]).unwrap().to_string())
}
} else {
fn read_until(c: u8, port: &mut Box<dyn SerialPort>) -> Result<String, std::io::Error> {
    let mut reader = BufReader::new(port);
    let mut buf: Vec<u8> = Vec::new();
    let mut len: usize;
    loop {
        match reader.fill_buf() {
            Ok(data) => {
                len = data.len();
                if data.len() > 0 {
                    buf.extend_from_slice(&data[..]);
                    if buf.last().unwrap_or(&0) == &c {
                        return Ok(str::from_utf8(&buf[..]).unwrap().to_string());
                    }
                } else {
                    return Ok(str::from_utf8(&buf[..]).unwrap().to_string());
                }
            }
            Err(err) => {
                return Err(err);
            }
        }
        reader.consume(len);
    }
}
}
}
