use super::nodes::{Node, Nodes};
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
            node: vec![start.clone()],
        };
    }

    // Assign relevant nodes a distance from the start node
    //println!("from {} to {}", &start.name, &stop.name);
    //println!("starts nei are {:?}", &start.neighbors);
    let mut distance_nodes: Vec<NodeDistance> = Vec::new();
    for node in &nodes.node {
        distance_nodes.push(NodeDistance {
            node: node.clone(),
            distance: None,
            visited: false,
        })
    }
    distance_nodes.push(NodeDistance {
        node: start.clone(),
        distance: Some(0),
        visited: false,
    });
    // Create map for faster node traversal
    let mut distance_map = HashMap::new();
    for i in 0..distance_nodes.len() {
        distance_map.insert(distance_nodes[i].node.name.clone(), i);
    }
    // Dijkstra's algorithm setup
    let mut current_index = distance_map[&start.name];
    let mut neighbor_index: usize;
    let mut mut_dn: &mut NodeDistance;
    let mut new_distance: u16;
    let mut discovered_index: Vec<usize> = vec![];
    // Dijkstra's algorithm assign distance values
    loop {
        //while current node is not end point
        new_distance = distance_nodes[current_index].distance.unwrap() + 1;
        for neighbor in distance_nodes[current_index].node.neighbors.clone() {
            // for each neighbor in current node
            if !distance_nodes[distance_map[&neighbor]].visited {
                //only add unvisited nodes
                discovered_index.push(distance_map[&neighbor]);
            }
            neighbor_index = distance_map[&neighbor];
            mut_dn = &mut distance_nodes[neighbor_index];
            match mut_dn.distance {
                // update distance if no current distance, or new distance less than existing
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
        if distance_nodes[current_index].node.name == stop.name {
            break;
        };
    }
    // Filter useless nodes
    distance_nodes.retain(|n| {
        if let Some(_) = n.distance {
            true
        } else {
            false
        }
    });
    distance_nodes.sort_by(|a, b| b.distance.unwrap().cmp(&a.distance.unwrap()));
    /*
    for node in &distance_nodes {
        println!("{} ({})", node.node.name, node.distance.unwrap());
    }
    */
    let mut remove_to: usize = 0;
    // removes nodes untill final node is found
    for i in 0..distance_nodes.len() {
        if distance_nodes[i].node.name != stop.name {
            remove_to = i + 1;
        } else {
            break;
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
    /*
    println!("----- distance nodes -----");
    for node in &distance_nodes {
        println!("{}", node.node.name);
    }
    */
    loop {
        let mut no_match: bool = true;
        while no_match {
            for neighbor in &distance_nodes[i].node.neighbors {
                if neighbor == &distance_nodes[i + 1].node.name {
                    no_match = false;
                    break;
                } else {
                    if i == distance_nodes.len() - 2 {
                        no_match = false;
                        break;
                    }
                }
            }
            if no_match {
                distance_nodes.remove(i + 1);
            }
        }
        if i == distance_nodes.len() - 2 {
            break;
        } else {
            i += 1;
        }
    }
    // filter nodes that are neighbors to start and finish, but not shortest path
    for i in 1..distance_nodes.len() - 1 {
        if distance_nodes[i].distance == distance_nodes[i - 1].distance {
            distance_nodes.remove(i);
        }
    }
    // main.rs does not import nor need NodeDistance, convert back to Node
    let mut path_nodes: Nodes = Nodes::new();
    for node in distance_nodes {
        path_nodes.node.push(node.node);
    }
    path_nodes.node.reverse();
    path_nodes.node.remove(0);
    /*
    println!("----- final -----");
    for node in &path_nodes.node {
        println!("{}", &node.name);
    }
    */
    path_nodes
}

/*
pub fn gen_gcode_paths(path_nodes: &Nodes) -> Vec<String> {
    let mut gcode_path: Vec<String> = vec![];
    for node in &path_nodes.node {
        gcode_path.push(format!("$J=X{} Y{} Z{} F250\n", node.x, node.y, node.z));
    }
    gcode_path
}
*/
