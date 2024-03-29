use super::actions::{Action, Actions};
use super::logger::Logger;
use super::nodes::{get_nodemap, Node, Nodes};
use super::run::do_nothing;
use super::style::style::Theme;
use crate::{NodeTracker, TabState as ParentTabState, CQ_MONO};
use iced::{
    button, pick_list, scrollable, text_input, tooltip, Align, Button, Checkbox, Column, Command,
    Container, Element, HorizontalAlignment, Length, PickList, Row, Scrollable, Space, Text,
    TextInput, Tooltip, VerticalAlignment,
};

use super::build::{delete_icon, down_icon, okay_icon, right_icon};
use super::grbl::{Command as Cmd, Grbl};
use chrono::prelude::*;
use chrono::DateTime;
use regex::Regex;
use std::cell::RefCell;
use std::cmp::min;
use std::collections::HashMap;
use std::fs;
use std::mem::discriminant;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub const LOGS: &str = "./logs";
pub const LOG_MAX: usize = 100; // max number of logs to show

pub struct Advanced {
    scroll: scrollable::State,
    state: TabState,
    tab_bar: TabBar,
    pub grbl_tab: GrblTab,
    nodes_tab: NodeTab,
    actions_tab: ActionTab,
    logs_tab: LogTab,
    parent_unsaved_tabs: Rc<RefCell<HashMap<ParentTabState, bool>>>,
    unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
}

#[derive(Clone, Debug, Hash)]
enum TabState {
    Grbl,
    Nodes,
    Actions,
    Logs,
}
impl PartialEq for TabState {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}
impl Eq for TabState {}

#[derive(Debug, Clone)]
pub enum AdvancedMessage {
    TabBar(TabBarMessage),
    GrblTab(GrblMessage),
    NodesTab(NodeTabMessage),
    ActionsTab(ActionTabMessage),
    LogsTab(LogTabMessage),
}

impl Advanced {
    pub fn update_logs(&mut self) {
        self.logs_tab.update_logs();
    }
    pub fn new(
        grbl: Grbl,
        logger: Logger,
        ref_nodes: Rc<RefCell<Nodes>>,
        ref_actions: Rc<RefCell<Actions>>,
        parent_unsaved_tabs: Rc<RefCell<HashMap<ParentTabState, bool>>>,
        node_map: Rc<RefCell<HashMap<String, usize>>>,
        homing_required: Rc<RefCell<bool>>,
        node_tracker: Arc<Mutex<NodeTracker>>,
    ) -> Self {
        let mut unsaved_tabs_local = HashMap::with_capacity(3);
        unsaved_tabs_local.insert(TabState::Nodes, false);
        unsaved_tabs_local.insert(TabState::Actions, false);
        unsaved_tabs_local.insert(TabState::Grbl, false);
        let unsaved_tabs = Rc::new(RefCell::new(unsaved_tabs_local));
        Advanced {
            scroll: scrollable::State::new(),
            state: TabState::Logs,
            tab_bar: TabBar::new(unsaved_tabs.clone()),
            grbl_tab: GrblTab::new(grbl, Vec::new(), logger.clone(), unsaved_tabs.clone()),
            nodes_tab: NodeTab::new(
                ref_nodes,
                logger.clone(),
                unsaved_tabs.clone(),
                node_map,
                homing_required,
                node_tracker,
            ),
            actions_tab: ActionTab::new(ref_actions, logger, unsaved_tabs.clone()),
            logs_tab: LogTab::new(),
            parent_unsaved_tabs,
            unsaved_tabs,
        }
    }

    pub fn update(&mut self, message: AdvancedMessage) -> Command<AdvancedMessage> {
        let mut command = Command::none();
        match message {
            AdvancedMessage::TabBar(TabBarMessage::Grbl) => {
                if !self.grbl_tab.unsaved {
                    self.grbl_tab.grbl.push_command(Cmd::new("$I".to_string()));
                    loop {
                        if let Some(num) = self.grbl_tab.grbl.queue_len() {
                            if num < 2 {
                                self.grbl_tab.grbl.push_command(Cmd::new("$I".to_string()));
                            }
                        }
                        if let Some(cmd) = self.grbl_tab.grbl.safe_pop() {
                            if cmd.command == "$I".to_string() {
                                self.grbl_tab.grbl.clear_all();
                                let r = Regex::new(r"[0-9]*\.+[0-9]*[a-z]*").unwrap();
                                let r2 = Regex::new(r"[0-9]{8}").unwrap();
                                if let Some(caps) = r.captures(&(cmd.result.as_ref().unwrap()[..]))
                                {
                                    self.grbl_tab.version = Some(caps[0].to_string());
                                }
                                if let Some(caps2) =
                                    r2.captures(&(cmd.result.as_ref().unwrap()[..]))
                                {
                                    self.grbl_tab.version_release_date = Some(date(&caps2[0]));
                                }

                                break;
                            }
                        }
                    }
                    self.grbl_tab.grbl.push_command(Cmd::new("$$".to_string()));
                    loop {
                        if let Some(num) = self.grbl_tab.grbl.queue_len() {
                            if num < 2 {
                                self.grbl_tab.grbl.push_command(Cmd::new("$$".to_string()));
                            }
                        }
                        if let Some(cmd) = self.grbl_tab.grbl.safe_pop() {
                            if cmd.command == "$$".to_string() {
                                self.grbl_tab.grbl.clear_all();
                                self.grbl_tab.modified_settings = cmd.result.unwrap().lines().fold(
                                    Vec::new(),
                                    |mut v, response| {
                                        let r: Vec<&str> = response.split("=").collect();
                                        if r.len() > 1 {
                                            v.push(GrblSetting::new(
                                                format!("{}", r[0]),
                                                r[1].to_string(),
                                            ));
                                        }
                                        v
                                    },
                                );
                                break;
                            }
                        }
                    }
                }
                self.update_logs();
                if self.grbl_tab.settings.len() == 0 {
                    self.grbl_tab.settings = self.grbl_tab.modified_settings.clone()
                }
                self.state = TabState::Grbl;
                self.tab_bar.change_state(TabState::Grbl)
            }
            AdvancedMessage::GrblTab(GrblMessage::SaveMessage(SaveBarMessage::Cancel)) => {
                self.grbl_tab.save_bar.message = "Unsaved Changes!".to_string();
                self.grbl_tab.modified_settings = self.grbl_tab.settings.clone();
                self.grbl_tab
                    .unsaved_tabs
                    .borrow_mut()
                    .insert(TabState::Grbl, false);
                self.grbl_tab.unsaved = false;
            }
            AdvancedMessage::TabBar(TabBarMessage::Nodes) => {
                self.state = TabState::Nodes;
                self.tab_bar.change_state(TabState::Nodes)
            }
            AdvancedMessage::TabBar(TabBarMessage::Actions) => {
                self.state = TabState::Actions;
                self.tab_bar.change_state(TabState::Actions);
            }
            AdvancedMessage::TabBar(TabBarMessage::Logs) => {
                self.state = TabState::Logs;
                self.tab_bar.change_state(TabState::Logs)
            }
            AdvancedMessage::GrblTab(msg) => {
                self.grbl_tab.update(msg);
            }
            AdvancedMessage::NodesTab(NodeTabMessage::AddConfigNode) => {
                command = self
                    .nodes_tab
                    .update(NodeTabMessage::AddConfigNode)
                    .map(move |msg| AdvancedMessage::NodesTab(msg));
                self.scroll.scroll_to_bottom();
            }
            AdvancedMessage::NodesTab(NodeTabMessage::Saved(_)) => self.update_logs(),
            AdvancedMessage::NodesTab(msg) => {
                command = self
                    .nodes_tab
                    .update(msg)
                    .map(move |msg| AdvancedMessage::NodesTab(msg))
            }
            AdvancedMessage::ActionsTab(ActionTabMessage::Saved(_)) => self.update_logs(),
            AdvancedMessage::ActionsTab(msg) => {
                command = self
                    .actions_tab
                    .update(msg)
                    .map(move |msg| AdvancedMessage::ActionsTab(msg))
            }
            AdvancedMessage::LogsTab(msg) => {
                command = self
                    .logs_tab
                    .update(msg)
                    .map(move |msg| AdvancedMessage::LogsTab(msg))
            }
        };
        self.parent_unsaved_tabs.borrow_mut().insert(
            ParentTabState::Advanced,
            self.unsaved_tabs.borrow().values().any(|v| *v),
        );
        command
    }

    pub fn view(&mut self) -> Element<AdvancedMessage> {
        let tab_bar = self
            .tab_bar
            .view()
            .map(move |msg| AdvancedMessage::TabBar(msg));
        let content = match self.state {
            TabState::Grbl => self
                .grbl_tab
                .view()
                .map(move |msg| AdvancedMessage::GrblTab(msg)),
            TabState::Nodes => self
                .nodes_tab
                .view()
                .map(move |msg| AdvancedMessage::NodesTab(msg)),
            TabState::Actions => self
                .actions_tab
                .view()
                .map(move |msg| AdvancedMessage::ActionsTab(msg)),
            TabState::Logs => self
                .logs_tab
                .view()
                .map(move |msg| AdvancedMessage::LogsTab(msg)),
        };
        let scrollable = Scrollable::new(&mut self.scroll)
            .push(Container::new(content).width(Length::Fill).center_x())
            .padding(40);
        Row::new().push(tab_bar).push(scrollable).into()
    }
}

struct TabBar {
    grbl_btn: button::State,
    nodes_btn: button::State,
    actions_btn: button::State,
    logs_btn: button::State,
    current_tab: TabState,
    unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
}

#[derive(Debug, Clone)]
pub enum TabBarMessage {
    Grbl,
    Nodes,
    Actions,
    Logs,
}

impl TabBar {
    fn new(unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>) -> Self {
        TabBar {
            grbl_btn: button::State::new(),
            nodes_btn: button::State::new(),
            actions_btn: button::State::new(),
            logs_btn: button::State::new(),
            current_tab: TabState::Logs,
            unsaved_tabs,
        }
    }

    fn change_state(&mut self, tab_state: TabState) {
        self.current_tab = tab_state;
    }

