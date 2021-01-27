extern crate serial;
use std::{thread, str};
use std::time::Duration;
use serial::prelude::*;
use serial::SystemPort;
use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub struct Status {
    pub status: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug)]
pub enum Errors {
    HomeRequired,
}

pub fn get_port() -> SystemPort {
        let mut try_port = serial::open("/dev/ttyUSB0");
        if try_port.is_err() {
            let mut i = 1;
            while try_port.is_err() && i < 10000{
                try_port = serial::open(&format!("/dev/ttyUSB{}",i));
                i+=1;
            }
            if i == 10000 {
                panic!("unable to find USB port");
            }
        }
        let mut port = try_port.expect("port error");
        
        port.reconfigure(&|settings| {
            settings.set_baud_rate(serial::Baud115200).unwrap();
            settings.set_char_size(serial::Bits8);
            settings.set_parity(serial::ParityNone);
            settings.set_stop_bits(serial::Stop1);
            settings.set_flow_control(serial::FlowNone);

            Ok(())
        }).unwrap();
        port.set_timeout(Duration::from_secs(60)).unwrap();
        port
}


pub fn status<T: SerialPort>(port: &mut T) -> Status {
    port.flush().unwrap();
    let mut buf: Vec<u8> = "?\n".as_bytes().to_owned();
    let mut output = String::new();
    port.write(&buf[..]).unwrap();
    thread::sleep(Duration::from_millis(500));
    while !output.contains(">\r\n"){
        port.read(&mut buf[..]).unwrap();
        output.push_str(str::from_utf8(&buf[..]).unwrap());
    }
    // Parse input string
    let split_output: Vec<&str> = output.split("|").collect();
    let status: Vec<&str> = split_output[0].split("\n").collect();
    let split_coords: Vec<&str> = split_output[1].split(",").collect();
    Status {
        //status: split_output[0].replace("<","").to_string(),
        status: status.last().unwrap().replace("<","").to_string(),
        x: split_coords[0][5..].parse().unwrap(),
        y: split_coords[1].parse().unwrap(),
        z: split_coords[2].parse().unwrap(),
    }
}

// ***** need to update to actually print if error *****
pub fn send(port: &mut SystemPort, gcode: String) -> Result<(), Errors> {
        // test for if port lost
        if port.read_cd().is_err() {
            *port = get_port();
        }

        let mut buf: Vec<u8> = "\r\n".as_bytes().to_owned(); //wake GRBL
        port.write(&buf[..]).unwrap();

        // Initialise GRBL if not already
        if port.read(&mut buf[..]).unwrap() == 1 { // 1 means not conncted for GRBL
            return Err(Errors::HomeRequired);
        }
        // Now send Gcode command
        buf = format!("{}\n", gcode).as_bytes().to_owned();
        port.write(&buf[..]).unwrap();
        let mut output = String::from("");
        while !output.contains("ok") {
            port.read(&mut buf[..]).unwrap();
            output.push_str(str::from_utf8(&buf[..]).unwrap());
        }
    Ok(())
}
pub fn home(port: &mut SystemPort) -> Result<(), String> {
    if port.read_cd().is_err() {
        *port = get_port();
    }
    let mut buf: Vec<u8> = "\r\n".as_bytes().to_owned(); //wake GRBL
    port.write(&buf[..]).unwrap();

    // Initialise GRBL if not already
    port.read(&mut buf[..]).unwrap();
    buf = "$H\n".as_bytes().to_owned();
    port.write(&buf[..]).unwrap();
    Ok(())
}
