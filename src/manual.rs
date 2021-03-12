use super::grbl::{Command as Cmd, Grbl};
use super::nodes::{Node, NodeGrid2d};
use crate::CQ_MONO;
use chrono::prelude::*;
use iced::{
    button, scrollable, text_input, Button, Checkbox, Column, Command, Container, Element,
    HorizontalAlignment, Length, Row, Scrollable, Space, Text, TextInput,
};
use regex::Regex;
use std::thread;
use std::time::Duration;

pub struct Manual {
    pub scroll: scrollable::State,
    pub bath_btns: Vec<Vec<Option<(Node, button::State)>>>,
    pub stop_btn: button::State,
    pub status: String,
    pub hover: bool,
    pub status_regex: Regex,
    grid_btn: button::State,
    terminal_btn: button::State,
    state: ManualState,
    terminal_responses: Vec<String>,
    terminal_input_state: text_input::State,
    terminal_input_value: String,
    grbl: Grbl,
}

#[derive(Debug, Clone)]
enum ManualState {
    Terminal,
    Grid,
}

#[derive(Debug, Clone)]
pub enum ManualMessage {
    ButtonPressed(String),
    ToggleBath(bool),
    Stop,
    TerminalTab,
    GridTab,
    TerminalInputChanged(String),
    TerminalInputSubmitted,
    PopResponse(()),
}

impl Manual {
    pub fn new(node_grid2d: NodeGrid2d, grbl: Grbl) -> Self {
        Manual {
            scroll: scrollable::State::new(),
            bath_btns: node_grid2d
                .grid
                .into_iter()
                .fold(Vec::new(), |mut vec, axis| {
                    vec.push(axis.into_iter().fold(Vec::new(), |mut axis_vec, node| {
                        if let Some(n) = node {
                            if !n.hide {
                                axis_vec.push(Some((n, button::State::new())));
                            }
                        } else {
                            axis_vec.push(None);
                        }
                        axis_vec
                    }));
                    vec
                }),
            status: "Click any button\nto start homing cycle".to_string(),
            stop_btn: button::State::new(),
            hover: true,
            status_regex: Regex::new(
                r"(?P<status>[A-Za-z]+).{6}(?P<X>[-\d.]+),(?P<Y>[-\d.]+),(?P<Z>[-\d.]+)",
            )
            .unwrap(),
            grid_btn: button::State::new(),
            terminal_btn: button::State::new(),
            state: ManualState::Grid,
            terminal_responses: Vec::new(),
            terminal_input_state: text_input::State::new(),
            terminal_input_value: String::new(),
            grbl,
        }
    }

    pub fn update(&mut self, message: ManualMessage) -> Command<ManualMessage> {
        match message {
            ManualMessage::ToggleBath(boolean) => self.hover = boolean,
            ManualMessage::TerminalTab => self.state = ManualState::Terminal,
            ManualMessage::GridTab => self.state = ManualState::Grid,
            ManualMessage::TerminalInputChanged(val) => self.terminal_input_value = val,
            ManualMessage::PopResponse(_) => match self.grbl.pop_command() {
                Some(cmd) => self.terminal_responses.insert(0,format!(
                    "{}| '{}' => {}",
                    cmd.response_time.unwrap().to_rfc2822(),
                    cmd.command,
                    cmd.result.unwrap()
                )),
                None => {println!("no cmd");}
            },
            ManualMessage::TerminalInputSubmitted => {
                let val = self.terminal_input_value.replace("\n", "").replace(" ", "");
                self.terminal_input_value = "".to_string();
                if &val[..] == "$$" {
                    self.terminal_responses.insert(0,format!("{}| '$$' => View and edit settings withing Bathtub! Advanced Tab => Grbl ;)", Local::now().to_rfc2822()))
                } else {
                    self.grbl.push_command(Cmd::new(val.to_uppercase()));
                    return Command::perform(gimme_a_second(), ManualMessage::PopResponse);
                }
            }
            _ => {}
        };
        Command::none()
    }

    pub fn view(&mut self) -> Element<ManualMessage> {
        let title = Text::new(self.status.clone())
            .width(Length::Fill)
            .size(40)
            .color([0.5, 0.5, 0.5])
            .font(CQ_MONO)
            .horizontal_alignment(HorizontalAlignment::Center);

        let tab_btns = Column::new().push(
            Row::new()
                .push(Space::with_width(Length::Fill))
                .push(
                    Button::new(
                        &mut self.grid_btn,
                        Text::new("Grid")
                            .font(CQ_MONO)
                            .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .padding(10)
                    .on_press(ManualMessage::GridTab)
                    .width(Length::Units(200)),
                )
                .push(
                    Button::new(
                        &mut self.terminal_btn,
                        Text::new("Terminal")
                            .font(CQ_MONO)
                            .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .padding(10)
                    .on_press(ManualMessage::TerminalTab)
                    .width(Length::Units(200)),
                )
                .push(Space::with_width(Length::Fill)),
        );

        match self.state {
            ManualState::Grid => {
                let button_grid = self
                    .bath_btns
                    .iter_mut()
                    .fold(Column::new(), |column, grid| {
                        column.push(
                            grid.into_iter()
                                .fold(Row::new(), |row, node_tup| {
                                    if let Some(nt) = node_tup {
                                        row.push(
                                            Button::new(
                                                &mut nt.1,
                                                Text::new(&nt.0.name)
                                                    .horizontal_alignment(
                                                        HorizontalAlignment::Center,
                                                    )
                                                    .font(CQ_MONO),
                                            )
                                            .padding(15)
                                            .width(Length::Fill)
                                            .on_press(
                                                ManualMessage::ButtonPressed(nt.0.name.clone()),
                                            ),
                                        )
                                    } else {
                                        row.push(Column::new().width(Length::Fill))
                                    }
                                })
                                .padding(3),
                        )
                    });

                let modifiers = Row::new()
                    .push(
                        Column::new()
                            .push(Space::with_height(Length::Units(10)))
                            .push(Checkbox::new(
                                self.hover,
                                "Hover Above",
                                ManualMessage::ToggleBath,
                            )),
                    )
                    .push(Space::with_width(Length::Units(25)))
                    .push(
                        Button::new(
                            &mut self.stop_btn,
                            Text::new("STOP")
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .font(CQ_MONO)
                                .size(30),
                        )
                        .padding(10)
                        .width(Length::Fill)
                        .on_press(ManualMessage::Stop),
                    );

                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .push(title)
                    .push(tab_btns)
                    .push(button_grid)
                    .push(modifiers);

                Scrollable::new(&mut self.scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
            ManualState::Terminal => {
                let terminal_input = TextInput::new(
                    &mut self.terminal_input_state,
                    "ADVANCED USAGE ONLY! BATHTUB DOES NOT CHECK IF THESE COMMANDS ARE SAFE",
                    &self.terminal_input_value,
                    ManualMessage::TerminalInputChanged,
                )
                .on_submit(ManualMessage::TerminalInputSubmitted)
                .padding(10);

                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .push(title)
                    .push(tab_btns)
                    .push(terminal_input)
                    .push(
                        self.terminal_responses
                            .iter()
                            .fold(Column::new(), |col, response| {
                                col.push(Text::new(response).font(CQ_MONO))
                            }),
                    );
                Scrollable::new(&mut self.scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
        }
    }
}

async fn gimme_a_second() {
    thread::sleep(Duration::from_secs(1));
}
