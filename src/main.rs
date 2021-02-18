#![feature(total_cmp)]
mod build;
mod grbl;
mod manual;
mod nodes;
mod paths;
mod run;
use run::Step;
use build::{Build, BuildMessage};
use grbl::{Grbl, Status};
use manual::{Manual, ManualMessage};
use nodes::{Node, NodeGrid2d, Nodes};
use regex::Regex;
use run::{Run, RunMessage};
use std::collections::HashMap;
use std::{fs, thread};
use std::time::Duration;
use std::sync::{Arc, Mutex, mpsc};
use serde::Deserialize;

use iced::{
    button, time, Align, Application, Button, Column, Command, Container, Element, Font,
    HorizontalAlignment, Length, Row, Settings, Space, Subscription, Text,
};

pub fn main() -> iced::Result {
    Bathtub::run(Settings::default())
}

//#[derive(Debug)]
enum Bathtub {
    Loading,
    Loaded(State),
}

struct State {
    state: TabState,
    tabs: Tabs,
    nodes: Nodes,
    node_map: HashMap<String, usize>,
    current_node: Node,
    grbl: Grbl,
    connected: bool,
    running: bool,
    recipie_regex: Regex,
    grbl_status: Option<Arc<Mutex<Option<Status>>>>,
}

impl State {
    async fn run_recipie(recipie: Vec<Step>, node_map: HashMap<String, usize>, nodes: Nodes, actions: Actions) -> Result<(), ()> {
        let grbl = grbl::new();
        grbl.send("$H".to_string()).unwrap();
        let mut current_node = nodes.node[node_map.get(&"MCL_16".to_string()).unwrap().clone()].clone();
        for step in recipie {
            // gen paths and send
            println!("{}", step.selected_destination);
            let next_node = &nodes.node[
                node_map
                .get(&format!("{}{}", step.selected_destination, "_inBath"))
                .unwrap()
                .clone()];
            let node_paths = paths::gen_node_paths(&nodes, &current_node, next_node);
            let gcode_paths = paths::gen_gcode_paths(&node_paths);
            for gcode_path in gcode_paths {
                println!("{}", gcode_path.replace("\n",""));
                grbl.send(gcode_path).unwrap();
                thread::sleep(Duration::from_millis(30))
            }
            current_node = next_node.clone();
            // wait for idle
            loop {
                if let Some(grbl_stat) = grbl.mutex_status.lock().unwrap().clone() {
                    if grbl_stat.x == next_node.x && grbl_stat.y == next_node.y && grbl_stat.z == next_node.z {break}
                }
            }
            let (tx, rx) = mpsc::channel();
            let step_c = step.clone();
            thread::spawn(move || {
                let seconds = step_c.hours_value.clone().parse::<u64>().unwrap_or(0) * 3600 + step_c.mins_value.parse::<u64>().unwrap_or(0) * 60 + step_c.secs_value.parse::<u64>().unwrap_or(0);
                thread::sleep(Duration::from_secs(seconds));
                tx.send("Stop").unwrap();

            });
            // send action steps
            let mut action_map = HashMap::new();
            for action in actions.action.clone() {
                action_map.insert(action.name, action.commands);
            }
            for command in action_map.get(&step.selected_action).unwrap() {
                if command != &"WAIT".to_string() {
                    grbl.send(command.clone()).unwrap();
                    //thread::sleep(Duration::from_millis(50));
                }
            }
            loop {
                //let mut recv = rx.try_recv();
                //println!("{:?}", recv);
                if rx.try_recv() == Ok("Stop") {
                    println!("recived stop");
                    //grbl.jog_cancel().unwrap();
                    grbl.jog_cancel().unwrap();
                    //grbl.send("~".to_string()).unwrap();
                    //thread::sleep(Duration::from_millis(1000));
                    break
                } else { if let Ok(response) = grbl.try_recv() {
                    if response.2 == "ok".to_string() {
                        println!("heard ok");
                        for command in action_map.get(&step.selected_action).unwrap() {
                            if command != &"WAIT".to_string() {
                                grbl.send(command.clone()).unwrap();
                                thread::sleep(Duration::from_millis(50));
                            }
                        }
                    }
                }
                }
            }
        }
        grbl.close().unwrap();
        thread::sleep(Duration::from_millis(1000));
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Actions {
    action: Vec<Action>
}

#[derive(Clone, Debug, Deserialize)]
struct Action {
    name: String,
    commands: Vec<String>
}

struct Tabs {
    manual: Manual,
    manual_btn: button::State,
    run: Run,
    run_btn: button::State,
    build: Build,
    build_btn: button::State,
    //settings: button::State,
}

enum TabState {
    Manual,
    Run,
    Build,
}

#[derive(Debug, Clone)]
struct LoadState {
    nodes: Nodes,
    node_map: HashMap<String, usize>,
    node_grid2d: NodeGrid2d,
}

#[derive(Debug, Clone)]
enum LoadError {
    _Placeholder,
}

#[derive(Debug, Clone)]
enum Message {
    ManualTab,
    BuildTab,
    RunTab,
    RecipieDone(Result<(), ()>),
    Manual(ManualMessage),
    Build(BuildMessage),
    Run(RunMessage),
    Loaded(Result<LoadState, LoadError>),
    Tick,
}

impl Application for Bathtub {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Bathtub, Command<Message>) {
        (
            Bathtub::Loading,
            Command::perform(LoadState::load(), Message::Loaded),
        )
    }

    fn title(&self) -> String {
        String::from("Bathtub")
    }

    fn subscription(&self) -> Subscription<Message> {
        match self {
            Bathtub::Loaded(state) => {
                if state.connected {
                    return time::every(std::time::Duration::from_millis(50))
                        .map(|_| Message::Tick);
                } else {
                    return Subscription::none();
                }
            }
            _ => Subscription::none(),
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        let mut command = Command::none(); // setup to allow nested match statements to return different command
        match self {
            Bathtub::Loading => {
                match message {
                    Message::Loaded(Ok(state)) => {
                        *self = Bathtub::Loaded(State {
                            //status: "Click any button\nto start homing cycle".to_string(),
                            state: TabState::Manual,
                            tabs: Tabs {
                                manual: Manual::new(state.node_grid2d),
                                manual_btn: button::State::new(),
                                run: Run::new(),
                                run_btn: button::State::new(),
                                build: Build::new(),
                                build_btn: button::State::new(),
                            },
                            nodes: state.nodes.clone(),
                            node_map: state.node_map.clone(),
                            current_node: state.nodes.node
                                [state.node_map.get(&"MCL_16".to_string()).unwrap().clone()]
                            .clone(),
                            connected: false,
                            running: false,
                            grbl: grbl::new(),
                            grbl_status: None,
                            recipie_regex: Regex::new(r"^[^.]+").unwrap(),
                        });
                    }
                    Message::Loaded(Err(_)) => {
                        panic!("somehow loaded had an error")
                        // need to add correct fail state, following is from the Todos example
                        //*self = Bathtub::Loaded(State::default());
                    }
                    _ => {}
                }
                Command::none()
            }
            Bathtub::Loaded(state) => {
                match message {
                    Message::ManualTab => state.state = TabState::Manual,
                    Message::BuildTab => state.state = TabState::Build,
                    Message::RunTab => {
                        state.tabs.run.search = fs::read_dir("./recipies").unwrap().fold(
                            Vec::new(),
                            |mut rec, file| {
                                if let Some(caps) = state
                                    .recipie_regex
                                    .captures(&file.unwrap().file_name().to_str().unwrap())
                                {
                                    rec.push(caps[0].to_string());
                                }
                                rec
                            },
                        );
                        state.tabs.run.search.sort();
                        state.state = TabState::Run
                    }
                    Message::Manual(ManualMessage::ButtonPressed(node)) => {
                        if !state.connected {
                            state.tabs.manual.status =
                                "Running Homing Cycle\nPlease wait".to_string();
                            state.grbl.send("$H".to_string()).unwrap();
                            state.connected = true;
                        }
                        state.grbl_status = Some(Arc::clone(&state.grbl.mutex_status));
                        let enter_bath: String;
                        if state.tabs.manual.in_bath {
                            enter_bath = "_inBath".to_string()
                        } else {
                            enter_bath = "".to_string()
                        }
                        let next_node = &state.nodes.node[state
                            .node_map
                            .get(&format!("{}{}", node.clone(), enter_bath))
                            .unwrap()
                            .clone()];
                        let node_paths =
                            paths::gen_node_paths(&state.nodes, &state.current_node, next_node);
                        let gcode_paths = paths::gen_gcode_paths(&node_paths);
                        // send gcode
                        for gcode_path in gcode_paths {
                            state.grbl.send(gcode_path).unwrap();
                        }
                        state.current_node = next_node.clone();
                    }
                    Message::Run(RunMessage::Run) => {
                        // TODO: need to create + check for flag for manual movement
                                if !state.running {
                                    let actions_toml = &fs::read_to_string("config/actions.toml").expect("unable to open config/actions.toml");
                                    state.running = true;
                                    state.grbl.close();
                                    command = Command::perform(State::run_recipie(state.tabs.run.steps.clone(), state.node_map.clone(), state.nodes.clone(), toml::from_str::<Actions>(actions_toml).unwrap()), Message::RecipieDone);
                                }
                    }
                    Message::Manual(msg) => state.tabs.manual.update(msg),
                    Message::Build(msg) => state.tabs.build.update(msg),
                    Message::Run(msg) => state.tabs.run.update(msg),
                    Message::RecipieDone(_) => {
                        state.grbl = grbl::new();
                        state.grbl_status = Some(Arc::clone(&state.grbl.mutex_status));
                        state.tabs.manual.status = "Click any button\nto start homing cycle".to_string();
                        state.current_node = state.nodes.node[state.node_map.get(&"MCL_16".to_string()).unwrap().clone()].clone();
                        state.running = false;
                        state.connected = false;
                    }
                    Message::Tick => {
                        if let Some(grbl_status) = &state.grbl_status {
                            if let Some(grbl_stat) = grbl_status.lock().unwrap().clone() {
                            state.tabs.manual.status = format!(
                                "{} state at\n({:.3}, {:.3}, {:.3})",
                                &grbl_stat.status,
                                &grbl_stat.x,
                                &grbl_stat.y,
                                &grbl_stat.z,
                            )
                            }
                        }
                    }
                    _ => {}
                }
                command
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        match self {
            Bathtub::Loading => loading_message(),
            Bathtub::Loaded(State {
                state,
                tabs,
                running,
                ..
            }) => match state {
                TabState::Manual => {
                    let content = Column::new().push(
                        Row::new()
                            .push(
                                Button::new(
                                    &mut tabs.manual_btn,
                                    Text::new("Manual")
                                        .horizontal_alignment(HorizontalAlignment::Center)
                                        .size(30),
                                )
                                .width(Length::Fill)
                                .padding(20)
                                .on_press(Message::ManualTab),
                            )
                            .push(
                                Button::new(
                                    &mut tabs.run_btn,
                                    Text::new("Run")
                                        .horizontal_alignment(HorizontalAlignment::Center)
                                        .size(30),
                                )
                                .width(Length::Fill)
                                .padding(20)
                                .on_press(Message::RunTab),
                            )
                            .push(
                                Button::new(
                                    &mut tabs.build_btn,
                                    Text::new("Build")
                                        .horizontal_alignment(HorizontalAlignment::Center)
                                        .size(30),
                                )
                                .width(Length::Fill)
                                .padding(20)
                                .on_press(Message::BuildTab),
                            ),
                    );
                    if *running {
                        content
                            .push(Space::with_height(Length::Units(100)))
                            .push(Text::new("Unavailable while running recipie").size(50))
                            .align_items(Align::Center)
                            .into()
                    } else {
                        content
                            .push(tabs.manual.view().map(move |msg| Message::Manual(msg)))
                            .into()
                    }
                }
                TabState::Run => Column::new()
                    .push(
                        Row::new()
                            .push(
                                Button::new(
                                    &mut tabs.manual_btn,
                                    Text::new("Manual")
                                        .horizontal_alignment(HorizontalAlignment::Center)
                                        .size(30),
                                )
                                .width(Length::Fill)
                                .padding(20)
                                .on_press(Message::ManualTab),
                            )
                            .push(
                                Button::new(
                                    &mut tabs.run_btn,
                                    Text::new("Run")
                                        .horizontal_alignment(HorizontalAlignment::Center)
                                        .size(30),
                                )
                                .width(Length::Fill)
                                .padding(20)
                                .on_press(Message::RunTab),
                            )
                            .push(
                                Button::new(
                                    &mut tabs.build_btn,
                                    Text::new("Build")
                                        .horizontal_alignment(HorizontalAlignment::Center)
                                        .size(30),
                                )
                                .width(Length::Fill)
                                .padding(20)
                                .on_press(Message::BuildTab),
                            ),
                    )
                    .push(tabs.run.view().map(move |msg| Message::Run(msg)))
                    .into(),
                TabState::Build => Column::new()
                    .push(
                        Row::new()
                            .push(
                                Button::new(
                                    &mut tabs.manual_btn,
                                    Text::new("Manual")
                                        .horizontal_alignment(HorizontalAlignment::Center)
                                        .size(30),
                                )
                                .width(Length::Fill)
                                .padding(20)
                                .on_press(Message::ManualTab),
                            )
                            .push(
                                Button::new(
                                    &mut tabs.run_btn,
                                    Text::new("Run")
                                        .horizontal_alignment(HorizontalAlignment::Center)
                                        .size(30),
                                )
                                .width(Length::Fill)
                                .padding(20)
                                .on_press(Message::RunTab),
                            )
                            .push(
                                Button::new(
                                    &mut tabs.build_btn,
                                    Text::new("Build")
                                        .horizontal_alignment(HorizontalAlignment::Center)
                                        .size(30),
                                )
                                .width(Length::Fill)
                                .padding(20)
                                .on_press(Message::BuildTab),
                            ),
                    )
                    .push(tabs.build.view().map(move |msg| Message::Build(msg)))
                    .into(),
            },
        }
    }
}

impl LoadState {
    fn new(nodes: Nodes, node_map: HashMap<String, usize>, node_grid2d: NodeGrid2d) -> LoadState {
        LoadState {
            nodes,
            node_map,
            node_grid2d,
        }
    }

    // This is just a placeholder. Will eventually read data from server
    async fn load() -> Result<LoadState, LoadError> {
        let nodes = nodes::gen_nodes();

        Ok(LoadState::new(
            nodes.clone(),
            nodes::get_nodemap(nodes.clone()),
            NodeGrid2d::from_nodes(nodes),
        ))
    }
}

fn loading_message<'a>() -> Element<'a, Message> {
    Container::new(
        Text::new("Loading...\n\nThis should be very quick.")
            .horizontal_alignment(HorizontalAlignment::Center)
            .size(50),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_y()
    .into()
}

const MONOSPACE_TYPEWRITTER: Font = Font::External {
    name: "MonospaceTypewritter",
    bytes: include_bytes!("../fonts/MonospaceTypewriter.ttf"),
};
