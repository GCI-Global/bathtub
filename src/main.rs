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
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::Duration;
use std::{fs, mem::discriminant, thread};

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
    tab_bar: TabBar,
    nodes: Rc<RefCell<Nodes>>,
    node_map: HashMap<String, usize>,
    prev_node: Arc<Mutex<Option<Node>>>,
    current_node: Arc<Mutex<Node>>,
    next_nodes: Arc<Mutex<Vec<Node>>>,
    actions: Rc<RefCell<Actions>>,
    grbl: Grbl,
    connected: bool,
    recipie_regex: Regex,
    grbl_status: Option<Arc<Mutex<Option<Status>>>>,
    recipie_state: Arc<(Mutex<RecipieState>, Condvar)>,
}

impl State {
    async fn run_recipie(
        grbl: Grbl,
        recipie_state: Arc<(Mutex<RecipieState>, Condvar)>,
        prev_node: Arc<Mutex<Option<Node>>>,
        current_node: Arc<Mutex<Node>>,
        next_nodes: Arc<Mutex<Vec<Node>>>,
        recipie: Vec<Step>,
        node_map: HashMap<String, usize>,
        nodes: Nodes,
        actions: Actions,
    ) -> Result<(), Errors> {
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
        let recipie_state2 = Arc::clone(&recipie_state);
        thread::spawn(move || {
            while !break_and_hold(Arc::clone(&recipie_state2)) {
                if let Some(grbl_stat) = gx.get_status() {
                    let mut nn = nx.lock().unwrap();
                    match nn.first() {
                        Some(n) => {
                            if (grbl_stat.x - n.x).abs() < 0.5
                                && (grbl_stat.y - n.y).abs() < 0.5
                                && (grbl_stat.z - n.z).abs() < 0.5
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
            }
        });
        for step in recipie {
            if step.require_input {
                let (recipie_state, _) = &*recipie_state;
                let mut recipie_state = recipie_state.lock().unwrap();
                *recipie_state = RecipieState::RequireInput;
            }
            if break_and_hold(Arc::clone(&recipie_state)) {
                break;
            }
            // gen paths and send
            let in_bath = match step.in_bath {
                true => "_inBath",
                false => "",
            };
            while !break_and_hold(Arc::clone(&recipie_state)) {
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
                while (*next_nodes.lock().unwrap()).len() != 0 {
                    let (recipie_state, _) = &*recipie_state;
                    match *recipie_state.lock().unwrap() {
                        //RecipieState::ManualRunning => {}
                        //RecipieState::RecipieRunning => {}
                        RecipieState::Stopped => break,
                        RecipieState::RecipiePaused => {
                            grbl.push_command(Cmd::new("\u{85}".to_string()));
                            set_pause_node(
                                Arc::clone(&current_node),
                                Arc::clone(&next_nodes),
                                grbl.clone(),
                            );
                        }
                        _ => {}
                    }
                }
                let cn = current_node.lock().unwrap();
                let nn = next_nodes.lock().unwrap();
                if cn.name != "paused_node" && nn.len() == 0 {
                    break;
                };
            }
            if break_and_hold(Arc::clone(&recipie_state)) {
                break;
            }
            let (tx, rx) = mpsc::channel();
            let step_c = step.clone();
            let recipie_state3 = Arc::clone(&recipie_state);
            thread::spawn(move || {
                let mut seconds = match (step_c.hours_value.clone().parse::<u64>().unwrap_or(0)
                    * 3600000
                    + step_c.mins_value.parse::<u64>().unwrap_or(0) * 60000
                    + step_c.secs_value.parse::<u64>().unwrap_or(0) * 1000)
                    .overflowing_sub(500)
                {
                    (n, false) => n,
                    (_, true) => 0,
                }; // calcualte then subtract half a second because of code delay
                while !break_and_hold(Arc::clone(&recipie_state3)) {
                    match seconds.overflowing_sub(500) {
                        (n, false) => seconds = n,
                        (_, true) => break,
                    };
                    thread::sleep(Duration::from_millis(500));
                }
                tx.send("Stop").unwrap_or(());
            });
            // send action steps
            // TODO: Hash map creation should be moved into state, not in loop
            let mut contains_wait = false;
            let mut action_map = HashMap::new();
            for action in actions.action.clone() {
                action_map.insert(action.name, action.commands);
            }
            if break_and_hold(Arc::clone(&recipie_state)) {
                break;
            }
            let action_commands = action_map.get(&step.selected_action).unwrap();
            for command in action_commands {
                if break_and_hold(Arc::clone(&recipie_state)) {
                    break;
                }
                if command != &"WAIT".to_string() {
                    grbl.push_command(Cmd::new(command.clone()));
                } else {
                    contains_wait = true
                }
            }
            loop {
                if break_and_hold(Arc::clone(&recipie_state)) {
                    break;
                }
                if rx.try_recv() == Ok("Stop") {
                    grbl.push_command(Cmd::new("\u{85}".to_string()));
                    break;
                } else if !contains_wait {
                    grbl.clear_responses();
                    if grbl.queue_len() < action_commands.len() {
                        for command in action_commands {
                            if break_and_hold(Arc::clone(&recipie_state)) {
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

#[derive(Clone, Copy)]
pub enum RecipieState {
    Stopped,
    ManualRunning,
    RecipieRunning,
    RecipiePaused,
    RequireInput,
}

struct Tabs {
    manual: Manual,
    run: Run,
    build: Build,
}

struct TabBar {
    manual_btn: button::State,
    build_btn: button::State,
    run_btn: button::State,
}

#[derive(Debug, Clone)]
enum TabBarMessage {
    Manual,
    Run,
    Build,
}

impl TabBar {
    fn new() -> Self {
        TabBar {
            manual_btn: button::State::new(),
            run_btn: button::State::new(),
            build_btn: button::State::new(),
        }
    }

    fn view(&mut self) -> Element<TabBarMessage> {
        Column::new()
            .push(
                Row::new()
                    .push(
                        Button::new(
                            &mut self.manual_btn,
                            Text::new("Manual")
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .size(30)
                                .font(CQ_MONO),
                        )
                        .width(Length::Fill)
                        .padding(20)
                        .on_press(TabBarMessage::Manual),
                    )
                    .push(
                        Button::new(
                            &mut self.run_btn,
                            Text::new("Run")
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .size(30)
                                .font(CQ_MONO),
                        )
                        .width(Length::Fill)
                        .padding(20)
                        .on_press(TabBarMessage::Run),
                    )
                    .push(
                        Button::new(
                            &mut self.build_btn,
                            Text::new("Build")
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .size(30)
                                .font(CQ_MONO),
                        )
                        .width(Length::Fill)
                        .padding(20)
                        .on_press(TabBarMessage::Build),
                    ),
            )
            .into()
    }
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
    TabBar(TabBarMessage),
    RecipieDone(Result<(), Errors>),
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
                        let recipie_state =
                            Arc::new((Mutex::new(RecipieState::Stopped), Condvar::new()));
                        let current_node = Arc::new(Mutex::new(
                            state.nodes.node
                                [state.node_map.get(&"HOME".to_string()).unwrap().clone()]
                            .clone(),
                        ));
                        let next_nodes = Arc::new(Mutex::new(Vec::new()));
                        *self = Bathtub::Loaded(State {
                            //status: "Click any button\nto start homing cycle".to_string(),
                            state: TabState::Manual,
                            tabs: Tabs {
                                manual: Manual::new(state.node_grid2d),
                                run: Run::new(Arc::clone(&recipie_state)),
                                build: Build::new(Rc::clone(&ref_node), Rc::clone(&ref_actions)),
                            },
                            tab_bar: TabBar::new(),
                            nodes: Rc::clone(&ref_node),
                            node_map: state.node_map.clone(),
                            prev_node: Arc::new(Mutex::new(None)),
                            current_node: Arc::clone(&current_node),
                            next_nodes: Arc::clone(&next_nodes),
                            actions: Rc::clone(&ref_actions),
                            connected: false,
                            grbl: grbl::new(),
                            grbl_status: None,
                            recipie_regex: Regex::new(r"^[^.]+").unwrap(),
                            recipie_state: Arc::clone(&recipie_state),
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
                    Message::TabBar(TabBarMessage::Manual) => state.state = TabState::Manual,
                    Message::TabBar(TabBarMessage::Build) => state.state = TabState::Build,
                    Message::TabBar(TabBarMessage::Run) => {
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
                        {
                            let (recipie_state, cvar) = &*state.recipie_state;
                            let mut recipie_state = recipie_state.lock().unwrap();
                            *recipie_state = RecipieState::Stopped;
                            cvar.notify_all();
                        }
                        state.grbl.push_command(Cmd::new("\u{85}".to_string()));
                        set_pause_node(
                            Arc::clone(&state.current_node),
                            Arc::clone(&state.next_nodes),
                            state.grbl.clone(),
                        );
                    }
                    Message::Manual(ManualMessage::ButtonPressed(node)) => {
                        let (recipie_state, _) = &*state.recipie_state;
                        let mut recipie_state = recipie_state.lock().unwrap();
                        *recipie_state = RecipieState::ManualRunning;
                        state.connected = true;
                        command = Command::perform(
                            State::run_recipie(
                                state.grbl.clone(),
                                Arc::clone(&state.recipie_state),
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
                            Message::RecipieDone,
                        )
                    }
                    Message::Run(RunMessage::Run) => {
                        let rs: RecipieState;
                        {
                            let (recipie_state, _) = &*state.recipie_state;
                            rs = *recipie_state.lock().unwrap();
                        }
                        if discriminant(&rs) == discriminant(&RecipieState::Stopped) {
                            {
                                let (recipie_state, cvar) = &*state.recipie_state;
                                let mut recipie_state = recipie_state.lock().unwrap();
                                *recipie_state = RecipieState::RecipieRunning;
                                cvar.notify_all();
                            }
                            state.connected = true;
                            command = Command::perform(
                                State::run_recipie(
                                    state.grbl.clone(),
                                    Arc::clone(&state.recipie_state),
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
                    Message::Run(RunMessage::Pause) => {
                        let (recipie_state, cvar) = &*state.recipie_state;
                        let mut recipie_state = recipie_state.lock().unwrap();
                        *recipie_state = RecipieState::RecipiePaused;
                        cvar.notify_all();
                        state.grbl.push_command(Cmd::new("\u{85}".to_string()));
                    }
                    Message::Run(RunMessage::Resume) => {
                        let (recipie_state, cvar) = &*state.recipie_state;
                        let mut recipie_state = recipie_state.lock().unwrap();
                        *recipie_state = RecipieState::RecipieRunning;
                        cvar.notify_all();
                    }
                    Message::Run(RunMessage::Stop) => {
                        {
                            let (recipie_state, cvar) = &*state.recipie_state;
                            let mut recipie_state = recipie_state.lock().unwrap();
                            *recipie_state = RecipieState::Stopped;
                            cvar.notify_all();
                        }
                        state.grbl.push_command(Cmd::new("\u{85}".to_string()));
                        set_pause_node(
                            Arc::clone(&state.current_node),
                            Arc::clone(&state.next_nodes),
                            state.grbl.clone(),
                        );
                    }
                    Message::Manual(msg) => state.tabs.manual.update(msg),
                    Message::Build(msg) => state.tabs.build.update(msg),
                    Message::Run(msg) => state.tabs.run.update(msg),
                    Message::RecipieDone(Ok(_)) => {
                        state.grbl_status = Some(Arc::clone(&state.grbl.mutex_status));
                        {
                            let (recipie_state, cvar) = &*state.recipie_state;
                            let mut recipie_state = recipie_state.lock().unwrap();
                            *recipie_state = RecipieState::Stopped;
                            cvar.notify_all();
                        }
                        state.connected = true;
                    }
                    Message::RecipieDone(Err(_err)) => {
                        {
                            let (recipie_state, cvar) = &*state.recipie_state;
                            let mut recipie_state = recipie_state.lock().unwrap();
                            *recipie_state = RecipieState::Stopped;
                            cvar.notify_all();
                        }
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
                tab_bar,
                recipie_state,
                ..
            }) => match state {
                TabState::Manual => {
                    let content =
                        Column::new().push(tab_bar.view().map(move |msg| Message::TabBar(msg)));
                    let rs: RecipieState;
                    {
                        let (recipie_state, _) = &**recipie_state;
                        rs = *recipie_state.lock().unwrap();
                    }
                    if discriminant(&rs) == discriminant(&RecipieState::RecipieRunning)
                        || discriminant(&rs) == discriminant(&RecipieState::RecipiePaused)
                    {
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
                TabState::Run => {
                    let content =
                        Column::new().push(tab_bar.view().map(move |msg| Message::TabBar(msg)));
                    let rs: RecipieState;
                    {
                        let (recipie_state, _) = &**recipie_state;
                        rs = *recipie_state.lock().unwrap();
                    }
                    if discriminant(&rs) == discriminant(&RecipieState::ManualRunning) {
                        content
                            .push(Space::with_height(Length::Units(100)))
                            .push(
                                Text::new("Unavailable while Manual control is active")
                                    .size(50)
                                    .font(CQ_MONO),
                            )
                            .align_items(Align::Center)
                            .into()
                    } else {
                        content
                            .push(tabs.run.view().map(move |msg| Message::Run(msg)))
                            .into()
                    }
                }
                TabState::Build => Column::new()
                    .push(tab_bar.view().map(move |msg| Message::TabBar(msg)))
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

// used by grbl control threads to see if they need to stop or wait for recipie to resume
// this function will block the thread if on pause, and return true if the thread should close
fn break_and_hold(recipie_state: Arc<(Mutex<RecipieState>, Condvar)>) -> bool {
    let mut stop = false;
    let (recipie_state, cvar) = &*recipie_state;
    let mut rs = recipie_state.lock().unwrap();
    while !stop {
        match *rs {
            RecipieState::Stopped => stop = true,
            RecipieState::RecipiePaused => rs = cvar.wait(rs).unwrap(),
            RecipieState::RequireInput => rs = cvar.wait(rs).unwrap(),
            _ => break,
        }
    }
    stop
}

fn set_pause_node(current_node: Arc<Mutex<Node>>, next_nodes: Arc<Mutex<Vec<Node>>>, grbl: Grbl) {
    let mut cn = current_node.lock().unwrap();
    let mut nn = next_nodes.lock().unwrap();
    match nn.first() {
        Some(nn) => {
            let s = grbl.get_status().unwrap();
            if cn.name != "paused_node" {
                *cn = Node {
                    name: "paused_node".to_string(),
                    x: s.x,
                    y: s.y,
                    z: s.z,
                    hide: true,
                    neighbors: vec![cn.name.clone(), nn.name.clone()],
                }
            }
        }
        None => (),
    }
    *nn = Vec::new();
}

const CQ_MONO: Font = Font::External {
    name: "CQ_MONO",
    bytes: include_bytes!("../fonts/CQ_MONO.otf"),
};
