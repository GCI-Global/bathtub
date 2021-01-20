extern crate serial;

use std::thread;
use std::io;
use std::time::Duration;

use serial::prelude::*;
use std::str;

mod paths;
mod nodes;

fn main() {
    let start = 15;
    let finish = 12;

    let nodes = nodes::gen_nodes();
    println!("From {} to {}", &nodes.node[start].name, &nodes.node[finish].name);
    let node_paths = paths::gen_node_paths(&nodes, &nodes.node[start], &nodes.node[finish]);
    for node in &node_paths.node {
        println!("{}", node.name);
    };
    let gcode_path = paths::gen_gcode_paths(&node_paths);

    // This is for interacting with Tubby, will get back to later
    let mut port = serial::open("/dev/ttyUSB0").expect("unable to find tty");
    interact(&mut port, &gcode_path).unwrap();
}

fn interact<T: SerialPort>(port: &mut T, gcode_path: &Vec<String>) -> io::Result<()> {
    port.reconfigure(&|settings| {
        settings.set_baud_rate(serial::Baud115200).unwrap();
        settings.set_char_size(serial::Bits8);
        settings.set_parity(serial::ParityNone);
        settings.set_stop_bits(serial::Stop1);
        settings.set_flow_control(serial::FlowNone);

        Ok(())
    }).unwrap();
    port.set_timeout(Duration::from_secs(60)).unwrap();
    
    // Initialize GRBL
    let mut buf: Vec<u8> = "\r\n\r\n".as_bytes().to_owned(); //wake GRBL then wait for server to start
    port.write(&buf[..]).unwrap();
    thread::sleep(Duration::from_secs(2));
    port.flush().unwrap();
    buf = "$H\n".as_bytes().to_owned(); //Unlock head
    println!("{:?}", &buf[..]);
    port.write(&buf[..]).unwrap();
    port.read(&mut buf[..]).unwrap(); //Should be able to parse this in the future for sucess/fail messages

    //send to above rinse 1
    buf = "G90 X0 Y-13.5 Z0\n".as_bytes().to_owned();
    port.write(&buf[..]).unwrap();
    thread::sleep(Duration::from_secs(2));
    let mut output = String::from("");
    for gcode in gcode_path {
        println!("{}",gcode);
        buf = gcode.as_bytes().to_owned();
        port.write(&buf[..]).unwrap();
        while !output.contains("ok") {
            port.read(&mut buf[..]).unwrap();
            output = format!("{}{}", output, str::from_utf8(&buf[..]).unwrap());
            //println!("{}", output);
        }
        output.clear();
        port.flush().unwrap();
    }
    /* read the output of grbl startup
    for _i in 0..5 {
        port.read(&mut buf[..]).unwrap();
        output = format!("{}{}", output, str::from_utf8(&buf[..]).unwrap());
        println!("{}", output);
        port.flush().unwrap();
        //println!("{:?}", str::from_utf8(&buf[..]));
    }
    */
    println!("{}", output);
    /* How to read the current status
    buf = "?\n".as_bytes().to_owned();
    port.write(&buf[..]).unwrap();
    output = "".to_string();
    loop {
        port.read(&mut buf[..]).unwrap();
        output = format!("{}{}", output, str::from_utf8(&buf[..]).unwrap());
        println!("{}",output);
        port.flush().unwrap();
    }
    */
    
    //port.flush().unwrap();
    //for path in gcode_path {
    //    port.flush().unwrap();
    //    buf = path[..].as_bytes().to_owned();
    //    port.write(&buf[..]).unwrap();
    //}
    //buf = gcode_path.as_bytes().to_owned();
    //port.write(&buf[..]).unwrap();
    //println!("{:?}", str::from_utf8(&buf[..]));
    Ok(())
}

// Ideas
// Things that should be in the config file
// 1. Add the path to the serial port (i think linux is /dev/ttyUSB0) not sure about windows yet
// 2. All usb settings should come from the config file
