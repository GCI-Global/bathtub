use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use std::sync::mpsc::{channel, SendError, Sender};
use std::{fs, thread};

use super::advanced::{Log, LOGS};

#[derive(Debug, Clone)]
pub struct Logger {
    sender: Sender<(String, String)>,
}

impl Logger {
    pub fn new() -> Logger {
        let (tx, rx) = channel();
        thread::spawn(move || loop {
            if let Ok((file_name, line)) = rx.recv() {
                let mut log = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(Path::new(&format!("./logs/{}", file_name)))
                    .unwrap();
                writeln!(log, "{}", line).unwrap();
            } else {
                break;
            }
        });
        Logger { sender: tx }
    }
    pub fn send_line(
        &self,
        file_name: String,
        line: String,
    ) -> Result<(), SendError<(String, String)>> {
        self.sender.send((file_name, line))
    }
    pub async fn search_files<'a>(val: String, file_name: String) -> (String, Option<Log>) {
        let test_string = fs::read_to_string(Path::new(&format!("{}/{}", LOGS, file_name)))
            .unwrap()
            .to_lowercase();
        if test_string.contains(&val) {
            (val, Some(Log::new(file_name)))
        } else {
            (val, None)
        }
    }
}
