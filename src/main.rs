#![feature(total_cmp)]
mod actions;
mod build;
mod grbl;
mod manual;
mod nodes;
mod paths;
mod run;
use actions::Actions;
use build::{Build, BuildMessage};
use grbl::{Command as Cmd, Grbl, Status};
use manual::{Manual, ManualMessage};
use nodes::{Node, NodeGrid2d, Nodes};
use regex::Regex;
use run::Step;
use run::{Run, RunMessage};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use std::{fs, thread};

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
    nodes: Rc<RefCell<Nodes>>,
    node_map: HashMap<String, usize>,
    prev_node: Arc<Mutex<Option<Node>>>,
    current_node: Arc<Mutex<Node>>,
    next_nodes: Arc<Mutex<Vec<Node>>>,
    actions: Rc<RefCell<Actions>>,
    grbl: Grbl,
    stop_tx: Option<mpsc::Sender<String>>,
    connected: bool,
    running: bool,
    recipie_regex: Regex,
    grbl_status: Option<Arc<Mutex<Option<Status>>>>,
}

impl State {
    async fn run_recipie(
        grbl: Grbl,
        stop_rx: mpsc::Receiver<String>,
        prev_node: Arc<Mutex<Option<Node>>>,
        current_node: Arc<Mutex<Node>>,
        next_nodes: Arc<Mutex<Vec<Node>>>,
        recipie: Vec<Step>,
        node_map: HashMap<String, usize>,
        nodes: Nodes,
        actions: Actions,
    ) -> Result<(), Errors> {
        let mut stop_received = false;
        if let Some(s) = grbl.get_status() {
            if s.status == "Alarm".to_string() {
                grbl.push_command(Cmd::new("$H".to_string()));
            }
        }
        // wait for homing to finish
        loop {
            if let Some(s) = grbl.get_status() {
                if s.status == "Idle".to_string() {
                    break;
                }
            }
        }
        // spawn thread to monitor active nodes
        let gx = grbl.clone();
        let px = Arc::clone(&prev_node);
        let cx = Arc::clone(&current_node);
        let nx = Arc::clone(&next_nodes);
        thread::spawn(move || loop {
            if let Some(grbl_stat) = gx.get_status() {
                let mut nn = nx.lock().unwrap();
                match nn.first() {
                    Some(n) => {
                        if (grbl_stat.x - n.x).abs() < 0.2
                            && (grbl_stat.y - n.y).abs() < 0.2
                            && (grbl_stat.z - n.z).abs() < 0.2
                        {
                            let mut cn = cx.lock().unwrap();
                            let mut pn = px.lock().unwrap();
                            *pn = Some(cn.clone());
                            *cn = nn.remove(0);
                        }
                    }
                    None => {}
                }
            }
            thread::sleep(Duration::from_millis(1))
        });
        for step in recipie {
            if stop_received || Ok("Stop".to_string()) == stop_rx.try_recv() {
                break;
            }
            // gen paths and send
            let in_bath = match step.in_bath {
                true => "_inBath",
                false => "",
            };
            {
                let cn = current_node.lock().unwrap();
                let mut nn = next_nodes.lock().unwrap();
                let future_node = &nodes.node[match node_map
                    .get(&format!("{}{}", step.selected_destination, in_bath))
                {
                    Some(n) => n,
                    _ => break,
                }
                .clone()];
                let node_paths = paths::gen_node_paths(&nodes, &cn, future_node);
                for node in node_paths.node {
                    nn.push(node.clone());
                    grbl.push_command(Cmd::new(format!(
                        "$J=X{} Y{} Z{} F250",
                        node.x, node.y, node.z
                    )));
                }
            }
            loop {
                {
                    let nn = next_nodes.lock().unwrap();
                    if nn.len() == 0 {
                        break;
                    }
                }
                thread::sleep(Duration::from_nanos(25));
            }
            if stop_received || Ok("Stop".to_string()) == stop_rx.try_recv() {
                break;
            }
            let (tx, rx) = mpsc::channel();
            let step_c = step.clone();
            thread::spawn(move || {
                let seconds = match (step_c.hours_value.clone().parse::<u64>().unwrap_or(0)
                    * 3600000
                    + step_c.mins_value.parse::<u64>().unwrap_or(0) * 60000
                    + step_c.secs_value.parse::<u64>().unwrap_or(0) * 1000)
                    .overflowing_sub(500)
                {
                    (n, false) => n,
                    (_, true) => 0,
                }; // calcualte then subtract half a second because of code delay
                thread::sleep(Duration::from_millis(seconds));
                tx.send("Stop").unwrap();
            });
            // send action steps
            // TODO: Hash map creation should be moved into state, not in loop
            let mut contains_wait = false;
            let mut action_map = HashMap::new();
            for action in actions.action.clone() {
                action_map.insert(action.name, action.commands);
            }
            if stop_received {
                break;
            };
            let action_commands = action_map.get(&step.selected_action).unwrap();
            for command in action_commands {
                if stop_received || Ok("Stop".to_string()) == stop_rx.try_recv() {
                    stop_received = true;
                    break;
                }
                if command != &"WAIT".to_string() {
                    grbl.push_command(Cmd::new(command.clone()));
                } else {
                    contains_wait = true
                }
            }
            loop {
                if stop_received || Ok("Stop".to_string()) == stop_rx.try_recv() {
                    stop_received = true;
                    break;
                }
                if rx.try_recv() == Ok("Stop") {
                    grbl.push_command(Cmd::new("\u{85}".to_string()));
                    break;
                } else if !contains_wait {
                    grbl.clear_responses();
                    if grbl.queue_len() < action_commands.len() {
                        for command in action_commands {
                            if stop_received || Ok("Stop".to_string()) == stop_rx.try_recv() {
                                stop_received = true;
                                break;
                            }
                            grbl.push_command(Cmd::new(command.clone()));
                        }
                    }
                }
            }
        }
        Ok(())
    }
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
    actions: Actions,
}

