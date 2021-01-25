extern crate serial;
use std::{thread, io, str};
use std::time::Duration;
use serial::prelude::*;

fn main()  {
    let mut port = serial::open("/dev/ttyusb0").expect("unable to find tty or tty in use by other application");
    interact(&mut port).unwrap()
}

fn interact<T: SerialPort>(port: &mut T) -> io::Result<()> {
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
    

    /*
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
    */
    println!("{}", output);
    Ok(())
}
