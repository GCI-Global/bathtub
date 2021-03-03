use super::actions::Actions;
use super::nodes::{Node, Nodes};
use crate::CQ_MONO;
use iced::{
    button, pick_list, scrollable, text_input, Align, Button, Column, Container, Element,
    HorizontalAlignment, Length, PickList, Row, Scrollable, Space, Text, TextInput,
    VerticalAlignment,
};

use super::build::{delete_icon, okay_icon};
use super::grbl::{Command as Cmd, Grbl};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Advanced {
    scroll: scrollable::State,
    state: TabState,
    //ref_nodes: Rc<RefCell<Nodes>>,
    ref_actions: Rc<RefCell<Actions>>,
    tab_bar: TabBar,
    grbl_tab: GrblTab,
    nodes_tab: NodeTab,
}

enum TabState {
    Grbl,
    Nodes,
    Actions,
    Logs,
}

#[derive(Debug, Clone)]
pub enum AdvancedMessage {
    TabBar(TabBarMessage),
    GrblTab(GrblMessage),
    NodesTab(NodeTabMessage),
}

impl Advanced {
    pub fn new(
        grbl: Grbl,
        ref_nodes: Rc<RefCell<Nodes>>,
        ref_actions: Rc<RefCell<Actions>>,
    ) -> Self {
        Advanced {
            scroll: scrollable::State::new(),
            state: TabState::Grbl,
            ref_actions,
            tab_bar: TabBar::new(),
            grbl_tab: GrblTab::new(grbl, Vec::new()),
            nodes_tab: NodeTab::new(ref_nodes),
        }
    }

    pub fn update(&mut self, message: AdvancedMessage) {
        match message {
            AdvancedMessage::TabBar(TabBarMessage::Grbl) => {
                if !self.grbl_tab.unsaved {
                    self.grbl_tab.grbl.push_command(Cmd::new("$$".to_string()));
                    if let Some(cmd) = self.grbl_tab.grbl.pop_command() {
                        if cmd.command == "$$".to_string() {
                            self.grbl_tab.settings =
                                cmd.result
                                    .unwrap()
                                    .lines()
                                    .fold(Vec::new(), |mut v, response| {
                                        let r: Vec<&str> = response.split("=").collect();
                                        if r.len() > 1 {
                                            v.push(GrblSetting::new(
                                                format!("{}", r[0]),
                                                r[1].to_string(),
                                            ));
                                        }
                                        v
                                    });
                        }
                    }
                }
                self.state = TabState::Grbl
            }
            AdvancedMessage::GrblTab(GrblMessage::SaveMessage(SaveBarMessage::Cancel)) => {
                self.grbl_tab.grbl.push_command(Cmd::new("$$".to_string()));
                loop {
                    if let Some(cmd) = self.grbl_tab.grbl.pop_command() {
                        if cmd.command == "$$".to_string() {
                            self.grbl_tab.settings =
                                cmd.result
                                    .unwrap()
                                    .lines()
                                    .fold(Vec::new(), |mut v, response| {
                                        let r: Vec<&str> = response.split("=").collect();
                                        if r.len() > 1 {
                                            v.push(GrblSetting::new(
                                                format!("{}", r[0]),
                                                r[1].to_string(),
                                            ));
                                        }
                                        v
                                    });
                            break;
                        }
                    }
                }
                self.grbl_tab.unsaved = false;
            }
            AdvancedMessage::TabBar(TabBarMessage::Nodes) => self.state = TabState::Nodes,
            AdvancedMessage::TabBar(TabBarMessage::Actions) => self.state = TabState::Actions,
            AdvancedMessage::TabBar(TabBarMessage::Logs) => self.state = TabState::Logs,
            AdvancedMessage::GrblTab(msg) => {
                self.grbl_tab.unsaved = true;
                self.grbl_tab.update(msg)
            }
            AdvancedMessage::NodesTab(msg) => self.nodes_tab.update(msg),
        }
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
            TabState::Actions => Column::new().into(),
            TabState::Logs => Column::new().into(),
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
}

#[derive(Debug, Clone)]
enum TabBarMessage {
    Grbl,
    Nodes,
    Actions,
    Logs,
}

impl TabBar {
    fn new() -> Self {
        TabBar {
            grbl_btn: button::State::new(),
            nodes_btn: button::State::new(),
            actions_btn: button::State::new(),
            logs_btn: button::State::new(),
        }
    }

