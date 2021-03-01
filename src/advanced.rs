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
            grbl_tab: GrblTab::new(grbl),
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
                                            v.push(GrblSetting {
                                                text: format!("{} =", r[0]),
                                                input_value: r[1].to_string(),
                                                input_state: text_input::State::new(),
                                            });
                                        }
                                        v
                                    });
                        }
                    }
                }
                self.state = TabState::Grbl
            }
            AdvancedMessage::TabBar(TabBarMessage::Nodes) => self.state = TabState::Nodes,
            AdvancedMessage::TabBar(TabBarMessage::Actions) => self.state = TabState::Actions,
            AdvancedMessage::TabBar(TabBarMessage::Logs) => self.state = TabState::Logs,
            _ => {}
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
            .width(Length::Fill)
            .push(Text::new("Unsaved Changes!").size(20).font(CQ_MONO))
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
            .into()
    }
}

struct GrblTab {
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
    TextChanged(String),
}

impl GrblTab {
    fn new(grbl: Grbl) -> Self {
        GrblTab {
            unsaved: false,
            grbl,
            settings: Vec::new(),
        }
    }

    fn view(&mut self) -> Element<'_, GrblMessage> {
        self.settings
            .iter_mut()
            .enumerate()
            .fold(Column::new().width(Length::Fill), |col, (_i, setting)| {
                col.push(
                    Row::new()
                        .push(
                            Column::new()
                                .push(Text::new(&setting.text))
                                .padding(10)
                                .spacing(10)
                                .width(Length::Units(100)),
                        )
                        .push(
                            TextInput::new(
                                &mut setting.input_state,
                                "",
                                &setting.input_value,
                                GrblMessage::TextChanged,
                            )
                            .padding(10)
                            .width(Length::Units(400)),
                        ),
                )
                .align_items(Align::Center)
                .spacing(5)
            })
            .into()
    }
}
