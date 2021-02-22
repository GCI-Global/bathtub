use super::nodes::{Node, NodeGrid2d};
use crate::CQ_MONO;
use iced::{
    button, scrollable, Button, Checkbox, Column, Container, Element, HorizontalAlignment, Length,
    Row, Scrollable, Text,
};
use regex::Regex;

pub struct Manual {
    pub scroll: scrollable::State,
    pub bath_btns: Vec<Vec<Option<(Node, button::State)>>>,
    pub pause_btn: button::State,
    pub resume_btn: button::State,
    pub status: String,
    pub in_bath: bool,
    pub status_regex: Regex,
}

#[derive(Debug, Clone)]
pub enum ManualMessage {
    ButtonPressed(String),
    ToggleBath(bool),
    Pause,
    Resume,
}

impl Manual {
    pub fn new(node_grid2d: NodeGrid2d) -> Self {
        Manual {
            scroll: scrollable::State::new(),
            bath_btns: node_grid2d
                .grid
                .into_iter()
                .fold(Vec::new(), |mut vec, axis| {
                    vec.push(axis.into_iter().fold(Vec::new(), |mut axis_vec, node| {
                        if let Some(n) = node {
                            axis_vec.push(Some((n, button::State::new())));
                        } else {
                            axis_vec.push(None);
                        }
                        axis_vec
                    }));
                    vec
                }),
            status: "Click any button\nto start homing cycle".to_string(),
            pause_btn: button::State::new(),
            resume_btn: button::State::new(),
            in_bath: false,
            status_regex: Regex::new(
                r"(?P<status>[A-Za-z]+).{6}(?P<X>[-\d.]+),(?P<Y>[-\d.]+),(?P<Z>[-\d.]+)",
            )
            .unwrap(),
        }
    }

    pub fn update(&mut self, message: ManualMessage) {
        match message {
            ManualMessage::ToggleBath(boolean) => self.in_bath = boolean,
            _ => (),
        }
    }

    pub fn view(&mut self) -> Element<ManualMessage> {
        let title = Text::new(self.status.clone())
            .width(Length::Fill)
            .size(40)
            .color([0.5, 0.5, 0.5])
            .font(CQ_MONO)
            .horizontal_alignment(HorizontalAlignment::Center);

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
                                            .horizontal_alignment(HorizontalAlignment::Center)
                                            .font(CQ_MONO),
                                    )
                                    .padding(15)
                                    .width(Length::Fill)
                                    .on_press(ManualMessage::ButtonPressed(nt.0.name.clone())),
                                )
                            } else {
                                row.push(Column::new().width(Length::Fill))
                            }
                        })
                        .padding(3),
                )
            });

        let inbath_toggle = Checkbox::new(self.in_bath, "Enter Bath", ManualMessage::ToggleBath);
        let pause = Button::new(&mut self.pause_btn, Text::new("Pause").font(CQ_MONO))
            .on_press(ManualMessage::Pause);
        let resume = Button::new(&mut self.resume_btn, Text::new("Resume").font(CQ_MONO))
            .on_press(ManualMessage::Resume);
        let content = Column::new()
            .max_width(800)
            .spacing(20)
            .push(title)
            .push(button_grid)
            .push(pause)
            .push(resume)
            .push(inbath_toggle);

        Scrollable::new(&mut self.scroll)
            .padding(40)
            .push(Container::new(content).width(Length::Fill).center_x())
            .into()
    }
}
