use super::actions::Actions;
use super::nodes::Nodes;
use crate::CQ_MONO;
use iced::{
    button, scrollable, text_input, Align, Button, Column, Container, Element, HorizontalAlignment,
    Length, Row, Scrollable, Space, Text, TextInput, VerticalAlignment,
};

use super::grbl::{Command as Cmd, Grbl};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Advanced {
    scroll: scrollable::State,
    state: TabState,
    ref_nodes: Rc<RefCell<Nodes>>,
    ref_actions: Rc<RefCell<Actions>>,
    tab_bar: TabBar,
    grbl_tab: GrblTab,
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
            ref_nodes,
            ref_actions,
            tab_bar: TabBar::new(),
            grbl_tab: GrblTab::new(grbl, Vec::new()),
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
            //_ => {}
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
            TabState::Nodes => Column::new().into(),
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