    fn view(&mut self) -> Element<TabBarMessage> {
        Column::new()
            .height(Length::Fill)
            .width(Length::Shrink)
            .push(
                Button::new(
                    &mut self.grbl_btn,
                    Text::new("GRBL")
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center)
                        .size(30)
                        .font(CQ_MONO),
                )
                .height(Length::Fill)
                .width(Length::Units(200))
                .padding(20)
                .on_press(TabBarMessage::Grbl),
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
                .height(Length::Fill)
                .width(Length::Units(200))
                .padding(20)
                .on_press(TabBarMessage::Actions),
            )
            .push(
                Button::new(
                    &mut self.logs_btn,
                    Text::new("Logs")
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center)
                        .size(30)
                        .font(CQ_MONO),
                )
                .height(Length::Fill)
                .width(Length::Units(200))
                .padding(20)
                .on_press(TabBarMessage::Logs),
            )
            .into()
    }
}

struct SaveBar {
    save_btn: button::State,
    cancel_btn: button::State,
}

#[derive(Debug, Clone)]
enum SaveBarMessage {
    Save,
    Cancel,
}

impl SaveBar {
    fn new() -> Self {
        SaveBar {
            save_btn: button::State::new(),
            cancel_btn: button::State::new(),
        }
    }

    fn view(&mut self) -> Element<'_, SaveBarMessage> {
        Row::new()
            .height(Length::Units(50))
            .width(Length::Fill)
            .push(Space::with_width(Length::Units(50)))
            .push(
                Text::new("Unsaved Changes!")
                    .vertical_alignment(VerticalAlignment::Center)
                    .size(20)
                    .font(CQ_MONO),
            )
            .push(Space::with_width(Length::Fill))
            .push(
                Button::new(
                    &mut self.save_btn,
                    Text::new("Save")
                        .size(20)
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
                .padding(10)
                .on_press(SaveBarMessage::Save),
            )
            .push(Space::with_width(Length::Units(25)))
            .push(
                Button::new(
                    &mut self.cancel_btn,
                    Text::new("Cancel")
                        .size(20)
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
                .padding(10)
                .on_press(SaveBarMessage::Cancel),
            )
            .push(Space::with_width(Length::Units(50)))
            .into()
    }
}

struct GrblTab {
    save_bar: SaveBar,
    unsaved: bool,
    grbl: Grbl,
    settings: Vec<GrblSetting>,
}

struct GrblSetting {
    text: String,
    input_value: String,
    input_state: text_input::State,
}

#[derive(Debug, Clone)]
enum GrblMessage {
    SettingChanged(usize, GrblSettingMessage),
    SaveMessage(SaveBarMessage),
}

impl GrblTab {
    fn new(grbl: Grbl, settings: Vec<GrblSetting>) -> Self {
        GrblTab {
            save_bar: SaveBar::new(),
            unsaved: false,
            grbl,
            settings,
        }
    }