    fn view(&mut self) -> Element<TabBarMessage> {
        Column::new()
            .height(Length::Fill)
            .width(Length::Shrink)
            .push(
                Button::new(
                    &mut self.logs_btn,
                    Text::new("Logs")
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center)
                        .size(30)
                        .font(CQ_MONO),
                )
                .style(match self.current_tab {
                    TabState::Logs => Theme::TabSelected,
                    _ => Theme::Blue,
                })
                .height(Length::Fill)
                .width(Length::Units(200))
                .padding(20)
                .on_press(TabBarMessage::Logs),
            )
            .push(
                Button::new(
                    &mut self.nodes_btn,
                    Text::new("Nodes")
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center)
                        .size(30)
                        .font(CQ_MONO),
                )
                .style(match self.current_tab {
                    TabState::Nodes => {
                        if *self.unsaved_tabs.borrow().get(&TabState::Nodes).unwrap() {
                            Theme::YellowSelected
                        } else {
                            Theme::TabSelected
                        }
                    }
                    _ => {
                        if *self.unsaved_tabs.borrow().get(&TabState::Nodes).unwrap() {
                            Theme::Yellow
                        } else {
                            Theme::Blue
                        }
                    }
                })
                .height(Length::Fill)
                .width(Length::Units(200))
                .padding(20)
                .on_press(TabBarMessage::Nodes),
            )
            .push(
                Button::new(
                    &mut self.actions_btn,
                    Text::new("Actions")
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center)
                        .size(30)
                        .font(CQ_MONO),
                )
                .style(match self.current_tab {
                    TabState::Actions => {
                        if *self.unsaved_tabs.borrow().get(&TabState::Actions).unwrap() {
                            Theme::YellowSelected
                        } else {
                            Theme::TabSelected
                        }
                    }
                    _ => {
                        if *self.unsaved_tabs.borrow().get(&TabState::Actions).unwrap() {
                            Theme::Yellow
                        } else {
                            Theme::Blue
                        }
                    }
                })
                .height(Length::Fill)
                .width(Length::Units(200))
                .padding(20)
                .on_press(TabBarMessage::Actions),
            )
            .push(
                Button::new(
                    &mut self.grbl_btn,
                    Text::new("GRBL")
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center)
                        .size(30)
                        .font(CQ_MONO),
                )
                .style(match self.current_tab {
                    TabState::Grbl => {
                        if *self.unsaved_tabs.borrow().get(&TabState::Grbl).unwrap() {
                            Theme::YellowSelected
                        } else {
                            Theme::TabSelected
                        }
                    }
                    _ => {
                        if *self.unsaved_tabs.borrow().get(&TabState::Grbl).unwrap() {
                            Theme::Yellow
                        } else {
                            Theme::Blue
                        }
                    }
                })
                .height(Length::Fill)
                .width(Length::Units(200))
                .padding(20)
                .on_press(TabBarMessage::Grbl),
            )
            .into()
    }
}

pub struct SaveBar {
    pub message: String,
    save_text: String,
    save_btn: button::State,
    cancel_btn: button::State,
}

#[derive(Debug, Clone)]
pub enum SaveBarMessage {
    Save,
    Cancel,
}

impl SaveBar {
    pub fn new_as() -> Self {
        SaveBar {
            save_btn: button::State::new(),
            cancel_btn: button::State::new(),
            message: "Unsaved Changes!".to_string(),
            save_text: "Save as".to_string(),
        }
    }

    pub fn new() -> Self {
        SaveBar {
            save_btn: button::State::new(),
            cancel_btn: button::State::new(),
            message: "Unsaved Changes!".to_string(),
            save_text: "Save".to_string(),
        }
    }

    pub fn view(&mut self) -> Element<'_, SaveBarMessage> {
        Container::new(
            Row::with_children(vec![
                Space::with_width(Length::Units(50)).into(),
                Column::with_children(vec![
                    Space::with_height(Length::Fill).into(),
                    Text::new(&self.message)
                        .vertical_alignment(VerticalAlignment::Center)
                        .size(22)
                        .font(CQ_MONO)
                        .into(),
                    Space::with_height(Length::Fill).into(),
                ])
                .into(),
                Space::with_width(Length::Fill).into(),
                Button::new(
                    &mut self.save_btn,
                    Text::new(&self.save_text)
                        .size(20)
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(Theme::Green)
                .padding(15)
                .on_press(SaveBarMessage::Save)
                .into(),
                Button::new(
                    &mut self.cancel_btn,
                    Text::new("Cancel")
                        .size(20)
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(Theme::Red)
                .padding(15)
                .on_press(SaveBarMessage::Cancel)
                .into(),
                Space::with_width(Length::Units(50)).into(),
            ])
            .height(Length::Units(50))
            .width(Length::Fill),
        )
        .style(if self.message[..] == *"Unsaved Changes!" {
            Theme::Yellow
        } else {
            Theme::Red
        })
        .into()
    }
}

pub struct GrblTab {
    save_bar: SaveBar,
    unsaved: bool,
    pub grbl: Grbl,
    logger: Logger,
    settings: Vec<GrblSetting>,
    modified_settings: Vec<GrblSetting>,
    version: Option<String>,
    version_release_date: Option<String>,
    unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
}

#[derive(Debug, Clone)]
struct GrblSetting {
    text: String,
    input_value: String,
    input_state: text_input::State,
}

#[derive(Debug, Clone)]
pub enum GrblMessage {
    SettingChanged(usize, GrblSettingMessage),
    SaveMessage(SaveBarMessage),
}

impl GrblTab {
    fn new(
        grbl: Grbl,
        settings: Vec<GrblSetting>,
        logger: Logger,
        unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
    ) -> Self {
        GrblTab {
            save_bar: SaveBar::new(),
            unsaved: false,
            grbl,
            settings: settings.clone(),
            modified_settings: settings,
            version: None,
            version_release_date: None,
            logger,
            unsaved_tabs,
        }
    }