#[derive(Debug, Clone)]
enum LoadError {
    _Placeholder,
}

#[derive(Debug, Clone)]
enum Errors {
    GRBLError,
}

#[derive(Debug, Clone)]
enum Message {
    ManualTab,
    BuildTab,
    RunTab,
    RecipieDone(Result<(), Errors>),
    ManualDone(Result<(), Errors>),
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
                        let ref_node = Rc::new(RefCell::new(state.nodes.clone()));
                        let ref_actions = Rc::new(RefCell::new(state.actions));
                        *self = Bathtub::Loaded(State {
                            //status: "Click any button\nto start homing cycle".to_string(),
                            state: TabState::Manual,
                            tabs: Tabs {
                                manual: Manual::new(state.node_grid2d),
                                manual_btn: button::State::new(),
                                run: Run::new(),
                                run_btn: button::State::new(),
                                build: Build::new(Rc::clone(&ref_node), Rc::clone(&ref_actions)),
                                build_btn: button::State::new(),
                            },
                            nodes: Rc::clone(&ref_node),
                            node_map: state.node_map.clone(),
                            prev_node: Arc::new(Mutex::new(None)),
                            current_node: Arc::new(Mutex::new(
                                state.nodes.node
                                    [state.node_map.get(&"Rinse 1".to_string()).unwrap().clone()]
                                .clone(),
                            ))
                            .clone(),
                            next_nodes: Arc::new(Mutex::new(Vec::new())),
                            actions: Rc::clone(&ref_actions),
                            connected: false,
                            running: false,
                            grbl: grbl::new(),
                            stop_tx: None,
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
                        state.tabs.run.update(RunMessage::TabActive);
                        state.state = TabState::Run
                    }
                    Message::Manual(ManualMessage::Stop) => {
                        if let Some(tx) = &state.stop_tx {
                            tx.send("Stop".to_string()).unwrap_or(());
                        }
                        state.grbl.push_command(Cmd::new("\u{85}".to_string()));
                        let pn = state.prev_node.lock().unwrap();
                        let mut cn = state.current_node.lock().unwrap();
                        let mut nn = state.next_nodes.lock().unwrap();
                        match nn.first() {
                            Some(nn) => {
                                let s = state.grbl.get_status().unwrap();
                                *cn = Node {
                                    name: "paused_node".to_string(),
                                    x: s.x,
                                    y: s.y,
                                    z: s.z,
                                    is_rinse: false,
                                    neighbors: if pn.as_ref().unwrap().name
                                        == "paused_node".to_string()
                                    {
                                        pn.as_ref().unwrap().neighbors.clone()
                                    } else {
                                        vec![cn.name.clone(), nn.name.clone()]
                                    },
                                }
                            }
                            None => (),
                        }
                        *nn = Vec::new();
                        //println!("pn: {:?}\ncn: {:?}\nnn: {:?}", pn, cn, nn);
                    }
                    Message::Manual(ManualMessage::ButtonPressed(node)) => {
                        let (tx, rx) = mpsc::channel();
                        state.stop_tx = Some(tx);
                        state.connected = true;
                        command = Command::perform(
                            State::run_recipie(
                                state.grbl.clone(),
                                rx,
                                Arc::clone(&state.prev_node),
                                Arc::clone(&state.current_node),
                                Arc::clone(&state.next_nodes),
                                vec![Step {
                                    step_num: 0.to_string(),
                                    selected_destination: node,
                                    selected_action: "Rest".to_string(),
                                    secs_value: 0.to_string(),
                                    mins_value: 0.to_string(),
                                    hours_value: 0.to_string(),
                                    in_bath: state.tabs.manual.in_bath,
                                    require_input: false,
                                }],
                                state.node_map.clone(),
                                state.nodes.borrow().clone(),
                                state.actions.borrow().clone(),
                            ),
                            Message::ManualDone,
                        )
                    }
                    Message::Run(RunMessage::Run) => {
                        // TODO: need to create + check for flag for manual movement
                        if !state.running {
                            state.running = true;
                            let (tx, rx) = mpsc::channel();
                            state.stop_tx = Some(tx);
                            command = Command::perform(
                                State::run_recipie(
                                    state.grbl.clone(),
                                    rx,
                                    Arc::clone(&state.prev_node),
                                    Arc::clone(&state.current_node),
                                    Arc::clone(&state.next_nodes),
                                    state.tabs.run.steps.clone(),
                                    state.node_map.clone(),
                                    state.nodes.borrow().clone(),
                                    state.actions.borrow().clone(),
                                ),
                                Message::RecipieDone,
                            );
                        }
                    }
                    Message::Manual(msg) => state.tabs.manual.update(msg),
                    Message::Build(msg) => state.tabs.build.update(msg),
                    Message::Run(msg) => state.tabs.run.update(msg),
                    Message::RecipieDone(Ok(_)) => {
                        state.grbl_status = Some(Arc::clone(&state.grbl.mutex_status));
                        state.running = false;
                        state.connected = true;
                    }
                    Message::RecipieDone(Err(_err)) => {
                        state.running = false;
                        state.connected = false
                    }
                    Message::Tick => {
                        let stat = state.grbl.get_status();
                        if let Some(s) = stat {
                            state.tabs.manual.status = format!(
                                "{} state at\n({:.3}, {:.3}, {:.3})",
                                &s.status, &s.x, &s.y, &s.z
                            )
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
                                        .size(30)
                                        .font(CQ_MONO),
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
                                        .size(30)
                                        .font(CQ_MONO),
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
                                        .size(30)
                                        .font(CQ_MONO),
                                )
                                .width(Length::Fill)
                                .padding(20)
                                .on_press(Message::BuildTab),
                            ),
                    );
                    if *running {
                        content
                            .push(Space::with_height(Length::Units(100)))
                            .push(
                                Text::new("Unavailable while running recipie")
                                    .size(50)
                                    .font(CQ_MONO),
                            )
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
                                        .size(30)
                                        .font(CQ_MONO),
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
                                        .size(30)
                                        .font(CQ_MONO),
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
                                        .size(30)
                                        .font(CQ_MONO),
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
                                        .size(30)
                                        .font(CQ_MONO),
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
                                        .size(30)
                                        .font(CQ_MONO),
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
                                        .size(30)
                                        .font(CQ_MONO),
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
    fn new(
        nodes: Nodes,
        node_map: HashMap<String, usize>,
        node_grid2d: NodeGrid2d,
        actions: Actions,
    ) -> LoadState {
        LoadState {
            nodes,
            node_map,
            node_grid2d,
            actions,
        }
    }

    // This is just a placeholder. Will eventually read data from server
    async fn load() -> Result<LoadState, LoadError> {
        let nodes = nodes::gen_nodes();
        Ok(LoadState::new(
            nodes.clone(),
            nodes::get_nodemap(nodes.clone()),
            NodeGrid2d::from_nodes(nodes),
            actions::gen_actions(),
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

const CQ_MONO: Font = Font::External {
    name: "CQ_MONO",
    bytes: include_bytes!("../fonts/CQ_MONO.otf"),
};
