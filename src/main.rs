mod grbl;
use std::time::Duration;

/*
struct Cnc {
    sender,
    receiver,
}
*/

fn main() {
    //let (cnc_tx, cnc_rx) = grbl::create_connection();
    //let cnc = Cnc{sender: cnc_tx, receiver: cnc_rx};
    let (cnc_send, cnc_recv) = grbl::create_connection();
    cnc_send.send("Hello".to_string()).unwrap();
    cnc_send.send("$H".to_string()).unwrap();
    cnc_send.send("G90 Y-10".to_string()).unwrap();
    cnc_send.send("?".to_string()).unwrap();
    cnc_send.send("\n".to_string()).unwrap();
    loop {
        if let Ok((time,cmd,msg)) = cnc_recv.try_recv() {
            println!("{}: {} -> {}",time, cmd, msg);
        }
    }
}