    fn update(&mut self, message: GrblMessage) {
        match message {
            GrblMessage::SettingChanged(i, GrblSettingMessage::TextChanged(val)) => {
                self.unsaved = true;
                self.unsaved_tabs.borrow_mut().insert(TabState::Grbl, true);
                self.modified_settings[i].input_value = val
            }
            GrblMessage::SaveMessage(SaveBarMessage::Save) => {
                for setting in &self.modified_settings {
                    self.grbl.push_command(Cmd::new(format!(
                        "{}={}",
                        &setting.text, &setting.input_value
                    )));
                }
                let mut error = false;
                loop {
                    if let Some(cmd) = self.grbl.pop_command() {
                        if cmd
                            .result
                            .as_ref()
                            .unwrap_or(&String::new())
                            .contains("error")
                        {
                            error = true;
                            self.save_bar.message = format!(
                                "{} {}. Settings Reverted.",
                                cmd.command,
                                cmd.result.unwrap()
                            );
                            self.modified_settings = self.settings.clone();
                        }
                        if let Some(num) = self.grbl.recv_queue_len() {
                            if num == 0 {
                                break;
                            }
                        }
                    }
                }
                if error {
                    for setting in &self.settings {
                        self.grbl.push_command(Cmd::new(format!(
                            "{}={}",
                            &setting.text, &setting.input_value
                        )));
                    }
                    if let Some(final_cmd) = self.settings.last() {
                        loop {
                            if let Some(cmd) = self.grbl.pop_command() {
                                if cmd.command
                                    == format!("{}={}", final_cmd.text, final_cmd.input_value)
                                {
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    self.save_bar.message = "Unsaved Changes!".to_string();
                    self.settings = self.modified_settings.clone();
                    self.logger.set_log_file(format!(
                        "{}; Advanced (Grbl) - Save",
                        Local::now().to_rfc2822()
                    ));
                    self.logger.send_line(String::new()).unwrap();
                    self.logger
                        .send_line("Updated 'Grbl' from:".to_string())
                        .unwrap();
                    self.logger
                        .send_line(self.settings.iter().fold(String::new(), |mut s, setting| {
                            s.push_str(
                                &format!("{} = {}\n", setting.text, setting.input_value)[..],
                            );
                            s
                        }))
                        .unwrap();
                    self.logger
                        .send_line("\n\nUpdated 'Grbl' to:".to_string())
                        .unwrap();
                    self.logger
                        .send_line(self.modified_settings.iter().fold(
                            String::new(),
                            |mut s, setting| {
                                s.push_str(
                                    &format!("{} = {}\n", setting.text, setting.input_value)[..],
                                );
                                s
                            },
                        ))
                        .unwrap();
                    self.unsaved = false;
                    self.unsaved_tabs.borrow_mut().insert(TabState::Grbl, false);
                }
            }
            _ => {}
        }
    }

    fn view(&mut self) -> Element<'_, GrblMessage> {
        let content = match self.unsaved {
            true => Column::new().align_items(Align::Center).push(
                self.save_bar
                    .view()
                    .map(move |msg| GrblMessage::SaveMessage(msg)),
            ),
            false => Column::new()
                .align_items(Align::Center)
                .push(Space::with_height(Length::Units(50))),
        };
        content
            .push(
                Text::new(format!(
                    "Version: {}",
                    self.version
                        .as_ref()
                        .unwrap_or(&"** Unavailable **".to_string())
                ))
                .horizontal_alignment(HorizontalAlignment::Left)
                .font(CQ_MONO)
                .size(40)
                .width(Length::Units(505)),
            )
            .push(
                Text::new(
                    self.version_release_date
                        .as_ref()
                        .unwrap_or(&"** Unavailable **".to_string()),
                )
                .horizontal_alignment(HorizontalAlignment::Left)
                .size(20)
                .width(Length::Units(505)),
            )
            .push(
                Text::new("https://github.com/gnea/grbl/wiki")
                .horizontal_alignment(HorizontalAlignment::Left)
                .size(20)
                .width(Length::Units(505)),
            )
            .push(if self.modified_settings.len() > 0 {
                self.modified_settings.iter_mut().enumerate().fold(
                    Column::new(),
                    |col, (i, setting)| {
                        col.push(
                            setting
                                .view()
                                .map(move |msg| GrblMessage::SettingChanged(i, msg)),
                        )
                    },
                )
            } else {
                Column::new().push(
                Container::new(Row::with_children(vec![Space::with_width(Length::Fill).into(), Text::new("Error loading settings.\nPlease press 'Grbl tab button again to attempt reload.'").horizontal_alignment(HorizontalAlignment::Center).into(), Space::with_width(Length::Fill).into()]).padding(10)).style(Theme::Red))
            })
            .into()
    }
}

#[derive(Debug, Clone)]
pub enum GrblSettingMessage {
    TextChanged(String),
}

impl GrblSetting {
    fn new(text: String, input_value: String) -> Self {
        GrblSetting {
            text,
            input_value,
            input_state: text_input::State::new(),
        }
    }

    fn view(&mut self) -> Element<'_, GrblSettingMessage> {
        Row::new()
            .padding(5)
            .push(
                Column::new()
                    .push(Text::new(&self.text))
                    .padding(10)
                    .width(Length::Units(75)),
            )
            .push(
                Column::new()
                    .push(Text::new("="))
                    .padding(10)
                    .width(Length::Units(30)),
            )
            .push(
                TextInput::new(
                    &mut self.input_state,
                    "",
                    &self.input_value,
                    GrblSettingMessage::TextChanged,
                )
                .style(Theme::Blue)
                .padding(10)
                .width(Length::Units(400)),
            )
            .into()
    }
}

struct NodeTab {
    unsaved: bool,
    save_bar: SaveBar,
    ref_nodes: Rc<RefCell<Nodes>>,
    modified_nodes: Rc<RefCell<Nodes>>,
    config_nodes: Vec<ConfigNode>,
    add_config_node_btn: button::State,
    logger: Logger,
    unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
    node_map: Rc<RefCell<HashMap<String, usize>>>,
    homing_required: Rc<RefCell<bool>>,
    node_tracker: Arc<Mutex<NodeTracker>>,
}

#[derive(Debug, Clone)]
pub enum NodeTabMessage {
    AddConfigNode,
    ConfigNode((usize, ConfigNodeMessage)),
    SaveMessage(SaveBarMessage),
    Saved(()),
}

impl NodeTab {
    fn new(
        ref_nodes: Rc<RefCell<Nodes>>,
        logger: Logger,
        unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
        node_map: Rc<RefCell<HashMap<String, usize>>>,
        homing_required: Rc<RefCell<bool>>,
        node_tracker: Arc<Mutex<NodeTracker>>,
    ) -> Self {
        // for abstraction purposes, UI interaction is 2d, but data storage is 3d, this
        // nested iter if to flatten the 3d nodes
        let node_pairs = ref_nodes
            .borrow()
            .clone()
            .node
            .into_iter()
            .filter(|n| !n.name.contains("_hover") || n.hide)
            .zip(
                ref_nodes
                    .borrow()
                    .clone()
                    .node
                    .into_iter()
                    .filter(|n| n.name.contains("_hover") || n.hide),
            );
        let modified_nodes = Rc::new(RefCell::new(Nodes {
            node: node_pairs
                .into_iter()
                .fold(Vec::new(), |mut v, (n, n_hover)| {
                    v.push(Node {
                        neighbors: n_hover
                            .neighbors
                            .into_iter()
                            .map(|name| name.replace("_hover", ""))
                            .filter(|name| &n.name != name)
                            .collect(),
                        name: n.name,
                        x: n.x,
                        y: n.y,
                        z: n.z,
                        hide: n.hide,
                    });
                    v
                }),
        }));
        let config_nodes = Rc::clone(&modified_nodes)
            .borrow()
            .node
            .iter()
            .filter(|n| !n.name.contains("_hover"))
            .fold(Vec::new(), |mut v, n| {
                v.push(ConfigNode::new(
                    n.name.clone(),
                    n.hide,
                    n.x,
                    n.y,
                    n.z,
                    n.neighbors.clone(),
                    Rc::clone(&modified_nodes),
                ));
                v
            });
        NodeTab {
            unsaved: false,
            save_bar: SaveBar::new(),
            ref_nodes: Rc::clone(&ref_nodes),
            modified_nodes: Rc::clone(&modified_nodes),
            config_nodes,
            add_config_node_btn: button::State::new(),
            logger,
            unsaved_tabs,
            node_map,
            homing_required,
            node_tracker,
        }
    }

    fn update(&mut self, message: NodeTabMessage) -> Command<NodeTabMessage> {
        let mut command = Command::none();
        match message {
            NodeTabMessage::ConfigNode((i, ConfigNodeMessage::Okay)) => {
                self.save_bar.message = "Unsaved Changes!".to_string();
                self.config_nodes[i].update(ConfigNodeMessage::Okay);
            }
            NodeTabMessage::ConfigNode((i, ConfigNodeMessage::NameChanged(name))) => {
                if &self.config_nodes[i].name[..] == "HOME" {
                    self.config_nodes[i].errors.home_required = true;
                } else if self.config_nodes.iter().any(|n| n.name == name) {
                    self.config_nodes[i].errors.names_match = true;
                } else {
                    if name != String::new() {
                        self.config_nodes[i].errors.clear_title();
                    } else {
                        self.config_nodes[i].errors.name_blank = true;
                    }
                    self.unsaved = true;
                    self.unsaved_tabs.borrow_mut().insert(TabState::Nodes, true);
                    // If a name is changed update all other instances of the node name
                    let original_name = self.config_nodes[i].name.clone();
                    let index = self
                        .modified_nodes
                        .borrow()
                        .node
                        .iter()
                        .position(|n| n.name == original_name)
                        .unwrap();
                    self.modified_nodes.borrow_mut().node[index].name = name.clone();
                    for node in &mut self.config_nodes {
                        for pick_list in &mut node.neighbors_pick_lists {
                            if &pick_list.parent == &original_name {
                                pick_list.parent = name.clone();
                            }
                            if pick_list.value.as_ref().unwrap() == &original_name {
                                pick_list.value = Some(name.clone());
                            }
                        }
                    }
                    self.config_nodes[i].update(ConfigNodeMessage::NameChanged(name))
                }
            }
            NodeTabMessage::ConfigNode((i, ConfigNodeMessage::Delete)) => {
                let delete_name = self.config_nodes[i].name.clone();
                let index = self
                    .modified_nodes
                    .borrow()
                    .node
                    .iter()
                    .position(|n| n.name == delete_name)
                    .unwrap();
                self.modified_nodes.borrow_mut().node.remove(index);
                for node in &mut self.config_nodes {
                    match node
                        .neighbors_pick_lists
                        .iter()
                        .position(|pick_list| pick_list.value.as_ref().unwrap() == &delete_name)
                    {
                        Some(i) => {
                            node.neighbors_pick_lists.remove(i);
                        }
                        None => {}
                    }
                }
                self.config_nodes.remove(i);
            }
            NodeTabMessage::ConfigNode((i, ConfigNodeMessage::Edit)) => {
                // only alow one to en in edit mode at a time
                for node in &mut self.config_nodes {
                    node.update(ConfigNodeMessage::Okay);
                }
                self.unsaved = true;
                self.unsaved_tabs.borrow_mut().insert(TabState::Nodes, true);
                self.config_nodes[i].update(ConfigNodeMessage::Edit)
            }
            NodeTabMessage::ConfigNode((i, ConfigNodeMessage::HideChanged(b))) => {
                (*self.modified_nodes.borrow_mut().node)[i].hide = match b {
                    Boolean::True => true,
                    Boolean::False => false,
                };
                self.config_nodes[i].update(ConfigNodeMessage::HideChanged(b));
            }
            NodeTabMessage::ConfigNode((i, msg)) => {
                self.unsaved = true;
                self.unsaved_tabs.borrow_mut().insert(TabState::Nodes, true);
                self.config_nodes[i].update(msg);
            }
            NodeTabMessage::SaveMessage(SaveBarMessage::Cancel) => {
                self.save_bar.message = "Unsaved Changes!".to_string();
                // for abstraction purposes, UI interaction is 2d, but data storage is 3d, this
                // nested iter if to flatten the 3d nodes
                let node_pairs = self
                    .ref_nodes
                    .borrow()
                    .clone()
                    .node
                    .into_iter()
                    .filter(|n| !n.name.contains("_hover") || n.hide)
                    .zip(
                        self.ref_nodes
                            .borrow()
                            .clone()
                            .node
                            .into_iter()
                            .filter(|n| n.name.contains("_hover") || n.hide),
                    );
                self.modified_nodes = Rc::new(RefCell::new(Nodes {
                    node: node_pairs
                        .into_iter()
                        .fold(Vec::new(), |mut v, (n, n_hover)| {
                            v.push(Node {
                                neighbors: n_hover
                                    .neighbors
                                    .into_iter()
                                    .map(|name| name.replace("_hover", ""))
                                    .filter(|name| &n.name != name)
                                    .collect(),
                                name: n.name,
                                x: n.x,
                                y: n.y,
                                z: n.z,
                                hide: n.hide,
                            });
                            v
                        }),
                }));
                self.config_nodes = Rc::clone(&self.modified_nodes)
                    .borrow()
                    .node
                    .iter()
                    .filter(|n| !n.name.contains("_hover"))
                    .fold(Vec::new(), |mut v, n| {
                        v.push(ConfigNode::new(
                            n.name.clone(),
                            n.hide,
                            n.x,
                            n.y,
                            n.z,
                            n.neighbors.clone(),
                            Rc::clone(&self.modified_nodes),
                        ));
                        v
                    });
                self.unsaved = false;
                self.unsaved_tabs
                    .borrow_mut()
                    .insert(TabState::Nodes, false);
            }
            NodeTabMessage::SaveMessage(SaveBarMessage::Save) => {
                for node in &mut self.config_nodes {
                    node.errors.clear_temp();
                }
                if self.config_nodes.iter().any(|c| match c.state {
                    ConfigNodeState::Editing => true,
                    ConfigNodeState::Idle => false,
                }) {
                    self.save_bar.message = "'Ok' all steps before saving".to_string();
                    for node in &mut self.config_nodes {
                        match node.state {
                            ConfigNodeState::Idle => {}
                            ConfigNodeState::Editing => node.errors.try_save_while_edit = true,
                        }
                    }
                } else if self
                    .config_nodes
                    .iter()
                    .any(|n| n.errors.all().iter().any(|e| *e))
                {
                    self.save_bar.message = "Clear all errors below, and save again.".to_string();
                } else {
                    self.save_bar.message = "Unsaved Changes!".to_string();
                    // because this code is bad, merge the data stored in two
                    // separate locations.
                    let mut nodes = self.modified_nodes.borrow().clone();
                    for i in 0..self.config_nodes.len() {
                        nodes.node[i].neighbors = self.config_nodes[i]
                            .neighbors_pick_lists
                            .iter()
                            .fold(Vec::new(), |mut v, pick_list| {
                                v.push(pick_list.value.clone().unwrap());
                                v
                            });
                        nodes.node[i].x = self.config_nodes[i].x.parse().unwrap();
                        nodes.node[i].y = self.config_nodes[i].y.parse().unwrap();
                        nodes.node[i].z = self.config_nodes[i].z.parse().unwrap();
                    }
                    let new_toml = toml::to_string_pretty(&nodes).unwrap();
                    nodes.add_height_nodes();
                    let old_toml = toml::to_string_pretty(&Nodes {
                        node: self
                            .ref_nodes
                            .borrow()
                            .clone()
                            .node
                            .into_iter()
                            .filter(|n| !n.name.contains("_hover"))
                            .collect(),
                    })
                    .unwrap_or(String::new());
                    fs::write("./config/baths.toml", &new_toml).expect("Unable to save baths");
                    // update application with saved data
                    *self.node_map.borrow_mut() = get_nodemap(&nodes);
                    *self.homing_required.borrow_mut() = true;
                    self.node_tracker.lock().unwrap().current = nodes.node[self
                        .node_map
                        .borrow()
                        .get(&"HOME".to_string())
                        .unwrap()
                        .clone()]
                    .clone();
                    (*self.ref_nodes.borrow_mut()).node = nodes.node;

                    // log the changes
                    self.logger.set_log_file(format!(
                        "{}; Advanced (Nodes) - Save",
                        Local::now().to_rfc2822()
                    ));
                    self.logger.send_line(String::new()).unwrap();
                    self.logger
                        .send_line("Updated 'Nodes' from:".to_string())
                        .unwrap();
                    self.logger.send_line(old_toml).unwrap();
                    self.logger
                        .send_line("\n\nUpdated 'Nodes' to:".to_string())
                        .unwrap();
                    self.logger.send_line(new_toml).unwrap();
                    command = Command::perform(do_nothing(), NodeTabMessage::Saved);
                    self.unsaved = false;
                    self.unsaved_tabs
                        .borrow_mut()
                        .insert(TabState::Nodes, false);
                }
            }
            NodeTabMessage::AddConfigNode => {
                self.unsaved = true;
                self.unsaved_tabs.borrow_mut().insert(TabState::Nodes, true);
                // generate unique name
                let mut i = 2;
                let name = if self
                    .modified_nodes
                    .borrow()
                    .node
                    .iter()
                    .any(|n| n.name == "New Node".to_string())
                {
                    while self
                        .modified_nodes
                        .borrow()
                        .node
                        .iter()
                        .any(|n| n.name == format!("New Node #{}", i))
                    {
                        i += 1;
                    }
                    format!("New Node #{}", i)
                } else {
                    "New Node".to_string()
                };

                // push node to UI and data
                self.modified_nodes.borrow_mut().node.push(Node {
                    name: name.clone(),
                    hide: false,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    neighbors: Vec::new(),
                });
                self.config_nodes.push(ConfigNode::new(
                    name,
                    false,
                    0.0,
                    0.0,
                    0.0,
                    Vec::new(),
                    Rc::clone(&self.modified_nodes),
                ));
            }
            _ => {}
        };
        command
    }

    fn view(&mut self) -> Element<'_, NodeTabMessage> {
        let content = match self.unsaved {
            true => Column::new().align_items(Align::Center).push(
                self.save_bar
                    .view()
                    .map(move |msg| NodeTabMessage::SaveMessage(msg)),
            ),
            false => Column::new()
                .align_items(Align::Center)
                .push(Space::with_height(Length::Units(50))),
        };
        content
            .push(self.config_nodes.iter_mut().enumerate().fold(
                Column::new(),
                |col, (i, config_node)| {
                    col.push(
                        Row::new().max_width(400).padding(20).push(
                            config_node
                                .view()
                                .map(move |msg| NodeTabMessage::ConfigNode((i, msg))),
                        ),
                    )
                },
            ))
            .push(Space::with_height(Length::Units(50)))
            .push(
                Button::new(
                    &mut self.add_config_node_btn,
                    Text::new("Add New Node")
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .size(20)
                        .font(CQ_MONO),
                )
                .style(Theme::Blue)
                .padding(10)
                .width(Length::Units(400))
                .on_press(NodeTabMessage::AddConfigNode),
            )
            .into()
    }
}

struct ConfigNode {
    name: String,
    error_message: Option<String>,
    name_state: text_input::State,
    hide: Boolean,
    hide_state: pick_list::State<Boolean>,
    x: String,
    y: String,
    z: String,
    modified_nodes: Rc<RefCell<Nodes>>,
    neighbors_pick_lists: Vec<StringPickList>,
    x_state: text_input::State,
    y_state: text_input::State,
    z_state: text_input::State,
    add_neighbor_btn: button::State,
    state: ConfigNodeState,
    edit_btn: button::State,
    okay_btn: button::State,
    delete_btn: button::State,
    errors: ConfigNodeErrors,
}

#[derive(Debug, Clone)]
struct ConfigNodeErrors {
    names_match: bool,
    name_blank: bool,
    number_blank: bool,
    not_numbers: bool,
    too_many_decimals: bool,
    home_required: bool,
    try_save_while_edit: bool,
}

impl ConfigNodeErrors {
    fn new() -> Self {
        ConfigNodeErrors {
            names_match: false,
            name_blank: false,
            number_blank: false,
            not_numbers: false,
            too_many_decimals: false,
            home_required: false,
            try_save_while_edit: false,
        }
    }
    fn clear_nums(&mut self) {
        self.number_blank = false;
        self.not_numbers = false;
        self.too_many_decimals = false;
    }
    fn clear_title(&mut self) {
        self.names_match = false;
        self.name_blank = false;
    }
    fn clear_temp(&mut self) {
        self.home_required = false;
    }
    fn all(&self) -> Vec<bool> {
        vec![
            self.names_match,
            self.name_blank,
            self.number_blank,
            self.not_numbers,
            self.too_many_decimals,
            self.home_required,
            self.try_save_while_edit,
        ]
    }
}

#[derive(Debug, Clone)]
enum ConfigNodeState {
    Editing,
    Idle,
}

#[derive(Debug, Clone)]
pub enum ConfigNodeMessage {
    NameChanged(String),
    HideChanged(Boolean),
    XChanged(String),
    YChanged(String),
    ZChanged(String),
    Neighbors(usize, StringPickListMessage),
    AddNeighbor,
    Edit,
    Okay,
    Delete,
}

impl ConfigNode {
    fn new(
        name: String,
        hide: bool,
        x: f32,
        y: f32,
        z: f32,
        neighbors: Vec<String>,
        modified_nodes: Rc<RefCell<Nodes>>,
    ) -> Self {
        ConfigNode {
            name_state: text_input::State::new(),
            error_message: None,
            hide: match hide {
                true => Boolean::True,
                false => Boolean::False,
            },
            hide_state: pick_list::State::default(),
            x: x.to_string(),
            y: y.to_string(),
            z: z.to_string(),
            neighbors_pick_lists: neighbors.iter().filter(|n| !n.contains("_hover")).fold(
                Vec::new(),
                |mut v, n| {
                    v.push(StringPickList::new(
                        n.clone(),
                        Rc::clone(&modified_nodes),
                        name.clone(),
                        neighbors
                            .clone()
                            .into_iter()
                            .filter(|node| **node != *n)
                            .collect(),
                    ));
                    v
                },
            ),
            name,
            modified_nodes,
            x_state: text_input::State::new(),
            y_state: text_input::State::new(),
            z_state: text_input::State::new(),
            add_neighbor_btn: button::State::new(),
            state: ConfigNodeState::Idle,
            edit_btn: button::State::new(),
            okay_btn: button::State::new(),
            delete_btn: button::State::new(),
            errors: ConfigNodeErrors::new(),
        }
    }

