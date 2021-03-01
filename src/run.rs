use crate::{RecipieState, CQ_MONO};
use iced::{
    button, pick_list, scrollable, Align, Button, Column, Container, Element, HorizontalAlignment,
    Length, PickList, Row, Scrollable, Space, Text, VerticalAlignment,
};

use super::build::ns;
use serde::Deserialize;
use std::sync::{Arc, Condvar, Mutex};
use std::{fs::File, mem::discriminant};

pub struct Run {
    scroll: scrollable::State,
    run_btn: button::State,
    stop_btn: button::State,
    pause_btn: button::State,
    resume_btn: button::State,
    pub search: Vec<String>,
    search_state: pick_list::State<String>,
    search_value: Option<String>,
    recipie_state: Arc<(Mutex<RecipieState>, Condvar)>,
    pub steps: Vec<Step>,
    continue_btns: Vec<Option<ContinueButton>>,
    pub active_recipie: Option<Vec<Step>>,
}

#[derive(Debug, Clone)]
pub enum RunMessage {
    Run,
    Stop,
    Pause,
    Resume,
    TabActive,
    RecipieChanged(String),
    Step,
}

impl Run {
    pub fn new(recipie_state: Arc<(Mutex<RecipieState>, Condvar)>) -> Self {
        Run {
            scroll: scrollable::State::new(),
            run_btn: button::State::new(),
            stop_btn: button::State::new(),
            pause_btn: button::State::new(),
            resume_btn: button::State::new(),
            search: Vec::new(),
            search_state: pick_list::State::default(),
            search_value: None,
            steps: Vec::new(),
            continue_btns: Vec::new(),
            active_recipie: None,
            recipie_state,
        }
    }

    pub fn update(&mut self, message: RunMessage) {
        match message {
            RunMessage::TabActive => {
                if let Some(sv) = self.search_value.clone() {
                    self.update(RunMessage::RecipieChanged(sv))
                }
            }
            RunMessage::Step => {
                for btn in &mut self.continue_btns {
                    if let Some(b) = btn {
                        b.display_value = match b.display_value.overflowing_sub(1) {
                            (n, false) => n,
                            (_, true) => 0,
                        }
                    }
                }
                let (recipie_state, cvar) = &*self.recipie_state;
                let mut recipie_state = recipie_state.lock().unwrap();
                *recipie_state = RecipieState::RecipieRunning;
                cvar.notify_all();
            }
            RunMessage::RecipieChanged(recipie) => {
                match File::open(format!("./recipies/{}.recipie", recipie)) {
                    Ok(file) => {
                        self.steps = Vec::new();
                        self.continue_btns = Vec::new();
                        let mut rdr = csv::Reader::from_reader(file);
                        let mut count = 1; // display vlue tracker
                        for step in rdr.deserialize() {
                            let step: Step = step.unwrap();
                            if step.require_input {
                                self.continue_btns.push(Some(ContinueButton::new(
                                    count,
                                    Arc::clone(&self.recipie_state),
                                )));
                                count += 1;
                            } else {
                                self.continue_btns.push(None)
                            }
                            self.steps.push(step);
                        }
                    }
                    Err(_err) => self.steps = Vec::new(),
                }
                self.search_value = Some(recipie);
            }
            _ => {}
        }
    }

