use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use toml;

#[derive(Clone, Debug)]
pub struct Actions {
    gcode: String,
    seconds: f32,
}

// Baths are Deserialized from config file, nodes are a generated 3d graph on nodes from the 2d
#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
pub struct Nodes {
    pub node: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct NodeGrid2d {
    pub grid: Vec<Vec<Option<Node>>>,
}

impl Nodes {
    pub fn new() -> Nodes {
        Nodes { node: vec![] }
    }
    fn to_nodes(nodes: Nodes) -> Nodes {
        let mut new_nodes: Vec<Node> = vec![];
        //let bath_iter = baths.bath.into_iter();
        for node in nodes.node {
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
                } else {
                    vec![format!("{}_hover", node.name)]
                },
            })
        }
        Nodes { node: new_nodes }
    }
}

// A 2d positioning relative grid split on the y axis where:
// vec![Node Node None Node]
// vec![None None Node Node]
impl NodeGrid2d {
    fn new(grid: Vec<Vec<Option<Node>>>) -> NodeGrid2d {
        NodeGrid2d { grid }
    }
    pub fn from_nodes(nodes: Nodes) -> NodeGrid2d {
        let mut node_vec = nodes.node.clone();
        // sort by y
        node_vec.retain(|n| !n.name.contains("_hover"));
        node_vec.sort_by(|a, b| (b.y).total_cmp(&a.y));
        // split into many y vecs
        let mut test_value: f32 = node_vec[0].y;
        let mut push_vec: usize = 0;
        let mut build_grid: Vec<Vec<Node>> = vec![Vec::new()];
        for node in node_vec {
            if (node.y - test_value).abs() < 1.0 {
                build_grid[push_vec].push(node);
            } else {
                push_vec += 1;
                test_value = node.y;
                build_grid.push(Vec::new());
                build_grid[push_vec].push(node);
            }
        }
        for i in 0..build_grid.len() {
            build_grid[i].sort_by(|a, b| (b.x).total_cmp(&a.x));
        }
        // find index of longest axis
        let longest_axis = build_grid
            .clone()
            .into_iter()
            .max_by(|x, y| x.len().cmp(&y.len()))
            .unwrap();

        // normalize row lengths with 'None' values
        let mut relative_grid: Vec<Vec<Option<Node>>> =
            build_grid
                .clone()
                .into_iter()
                .fold(Vec::new(), |mut row, axis_vec| {
                    row.push(axis_vec.into_iter().fold(Vec::new(), |mut axis, node| {
                        axis.push(Some(node));
                        axis
                    }));
                    row
                });
        for i in 0..build_grid.len() {
            for j in 0..longest_axis.len() {
                if j == relative_grid[i].len()
                    || longest_axis[j].x - relative_grid[i][j].clone().unwrap().x > 1.0
                {
                    relative_grid[i].insert(j, None)
                }
            }
        }
        NodeGrid2d::new(relative_grid)
    }
}

pub fn gen_nodes() -> Result<Nodes, ()> {
    Ok(Nodes::to_nodes(get_baths_config()?))
}

pub fn get_nodemap(nodes: Nodes) -> HashMap<String, usize> {
    nodes
        .node
        .into_iter()
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