    fn set_error(&mut self, msg: impl ToString) {
        self.error_message = Some(msg.to_string());
    }

    fn clear_error(&mut self) {
        self.error_message = None;
    }

    fn update(&mut self, message: ConfigNodeMessage) {
        self.errors.clear_temp();
        // TODO: Disallow save if multiple nodes have same name
        match message {
            ConfigNodeMessage::NameChanged(name) => {
                for pick_list in &mut self.neighbors_pick_lists {
                    pick_list.parent = name.clone();
                }
                self.name = name;
            }
            ConfigNodeMessage::HideChanged(b) => self.hide = b,
            // TODO: Highlight red if not valid f32 or more than 3 decimals
            ConfigNodeMessage::XChanged(x) => {
                self.x = x;
                if self.x.is_empty() {
                    self.errors.number_blank = true;
                } else {
                    match validate_nums(vec![&self.x, &self.y, &self.z], 3) {
                        ValidateNums::NotNums => self.errors.not_numbers = true,
                        ValidateNums::TooManyDecimals => {
                            self.errors.not_numbers = false;
                            self.errors.too_many_decimals = true;
                        }
                        ValidateNums::Okay => self.errors.clear_nums(),
                    }
                }
            }
            ConfigNodeMessage::YChanged(y) => {
                self.y = y;
                if self.y.is_empty() {
                    self.errors.number_blank = true;
                } else {
                    match validate_nums(vec![&self.x, &self.y, &self.z], 3) {
                        ValidateNums::NotNums => self.errors.not_numbers = true,
                        ValidateNums::TooManyDecimals => {
                            self.errors.not_numbers = false;
                            self.errors.too_many_decimals = true;
                        }
                        ValidateNums::Okay => self.errors.clear_nums(),
                    }
                }
            }
            ConfigNodeMessage::ZChanged(z) => {
                self.z = z;
                if self.z.is_empty() {
                    self.errors.number_blank = true;
                } else {
                    match validate_nums(vec![&self.x, &self.y, &self.z], 3) {
                        ValidateNums::NotNums => self.errors.not_numbers = true,
                        ValidateNums::TooManyDecimals => {
                            self.errors.not_numbers = false;
                            self.errors.too_many_decimals = true;
                        }
                        ValidateNums::Okay => self.errors.clear_nums(),
                    }
                }
            }
            ConfigNodeMessage::Neighbors(i, StringPickListMessage::Delete) => {
                self.neighbors_pick_lists.remove(i);
                let siblings =
                    self.neighbors_pick_lists
                        .iter()
                        .fold(Vec::new(), |mut v, pick_list| {
                            v.push(pick_list.value.clone().unwrap());
                            v
                        });
                for pick_list in &mut self.neighbors_pick_lists {
                    pick_list.siblings = siblings
                        .clone()
                        .into_iter()
                        .filter(|sibling| sibling != pick_list.value.as_ref().unwrap())
                        .collect(); // remove itself from siblings
                }
            }
            ConfigNodeMessage::Neighbors(i, StringPickListMessage::Changed(new_value)) => {
                // update siblings of sibling nodes so that no node can have two identical
                // neighbors
                let old_value = self.neighbors_pick_lists[i].value.clone();
                let siblings =
                    self.neighbors_pick_lists
                        .iter()
                        .fold(Vec::new(), |mut v, pick_list| {
                            v.push(if pick_list.value == old_value {
                                new_value.clone()
                            } else {
                                pick_list.value.clone().unwrap()
                            });
                            v
                        });
                for pick_list in &mut self.neighbors_pick_lists {
                    pick_list.siblings = siblings
                        .clone()
                        .into_iter()
                        .filter(|sibling| sibling != pick_list.value.as_ref().unwrap())
                        .collect(); // remove itself from siblings
                }

                self.neighbors_pick_lists[i].update(StringPickListMessage::Changed(new_value));
            }
            ConfigNodeMessage::AddNeighbor => self.neighbors_pick_lists.push(StringPickList::new(
                "Choose Neighbor".to_string(),
                Rc::clone(&self.modified_nodes),
                self.name.clone(),
                self.neighbors_pick_lists
                    .iter()
                    .fold(Vec::new(), |mut v, pick_list| {
                        v.push(pick_list.value.clone().unwrap());
                        v
                    }),
            )),
            ConfigNodeMessage::Edit => self.state = ConfigNodeState::Editing,
            ConfigNodeMessage::Okay => {
                self.errors.try_save_while_edit = false;
                for i in (0..self.neighbors_pick_lists.len()).rev() {
                    if self.neighbors_pick_lists[i].value.as_ref().unwrap()
                        == &"Choose Neighbor".to_string()
                    {
                        self.neighbors_pick_lists.remove(i);
                    }
                }
                //self.neighbors_pick_lists.into_iter().filter(|pick_list| pick_list.value.as_ref().unwrap() == "Choose Neighbor").collect::<Vec<StringPickList>>();
                self.state = ConfigNodeState::Idle
            }
            _ => {}
        }
    }

