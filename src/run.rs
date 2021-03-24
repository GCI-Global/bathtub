use crate::{RecipeState, CQ_MONO};
use iced::{
    button, pick_list, scrollable, text_input, tooltip, Align, Button, Column, Command, Container,
    Element, HorizontalAlignment, Length, PickList, Row, Scrollable, Space, Text, TextInput,
    Tooltip, VerticalAlignment,
};

use super::build::{attention_icon, ns, pause_icon, play_icon, Input, Recipe, SaveRecipe};
use super::logger::Logger;
use super::style::style::Theme;
use chrono::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::{fs, mem::discriminant};

pub struct Run {
    scroll: scrollable::State,
    large_start_btn: button::State,
    start_btn: button::State,
    cancel_btn: button::State,
    finish_btn: button::State,
    stop_btn: button::State,
    stop_confirm_btn: button::State,
    pause_btn: button::State,
    resume_btn: button::State,
    pub search_options: Vec<String>,
    search_state: pick_list::State<String>,
    pub search_value: Option<String>,
    recipe_regex: Regex,
    recipe_state: Arc<(Mutex<RecipeState>, Condvar)>,
    pub recipe: Option<Recipe>,
    continue_btns: Vec<Option<ContinueButton>>,
    pub active_recipie: Option<Vec<Step>>,
    pub state: RunState,
    required_before_inputs: Vec<RequiredInput>,
    pub required_after_inputs: Vec<RequiredInput>,
    logger: Logger,
    homing_required: Rc<RefCell<bool>>,
}

#[derive(Debug, Clone)]
pub enum RunState {
    Standard,
    BeforeRequiredInput,
    AfterRequiredInput,
    StopConfirm,
}

#[derive(Debug, Clone)]
pub enum RunMessage {
    Start,
    Cancel,
    Run(()),
    Finish,
    Stop,
    RequireStopConfirm,
    Pause(()),
    Resume,
    UpdateSearch,
    SearchChanged(String),
    RequiredBeforeInput(usize, RequiredInputMessage),
    RequiredAfterInput(usize, RequiredInputMessage),
    Step,
}

impl Run {
    pub fn new(
        recipe_state: Arc<(Mutex<RecipeState>, Condvar)>,
        logger: Logger,
        homing_required: Rc<RefCell<bool>>,
    ) -> Self {
        Run {
            scroll: scrollable::State::new(),
            large_start_btn: button::State::new(),
            start_btn: button::State::new(),
            cancel_btn: button::State::new(),
            finish_btn: button::State::new(),
            stop_btn: button::State::new(),
            stop_confirm_btn: button::State::new(),
            pause_btn: button::State::new(),
            resume_btn: button::State::new(),
            search_options: Vec::new(),
            search_state: pick_list::State::default(),
            search_value: None,
            continue_btns: Vec::new(),
            recipe: None,
            active_recipie: None,
            recipe_regex: Regex::new(r"^[^.]+").unwrap(),
            recipe_state,
            state: RunState::Standard,
            required_before_inputs: Vec::new(),
            required_after_inputs: Vec::new(),
            logger,
            homing_required,
        }
    }

