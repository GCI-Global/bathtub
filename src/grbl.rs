use regex::Regex;
use std::io::Write;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use std::{str, thread};

use chrono::prelude::*;
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};

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
        if let Ok(mut rb) = self.response_buffer.try_lock() {
            rb.pop()
        } else {
            None
        }
    }
    pub fn safe_pop(&self) -> Option<Command> {
        if let Ok(mut rb) = self.response_buffer.try_lock() {
            if rb.len() > 0 {
                rb.pop()
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn clear_all(&self) {
        let mut rb = self.response_buffer.lock().unwrap();
        let mut cb = self.command_buffer.lock().unwrap();
        rb.clear();
        cb.clear();
    }
    pub fn is_ok(&self) -> bool {
        self.ok_tx.send(()).is_ok()
    }
    pub fn get_status(&self) -> Option<Status> {
        if let Ok(status) = self.mutex_status.try_lock() {
            status.clone()
        } else {
            None
        }
    }
    pub fn clear_responses(&self) -> Vec<Command> {
        let mut rb = self.response_buffer.lock().unwrap();
        let rb_c = rb.clone();
        rb.clear();
        rb_c
    }
    pub fn queue_len(&self) -> Option<usize> {
        match self.command_buffer.lock() {
            Ok(cb) => return Some(cb.len()),
            Err(_) => None,
        }
    }
    pub fn recv_queue_len(&self) -> Option<usize> {
        if let Ok(rb) = self.response_buffer.try_lock() {
            Some(rb.len())
        } else {
            None
        }
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
            if now.elapsed().as_millis() >= 100
                && if let Ok(cb) = cb_c.try_lock() {
                    cb.len() == 0
                } else {
                    false
                }
            {
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
                    if let Ok(mut lctn) = status.try_lock() {
                        *lctn = Some(loc);
                    }
                }
            } else {
                if let Ok(mut cb) = cb_c.try_lock() {
                    if let Some(mut cmd) = cb.pop() {
                        send(&mut port, &mut cmd);
                        let mut rb = rb_c.lock().unwrap();
                        rb.push(cmd);
                    }
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
        if let Ok(port) = serialport::new(p.port_name, 115_200)
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
    port.write_all(&buf[..]).unwrap();
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
fn read_until(_c: u8, port: &mut Box<dyn SerialPort>) -> Result<String, std::io::Error> {
    //let mut reader = BufReader::new(port);
    let mut buf: Vec<u8> = vec![0;32];
    let mut result_buf: Vec<u8> = Vec::new();
    let mut cont = true;
    while cont {
        println!("{}", buf.len());
        match port.read(&mut buf) {
            Ok(_num) => {
                result_buf.extend_from_slice(&buf[..]);
                if result_buf[0] != 0x24 {
                    result_buf = result_buf.into_iter().take_while(|u| *u != 0xA).collect();
                } else {
                    let len1 = result_buf.len();
                    result_buf = result_buf.into_iter().take_while(|b| *b != 0x6F).collect();
                    let len2 = result_buf.len();
                    if len1 != len2 {result_buf.pop();}
                }
            }
            Err(_err) => {
                result_buf = result_buf.into_iter().filter(|b| *b != 0x0).collect::<Vec<u8>>();
                if **result_buf.last().as_ref().unwrap_or(&&0x1) == 0xD ||
                    **result_buf.last().as_ref().unwrap_or(&&0x1) == 0x47 ||
                    **result_buf.first().as_ref().unwrap_or(&&0x1) == 0x0 {
                        cont = false;
                    }
            },
        }
    }
    Ok(str::from_utf8(&result_buf[..]).unwrap_or("**Parsing reponse failed**").to_string())
}
} else {
fn read_until(c: u8, port: &mut Box<dyn SerialPort>) -> Result<String, std::io::Error> {
    use std::io::BufRead;
use std::io::BufReader;
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