    fn view(&mut self) -> Element<'_, ConfigNodeMessage> {
        if self.errors.home_required {
            self.set_error("'Home' is required");
        } else if self.errors.name_blank {
            self.set_error("Must have a name.");
        } else if self.errors.names_match {
            self.set_error("Names must be unique.");
        } else if self.errors.not_numbers {
            self.set_error("All positions must be numbers.");
        } else if self.errors.number_blank {
            self.set_error("Positions cannot be empty.");
        } else if self.errors.too_many_decimals {
            self.set_error("Limit to 3 decimals.");
        } else if self.errors.try_save_while_edit {
            self.set_error("Click green checkbox before saving.");
        } else {
            self.clear_error();
        }
        match self.state {
            ConfigNodeState::Editing => {
                Column::new()
                    .push(if let Some(msg) = &self.error_message {
                        Container::new(
                            Row::with_children(vec![
                                Space::with_width(Length::Fill).into(),
                                Text::new(msg).into(),
                                Space::with_width(Length::Fill).into(),
                            ])
                            .padding(10),
                        )
                        .height(Length::Units(40))
                        .style(Theme::Red)
                    } else {
                        Container::new(Space::with_height(Length::Units(40)))
                    })
                    .push(
                        Row::new()
                            .push(
                                TextInput::new(
                                    &mut self.name_state,
                                    "Name",
                                    &self.name,
                                    ConfigNodeMessage::NameChanged,
                                )
                                .style(Theme::Blue)
                                .padding(10),
                            )
                            .push(
                                Button::new(&mut self.okay_btn, okay_icon())
                                    .on_press(ConfigNodeMessage::Okay)
                                    .width(Length::Units(50))
                                    .padding(10)
                                    .style(Theme::Green),
                            )
                            .push(if self.name != "HOME".to_string() {
                                Tooltip::new(
                                    Button::new(&mut self.delete_btn, delete_icon())
                                        .on_press(ConfigNodeMessage::Delete)
                                        .width(Length::Units(50))
                                        .padding(10)
                                        .style(Theme::Red),
                                    "",
                                    tooltip::Position::Top,
                                )
                            } else {
                                Tooltip::new(
                                    Button::new(&mut self.delete_btn, delete_icon())
                                        .style(Theme::RedDisabled)
                                        .width(Length::Units(50))
                                        .padding(10),
                                    "'HOME' required by Bathub.\nCannot be deleted.",
                                    tooltip::Position::FollowCursor,
                                )
                                .style(Theme::Red)
                            }),
                    )
                    .push(
                        Row::new()
                            .padding(5)
                            .push(Text::new("Hidden:"))
                            .push(Space::with_width(Length::Units(36)))
                            .push(
                                PickList::new(
                                    &mut self.hide_state,
                                    &Boolean::ALL[..],
                                    Some(self.hide),
                                    ConfigNodeMessage::HideChanged,
                                )
                                .style(Theme::Blue)
                                .padding(10)
                                .width(Length::Fill),
                            ),
                    )
                    .push(
                        Row::new()
                            .padding(5)
                            .push(Text::new("X Pos (cm):"))
                            .push(Space::with_width(Length::Units(10)))
                            .push(
                                TextInput::new(
                                    &mut self.x_state,
                                    "0.000",
                                    &self.x,
                                    ConfigNodeMessage::XChanged,
                                )
                                .style(Theme::Blue)
                                .font(CQ_MONO)
                                .padding(10)
                                .max_width(400),
                            ),
                    )
                    .push(
                        Row::new()
                            .padding(5)
                            .push(Text::new("Y Pos (cm):"))
                            .push(Space::with_width(Length::Units(10)))
                            .push(
                                TextInput::new(
                                    &mut self.y_state,
                                    "0.000",
                                    &self.y,
                                    ConfigNodeMessage::YChanged,
                                )
                                .style(Theme::Blue)
                                .font(CQ_MONO)
                                .padding(10)
                                .max_width(400),
                            ),
                    )
                    .push(
                        Row::new()
                            .padding(5)
                            .push(Text::new("Z Pos (cm):"))
                            .push(Space::with_width(Length::Units(10)))
                            .push(
                                TextInput::new(
                                    &mut self.z_state,
                                    "0.000",
                                    &self.z,
                                    ConfigNodeMessage::ZChanged,
                                )
                                .style(Theme::Blue)
                                .font(CQ_MONO)
                                .padding(10)
                                .max_width(400),
                            ),
                    )
                    .push(
                        Row::new()
                            .padding(5)
                            .push(Text::new("Neighbors:"))
                            .push(Space::with_width(Length::Units(10)))
                            .push(
                                Column::new()
                                    .max_width(400)
                                    .push(self.neighbors_pick_lists.iter_mut().enumerate().fold(
                                        Column::new(),
                                        |col, (i, pick_list)| {
                                            if i == 0 {
                                                pick_list.disable_delete()
                                            } else {
                                                pick_list.enable_delete()
                                            }
                                            col.push(pick_list.view().map(move |msg| {
                                                ConfigNodeMessage::Neighbors(i, msg)
                                            }))
                                        },
                                    ))
                                    .push(Space::with_height(Length::Units(5)))
                                    .push(
                                        Button::new(
                                            &mut self.add_neighbor_btn,
                                            Text::new("Add Neighbor")
                                                .font(CQ_MONO)
                                                .horizontal_alignment(HorizontalAlignment::Center)
                                                .font(CQ_MONO),
                                        )
                                        .style(Theme::Blue)
                                        .on_press(ConfigNodeMessage::AddNeighbor)
                                        .width(Length::Fill)
                                        .padding(10),
                                    ),
                            ),
                    )
                    .into()
            }
            ConfigNodeState::Idle => Column::new()
                .push(if let Some(msg) = &self.error_message {
                    Container::new(
                        Row::with_children(vec![
                            Space::with_width(Length::Fill).into(),
                            Text::new(msg).into(),
                            Space::with_width(Length::Fill).into(),
                        ])
                        .padding(10),
                    )
                    .height(Length::Units(40))
                    .style(Theme::Red)
                } else {
                    Container::new(Space::with_height(Length::Units(40)))
                })
                .push(
                    Row::new()
                        .push(
                            TextInput::new(
                                &mut self.name_state,
                                "Name",
                                &self.name,
                                ConfigNodeMessage::NameChanged,
                            )
                            .style(Theme::Blue)
                            .padding(10),
                        )
                        .push(
                            Button::new(
                                &mut self.edit_btn,
                                Text::new("Edit").horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .style(Theme::Blue)
                            .on_press(ConfigNodeMessage::Edit)
                            .width(Length::Units(100))
                            .padding(10),
                        ),
                )
                .push(
                    Row::new()
                        .padding(5)
                        .push(Text::new("Hidden:").vertical_alignment(VerticalAlignment::Center))
                        .push(Space::with_width(Length::Units(38)))
                        .push(Text::new(format!("{}", self.hide))),
                )
                .push(
                    Row::new()
                        .padding(5)
                        .push(Text::new("X Pos (cm):"))
                        .push(Space::with_width(Length::Units(10)))
                        .push(Text::new(&self.x).font(CQ_MONO)),
                )
                .push(
                    Row::new()
                        .padding(5)
                        .push(Text::new("Y Pos (cm):"))
                        .push(Space::with_width(Length::Units(10)))
                        .push(Text::new(&self.y).font(CQ_MONO)),
                )
                .push(
                    Row::new()
                        .padding(5)
                        .push(Text::new("Z Pos (cm):"))
                        .push(Space::with_width(Length::Units(10)))
                        .push(Text::new(&self.z).font(CQ_MONO)),
                )
                .push(
                    Row::new()
                        .padding(5)
                        .push(Text::new("Neighbors:"))
                        .push(Space::with_width(Length::Units(10)))
                        .push(
                            self.neighbors_pick_lists.iter().fold(
                                Column::new(),
                                |col, pick_list| {
                                    col.push(Text::new(pick_list.value.as_ref().unwrap()))
                                },
                            ),
                        ),
                )
                .into(),
        }
    }
}

// Created so that it can be folded into vec and stored in ConfigNode, because the number of
// picklists cannot be known at compile time
struct StringPickList {
    value: Option<String>,
    state: pick_list::State<String>,
    modified_nodes: Rc<RefCell<Nodes>>,
    delete_btn: button::State,
    parent: String,
    siblings: Vec<String>,
    delete_disabled: bool,
}

#[derive(Debug, Clone)]
pub enum StringPickListMessage {
    Changed(String),
    Delete,
}

impl StringPickList {
    fn new(
        value: String,
        modified_nodes: Rc<RefCell<Nodes>>,
        parent: String,
        siblings: Vec<String>,
    ) -> Self {
        StringPickList {
            value: Some(value),
            state: pick_list::State::default(),
            modified_nodes,
            parent,
            siblings,
            delete_btn: button::State::new(),
            delete_disabled: false,
        }
    }

    fn disable_delete(&mut self) {
        self.delete_disabled = true;
    }

    fn enable_delete(&mut self) {
        self.delete_disabled = false;
    }

