use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use std::sync::{Arc, Mutex};
use std::{fs, thread};

use super::advanced::{Log, LOGS};

#[derive(Debug, Clone)]
pub struct Logger {
    sender: Sender<String>,
    current_log: Arc<Mutex<String>>,
}

impl Logger {
    pub fn new() -> Logger {
        let (tx, rx): (Sender<String>, Receiver<String>) = channel();
        let current_log = Arc::new(Mutex::new(String::new()));
        let current_log2 = Arc::clone(&current_log);
        thread::spawn(move || loop {
            if let Ok(line) = rx.recv() {
                let mut file_name = current_log2.lock().unwrap();
                if line.ends_with("\n\rset_log_file") {
                    *file_name = line.replace("\n\rset_log_file", "");
                } else {
                    let mut log = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(Path::new(&format!("./logs/{}", *file_name)))
                        .unwrap();
                    writeln!(log, "{}", line).unwrap();
                }
            } else {
                break;
            }
        });
        Logger {
            sender: tx,
            current_log,
        }
    }

    pub fn set_log_file(&mut self, mut file_name: String) {
        file_name.push_str("\n\rset_log_file");
        self.sender.send(file_name).unwrap();
        /*
        let mut current_log = self.current_log.lock().unwrap();
        *current_log = file_name;
        */
    }

    pub fn send_line(&self, line: String) -> Result<(), SendError<String>> {
        self.sender.send(line)
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
