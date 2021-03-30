use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use toml;

#[derive(Clone, Debug)]
pub struct Actions {
    gcode: String,
    seconds: f32,
}

// Baths are Deserialized from config file, nodes are a generated 3d graph on nodes from the 2d
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Node {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub hide: bool,
    pub neighbors: Vec<String>,
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Nodes {
    pub node: Vec<Node>,
}

impl Nodes {
    pub fn new() -> Nodes {
        Nodes { node: vec![] }
    }
    pub fn add_height_nodes(&mut self) {
        let mut new_nodes: Vec<Node> = vec![];
        //let bath_iter = baths.bath.into_iter();
        for node in &mut self.node {
            // create node for hovering
            if !node.hide {
                // if hidden then we do not want to auto-generate realted nodes
                new_nodes.push(Node {
                    name: format!("{}_hover", node.name),
                    x: node.x,
                    y: node.y,
                    z: -1.0,
                    hide: false,
                    neighbors: node
                        .neighbors
                        .iter()
                        .fold(vec![node.name.clone()], |mut v, n| {
                            v.push(format!("{}_hover", n));
                            v
                        }),
                });
            }

            // create node for head in bath
            new_nodes.push(Node {
                name: node.name.clone(),
                x: node.x,
                y: node.y,
                z: node.z,
                hide: node.hide,
                neighbors: if node.hide {
                    node.neighbors
                        .clone()
                        .into_iter()
                        .map(|mut n| {
                            n.push_str("_hover");
                            n
                        })
                        .collect()
                } else {
                    vec![format!("{}_hover", node.name)]
                },
            })
        }
        self.node = new_nodes;
    }
}

pub fn gen_nodes() -> Result<Nodes, ()> {
    let mut nodes = get_baths_config()?;
    nodes.add_height_nodes();
    Ok(nodes)
}

pub fn get_nodemap(nodes: &Nodes) -> HashMap<String, usize> {
    nodes
        .node
        .iter()
        .enumerate()
        .fold(HashMap::new(), |mut node_map, (i, node)| {
            node_map.insert(node.name.clone(), i);
            node_map
        })
}

fn get_baths_config() -> Result<Nodes, ()> {
    match &fs::read_to_string("config/baths.toml") {
        Ok(file) => Ok(toml::from_str::<Nodes>(file).unwrap()),
        Err(_) => Err(()),
    }
}