    fn update(&mut self, message: StringPickListMessage) {
        match message {
            StringPickListMessage::Changed(s) => self.value = Some(s),
            _ => {}
        }
    }

    fn view(&mut self) -> Element<'_, StringPickListMessage> {
        let parent = self.parent.clone();
        let siblings = self.siblings.clone();
        Row::new()
            .push(
                PickList::new(
                    &mut self.state,
                    self.modified_nodes
                        .borrow()
                        .node
                        .iter()
                        .filter(|n| {
                            !n.name.contains("_hover")
                                && n.name != "HOME".to_string()
                                && *n.name != parent
                                && !siblings.iter().any(|s| s == &n.name)
                        })
                        .fold(Vec::new(), |mut v, n| {
                            v.push(n.name.clone());
                            v
                        }),
                    self.value.clone(),
                    StringPickListMessage::Changed,
                )
                .style(Theme::Blue)
                .width(Length::Fill)
                .padding(10),
            )
            .push(if self.delete_disabled {
                Tooltip::new(
                    Button::new(&mut self.delete_btn, delete_icon())
                        .style(Theme::RedDisabled)
                        .padding(10)
                        .width(Length::Units(50)),
                    "All nodes must have at least one neighbor.",
                    tooltip::Position::FollowCursor,
                )
                .style(Theme::Red)
            } else {
                Tooltip::new(
                    Button::new(&mut self.delete_btn, delete_icon())
                        .style(Theme::Red)
                        .on_press(StringPickListMessage::Delete)
                        .padding(10)
                        .width(Length::Units(50)),
                    "",
                    tooltip::Position::Top,
                )
            })
            .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boolean {
    True,
    False,
}

impl Boolean {
    const ALL: [Boolean; 2] = [Boolean::False, Boolean::True];
}

impl Default for Boolean {
    fn default() -> Boolean {
        Boolean::False
    }
}

impl std::fmt::Display for Boolean {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Boolean::True => "True",
                Boolean::False => "False",
            }
        )
    }
}

// ======================= //
// ----- ACTIONS TAB ---- //
// ===================== //
struct ActionTab {
    unsaved: bool,
    save_bar: SaveBar,
    ref_actions: Rc<RefCell<Actions>>,
    modified_actions: Rc<RefCell<Actions>>,
    config_actions: Vec<ConfigAction>,
    add_config_action_btn: button::State,
    logger: Logger,
    unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
}

#[derive(Debug, Clone)]
pub enum ActionTabMessage {
    AddConfigAction,
    ConfigAction(usize, ConfigActionMessage),
    SaveMessage(SaveBarMessage),
    Saved(()),
}

impl ActionTab {
    fn new(
        ref_actions: Rc<RefCell<Actions>>,
        logger: Logger,
        unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
    ) -> Self {
        let modified_actions = Rc::new(RefCell::new(ref_actions.borrow().clone()));
        ActionTab {
            unsaved: false,
            save_bar: SaveBar::new(),
            ref_actions: Rc::clone(&ref_actions),
            modified_actions: Rc::clone(&modified_actions),
            config_actions: Rc::clone(&modified_actions).borrow().action.iter().fold(
                Vec::new(),
                |mut v, a| {
                    v.push(ConfigAction::new(a.name.clone(), a.commands.clone()));
                    v
                },
            ),
            add_config_action_btn: button::State::new(),
            logger,
            unsaved_tabs,
        }
    }

    fn update(&mut self, message: ActionTabMessage) -> Command<ActionTabMessage> {
        let mut command = Command::none();
        match message {
            ActionTabMessage::ConfigAction(i, ConfigActionMessage::Okay) => {
                self.save_bar.message = "Unsaved Changes!".to_string();
                self.config_actions[i].update(ConfigActionMessage::Okay);
                // clone value from UI into state values
                let index = self
                    .modified_actions
                    .borrow()
                    .action
                    .iter()
                    .position(|a| a.name == self.config_actions[i].name)
                    .unwrap();
                self.modified_actions.borrow_mut().action[index].commands = self.config_actions[i]
                    .command_inputs
                    .iter()
                    .fold(Vec::new(), |mut v, c| {
                        v.push(c.value.clone());
                        v
                    })
            }
            ActionTabMessage::ConfigAction(i, ConfigActionMessage::Delete) => {
                self.config_actions.remove(i);
                (*self.modified_actions.borrow_mut()).action.remove(i);
            }
            ActionTabMessage::ConfigAction(i, ConfigActionMessage::NameChanged(name)) => {
                if self.config_actions.iter().any(|a| a.name == name) {
                    self.config_actions[i].errors.names_match = true;
                } else {
                    if name != String::new() {
                        self.config_actions[i].errors.clear_title();
                    } else {
                        self.config_actions[i].errors.name_blank = true;
                    }
                    // mark unsaved, change value in UI and modfied_nodes
                    self.unsaved = true;
                    self.unsaved_tabs
                        .borrow_mut()
                        .insert(TabState::Actions, true);
                    let index = self
                        .modified_actions
                        .borrow()
                        .action
                        .iter()
                        .position(|a| a.name == self.config_actions[i].name)
                        .unwrap();
                    self.modified_actions.borrow_mut().action[index].name = name.clone();
                    self.config_actions[i].update(ConfigActionMessage::NameChanged(name))
                }
            }
            ActionTabMessage::ConfigAction(i, ConfigActionMessage::Edit) => {
                for config_action in &mut self.config_actions {
                    config_action.update(ConfigActionMessage::Okay);
                }
                self.unsaved = true;
                self.unsaved_tabs
                    .borrow_mut()
                    .insert(TabState::Actions, true);
                self.config_actions[i].update(ConfigActionMessage::Edit);
            }
            ActionTabMessage::ConfigAction(i, msg) => self.config_actions[i].update(msg),
            ActionTabMessage::AddConfigAction => {
                self.unsaved = true;
                self.unsaved_tabs
                    .borrow_mut()
                    .insert(TabState::Actions, true);
                // generate unique name
                let mut i = 2;
                let name = if self
                    .modified_actions
                    .borrow()
                    .action
                    .iter()
                    .any(|n| n.name == "New Action".to_string())
                {
                    while self
                        .modified_actions
                        .borrow()
                        .action
                        .iter()
                        .any(|n| n.name == format!("New Action #{}", i))
                    {
                        i += 1;
                    }
                    format!("New Action #{}", i)
                } else {
                    "New Action".to_string()
                };

                // push node to UI and data
                self.modified_actions.borrow_mut().action.push(Action {
                    name: name.clone(),
                    commands: Vec::new(),
                });
                self.config_actions
                    .push(ConfigAction::new(name, Vec::new()));
            }
            ActionTabMessage::SaveMessage(SaveBarMessage::Save) => {
                if self.config_actions.iter().any(|c| match c.state {
                    ConfigActionState::Editing => true,
                    ConfigActionState::Idle => false,
                }) {
                    self.save_bar.message = "'Ok' all steps before saving".to_string();
                    for action in &mut self.config_actions {
                        match action.state {
                            ConfigActionState::Idle => {}
                            ConfigActionState::Editing => action.errors.try_save_while_edit = true,
                        }
                    }
                } else if self
                    .config_actions
                    .iter()
                    .any(|n| n.errors.all().iter().any(|e| *e))
                {
                    self.save_bar.message = "Clear all errors below, and save again.".to_string();
                } else {
                    self.save_bar.message = "Unsaved Changes!".to_string();
                    let old_toml = toml::to_string_pretty(&*self.ref_actions.borrow()).unwrap();
                    let new_toml =
                        toml::to_string_pretty(&*self.modified_actions.borrow()).unwrap();
                    fs::write("./config/actions.toml", &new_toml).expect("Unable to save Actions");

                    *self.ref_actions.borrow_mut() = self.modified_actions.borrow().clone();
                    self.logger.set_log_file(format!(
                        "{}; Advanced (Actions) - Save",
                        Local::now().to_rfc2822()
                    ));
                    self.logger.send_line(String::new()).unwrap();
                    self.logger
                        .send_line("Updated 'Actions' from:".to_string())
                        .unwrap();
                    self.logger.send_line(old_toml).unwrap();
                    self.logger
                        .send_line("\n\nUpdated 'Actions' to:".to_string())
                        .unwrap();
                    self.logger.send_line(new_toml).unwrap();
                    command = Command::perform(do_nothing(), ActionTabMessage::Saved);
                    self.unsaved = false;
                    self.unsaved_tabs
                        .borrow_mut()
                        .insert(TabState::Actions, false);
                }
            }
            ActionTabMessage::SaveMessage(SaveBarMessage::Cancel) => {
                self.save_bar.message = "Unsaved Changes!".to_string();
                self.modified_actions = Rc::new(RefCell::new(self.ref_actions.borrow().clone()));
                self.config_actions = Rc::clone(&self.modified_actions)
                    .borrow()
                    .action
                    .iter()
                    .fold(Vec::new(), |mut v, a| {
                        v.push(ConfigAction::new(a.name.clone(), a.commands.clone()));
                        v
                    });

                self.unsaved = false;
                self.unsaved_tabs
                    .borrow_mut()
                    .insert(TabState::Actions, false);
            }
            _ => {}
        }
        command
    }

    fn view(&mut self) -> Element<'_, ActionTabMessage> {
        let content = match self.unsaved {
            true => Column::new().align_items(Align::Center).push(
                self.save_bar
                    .view()
                    .map(move |msg| ActionTabMessage::SaveMessage(msg)),
            ),
            false => Column::new()
                .align_items(Align::Center)
                .push(Space::with_height(Length::Units(50))),
        };
        content
            .push(self.config_actions.iter_mut().enumerate().fold(
                Column::new(),
                |col, (i, config_action)| {
                    col.push(
                        Row::new().max_width(400).padding(20).push(
                            config_action
                                .view()
                                .map(move |msg| ActionTabMessage::ConfigAction(i, msg)),
                        ),
                    )
                },
            ))
            .push(Space::with_height(Length::Units(50)))
            .push(
                Button::new(
                    &mut self.add_config_action_btn,
                    Text::new("Add Action")
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .size(20)
                        .font(CQ_MONO),
                )
                .style(Theme::Blue)
                .padding(10)
                .width(Length::Units(400))
                .on_press(ActionTabMessage::AddConfigAction),
            )
            .into()
    }
}

struct ConfigAction {
    name: String,
    error_message: Option<String>,
    name_state: text_input::State,
    command_inputs: Vec<CommandInput>,
    add_command_btn: button::State,
    state: ConfigActionState,
    edit_btn: button::State,
    okay_btn: button::State,
    delete_btn: button::State,
    errors: ConfigActionErrors,
}