    pub fn update(&mut self, message: RunMessage) -> Command<RunMessage> {
        let mut command = Command::none();
        match message {
            RunMessage::Step => {
                for btn in &mut self.continue_btns {
                    if let Some(b) = btn {
                        b.display_value = match b.display_value.overflowing_sub(1) {
                            (n, false) => n,
                            (_, true) => 0,
                        }
                    }
                }
                let (recipe_state, cvar) = &*self.recipe_state;
                let mut recipe_state = recipe_state.lock().unwrap();
                *recipe_state = RecipeState::RecipeRunning;
                cvar.notify_all();
            }
            RunMessage::UpdateSearch => {
                // check for and update with new recipe files
                self.search_options =
                    fs::read_dir("./recipes")
                        .unwrap()
                        .fold(Vec::new(), |mut rec, file| {
                            if let Some(caps) = self
                                .recipe_regex
                                .captures(&file.unwrap().file_name().to_str().unwrap())
                            {
                                rec.push(caps[0].to_string());
                            }
                            rec
                        });
                self.search_options.sort();
                // update the ui with recipie if it was changed
                if !self
                    .search_options
                    .iter()
                    .any(|o| o == self.search_value.as_ref().unwrap_or(&String::new()))
                {
                    self.search_value = None
                }
                update_recipe(self);
            }
            RunMessage::SearchChanged(recipe) => {
                self.search_value = Some(recipe);
                update_recipe(self);
            }
            RunMessage::Start => {
                if self.required_before_inputs.len() == 0
                    || self
                        .required_before_inputs
                        .iter()
                        .any(|input| input.input_value != "".to_string())
                {
                    let log_title = format!(
                        "{}| Run - {}",
                        Local::now().to_rfc2822(),
                        self.search_value.as_ref().unwrap()
                    );
                    self.logger.set_log_file(log_title.clone());
                    self.logger
                        .send_line("--------------------".to_string())
                        .unwrap();
                    for input in &self.required_before_inputs {
                        self.logger
                            .send_line(format!(
                                "{} => {}: {}",
                                Local::now().to_rfc2822(),
                                &input.title,
                                &input.input_value
                            ))
                            .unwrap();
                    }
                    self.logger
                        .send_line("--------------------".to_string())
                        .unwrap();
                    self.state = RunState::Standard;
                    command = Command::perform(do_nothing(), RunMessage::Run);
                } else {
                    self.state = RunState::BeforeRequiredInput
                }
            }
            RunMessage::Finish => {
                if self.required_after_inputs.len() == 0
                    || self
                        .required_after_inputs
                        .iter()
                        .any(|input| input.input_value != "".to_string())
                {
                    self.logger
                        .send_line("--------------------".to_string())
                        .unwrap();
                    for input in &self.required_after_inputs {
                        self.logger
                            .send_line(format!(
                                "{} => {}: {}",
                                Local::now().to_rfc2822(),
                                input.title,
                                input.input_value
                            ))
                            .unwrap();
                    }
                    self.logger.set_log_file("".to_string());
                    self.state = RunState::Standard;
                }
            }
            RunMessage::Cancel => {
                for input in &mut self.required_before_inputs {
                    input.input_value = "".to_string();
                }
                self.state = RunState::Standard;
            }
            RunMessage::RequiredBeforeInput(i, msg) => self.required_before_inputs[i].update(msg),
            RunMessage::RequiredAfterInput(i, msg) => self.required_after_inputs[i].update(msg),
            RunMessage::RequireStopConfirm => {
                self.state = RunState::StopConfirm;
                command = Command::perform(do_nothing(), RunMessage::Pause);
            }
            // handled in main.rs
            RunMessage::Run(_) => {}
            RunMessage::Stop => {}
            RunMessage::Pause(_) => {}
            RunMessage::Resume => {}
        };
        command
    }

