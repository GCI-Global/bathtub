#![feature(total_cmp)]
mod actions;
mod advanced;
mod build;
mod grbl;
mod logger;
mod manual;
mod nodes;
mod paths;
mod run;
use actions::Actions;
use advanced::{Advanced, AdvancedMessage};
use build::{Build, BuildMessage};
use chrono::prelude::*;
use grbl::{Command as Cmd, Grbl, Status};
use logger::Logger;
use manual::{Manual, ManualMessage};
use nodes::{Node, NodeGrid2d, Nodes};
use regex::Regex;
use run::Step;
use run::{Run, RunMessage, RunState};
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
    logger: Logger,
    connected: bool,
    recipe_regex: Regex,
    grbl_status: Option<Arc<Mutex<Option<Status>>>>,
    recipe_state: Arc<(Mutex<RecipeState>, Condvar)>,
}

impl State {
    async fn run_recipie(
        grbl: Grbl,
        logger: Logger,
        recipe_state: Arc<(Mutex<RecipeState>, Condvar)>,
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
        let recipe_state2 = Arc::clone(&recipe_state);
        let logger2 = logger.clone();
        thread::spawn(move || {
            while !break_and_hold(Arc::clone(&recipe_state2)) {
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
                                logger2
                                    .send_line(format!(
                                        "{} => Arrived @{}",
                                        Local::now().to_rfc2822(),
                                        &*cn
                                    ))
                                    .unwrap();
                            }
                        }
                        None => {}
                    }
                }
                thread::sleep(Duration::from_millis(1))
            }
        });
        for step in recipie {
            grbl.clear_responses();
            let notify_user_input_recv = if step.wait {
                logger
                    .send_line(format!(
                        "{} => Step {}) Waiting for user input",
                        Local::now().to_rfc2822(),
                        step.step_num
                    ))
                    .unwrap();
                let (recipe_state, _) = &*recipe_state;
                let mut recipe_state = recipe_state.lock().unwrap();
                *recipe_state = RecipeState::RequireInput;
                true
            } else {
                false
            };
            if break_and_hold(Arc::clone(&recipe_state)) {
                logger
                    .send_line(format!("{} => Stopped By User", Local::now().to_rfc2822()))
                    .unwrap();
                break;
            }
            if notify_user_input_recv {
                logger
                    .send_line(format!(
                        "{} => Step {}) User input received. Continuing.",
                        Local::now().to_rfc2822(),
                        step.step_num
                    ))
                    .unwrap();
            }
            logger
                .send_line(format!(
                    "{} => Step {}) Going to {}",
                    Local::now().to_rfc2822(),
                    step.step_num,
                    step.selected_destination
                ))
                .unwrap();
            // gen paths and send
            let hover = match step.hover {
                true => "_hover",
                false => "",
            };
            while !break_and_hold(Arc::clone(&recipe_state)) {
                {
                    let cn = current_node.lock().unwrap();
                    let mut nn = next_nodes.lock().unwrap();
                    let future_node = &nodes.node[match node_map
                        .get(&format!("{}{}", step.selected_destination, hover))
                    {
                        Some(n) => n,
                        _ => break,
                    }
                    .clone()];
                    logger
                        .send_line(format!(
                            "{} => Step {}) From {}:({},{},{}) to {}:({},{},{})",
                            Local::now().to_rfc2822(),
                            step.step_num,
                            cn.name,
                            cn.x,
                            cn.y,
                            cn.z,
                            future_node.name,
                            future_node.x,
                            future_node.y,
                            future_node.z,
                        ))
                        .unwrap();
                    let node_paths = paths::gen_node_paths(&nodes, &cn, future_node);
                    logger
                        .send_line(format!(
                            "{} => Step {}) on path {}",
                            Local::now().to_rfc2822(),
                            step.step_num,
                            node_paths.node.iter().enumerate().fold(
                                String::new(),
                                |mut s, (i, n)| {
                                    s.push_str(&n.name[..]);
                                    if i + 1 != node_paths.node.len() {
                                        s.push_str(" > ");
                                    }
                                    s
                                }
                            ),
                        ))
                        .unwrap();
                    for node in node_paths.node {
                        nn.push(node.clone());
                        logger
                            .send_line(format!(
                                "{} => Step {}) Sending pathing G-code '{}'",
                                Local::now().to_rfc2822(),
                                step.step_num,
                                format!("$J=X{} Y{} Z{} F250", node.x, node.y, node.x),
                            ))
                            .unwrap();
                        grbl.push_command(Cmd::new(format!(
                            "$J=X{} Y{} Z{} F250",
                            node.x, node.y, node.z
                        )));
                    }
                }
                while (*next_nodes.lock().unwrap()).len() != 0 {
                    let (recipe_state, _) = &*recipe_state;
                    match *recipe_state.lock().unwrap() {
                        RecipeState::Stopped => break,
                        RecipeState::RecipePaused => {
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
            if break_and_hold(Arc::clone(&recipe_state)) {
                logger
                    .send_line(format!("{} => Stopped By User", Local::now().to_rfc2822()))
                    .unwrap();
                break;
            }
            let (tx, rx) = mpsc::channel();
            let step_c = step.clone();
            let recipe_state3 = Arc::clone(&recipe_state);
            logger
                .send_line(format!(
                    "{} => Step {}) starting {}",
                    Local::now().to_rfc2822(),
                    step.step_num,
                    step.selected_action
                ))
                .unwrap();
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
                while !break_and_hold(Arc::clone(&recipe_state3)) {
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
            if break_and_hold(Arc::clone(&recipe_state)) {
                logger
                    .send_line(format!("{} => Stopped By User", Local::now().to_rfc2822()))
                    .unwrap();
                break;
            }
            let action_commands = action_map.get(&step.selected_action).unwrap();
            for command in action_commands {
                if break_and_hold(Arc::clone(&recipe_state)) {
                    logger
                        .send_line(format!("{} => Stopped By User", Local::now().to_rfc2822()))
                        .unwrap();
                    break;
                }
                if command != &"WAIT".to_string() {
                    grbl.push_command(Cmd::new(command.clone()));
                } else {
                    contains_wait = true
                }
            }
            loop {
                if break_and_hold(Arc::clone(&recipe_state)) {
                    logger
                        .send_line(format!("{} => Stopped By User", Local::now().to_rfc2822()))
                        .unwrap();
                    break;
                }
                if rx.try_recv() == Ok("Stop") {
                    grbl.push_command(Cmd::new("\u{85}".to_string()));
                    logger
                        .send_line(format!(
                            "{} => Step {}) finished {}",
                            Local::now().to_rfc2822(),
                            step.step_num,
                            step.selected_action
                        ))
                        .unwrap();
                    break;
                } else if !contains_wait {
                    for response in grbl.clear_responses() {
                        logger
                            .send_line(format!(
                                "{} => Step {}) G-code '{}' responded '{}'",
                                response.response_time.unwrap().to_rfc2822(),
                                step.step_num,
                                response.command,
                                response.result.unwrap()
                            ))
                            .unwrap();
                    }
                    grbl.clear_responses();
                    if grbl.queue_len() < action_commands.len() {
                        for command in action_commands {
                            if break_and_hold(Arc::clone(&recipe_state)) {
                                logger
                                    .send_line(format!(
                                        "{} => Stopped By User",
                                        Local::now().to_rfc2822()
                                    ))
                                    .unwrap();
                                break;
                            }
                            logger
                                .send_line(format!(
                                    "{} => Step {}) Sending action G-code '{}'",
                                    Local::now().to_rfc2822(),
                                    step.step_num,
                                    command.clone()
                                ))
                                .unwrap();
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
pub enum RecipeState {
    Stopped,
    ManualRunning,
    RecipeRunning,
    RecipePaused,
    RequireInput,
}

struct Tabs {
    manual: Manual,
    run: Run,
    build: Build,
    advanced: Advanced,
}

struct TabBar {
    manual_btn: button::State,
    build_btn: button::State,
    run_btn: button::State,
    advanced_btn: button::State,
}

#[derive(Debug, Clone)]
enum TabBarMessage {
    Manual,
    Run,
    Build,
    Advanced,
}

impl TabBar {
    fn new() -> Self {
        TabBar {
            manual_btn: button::State::new(),
            run_btn: button::State::new(),
            build_btn: button::State::new(),
            advanced_btn: button::State::new(),
        }
    }

    fn view(&mut self) -> Element<TabBarMessage> {
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
            )
            .push(
                Button::new(
                    &mut self.advanced_btn,
                    Text::new("Advanced")
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .size(30)
                        .font(CQ_MONO),
                )
                .width(Length::Fill)
                .padding(20)
                .on_press(TabBarMessage::Advanced),
            )
            .into()
    }
}

enum TabState {
    Manual,
    Run,
    Build,
    Advanced,
}

#[derive(Debug, Clone)]
struct LoadState {
    nodes: Nodes,
    node_map: HashMap<String, usize>,
    actions: Actions,
}

#[derive(Debug, Clone)]
enum LoadError {
    Nodes,
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
    Advanced(AdvancedMessage),
    Loaded(Result<LoadState, LoadError>),
    Tick,
}

impl<'a> Application for Bathtub {
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
                        let recipe_state =
                            Arc::new((Mutex::new(RecipeState::Stopped), Condvar::new()));
                        let current_node = Arc::new(Mutex::new(
                            state.nodes.node
                                [state.node_map.get(&"HOME".to_string()).unwrap().clone()]
                            .clone(),
                        ));
                        let next_nodes = Arc::new(Mutex::new(Vec::new()));
                        let grbl = grbl::new();
                        let logger = Logger::new();
                        *self = Bathtub::Loaded(State {
                            //status: "Click any button\nto start homing cycle".to_string(),
                            state: TabState::Manual,
                            tabs: Tabs {
                                manual: Manual::new(Rc::clone(&ref_node), grbl.clone()),
                                run: Run::new(Arc::clone(&recipe_state), logger.clone()),
                                build: Build::new(
                                    Rc::clone(&ref_node),
                                    Rc::clone(&ref_actions),
                                    logger.clone(),
                                ),
                                advanced: Advanced::new(
                                    grbl.clone(),
                                    logger.clone(),
                                    Rc::clone(&ref_node),
                                    Rc::clone(&ref_actions),
                                ),
                            },
                            tab_bar: TabBar::new(),
                            nodes: Rc::clone(&ref_node),
                            node_map: state.node_map.clone(),
                            prev_node: Arc::new(Mutex::new(None)),
                            current_node: Arc::clone(&current_node),
                            next_nodes: Arc::clone(&next_nodes),
                            actions: Rc::clone(&ref_actions),
                            connected: false,
                            grbl: grbl.clone(),
                            logger: logger.clone(),
                            grbl_status: None,
                            recipe_regex: Regex::new(r"^[^.]+").unwrap(),
                            recipe_state: Arc::clone(&recipe_state),
                        });
                    }
                    Message::Loaded(Err(_)) => {
                        panic!("somehow loaded had an error")
                        // need to figure out how to notify user of errors
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
                    Message::TabBar(TabBarMessage::Advanced) => {
                        state.state = TabState::Advanced;
                    }
                    Message::TabBar(TabBarMessage::Run) => {
                        state.tabs.run.search =
                            fs::read_dir("./recipes")
                                .unwrap()
                                .fold(Vec::new(), |mut rec, file| {
                                    if let Some(caps) = state
                                        .recipe_regex
                                        .captures(&file.unwrap().file_name().to_str().unwrap())
                                    {
                                        rec.push(caps[0].to_string());
                                    }
                                    rec
                                });
                        state.tabs.run.search.sort();
                        state.tabs.run.update(RunMessage::TabActive);
                        state.state = TabState::Run
                    }
                    Message::Manual(ManualMessage::Stop) => {
                        {
                            let (recipe_state, cvar) = &*state.recipe_state;
                            let mut recipe_state = recipe_state.lock().unwrap();
                            *recipe_state = RecipeState::Stopped;
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
                        let (recipe_state, _) = &*state.recipe_state;
                        let mut recipe_state = recipe_state.lock().unwrap();
                        *recipe_state = RecipeState::ManualRunning;
                        state.connected = true;
                        let log_title =
                            format!("{}| Manual - Going to {}", Local::now().to_rfc2822(), &node);
                        state.logger.set_log_file(log_title.clone());
                        state.tabs.advanced.update_logs();

                        command = Command::perform(
                            State::run_recipie(
                                state.grbl.clone(),
                                state.logger.clone(),
                                Arc::clone(&state.recipe_state),
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
                                    hover: state.tabs.manual.hover,
                                    wait: false,
                                }],
                                state.node_map.clone(),
                                state.nodes.borrow().clone(),
                                state.actions.borrow().clone(),
                            ),
                            Message::RecipieDone,
                        )
                    }
                    Message::Run(RunMessage::Run(_)) => {
                        let rs: RecipeState;
                        {
                            let (recipe_state, _) = &*state.recipe_state;
                            rs = *recipe_state.lock().unwrap();
                        }
                        if discriminant(&rs) == discriminant(&RecipeState::Stopped) {
                            {
                                let (recipe_state, cvar) = &*state.recipe_state;
                                let mut recipe_state = recipe_state.lock().unwrap();
                                *recipe_state = RecipeState::RecipeRunning;
                                cvar.notify_all();
                            }
                            state.connected = true;
                            // we only update the list of logs on load, and when we create a new
                            // log file
                            state.tabs.advanced.update_logs();
                            command = Command::perform(
                                State::run_recipie(
                                    state.grbl.clone(),
                                    state.logger.clone(),
                                    Arc::clone(&state.recipe_state),
                                    Arc::clone(&state.prev_node),
                                    Arc::clone(&state.current_node),
                                    Arc::clone(&state.next_nodes),
                                    state.tabs.run.recipe.as_ref().unwrap().steps.clone(),
                                    state.node_map.clone(),
                                    state.nodes.borrow().clone(),
                                    state.actions.borrow().clone(),
                                ),
                                Message::RecipieDone,
                            );
                        }
                    }
                    Message::Run(RunMessage::Pause(_)) => {
                        state
                            .logger
                            .send_line(format!("{} => Paused by user", Local::now().to_rfc2822()))
                            .unwrap();
                        let (recipe_state, cvar) = &*state.recipe_state;
                        let mut recipe_state = recipe_state.lock().unwrap();
                        *recipe_state = RecipeState::RecipePaused;
                        cvar.notify_all();
                        state.grbl.push_command(Cmd::new("\u{85}".to_string()));
                    }
                    Message::Run(RunMessage::Resume) => {
                        state
                            .logger
                            .send_line(format!("{} => Resumed by user", Local::now().to_rfc2822()))
                            .unwrap();
                        let (recipe_state, cvar) = &*state.recipe_state;
                        let mut recipe_state = recipe_state.lock().unwrap();
                        *recipe_state = RecipeState::RecipeRunning;
                        cvar.notify_all();
                    }
                    Message::Run(RunMessage::Stop) => {
                        {
                            let (recipe_state, cvar) = &*state.recipe_state;
                            let mut recipe_state = recipe_state.lock().unwrap();
                            *recipe_state = RecipeState::Stopped;
                            cvar.notify_all();
                        }
                        state.grbl.push_command(Cmd::new("\u{85}".to_string()));
                        set_pause_node(
                            Arc::clone(&state.current_node),
                            Arc::clone(&state.next_nodes),
                            state.grbl.clone(),
                        );
                        state.tabs.run.state = RunState::AfterRequiredInput;
                    }
                    Message::RecipieDone(Ok(_)) => {
                        state
                            .logger
                            .send_line(format!("{} => Done", Local::now().to_rfc2822()))
                            .unwrap();
                        state.grbl_status = Some(Arc::clone(&state.grbl.mutex_status));
                        {
                            let (recipe_state, cvar) = &*state.recipe_state;
                            let mut recipe_state = recipe_state.lock().unwrap();
                            *recipe_state = RecipeState::Stopped;
                            cvar.notify_all();
                        }
                        state.tabs.run.state = RunState::AfterRequiredInput;
                        state.connected = true;
                    }
                    Message::RecipieDone(Err(_err)) => {
                        {
                            let (recipe_state, cvar) = &*state.recipe_state;
                            let mut recipe_state = recipe_state.lock().unwrap();
                            *recipe_state = RecipeState::Stopped;
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
                    Message::Manual(msg) => {
                        command = state
                            .tabs
                            .manual
                            .update(msg)
                            .map(move |msg| Message::Manual(msg))
                    }
                    Message::Build(BuildMessage::Saved(_)) => state.tabs.advanced.update_logs(),
                    Message::Build(msg) => {
                        command = state
                            .tabs
                            .build
                            .update(msg)
                            .map(move |msg| Message::Build(msg))
                    }
                    Message::Run(msg) => {
                        command = state.tabs.run.update(msg).map(move |msg| Message::Run(msg));
                    }
                    Message::Advanced(msg) => {
                        command = state
                            .tabs
                            .advanced
                            .update(msg)
                            .map(move |msg| Message::Advanced(msg));
                    }
                    _ => {}
                }
                command
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        match self {
            Bathtub::Loading => loading_message("Loading . . ."),
            Bathtub::Loaded(State {
                state,
                tabs,
                tab_bar,
                recipe_state,
                ..
            }) => match state {
                TabState::Manual => {
                    let content =
                        Column::new().push(tab_bar.view().map(move |msg| Message::TabBar(msg)));
                    let rs: RecipeState;
                    {
                        let (recipe_state, _) = &**recipe_state;
                        rs = *recipe_state.lock().unwrap();
                    }
                    if discriminant(&rs) == discriminant(&RecipeState::RecipeRunning)
                        || discriminant(&rs) == discriminant(&RecipeState::RecipePaused)
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
                    let rs: RecipeState;
                    {
                        let (recipe_state, _) = &**recipe_state;
                        rs = *recipe_state.lock().unwrap();
                    }
                    if discriminant(&rs) == discriminant(&RecipeState::ManualRunning) {
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
                TabState::Advanced => {
                    let content =
                        Column::new().push(tab_bar.view().map(move |msg| Message::TabBar(msg)));
                    let rs: RecipeState;
                    {
                        let (recipe_state, _) = &**recipe_state;
                        rs = *recipe_state.lock().unwrap();
                    }
                    if discriminant(&rs) != discriminant(&RecipeState::Stopped) {
                        content
                            .push(Space::with_height(Length::Units(100)))
                            .push(
                                Text::new("Unavailable while GRBL in motion")
                                    .size(50)
                                    .font(CQ_MONO),
                            )
                            .align_items(Align::Center)
                            .into()
                    } else {
                        content
                            .push(tabs.advanced.view().map(move |msg| Message::Advanced(msg)))
                            .into()
                    }
                }
            },
        }
    }
}

impl LoadState {
    fn new(nodes: Nodes, node_map: HashMap<String, usize>, actions: Actions) -> LoadState {
        LoadState {
            nodes,
            node_map,
            actions,
        }
    }

    // This is just a placeholder. Will eventually read data from server
    async fn load() -> Result<LoadState, LoadError> {
        // try to read file 3 times before returning error
        let mut nodes = Nodes { node: Vec::new() };
        for i in 0..3 {
            match nodes::gen_nodes() {
                Ok(n) => {
                    nodes = n;
                    break;
                }
                Err(_err) if i == 2 => return Err(LoadError::Nodes),
                Err(_) => thread::sleep(Duration::from_millis(50)),
            };
        }
        Ok(LoadState::new(
            nodes.clone(),
            nodes::get_nodemap(nodes.clone()),
            actions::gen_actions(),
        ))
    }
}

fn loading_message<'a>(msg: &str) -> Element<'a, Message> {
    Container::new(
        Text::new(msg)
            .horizontal_alignment(HorizontalAlignment::Center)
            .size(50),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_y()
    .center_x()
    .into()
}

// used by grbl control threads to see if they need to stop or wait for recipie to resume
// this function will block the thread if on pause, and return true if the thread should close
fn break_and_hold(recipe_state: Arc<(Mutex<RecipeState>, Condvar)>) -> bool {
    let mut stop = false;
    let (recipe_state, cvar) = &*recipe_state;
    let mut rs = recipe_state.lock().unwrap();
    while !stop {
        match *rs {
            RecipeState::Stopped => stop = true,
            RecipeState::RecipePaused => rs = cvar.wait(rs).unwrap(),
            RecipeState::RequireInput => rs = cvar.wait(rs).unwrap(),
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
