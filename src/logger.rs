use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use std::sync::{Arc, Mutex};
use std::{fs, thread};

use super::advanced::{Log, LOGS};
use users::{get_current_uid, get_user_by_uid};

#[derive(Debug, Clone)]
pub struct Logger {
    sender: Sender<String>,
}

impl Logger {
    pub fn new() -> Logger {
        let (tx, rx): (Sender<String>, Receiver<String>) = channel();
        let mut file_name = String::new();
        thread::spawn(move || loop {
            if let Ok(line) = rx.recv() {
                if line.ends_with("\n\rset_log_file") {
                    file_name = line.replace("\n\rset_log_file", "");
                } else {
                    match OpenOptions::new()
                        .append(true)
                        .open(Path::new(&format!("./logs/{}", file_name)))
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
                                "Current Operating System User: {}",
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
fn get_username() -> windows::Result<String> {
mod bindings {
    ::windows::include_bindings!();
}
use bindings::windows::{
    foundation::IReference,
    system::{KnownUserProperties, User, UserType},
};
use windows::{HString, Interface};
    windows::initialize_sta()?;

    let users = User::find_all_async_by_type(UserType::LocalUser)?.get()?;
    assert!(users.size().unwrap() >= 1);
    let user = users.get_at(0)?;

    let user_name: IReference<HString> = user
        .get_property_async(KnownUserProperties::account_name()?)?
        .get()?
        .cast()?;
    let user_name = user_name.get_string()?;
    Ok(user_name)
}
} else {
    fn get_username() -> Option<String> {
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