    pub fn view(&mut self) -> Element<RunMessage> {
        match self.state {
            RunState::Standard => {
                let search: Element<_>;
                {
                    let (recipe_state, _) = &*self.recipe_state;
                    search = match *recipe_state.lock().unwrap() {
                        RecipeState::Stopped => Row::new()
                            .push(
                                PickList::new(
                                    &mut self.search_state,
                                    &self.search_options[..],
                                    self.search_value.clone(),
                                    RunMessage::SearchChanged,
                                )
                                .style(Theme::Blue)
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
                        let (recipe_state, _) = &*self.recipe_state;
                        match *recipe_state.lock().unwrap() {
                            RecipeState::Stopped => {
                                Row::new().push(if let Some(recipe) = &self.recipe {
                                    if recipe.steps.len() > 0 {
                                        Tooltip::new(
                                            Button::new(
                                                &mut self.large_start_btn,
                                                Text::new("Start")
                                                    .size(30)
                                                    .horizontal_alignment(
                                                        HorizontalAlignment::Center,
                                                    )
                                                    .font(CQ_MONO),
                                            )
                                            .style(Theme::Green)
                                            .on_press(RunMessage::Start)
                                            .padding(10)
                                            .width(Length::Units(500)),
                                            if *self.homing_required.borrow() {
                                                "Will run homing cycle first!"
                                            } else {
                                                ""
                                            },
                                            tooltip::Position::FollowCursor,
                                        )
                                        .size(25)
                                        .padding(5)
                                        .style(
                                            if *self.homing_required.borrow() {
                                                Theme::Active
                                            } else {
                                                Theme::Disabled
                                            },
                                        )
                                    } else {
                                        Tooltip::new(
                                            Space::with_width(Length::Shrink),
                                            "",
                                            tooltip::Position::Top,
                                        )
                                    }
                                } else {
                                    Tooltip::new(
                                        Space::with_width(Length::Shrink),
                                        "",
                                        tooltip::Position::Top,
                                    )
                                })
                            }
                            RecipeState::RecipeRunning => Row::new()
                                .push(
                                    Button::new(
                                        &mut self.stop_btn,
                                        Text::new("Stop")
                                            .size(30)
                                            .horizontal_alignment(HorizontalAlignment::Center)
                                            .font(CQ_MONO),
                                    )
                                    .style(Theme::Red)
                                    .on_press(RunMessage::RequireStopConfirm)
                                    .padding(10)
                                    .width(Length::Units(200)),
                                )
                                .push(Space::with_width(Length::Units(100)))
                                .push(
                                    Button::new(
                                        &mut self.pause_btn,
                                        pause_icon()
                                            .size(30)
                                            .horizontal_alignment(HorizontalAlignment::Center),
                                    )
                                    .style(Theme::Blue)
                                    .on_press(RunMessage::Pause(()))
                                    .padding(10)
                                    .width(Length::Units(200)),
                                ),
                            RecipeState::HomingRun => Row::new()
                                .push(
                                    Tooltip::new(
                                        Button::new(
                                            &mut self.stop_btn,
                                            Text::new("Stop")
                                                .size(30)
                                                .horizontal_alignment(HorizontalAlignment::Center)
                                                .font(CQ_MONO),
                                        )
                                        .style(Theme::RedDisabled)
                                        .padding(10)
                                        .width(Length::Units(200)),
                                        "Unavailable while homing!",
                                        tooltip::Position::FollowCursor,
                                    )
                                    .style(Theme::Red),
                                )
                                .push(Space::with_width(Length::Units(100)))
                                .push(
                                    Tooltip::new(
                                        Button::new(
                                            &mut self.pause_btn,
                                            pause_icon()
                                                .size(30)
                                                .horizontal_alignment(HorizontalAlignment::Center),
                                        )
                                        .style(Theme::BlueDisabled)
                                        .padding(10)
                                        .width(Length::Units(200)),
                                        "Unavailable while homing!",
                                        tooltip::Position::FollowCursor,
                                    )
                                    .style(Theme::Red),
                                ),
                            RecipeState::RecipePaused => Row::new()
                                .push(
                                    Button::new(
                                        &mut self.stop_btn,
                                        Text::new("Stop")
                                            .size(30)
                                            .horizontal_alignment(HorizontalAlignment::Center)
                                            .font(CQ_MONO),
                                    )
                                    .style(Theme::Red)
                                    .on_press(RunMessage::RequireStopConfirm)
                                    .padding(10)
                                    .width(Length::Units(200)),
                                )
                                .push(Space::with_width(Length::Units(100)))
                                .push(
                                    Button::new(
                                        &mut self.resume_btn,
                                        play_icon()
                                            .size(30)
                                            .horizontal_alignment(HorizontalAlignment::Center),
                                    )
                                    .style(Theme::Blue)
                                    .on_press(RunMessage::Resume)
                                    .padding(10)
                                    .width(Length::Units(200)),
                                ),
                            RecipeState::RequireInput => Row::new()
                                .push(
                                    Button::new(
                                        &mut self.stop_btn,
                                        Text::new("Stop")
                                            .size(30)
                                            .horizontal_alignment(HorizontalAlignment::Center)
                                            .font(CQ_MONO),
                                    )
                                    .on_press(RunMessage::RequireStopConfirm)
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

                let recipie: Element<_> =
                    match self.recipe.as_mut() {
                        Some(recipe) => {
                            if recipe.steps.len() > 0 {
                                recipe
                                    .steps
                                    .iter_mut()
                                    .zip(self.continue_btns.iter_mut())
                                    .enumerate()
                                    .fold(Column::new(), |col, (i, (step, btn))| {
                                        if let Some(b) = btn {
                                            col.push(
                                                Container::new(
                                                    Column::new()
                                                        .height(Length::Units(50))
                                                        .push(Space::with_height(Length::Fill))
                                                        .push(
                                                            Row::new()
                                                                .width(Length::Units(500))
                                                                .push(step.view().map(
                                                                    move |_msg| RunMessage::Step,
                                                                ))
                                                                .push(b.view().map(move |_msg| {
                                                                    RunMessage::Step
                                                                })),
                                                        )
                                                        .push(Space::with_height(Length::Fill)),
                                                )
                                                .style(if i % 2 == 0 {
                                                    Theme::LightGray
                                                } else {
                                                    Theme::LighterGray
                                                }),
                                            )
                                        } else {
                                            col.push(
                                                Container::new(
                                                    Column::new()
                                                        .height(Length::Units(50))
                                                        .push(Space::with_height(Length::Fill))
                                                        .push(
                                                            Row::new()
                                                                .width(Length::Units(500))
                                                                .push(step.view().map(
                                                                    move |_msg| RunMessage::Step,
                                                                )),
                                                        )
                                                        .push(Space::with_height(Length::Fill)),
                                                )
                                                .style(if i % 2 == 0 {
                                                    Theme::LightGray
                                                } else {
                                                    Theme::LighterGray
                                                }),
                                            )
                                        }
                                    })
                                    .into()
                            } else {
                                Container::new(
                                    Row::with_children(vec![
                                        Space::with_width(Length::Fill).into(),
                                        Text::new("This Recipie has no steps. It cannot be run.")
                                            .into(),
                                        Space::with_width(Length::Fill).into(),
                                    ])
                                    .padding(10),
                                )
                                .style(Theme::Red)
                                .into()
                            }
                        }
                        None => Column::new().into(),
                    };

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
            RunState::StopConfirm => {
                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(
                        Text::new("Paused").font(CQ_MONO).size(40)    
                    )
                    .push(Text::new("Are you sure you want to stop this Recipie?\nYou cannot return to this point in the recipie if you do.").horizontal_alignment(HorizontalAlignment::Center).size(30))
                            .push(Row::with_children(vec![
                                Space::with_width(Length::Fill).into(),
                                Button::new(
                                    &mut self.stop_confirm_btn,
                                    Text::new("Yes, stop now.")
                                        .font(CQ_MONO)
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                ).style(Theme::Red)
                                .on_press(RunMessage::Stop)
                                .padding(10)
                                .width(Length::Units(200))
                                .into(),
                                Space::with_width(Length::Units(100)).into(),
                                Button::new(
                                    &mut self.cancel_btn,
                                    Text::new("No, just pause.")
                                        .font(CQ_MONO)
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                ).style(Theme::Blue)
                                .on_press(RunMessage::Cancel)
                                .width(Length::Units(200))
                                .padding(10)
                                .into(),
                                Space::with_width(Length::Fill).into(),
                            ]));
                Scrollable::new(&mut self.scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
            RunState::BeforeRequiredInput => {
                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(
                        self.required_before_inputs
                            .iter_mut()
                            .enumerate()
                            .fold(Column::new().spacing(10), |col, (i, input)| {
                                col.push(
                                    input
                                        .view()
                                        .map(move |msg| RunMessage::RequiredBeforeInput(i, msg)),
                                )
                            })
                            .push(Row::with_children(vec![
                                Space::with_width(Length::Fill).into(),
                                Button::new(
                                    &mut self.start_btn,
                                    Text::new("Start")
                                        .font(CQ_MONO)
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                )
                                .style(Theme::Green)
                                .on_press(RunMessage::Start)
                                .padding(10)
                                .width(Length::Units(200))
                                .into(),
                                Space::with_width(Length::Units(100)).into(),
                                Button::new(
                                    &mut self.cancel_btn,
                                    Text::new("Cancel")
                                        .font(CQ_MONO)
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                )
                                .style(Theme::Red)
                                .on_press(RunMessage::Cancel)
                                .width(Length::Units(200))
                                .padding(10)
                                .into(),
                                Space::with_width(Length::Fill).into(),
                            ])),
                    );
                Scrollable::new(&mut self.scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
            RunState::AfterRequiredInput => {
                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(
                        self.required_after_inputs
                            .iter_mut()
                            .enumerate()
                            .fold(Column::new().spacing(10), |col, (i, input)| {
                                col.push(
                                    input
                                        .view()
                                        .map(move |msg| RunMessage::RequiredAfterInput(i, msg)),
                                )
                            })
                            .push(Row::with_children(vec![
                                Space::with_width(Length::Fill).into(),
                                Button::new(
                                    &mut self.finish_btn,
                                    Text::new("Finish")
                                        .font(CQ_MONO)
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                )
                                .style(Theme::Blue)
                                .on_press(RunMessage::Finish)
                                .padding(10)
                                .width(Length::Units(500))
                                .into(),
                                Space::with_width(Length::Fill).into(),
                            ])),
                    );
                Scrollable::new(&mut self.scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
        }
    }
}

pub struct RequiredInput {
    input_state: text_input::State,
    input_value: String,
    title: String,
}

#[derive(Debug, Clone)]
pub enum RequiredInputMessage {
    InputChanged(String),
}

impl RequiredInput {
    fn new(title: String) -> Self {
        RequiredInput {
            title,
            input_value: "".to_string(),
            input_state: text_input::State::new(),
        }
    }

    fn update(&mut self, message: RequiredInputMessage) {
        match message {
            RequiredInputMessage::InputChanged(input) => self.input_value = input,
        }
    }

    fn view(&mut self) -> Element<'_, RequiredInputMessage> {
        Row::with_children(vec![
            Column::new()
                .push(Space::with_height(Length::Units(10)))
                .push(
                    Text::new(format!("{}:", &self.title))
                        .font(CQ_MONO)
                        .size(20)
                        .width(Length::Units(200)),
                )
                .into(),
            TextInput::new(
                &mut self.input_state,
                "",
                &self.input_value,
                RequiredInputMessage::InputChanged,
            )
            .style(Theme::Blue)
            .padding(10)
            .into(),
        ])
        .into()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Step {
    pub step_num: String,
    pub selected_destination: String,
    pub selected_action: String,
    pub secs_value: String,
    pub mins_value: String,
    pub hours_value: String,
    pub hover: bool,
    pub wait: bool,
}

#[derive(Debug, Clone)]
pub enum StepMessage {}

impl Step {
    fn view(&mut self) -> Element<StepMessage> {
        let e = "".to_string(); //empty
        let eb = match self.hover {
            true => "\n Hover Above",
            false => "",
        };
        let ri = match self.wait {
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
    recipie_state: Arc<(Mutex<RecipeState>, Condvar)>,
    continue_btn: button::State,
}

impl ContinueButton {
    fn new(display_value: usize, recipie_state: Arc<(Mutex<RecipeState>, Condvar)>) -> Self {
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
                == discriminant(&RecipeState::RequireInput)
            {
                Column::new()
                    .push(
                        Row::new().push(Space::with_width(Length::Units(25))).push(
                            Button::new(&mut self.continue_btn, attention_icon().size(30))
                                .width(Length::Units(50))
                                .padding(10)
                                .on_press(ContinueButtonMessage::Continue)
                                .style(Theme::Yellow),
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

pub async fn do_nothing() {
    ()
}
fn update_recipe(tab: &mut Run) {
    match &fs::read_to_string(format!(
        "./recipes/{}.toml",
        tab.search_value.as_ref().unwrap_or(&String::new())
    )) {
        Ok(toml_str) => {
            let save_rec: SaveRecipe = toml::from_str(toml_str).unwrap();
            let rec = Recipe {
                required_inputs: Input {
                    before: if let Some(b) = save_rec.required_inputs.before {
                        b
                    } else {
                        Vec::new()
                    },
                    after: if let Some(a) = save_rec.required_inputs.after {
                        a
                    } else {
                        Vec::new()
                    },
                },
                steps: if let Some(s) = save_rec.steps {
                    s
                } else {
                    Vec::new()
                },
            };
            tab.continue_btns = Vec::with_capacity(rec.steps.len());
            let mut count = 1;
            for step in &rec.steps {
                if step.wait {
                    tab.continue_btns.push(Some(ContinueButton::new(
                        count,
                        Arc::clone(&tab.recipe_state),
                    )));
                    count += 1;
                } else {
                    tab.continue_btns.push(None);
                }
            }
            tab.required_before_inputs = rec.required_inputs.before.iter().fold(
                Vec::with_capacity(rec.required_inputs.before.len()),
                |mut v, input| {
                    v.push(RequiredInput::new(input.clone()));
                    v
                },
            );
            tab.required_after_inputs = rec.required_inputs.after.iter().fold(
                Vec::with_capacity(rec.required_inputs.after.len()),
                |mut v, input| {
                    v.push(RequiredInput::new(input.clone()));
                    v
                },
            );
            tab.recipe = Some(rec);
        }
        // TODO: Display Error when unable to read file
        Err(_err) => {
            tab.recipe = None;
        }
    }
}