#[derive(Debug, Clone)]
struct ConfigActionErrors {
    names_match: bool,
    name_blank: bool,
    try_save_while_edit: bool,
}

impl ConfigActionErrors {
    fn new() -> Self {
        ConfigActionErrors {
            names_match: false,
            name_blank: false,
            try_save_while_edit: false,
        }
    }
    fn clear_title(&mut self) {
        self.names_match = false;
        self.name_blank = false;
    }
    fn all(&self) -> Vec<bool> {
        vec![self.names_match, self.name_blank, self.try_save_while_edit]
    }
}

#[derive(Debug, Clone)]
enum ConfigActionState {
    Editing,
    Idle,
}

#[derive(Debug, Clone)]
pub enum ConfigActionMessage {
    NameChanged(String),
    Commands(usize, CommandInputMessage),
    AddCommand,
    Edit,
    Okay,
    Delete,
}

impl ConfigAction {
    fn new(name: String, commands: Vec<String>) -> Self {
        ConfigAction {
            name_state: text_input::State::new(),
            error_message: None,
            command_inputs: commands.iter().fold(Vec::new(), |mut v, cmd| {
                v.push(CommandInput::new(cmd.clone()));
                v
            }),
            name,
            add_command_btn: button::State::new(),
            state: ConfigActionState::Idle,
            edit_btn: button::State::new(),
            okay_btn: button::State::new(),
            delete_btn: button::State::new(),
            errors: ConfigActionErrors::new(),
        }
    }

    fn set_error(&mut self, msg: impl ToString) {
        self.error_message = Some(msg.to_string());
    }

    fn clear_error(&mut self) {
        self.error_message = None;
    }

    fn update(&mut self, message: ConfigActionMessage) {
        // TODO: Disallow save if multiple actions have same name
        match message {
            ConfigActionMessage::Edit => self.state = ConfigActionState::Editing,
            ConfigActionMessage::Okay => {
                self.errors.try_save_while_edit = false;
                for i in (0..self.command_inputs.len()).rev() {
                    if self.command_inputs[i].value == "G-code Command".to_string() {
                        self.command_inputs.remove(i);
                    }
                }
                self.state = ConfigActionState::Idle
            }
            ConfigActionMessage::NameChanged(name) => self.name = name,
            ConfigActionMessage::Commands(i, CommandInputMessage::Delete) => {
                self.command_inputs.remove(i);
            }
            ConfigActionMessage::Commands(i, msg) => self.command_inputs[i].update(msg),
            ConfigActionMessage::AddCommand => {
                self.command_inputs.push(CommandInput::new("".to_string()))
            }
            _ => {}
        }
    }

    fn view(&mut self) -> Element<'_, ConfigActionMessage> {
        if self.errors.names_match {
            self.set_error("Names must be unique.")
        } else if self.errors.name_blank {
            self.set_error("Must have a name.")
        } else if self.errors.try_save_while_edit {
            self.set_error("Click green checkbox before saving.")
        } else {
            self.clear_error()
        }
        match self.state {
            ConfigActionState::Editing => Column::new()
                .push(if let Some(msg) = &self.error_message {
                    Container::new(
                        Row::with_children(vec![
                            Space::with_width(Length::Fill).into(),
                            Text::new(msg).into(),
                            Space::with_width(Length::Fill).into(),
                        ])
                        .padding(10),
                    )
                    .height(Length::Units(40))
                    .style(Theme::Red)
                } else {
                    Container::new(Space::with_height(Length::Units(40)))
                })
                .push(
                    Row::new()
                        .push(
                            TextInput::new(
                                &mut self.name_state,
                                "Name",
                                &self.name,
                                ConfigActionMessage::NameChanged,
                            )
                            .style(Theme::Blue)
                            .padding(10),
                        )
                        .push(
                            Button::new(&mut self.okay_btn, okay_icon())
                                .on_press(ConfigActionMessage::Okay)
                                .width(Length::Units(50))
                                .padding(10)
                                .style(Theme::Green),
                        )
                        .push(
                            Button::new(&mut self.delete_btn, delete_icon())
                                .style(Theme::Red)
                                .on_press(ConfigActionMessage::Delete)
                                .width(Length::Units(50))
                                .padding(10),
                        ),
                )
                .push(
                    Row::new()
                        .padding(5)
                        .push(Text::new("G-code\nCommands:"))
                        .push(Space::with_width(Length::Units(25)))
                        .push(
                            Column::new()
                                .push(
                                    self.command_inputs
                                        .iter_mut()
                                        .enumerate()
                                        .fold(Column::new(), |col, (i, input)| {
                                            col.push(input.view().map(move |msg| {
                                                ConfigActionMessage::Commands(i, msg)
                                            }))
                                        })
                                        .push(
                                            Button::new(
                                                &mut self.add_command_btn,
                                                Text::new("Add Command")
                                                    .horizontal_alignment(
                                                        HorizontalAlignment::Center,
                                                    )
                                                    .font(CQ_MONO),
                                            )
                                            .style(Theme::Blue)
                                            .on_press(ConfigActionMessage::AddCommand)
                                            .width(Length::Fill)
                                            .padding(10),
                                        ),
                                )
                                .max_width(400),
                        ),
                )
                .into(),
            ConfigActionState::Idle => Column::new()
                .push(if let Some(msg) = &self.error_message {
                    Container::new(
                        Row::with_children(vec![
                            Space::with_width(Length::Fill).into(),
                            Text::new(msg).into(),
                            Space::with_width(Length::Fill).into(),
                        ])
                        .padding(10),
                    )
                    .height(Length::Units(40))
                    .style(Theme::Red)
                } else {
                    Container::new(Space::with_height(Length::Units(40)))
                })
                .push(
                    Row::new()
                        .push(
                            TextInput::new(
                                &mut self.name_state,
                                "Name",
                                &self.name,
                                ConfigActionMessage::NameChanged,
                            )
                            .style(Theme::Blue)
                            .padding(10),
                        )
                        .push(
                            Button::new(
                                &mut self.edit_btn,
                                Text::new("Edit").horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .style(Theme::Blue)
                            .on_press(ConfigActionMessage::Edit)
                            .width(Length::Units(100))
                            .padding(10),
                        ),
                )
                .push(
                    Row::new()
                        .padding(5)
                        .push(Text::new("G-code\nCommands:"))
                        .push(Space::with_width(Length::Units(25)))
                        .push(
                            self.command_inputs
                                .iter()
                                .fold(Column::new(), |col, input| {
                                    col.push(Text::new(&input.value))
                                }),
                        ),
                )
                .into(),
        }
    }
}

struct CommandInput {
    state: text_input::State,
    delete_btn: button::State,
    value: String,
}

#[derive(Debug, Clone)]
pub enum CommandInputMessage {
    InputChanged(String),
    Delete,
}

impl CommandInput {
    fn new(value: String) -> Self {
        CommandInput {
            state: text_input::State::new(),
            value,
            delete_btn: button::State::new(),
        }
    }

    fn update(&mut self, message: CommandInputMessage) {
        match message {
            CommandInputMessage::InputChanged(input) => self.value = input,
            _ => {}
        }
    }

    fn view(&mut self) -> Element<'_, CommandInputMessage> {
        Row::new()
            .push(
                TextInput::new(
                    &mut self.state,
                    "Gcode Command",
                    &self.value[..],
                    CommandInputMessage::InputChanged,
                )
                .style(Theme::Blue)
                .padding(10),
            )
            .push(
                Button::new(&mut self.delete_btn, delete_icon())
                    .style(Theme::Red)
                    .width(Length::Units(50))
                    .padding(10)
                    .on_press(CommandInputMessage::Delete),
            )
            .into()
    }
}

struct LogTab {
    logs: Vec<Log>,
    unsearched_files: Vec<String>,
    search_bars: Vec<SearchBar>,
    date_regex: Regex,
}

#[derive(Debug, Clone)]
pub enum LogTabMessage {
    SearchChanged(usize, SearchBarMessage),
    Log(usize, LogMessage),
    AddLog((Vec<String>, Option<Log>)),
}

impl LogTab {
    fn new() -> Self {
        let date_regex = Regex::new(r"[^;]+").unwrap();
        let mut log_files: Vec<_> = fs::read_dir(Path::new(LOGS))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        log_files.sort_by(|a, b| {
            // convert from title to sort by seconds, just sorting by name sorts by day
            let b_caps = date_regex.captures(&b[..]).unwrap();
            let a_caps = date_regex.captures(&a[..]).unwrap();
            DateTime::parse_from_rfc2822(&b_caps[0].replace("_", ":"))
                .unwrap()
                .timestamp()
                .cmp(
                    &DateTime::parse_from_rfc2822(&a_caps[0].replace("_", ":"))
                        .unwrap()
                        .timestamp(),
                )
        });
        log_files.truncate(LOG_MAX);
        LogTab {
            logs: log_files.into_iter().map(|f| Log::new(f)).collect(),
            unsearched_files: Vec::new(),
            search_bars: vec![SearchBar::new(0)],
            date_regex,
        }
    }

    pub fn update_logs(&mut self) {
        self.search_bars = vec![SearchBar::new(0)];
        let mut log_files: Vec<_> = if let Ok(files) = fs::read_dir(Path::new(LOGS)) {
            files
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect()
        } else {Vec::new()
        };
        log_files.sort_by(|a, b| {
            // convert from title to sort by seconds, just sorting by name sorts by day
            let b_caps = self.date_regex.captures(&b[..]).unwrap();
            let a_caps = self.date_regex.captures(&a[..]).unwrap();
            DateTime::parse_from_rfc2822(&b_caps[0].replace("_", ":"))
                .unwrap()
                .timestamp()
                .cmp(
                    &DateTime::parse_from_rfc2822(&a_caps[0].replace("_", ":"))
                        .unwrap()
                        .timestamp(),
                )
        });
        log_files.truncate(LOG_MAX);
        self.logs = log_files.into_iter().map(|f| Log::new(f)).collect();
    }