    fn update(&mut self, message: GrblMessage) {
        match message {
            GrblMessage::SettingChanged(i, GrblSettingMessage::TextChanged(val)) => {
                self.settings[i].input_value = val
            }
            GrblMessage::SaveMessage(SaveBarMessage::Save) => {
                for setting in &self.settings {
                    self.grbl.push_command(Cmd::new(format!(
                        "{}={}",
                        &setting.text, &setting.input_value
                    )));
                }
                if let Some(final_cmd) = self.settings.last().clone() {
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
                self.unsaved = false;
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
                self.settings
                    .iter_mut()
                    .enumerate()
                    .fold(Column::new(), |col, (i, setting)| {
                        col.push(
                            setting
                                .view()
                                .map(move |msg| GrblMessage::SettingChanged(i, msg)),
                        )
                    }),
            )
            .into()
    }
}

#[derive(Debug, Clone)]
enum GrblSettingMessage {
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
}

#[derive(Debug, Clone)]
enum NodeTabMessage {
    AddConfigNode,
    ConfigNode((usize, ConfigNodeMessage)),
    SaveMessage(SaveBarMessage),
}

impl NodeTab {
    fn new(ref_nodes: Rc<RefCell<Nodes>>) -> Self {
        let modified_nodes = Rc::new(RefCell::new(ref_nodes.borrow().clone()));
        NodeTab {
            unsaved: false,
            save_bar: SaveBar::new(),
            ref_nodes: Rc::clone(&ref_nodes),
            modified_nodes: Rc::clone(&modified_nodes),
            config_nodes: ref_nodes
                .borrow()
                .node
                .iter()
                .filter(|n| !n.name.contains("_inBath"))
                .fold(Vec::new(), |mut v, n| {
                    v.push(ConfigNode::new(
                        n.name.clone(),
                        n.hide,
                        n.x,
                        n.y,
                        n.z,
                        n.neighbors.clone(),
                        Rc::clone(&ref_nodes),
                        Rc::clone(&modified_nodes),
                    ));
                    v
                }),
        }
    }

    fn update(&mut self, message: NodeTabMessage) {
        match message {
            NodeTabMessage::ConfigNode((i, ConfigNodeMessage::NameChanged(name))) => {
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
            NodeTabMessage::ConfigNode((i, ConfigNodeMessage::Delete)) => {
                self.config_nodes.remove(i);
            }
            NodeTabMessage::ConfigNode((i, ConfigNodeMessage::Edit)) => {
                // only alow one to en in edit mode at a time
                for node in &mut self.config_nodes {
                    node.state = ConfigNodeState::Idle;
                }
                self.unsaved = true;
                self.config_nodes[i].update(ConfigNodeMessage::Edit)
            }
            NodeTabMessage::ConfigNode((i, msg)) => {
                self.unsaved = true;
                self.config_nodes[i].update(msg);
            }
            NodeTabMessage::SaveMessage(SaveBarMessage::Cancel) => {
                self.modified_nodes = Rc::new(RefCell::new(self.ref_nodes.borrow().clone()));
                self.config_nodes = Rc::clone(&self.ref_nodes)
                    .borrow()
                    .node
                    .iter()
                    .filter(|n| !n.name.contains("_inBath"))
                    .fold(Vec::new(), |mut v, n| {
                        v.push(ConfigNode::new(
                            n.name.clone(),
                            n.hide,
                            n.x,
                            n.y,
                            n.z,
                            n.neighbors.clone(),
                            Rc::clone(&self.ref_nodes),
                            Rc::clone(&self.modified_nodes),
                        ));
                        v
                    });
                self.unsaved = false;
            }
            NodeTabMessage::SaveMessage(SaveBarMessage::Save) => {
                let mut rn = self.ref_nodes.borrow_mut();
                let node = self
                    .config_nodes
                    .iter()
                    .fold(Vec::new(), |mut v, config_node| {
                        v.push(Node {
                            name: config_node.name.clone(),
                            hide: match config_node.hide {
                                Boolean::False => false,
                                Boolean::True => true,
                            },
                            x: config_node.x.parse().unwrap(),
                            y: config_node.y.parse().unwrap(),
                            z: config_node.z.parse().unwrap(),
                            neighbors: Vec::new(),
                        });
                        v
                    });
                *rn = Nodes { node }
            }
            _ => {}
        }
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
            .into()
    }
}

struct ConfigNode {
    name: String,
    name_state: text_input::State,
    hide: Boolean,
    hide_state: pick_list::State<Boolean>,
    x: String,
    y: String,
    z: String,
    ref_nodes: Rc<RefCell<Nodes>>,
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
}

#[derive(Debug, Clone)]
enum ConfigNodeState {
    Editing,
    Idle,
}

#[derive(Debug, Clone)]
enum ConfigNodeMessage {
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
        ref_nodes: Rc<RefCell<Nodes>>,
        modified_nodes: Rc<RefCell<Nodes>>,
    ) -> Self {
        ConfigNode {
            name_state: text_input::State::new(),
            hide: match hide {
                true => Boolean::True,
                false => Boolean::False,
            },
            hide_state: pick_list::State::default(),
            x: x.to_string(),
            y: y.to_string(),
            z: z.to_string(),
            neighbors_pick_lists: neighbors.iter().filter(|n| !n.contains("_inBath")).fold(
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
            ref_nodes,
            x_state: text_input::State::new(),
            y_state: text_input::State::new(),
            z_state: text_input::State::new(),
            add_neighbor_btn: button::State::new(),
            state: ConfigNodeState::Idle,
            edit_btn: button::State::new(),
            okay_btn: button::State::new(),
            delete_btn: button::State::new(),
        }
    }

    fn update(&mut self, message: ConfigNodeMessage) {
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
            ConfigNodeMessage::XChanged(x) => self.x = x,
            ConfigNodeMessage::YChanged(y) => self.y = y,
            ConfigNodeMessage::ZChanged(z) => self.z = z,
            ConfigNodeMessage::Neighbors(i, StringPickListMessage::Delete) => {
                self.neighbors_pick_lists.remove(i);
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
            ConfigNodeMessage::Okay => self.state = ConfigNodeState::Idle,
            _ => {}
        }
    }

    fn view(&mut self) -> Element<'_, ConfigNodeMessage> {
        match self.state {
            ConfigNodeState::Editing => Column::new()
                .push(
                    Row::new()
                        .push(
                            TextInput::new(
                                &mut self.name_state,
                                "Name",
                                &self.name,
                                ConfigNodeMessage::NameChanged,
                            )
                            .padding(10),
                        )
                        .push(
                            Button::new(&mut self.okay_btn, okay_icon())
                                .on_press(ConfigNodeMessage::Okay)
                                .width(Length::Units(50))
                                .padding(10),
                        )
                        .push(
                            Button::new(&mut self.delete_btn, delete_icon())
                                .on_press(ConfigNodeMessage::Delete)
                                .width(Length::Units(50))
                                .padding(10),
                        ),
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
                                .push(
                                    self.neighbors_pick_lists
                                        .iter_mut()
                                        .enumerate()
                                        .fold(Column::new(), |col, (i, pick_list)| {
                                            col.push(pick_list.view().map(move |msg| {
                                                ConfigNodeMessage::Neighbors(i, msg)
                                            }))
                                        })
                                        .push(
                                            Button::new(
                                                &mut self.add_neighbor_btn,
                                                Text::new("Add Neighbor").horizontal_alignment(
                                                    HorizontalAlignment::Center,
                                                ),
                                            )
                                            .on_press(ConfigNodeMessage::AddNeighbor)
                                            .width(Length::Fill)
                                            .padding(10),
                                        ),
                                )
                                .max_width(400),
                        ),
                )
                .into(),
            ConfigNodeState::Idle => Column::new()
                .push(
                    Row::new()
                        .push(
                            TextInput::new(
                                &mut self.name_state,
                                "Name",
                                &self.name,
                                ConfigNodeMessage::NameChanged,
                            )
                            .padding(10),
                        )
                        .push(
                            Button::new(
                                &mut self.edit_btn,
                                Text::new("Edit").horizontal_alignment(HorizontalAlignment::Center),
                            )
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
                        .push(Text::new(&self.x)),
                )
                .push(
                    Row::new()
                        .padding(5)
                        .push(Text::new("Y Pos (cm):"))
                        .push(Space::with_width(Length::Units(10)))
                        .push(Text::new(&self.y)),
                )
                .push(
                    Row::new()
                        .padding(5)
                        .push(Text::new("Z Pos (cm):"))
                        .push(Space::with_width(Length::Units(10)))
                        .push(Text::new(&self.z)),
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
}

#[derive(Debug, Clone)]
enum StringPickListMessage {
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
        }
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
                            !n.name.contains("_inBath")
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
                .width(Length::Fill)
                .padding(10),
            )
            .push(
                Button::new(&mut self.delete_btn, delete_icon())
                    .on_press(StringPickListMessage::Delete)
                    .padding(10),
            )
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
