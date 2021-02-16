#![feature(total_cmp)]
mod build;
mod grbl;
mod manual;
mod nodes;
mod paths;
mod run;
use build::{Build, BuildMessage};
use grbl::Grbl;
use manual::{Manual, ManualMessage};
use nodes::{Node, NodeGrid2d, Nodes};
use run::{Run, RunMessage};
use std::collections::HashMap;
use std::fs;
use regex::Regex;

use iced::{
    button, time, Application, Button, Column, Command, Container, Element, Font,
    HorizontalAlignment, Length, Row, Settings, Subscription, Text,
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
    recipie_regex: Regex,
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
                            grbl: grbl::new(),
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
                        //println!("recipies: {:?}", fs::read_dir("./recipies").unwrap().fold(Vec::new(), |mut recipies, file| {recipies.push(file.unwrap().file_name()); recipies}));
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
                        let enter_bath: String;
                        println!("{}", node);
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
                    Message::Manual(msg) => state.tabs.manual.update(msg),
                    Message::Build(msg) => state.tabs.build.update(msg),
                    Message::Run(msg) => state.tabs.run.update(msg),
                    Message::Tick => {
                        state.grbl.send("?".to_string()).unwrap();
                        for _i in 0..3 {
                            match state.grbl.try_recv() {
                                Ok((_, cmd, msg)) if cmd == "?".to_string() => {
                                    if let Some(caps) =
                                        state.tabs.manual.status_regex.captures(&msg[..])
                                    {
                                        state.tabs.manual.status = format!(
                                            "{} state at\n({:.3}, {:.3}, {:.3})",
                                            &caps["status"],
                                            &caps["X"].parse::<f32>().unwrap(), // convert to f32 for decimal places
                                            &caps["Y"].parse::<f32>().unwrap(),
                                            &caps["Z"].parse::<f32>().unwrap()
                                        );
                                    }
                                }
                                // do nothing for now. Will likely create a log of gcode commands
                                // in future
                                _ => (),
                            }
                        }
                    }
                    _ => {}
                }
                Command::none()
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        match self {
            Bathtub::Loading => loading_message(),
            Bathtub::Loaded(State { state, tabs, .. }) => match state {
                TabState::Manual => Column::new()
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
                    .push(tabs.manual.view().map(move |msg| Message::Manual(msg)))
                    .into(),
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