    fn update(&mut self, message: LogTabMessage) -> Command<LogTabMessage> {
        match message {
            LogTabMessage::AddLog((vals, log)) => {
                if self.logs.len() <= LOG_MAX
                    && vals
                        .iter()
                        .zip(self.search_bars.iter().fold(
                            Vec::with_capacity(self.search_bars.len()),
                            |mut v, bar| {
                                v.push(&bar.value);
                                v
                            },
                        ))
                        .all(|(a, b)| a == &b.to_lowercase())
                {
                    if let Some(log) = log {
                        self.logs.push(log);
                    }
                    if self.unsearched_files.len() > 0 {
                        Command::perform(
                            Logger::search_files(vals, self.unsearched_files.remove(0)),
                            LogTabMessage::AddLog,
                        )
                    } else {
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            }
            LogTabMessage::SearchChanged(i, SearchBarMessage::InputChanged(val)) => {
                // run search as multithreaded Commands to speed up search
                // update bar and add new if necessary
                self.search_bars[i].value = val.clone();
                if self.search_bars.len() - 1 == i {
                    self.search_bars.push(SearchBar::new(i + 1));
                }
                // remove empty search bars
                if val == "".to_string() {
                    if i == 0 {
                        self.update_logs();
                        return Command::none();
                    } else {
                        self.search_bars.remove(i);
                        for i in 0..self.search_bars.len() {
                            self.search_bars[i].num = i;
                            self.search_bars[i].message = if i == 0 {
                                "Search".to_string()
                            } else {
                                format!("Search term {} (Optional)", i + 1)
                            };
                        }
                    }
                }
                // reset order to be chronological if nothing searched
                if self.search_bars.len() == 1 && self.search_bars[0].value == "".to_string() {
                    self.update_logs();
                    Command::none()
                } else {
                    self.logs = Vec::with_capacity(LOG_MAX);
                    self.unsearched_files = if let Ok(files) = fs::read_dir(Path::new(LOGS)) {
                        files.fold(
                        Vec::with_capacity(LOG_MAX),
                        |mut v, file| {
                            v.push(file.unwrap().file_name().to_string_lossy().to_string());
                            v
                        },
                    )
                } else {Vec::new()};
                    // Note: limit to 15 active search threads as limit on windows
                    Command::batch((0..min(15, self.unsearched_files.len())).into_iter().fold(
                        Vec::with_capacity(15),
                        |mut v, _i| {
                            v.push(Command::perform(
                                Logger::search_files(
                                    self.search_bars.iter().fold(
                                        Vec::with_capacity(self.search_bars.len()),
                                        |mut v, bar| {
                                            v.push(bar.value.to_lowercase().clone());
                                            v
                                        },
                                    ),
                                    self.unsearched_files.remove(0),
                                ),
                                LogTabMessage::AddLog,
                            ));
                            v
                        },
                    ))
                }
            }
            LogTabMessage::Log(i, msg) => {
                self.logs[i].update(msg);
                Command::none()
            }
        }
    }

    fn view(&mut self) -> Element<'_, LogTabMessage> {
        let date_regex = self.date_regex.clone();
        self.logs.sort_by(|a, b| {
            // convert from title to sort by seconds, just sorting by name sorts by day
            let b_caps = date_regex.captures(&b.title[..]).unwrap();
            let a_caps = date_regex.captures(&a.title[..]).unwrap();
            DateTime::parse_from_rfc2822(&b_caps[0].replace("_", ":"))
                .unwrap()
                .timestamp()
                .cmp(
                    &DateTime::parse_from_rfc2822(&a_caps[0].replace("_", ":"))
                        .unwrap()
                        .timestamp(),
                )
        });
        let logs = self.logs.iter_mut().take(LOG_MAX);
        let logs_count = logs.len();
        Column::new()
            .push(
                self.search_bars
                    .iter_mut()
                    .enumerate()
                    .fold(Column::new(), |col, (i, bar)| {
                        col.push(
                            bar.view()
                                .map(move |msg| LogTabMessage::SearchChanged(i, msg)),
                        )
                    }),
            )
            .push(logs.enumerate().fold(Column::new(), |col, (i, log)| {
                log.set_style(if i % 2 == 0 {
                    Theme::Blue
                } else {
                    Theme::DarkBlue
                });
                col.push(log.view().map(move |msg| LogTabMessage::Log(i, msg)))
            }))
            .push(if logs_count == LOG_MAX {
                Row::with_children(vec![Text::new(format!(
                    "Showing first {}. Use search to narrow down results.",
                    LOG_MAX
                ))
                .font(CQ_MONO)
                .width(Length::Fill)
                .horizontal_alignment(HorizontalAlignment::Center)
                .into()])
                .spacing(10)
            } else {
                Row::with_children(vec![Text::new(if self.unsearched_files.len() > 0 {
                    "Searching . . ."
                } else {
                    "Showing all results."
                })
                .font(CQ_MONO)
                .width(Length::Fill)
                .horizontal_alignment(HorizontalAlignment::Center)
                .into()])
            })
            .into()
    }
}

#[derive(Clone, Debug)]
pub struct SearchBar {
    title: String,
    message: String,
    num: usize,
    value: String,
    state: text_input::State,
}

#[derive(Clone, Debug)]
pub enum SearchBarMessage {
    InputChanged(String),
}

impl SearchBar {
    fn new(num: usize) -> Self {
        SearchBar {
            title: if num == 0 {
                "Contains:".to_string()
            } else {
                "and:".to_string()
            },
            message: if num == 0 {
                "Search".to_string()
            } else {
                format!("Search term {} (Optional)", num + 1)
            },
            num,
            value: String::new(),
            state: text_input::State::new(),
        }
    }

    fn view(&mut self) -> Element<'_, SearchBarMessage> {
        Row::new()
            .push((0..self.num).fold(Row::new(), |r, _i| {
                r.push(Space::with_width(Length::Units(30)))
            }))
            .push(
                Row::new()
                    .push(
                        Text::new(&self.title)
                            .size(20)
                            .width(Length::Units(80))
                            .horizontal_alignment(HorizontalAlignment::Right),
                    )
                    .padding(10),
            )
            .push(
                TextInput::new(
                    &mut self.state,
                    &self.message[..],
                    &self.value,
                    SearchBarMessage::InputChanged,
                )
                .style(Theme::Blue)
                .padding(10),
            )
            .into()
    }
}

#[derive(Clone, Debug)]
pub struct Log {
    title: String,
    content: String,
    opened: bool,
    toggle_view_btn: button::State,
    hide_gcode: bool,
    style: Theme,
    show_checkbox: bool,
}

#[derive(Debug, Clone)]
pub enum LogMessage {
    ToggleView,
    ToggleGcode(bool),
}

impl Log {
    pub fn new(title: String) -> Self {
        Log {
            title,
            content: "".to_string(), // leave empty until opened
            opened: false,
            toggle_view_btn: button::State::new(),
            hide_gcode: true,
            style: Theme::Blue,
            show_checkbox: false,
        }
    }

    fn set_style(&mut self, style: Theme) {
        self.style = style;
    }

    fn update(&mut self, message: LogMessage) {
        match message {
            LogMessage::ToggleView => {
                if self.opened {
                    self.opened = false;
                } else {
                    let content =
                        fs::read_to_string(Path::new(&format!("{}/{}", LOGS, &self.title)))
                            .unwrap_or(format!("Error: Unable tp read file {}!", &self.title));
                    self.show_checkbox = content.contains("G-code");
                    match self.hide_gcode {
                        true => {
                            self.content = content.lines().filter(|l| !l.contains("G-code")).fold(
                                String::new(),
                                |mut s, line| {
                                    s.push_str(line);
                                    s.push_str("\n");
                                    s
                                },
                            )
                        }
                        false => {
                            self.content = content;
                        }
                    };

                    self.opened = true;
                }
            }
            LogMessage::ToggleGcode(b) => {
                match self.hide_gcode {
                    false => {
                        self.content =
                            fs::read_to_string(Path::new(&format!("{}/{}", LOGS, &self.title)))
                                .unwrap_or(format!("Error: Unable tp read file {}!", &self.title))
                                .lines()
                                .filter(|l| !l.contains("G-code"))
                                .fold(String::new(), |mut s, line| {
                                    s.push_str(line);
                                    s.push_str("\n");
                                    s
                                })
                    }
                    true => {
                        self.content =
                            fs::read_to_string(Path::new(&format!("{}/{}", LOGS, &self.title)))
                                .unwrap_or(format!("Error: Unable to read file {}!", &self.title))
                    }
                };
                self.hide_gcode = b;
            }
        }
    }

    fn view(&mut self) -> Element<'_, LogMessage> {
        match self.opened {
            true => Column::new()
                .push(
                    Button::new(
                        &mut self.toggle_view_btn,
                        Row::new()
                            .push(down_icon())
                            .push(Text::new(&self.title).font(CQ_MONO)),
                    )
                    .style(self.style)
                    .padding(10)
                    .width(Length::Fill)
                    .on_press(LogMessage::ToggleView),
                )
                .push(if self.show_checkbox {
                    Row::new()
                        .push(Space::with_width(Length::Fill))
                        .push(
                            Checkbox::new(
                                self.hide_gcode,
                                "Hide G-code lines",
                                LogMessage::ToggleGcode,
                            )
                            .style(Theme::Blue),
                        )
                        .padding(10)
                        .push(Space::with_width(Length::Fill))
                } else {
                    Row::new()
                })
                .push(
                    Row::new()
                        .push(Text::new(&self.content).width(Length::Fill).font(CQ_MONO))
                        .padding(20),
                )
                .into(),
            false => Column::new()
                .spacing(5)
                .push(
                    Button::new(
                        &mut self.toggle_view_btn,
                        Row::new()
                            .push(right_icon())
                            .push(Text::new(&self.title).font(CQ_MONO)),
                    )
                    .style(self.style)
                    .padding(10)
                    .width(Length::Fill)
                    .on_press(LogMessage::ToggleView),
                )
                .into(),
        }
    }
}

fn date(date_num: &str) -> String {
    format!(
        "{} {}, {}",
        match &date_num[4..6] {
            "01" => "January",
            "02" => "Febuary",
            "03" => "March",
            "04" => "April",
            "05" => "May",
            "06" => "June",
            "07" => "July",
            "08" => "August",
            "09" => "September",
            "10" => "October",
            "11" => "November",
            "12" => "December",
            _ => "",
        },
        &date_num[6..8],
        &date_num[0..4]
    )
}
pub fn validate_nums(nums: Vec<&String>, count: usize) -> ValidateNums {
    if nums.iter().any(|num| {
        if !num.is_empty() {
            num.parse::<f32>().is_err()
        } else {
            false
        }
    }) {
        ValidateNums::NotNums
    } else if nums.iter().any(|num| {
        num.split(".")
            .enumerate()
            .fold(String::new(), |mut s, (i, part)| {
                if i == 1 {
                    s.push_str(part)
                };
                s
            })
            .len()
            > count
    }) {
        // split the decimals and count them
        ValidateNums::TooManyDecimals
    } else {
        ValidateNums::Okay
    }
}

pub enum ValidateNums {
    Okay,
    NotNums,
    TooManyDecimals,
}
