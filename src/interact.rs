extern crate serial;
use std::{thread, io, str};
use std::time::Duration;
use serial::prelude::*;

use super::paths;
use super::nodes::Node;

struct Status {
    busy: bool,
    current_node: Option<Node>,
}

impl Status {
    fn new(nodes: Nodes) -> Status {
        Status {
            busy: false,
            current_node: None,
        }
    }
}

fn setup() -> &Status {
    Status::new()
}

fn send(node: &Node) -> &Status {
    
}


// fn get_status() -> Status