    pub fn view(&mut self) -> Element<RunMessage> {
        let search: Element<_>;
        {
            let (recipie_state, _) = &*self.recipie_state;
            search = match *recipie_state.lock().unwrap() {
                RecipieState::Stopped => Row::new()
                    .push(
                        PickList::new(
                            &mut self.search_state,
                            &self.search[..],
                            self.search_value.clone(),
                            RunMessage::RecipieChanged,
                        )
                        .padding(10)
                        .width(Length::Units(500)),
                    )
                    .into(),
                _ => Row::new()
                    .push(
                        Text::new(self.search_value.clone().unwrap_or("".to_string()))
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .size(30)
                            .font(CQ_MONO),
                    )
                    .padding(6)
                    .into(),
            }
        }
        let run = match self.search_value {
            Some(_) => {
                let (recipie_state, _) = &*self.recipie_state;
                match *recipie_state.lock().unwrap() {
                    RecipieState::Stopped => Row::new().push(
                        Button::new(
                            &mut self.run_btn,
                            Text::new("Run")
                                .size(30)
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .font(CQ_MONO),
                        )
                        .on_press(RunMessage::Run)
                        .padding(10)
                        .width(Length::Units(500)),
                    ),
                    RecipieState::RecipieRunning => Row::new()
                        .push(
                            Button::new(
                                &mut self.stop_btn,
                                Text::new("Stop")
                                    .size(30)
                                    .horizontal_alignment(HorizontalAlignment::Center)
                                    .font(CQ_MONO),
                            )
                            .on_press(RunMessage::Stop)
                            .padding(10)
                            .width(Length::Units(200)),
                        )
                        .push(Space::with_width(Length::Units(100)))
                        .push(
                            Button::new(
                                &mut self.pause_btn,
                                Text::new("Pause")
                                    .size(30)
                                    .horizontal_alignment(HorizontalAlignment::Center)
                                    .font(CQ_MONO),
                            )
                            .on_press(RunMessage::Pause)
                            .padding(10)
                            .width(Length::Units(200)),
                        ),
                    RecipieState::RecipiePaused => Row::new()
                        .push(
                            Button::new(
                                &mut self.stop_btn,
                                Text::new("Stop")
                                    .size(30)
                                    .horizontal_alignment(HorizontalAlignment::Center)
                                    .font(CQ_MONO),
                            )
                            .on_press(RunMessage::Stop)
                            .padding(10)
                            .width(Length::Units(200)),
                        )
                        .push(Space::with_width(Length::Units(100)))
                        .push(
                            Button::new(
                                &mut self.resume_btn,
                                Text::new("Resume")
                                    .size(30)
                                    .horizontal_alignment(HorizontalAlignment::Center)
                                    .font(CQ_MONO),
                            )
                            .on_press(RunMessage::Resume)
                            .padding(10)
                            .width(Length::Units(200)),
                        ),
                    RecipieState::RequireInput => Row::new()
                        .push(
                            Button::new(
                                &mut self.stop_btn,
                                Text::new("Stop")
                                    .size(30)
                                    .horizontal_alignment(HorizontalAlignment::Center)
                                    .font(CQ_MONO),
                            )
                            .on_press(RunMessage::Stop)
                            .padding(10)
                            .width(Length::Units(200)),
                        )
                        .push(Space::with_width(Length::Units(100)))
                        .push(
                            Text::new("Waiting for user input")
                                .font(CQ_MONO)
                                .size(20)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        ),
                    _ => Row::new(),
                }
            }
            None => Row::new(),
        };

        let recipie: Element<_> = self
            .steps
            .iter_mut()
            .zip(self.continue_btns.iter_mut())
            .fold(Column::new().spacing(15), |col, (step, btn)| {
                if let Some(b) = btn {
                    //println!("adding btn {} next to {}",b.display_value, step.selected_destination);
                    col.push(
                        Row::new()
                            .push(step.view().map(move |_msg| RunMessage::Step))
                            .push(b.view().map(move |_msg| RunMessage::Step)),
                    )
                } else {
                    col.push(Row::new().push(step.view().map(move |_msg| RunMessage::Step)))
                }
            })
            .into();

        let content = Column::new()
            .max_width(800)
            .spacing(20)
            .push(search)
            .push(run)
            .push(recipie)
            .align_items(Align::Center);

        Scrollable::new(&mut self.scroll)
            .padding(40)
            .push(Container::new(content).width(Length::Fill).center_x())
            .into()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Step {
    pub step_num: String,
    pub selected_destination: String,
    pub selected_action: String,
    pub secs_value: String,
    pub mins_value: String,
    pub hours_value: String,
    pub in_bath: bool,
    pub require_input: bool,
}

#[derive(Debug, Clone)]
pub enum StepMessage {}

impl Step {
    fn view(&mut self) -> Element<StepMessage> {
        let e = "".to_string(); //empty
        let eb = match self.in_bath {
            true => "\n in bath",
            false => "",
        };
        let ri = match self.require_input {
            true => "Wait for user input then\n ",
            false => "",
        };
        let step_time_text = match (
            self.hours_value.clone(),
            self.mins_value.clone(),
            self.secs_value.clone(),
        ) {
            (h, m, s) if h == e && m == e && s == e => {
                format!("{}{} for 0 seconds", ri, self.selected_action)
            }
            (h, m, s) if h == e && m == e => {
                format!("{}{} for {} second{}", ri, self.selected_action, s, ns(&s))
            }
            (h, m, s) if h == e && s == e => {
                format!("{}{} for {} minute{}", ri, self.selected_action, m, ns(&m))
            }
            (h, m, s) if m == e && s == e => {
                format!("{}{} for {} hour{}", ri, self.selected_action, h, ns(&h))
            }
            (h, m, s) if h == e => format!(
                "{}{} for {} minute{} and {} second{}",
                ri,
                self.selected_action,
                m,
                ns(&m),
                s,
                ns(&s)
            ),
            (h, m, s) if m == e => format!(
                "{}{} for {} hour{} and {} second{}",
                ri,
                self.selected_action,
                h,
                ns(&h),
                s,
                ns(&s)
            ),
            (h, m, s) if s == e => format!(
                "{}{} for {} hour{} and {} minute{}",
                ri,
                self.selected_action,
                h,
                ns(&h),
                m,
                ns(&m)
            ),
            (h, m, s) => format!(
                "{}{} for {} hour{}, {} minute{} and {} second{}",
                ri,
                self.selected_action,
                h,
                ns(&h),
                m,
                ns(&m),
                s,
                ns(&s)
            ),
        };
        let content = Row::new()
            .align_items(Align::Center)
            .push(
                Text::new(format!("{}", self.step_num))
                    .width(Length::Units(75))
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .font(CQ_MONO),
            )
            .push(
                // Destination
                Column::new().push(
                    Text::new(format!("{}{}", self.selected_destination, eb))
                        .width(Length::Units(120))
                        .vertical_alignment(VerticalAlignment::Center)
                        .font(CQ_MONO),
                ),
            )
            .push(
                // action
                Column::new()
                    .push(
                        Text::new(step_time_text)
                            .vertical_alignment(VerticalAlignment::Center)
                            .font(CQ_MONO),
                    )
                    .width(Length::Shrink)
                    .align_items(Align::Center),
            );

        content.into()
    }
}

#[derive(Debug, Clone)]
pub enum ContinueButtonMessage {
    Continue,
}

struct ContinueButton {
    display_value: usize, // keep track of what buttons have been shown and what order they are in such that only one is shown when paused.
    recipie_state: Arc<(Mutex<RecipieState>, Condvar)>,
    continue_btn: button::State,
}

impl ContinueButton {
    fn new(display_value: usize, recipie_state: Arc<(Mutex<RecipieState>, Condvar)>) -> Self {
        ContinueButton {
            display_value,
            recipie_state,
            continue_btn: button::State::new(),
        }
    }

    fn view(&mut self) -> Element<ContinueButtonMessage> {
        if self.display_value == 1 {
            let (recipie_state, _) = &*self.recipie_state;
            if discriminant(&*recipie_state.lock().unwrap())
                == discriminant(&RecipieState::RequireInput)
            {
                Column::new()
                    .push(
                        Row::new().push(Space::with_width(Length::Units(25))).push(
                            Button::new(
                                &mut self.continue_btn,
                                Text::new("Continue").font(CQ_MONO).size(30),
                            )
                            .width(Length::Shrink)
                            .padding(10)
                            .on_press(ContinueButtonMessage::Continue),
                        ),
                    )
                    .into()
            } else {
                Column::new().into()
            }
        } else {
            Column::new().into()
        }
    }
}
