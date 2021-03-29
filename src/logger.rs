use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use std::{fs, thread};

use super::advanced::{Log, LOGS};

const WIN_CHARS: [&str; 9] = ["<", ">", ":", "\"", "/", "\\", "|", "?", "*"];

#[derive(Debug, Clone)]
pub struct Logger {
    sender: Sender<String>,
}

impl Logger {
    pub fn new() -> Logger {
        let (tx, rx): (Sender<String>, Receiver<String>) = channel();
        let mut file_name = String::new();
        thread::spawn(move || loop {
            if let Ok(mut line) = rx.recv() {
                line = replace_os_char(line);
                if line.ends_with("\n\rset_log_file") {
                    file_name = line.replace("\n\rset_log_file", "");
                } else {
                    match OpenOptions::new()
                        .append(true)
                        .open(Path::new(&format!("{}/{}", LOGS, file_name)))
                    {
                        Ok(mut log) => writeln!(log, "{}", line).unwrap(),
                        Err(_) => {
                            let mut log = OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(Path::new(&format!("{}/{}", LOGS, file_name)))
                                .unwrap();
                            writeln!(log, "{}", file_name).unwrap();
                            writeln!(
                                log,
                                "Created by Operating System User: {}",
                                match get_username() {
                                    Some(username) => username,
                                    None => "**Unavailable**".to_string(),
                                }
                            )
                            .unwrap();
                            writeln!(log, "--------------------").unwrap();
                        }
                    }
                }
            } else {
                break;
            }
        });
        Logger { sender: tx }
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
    pub async fn search_files<'a>(
        vals: Vec<String>,
        file_name: String,
    ) -> (Vec<String>, Option<Log>) {
        let test_string = fs::read_to_string(Path::new(&format!("{}/{}", LOGS, file_name)))
            .unwrap()
            .to_lowercase();
        if vals.iter().all(|val| test_string.contains(val)) {
            (vals, Some(Log::new(file_name)))
        } else {
            (vals, None)
        }
    }
}

// TODO: Test if this actially works on windows, currently linux compiler just ignores windows
// function
cfg_if::cfg_if! {
    if #[cfg(windows)] {
fn get_username() -> Option<String> {
    //Some(env!("USERNAME").to_string())
    Some("Currently unavailable on windows".to_string())
}
} else {
    fn get_username() -> Option<String> {
        use users::{get_current_uid, get_user_by_uid};
        match get_user_by_uid(get_current_uid())
        .unwrap()
        .name()
        .to_str() {
            Some(s) => Some(s.to_string()),
            None => None,
        }
    }
}
}

pub fn replace_os_char(mut s: String) -> String {
    if cfg!(windows) {
        for c in &WIN_CHARS {
            s = s.replace(c, "_");
        }
    } else if cfg!(linux) {
        s = s.replace("/", "_");
    } else if cfg!(mac) {
        s = s.replace(":", "_");
    }
    s
}
