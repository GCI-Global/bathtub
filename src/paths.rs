use super::nodes::{Nodes, Node};
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct NodeDistance {
    node: Node,
    distance: Option<u16>,
    visited: bool,
}

pub fn gen_node_paths(nodes: &Nodes, start: &Node, stop: &Node) -> Nodes {
    // return start node if start == stop
    if start.name == stop.name {
        return Nodes {
            node: vec![start.clone()]
        }
    }

    // Assign relevant nodes a distance from the start node
    let mut distance_nodes: Vec<NodeDistance> = vec![];
    for node in &nodes.node {
        if node.name == start.name {
            distance_nodes.push(
                NodeDistance {
                    node: node.clone(),
                    distance: Some(0),
                    visited: false,
                }
            )
        } else {
            distance_nodes.push(
                NodeDistance{
                    node: node.clone(),
                    distance: None,
                    visited: false,
                }
            )
        }
    }
    // Create map for faster node traversal
    let mut node_map = HashMap::new();
    for i in 0..distance_nodes.len() {
        node_map.insert(distance_nodes[i].node.name.clone(), i);
    }
    // Dijkstra's algorithm setup
    let mut current_index = node_map[&start.name];
    let mut neighbor_index: usize;
    let mut mut_dn: &mut NodeDistance;
    let mut new_distance: u16;
    let mut discovered_index: Vec<usize> = vec![];
    // Dijkstra's algorithm assign distance values
    loop { //while current node is not end point
        new_distance = distance_nodes[current_index].distance.unwrap() + 1;
        for neighbor in distance_nodes[current_index].node.neighbors.clone() { // for each neighbor in current node
            if !distance_nodes[node_map[&neighbor]].visited { //only add unvisited nodes
                discovered_index.push(node_map[&neighbor]);
            }
            neighbor_index = node_map[&neighbor];
            mut_dn = &mut distance_nodes[neighbor_index];
            match mut_dn.distance { // update distance if no current distance, or new distance less than existing
                Some(d) if d < new_distance => mut_dn.distance = Some(d),
                _ => mut_dn.distance = Some(new_distance),
            }
        }
        // Set current node as visited
        mut_dn = &mut distance_nodes[current_index];
        mut_dn.visited = true;
        // Change current node to a discovered unvisited node
        match discovered_index.pop() {
            Some(i) => current_index = i,
            _ => break,
        }
        if distance_nodes[current_index].node.name == stop.name {break};
    }
    // Filter useless nodes
    distance_nodes.retain(|n| if let Some(_) = n.distance {true} else {false});
    distance_nodes.sort_by(|a, b| b.distance.unwrap().cmp(&a.distance.unwrap()));
    let mut remove_to: usize = 0;
    // removes nodes untill final node is found
    for i in 0..distance_nodes.len() {
        if distance_nodes[i].node.name != stop.name {
            remove_to = i+1;
        } else {
            break
        }
    }
    for _i in 0..remove_to {
        distance_nodes.remove(0);
    }
    // Remove sequntial nodes that are not neighbors
    // This is because in the forward direction 2+ nodes can have the same
    // distance from the same distance from the origin, but the both are not
    // neighbors to the following node, thus removing all incorrect nodes
    // that are not filtered from Dijkstra's algorithm
    let mut i = 0;
    loop {
        let mut no_match: bool = true;
        while no_match {
            for neighbor in distance_nodes[i].node.neighbors.clone() {
                if neighbor == distance_nodes[i + 1].node.name {
                    no_match = false;
                    break
                } else {
                    //distance_nodes.remove(i + 1);
                    if i == distance_nodes.len()-2 {no_match = false; break}
                }
            }
            if no_match {
                distance_nodes.remove(i+1);
            }
        }
        if i == distance_nodes.len() - 2 {
            break
        } else {
            i += 1;
        }
    }
    // main.rs does not import nor need NodeDistance, convert back to Node
    let mut path_nodes: Nodes = Nodes::new();
    for node in distance_nodes {
        path_nodes.node.push(node.node);
    }
    path_nodes.node.reverse();
    path_nodes
}
    
pub fn gen_gcode_paths(path_nodes: &Nodes) -> Vec<String> {
    let mut gcode_path: Vec<String> = vec![];
    for node in &path_nodes.node {
        gcode_path.push(format!("$J=X{} Y{} Z{} F250\n", node.x, node.y, node.z));
    }
    gcode_path
}    






