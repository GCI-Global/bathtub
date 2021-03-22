use super::actions::Actions;
use super::advanced::{SaveBar, SaveBarMessage};
use super::logger::Logger;
use super::nodes::Nodes;
use super::run::do_nothing;
use super::run::Step;
use super::style::style::Theme;
use crate::{TabState, CQ_MONO};
use chrono::prelude::*;
use iced::{
    button, pick_list, scrollable, text_input, Align, Button, Checkbox, Column, Command, Container,
    Element, Font, HorizontalAlignment, Length, PickList, Row, Scrollable, Space, Text, TextInput,
    VerticalAlignment,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

pub struct Build {
    unsaved: bool,
    save_bar: SaveBar,
    scroll: scrollable::State,
    nodes_ref: Rc<RefCell<Nodes>>,
    actions_ref: Rc<RefCell<Actions>>,
    steps: Vec<BuildStep>,
    before_inputs: Vec<RequiredInput>,
    after_inputs: Vec<RequiredInput>,
    modified_steps: Vec<BuildStep>,
    modified_before_inputs: Vec<RequiredInput>,
    modified_after_inputs: Vec<RequiredInput>,
    add_step: AddStep,
    recipe_name: text_input::State,
    recipe_name_value: String,
    required_input_tab_btn: button::State,
    steps_tab_btn: button::State,
    add_input_before_btn: button::State,
    add_input_after_btn: button::State,
    state: BuildState,
    logger: Logger,
    unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
}

enum BuildState {
    Steps,
    RequiredInput,
}

#[derive(Debug, Clone)]
pub enum BuildMessage {
    StepMessage(usize, StepMessage),
    BeforeRequiredInputMessage(usize, RequiredInputMessage),
    AfterRequiredInputMessage(usize, RequiredInputMessage),
    AddStepMessage(AddStepMessage),
    UserChangedName(String),
    SaveMessage(SaveBarMessage),
    StepsTab,
    RequiredInputTab,
    AddInputBefore,
    AddInputAfter,
    Saved(()),
}

impl Build {
    pub fn new(
        nodes_ref: Rc<RefCell<Nodes>>,
        actions_ref: Rc<RefCell<Actions>>,
        logger: Logger,
        unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
    ) -> Self {
        Build {
            unsaved: false,
            save_bar: SaveBar::new(),
            scroll: scrollable::State::new(),
            add_step: AddStep::new(1, 0, Rc::clone(&nodes_ref), Rc::clone(&actions_ref)),
            nodes_ref,
            actions_ref,
            recipe_name: text_input::State::new(),
            recipe_name_value: "".to_string(),
            required_input_tab_btn: button::State::new(),
            steps_tab_btn: button::State::new(),
            add_input_before_btn: button::State::new(),
            add_input_after_btn: button::State::new(),
            steps: Vec::new(),
            before_inputs: Vec::new(),
            after_inputs: Vec::new(),
            modified_steps: Vec::new(),
            modified_before_inputs: Vec::new(),
            modified_after_inputs: Vec::new(),
            state: BuildState::Steps,
            logger,
            unsaved_tabs,
        }
    }

    pub fn update(&mut self, message: BuildMessage) -> Command<BuildMessage> {
        let mut command = Command::none();
        match message {
            BuildMessage::StepMessage(i, StepMessage::Delete) => {
                self.modified_steps.remove(i);
                for i in 0..self.modified_steps.len() {
                    self.modified_steps[i].step_num = Some(i + 1);
                }
                for i in 0..self.modified_steps.len() {
                    self.modified_steps[i].steps_len = self.modified_steps.len();
                }
                self.add_step.step_num = Some(self.modified_steps.len() + 1);
                self.add_step.steps_len = self.modified_steps.len();
            }
            BuildMessage::StepMessage(i, StepMessage::NewNum(num)) => {
                self.modified_steps[i].step_num = Some(num);
                if num <= i {
                    for j in num - 1..i {
                        self.modified_steps[j].step_num =
                            Some(self.modified_steps[j].step_num.unwrap() + 1);
                    }
                } else {
                    for j in i + 1..num {
                        self.modified_steps[j].step_num =
                            Some(self.modified_steps[j].step_num.unwrap() - 1);
                    }
                }
                self.modified_steps
                    .sort_by(|a, b| a.step_num.partial_cmp(&b.step_num).unwrap());
            }
            BuildMessage::StepMessage(i, StepMessage::Edit) => {
                self.unsaved = true;
                self.unsaved_tabs.borrow_mut().insert(TabState::Build, true);
                // reset all to idle so ony one can be edited at a time
                for step in &mut self.modified_steps {
                    step.update(StepMessage::Okay)
                }
                self.modified_steps[i].update(StepMessage::Edit)
            }
            BuildMessage::StepMessage(i, msg) => {
                if let Some(step) = self.modified_steps.get_mut(i) {
                    step.update(msg)
                }
            }
            BuildMessage::AddStepMessage(AddStepMessage::Add(
                dest,
                hover,
                action,
                nodes,
                hours,
                mins,
                secs,
                req_input,
            )) => {
                if let Some(d) = dest {
                    self.unsaved = true;
                    self.unsaved_tabs.borrow_mut().insert(TabState::Build, true);
                    for i in nodes.unwrap() - 1..self.modified_steps.len() {
                        self.modified_steps[i].step_num =
                            Some(self.modified_steps[i].step_num.unwrap() + 1);
                    }
                    self.modified_steps.push(BuildStep::new(
                        nodes,
                        self.steps.len(),
                        Rc::clone(&self.nodes_ref),
                        Rc::clone(&self.actions_ref),
                        Some(d),
                        hover,
                        action,
                        hours,
                        mins,
                        secs,
                        req_input,
                    ));

                    self.modified_steps
                        .sort_by(|a, b| a.step_num.partial_cmp(&b.step_num).unwrap());
                    for i in 0..self.modified_steps.len() {
                        self.modified_steps[i].steps_len = self.modified_steps.len();
                    }
                    self.scroll.scroll_to_bottom();
                    self.add_step.step_num = Some(self.modified_steps.len() + 1);
                    self.add_step.steps_len = self.modified_steps.len();
                    self.add_step.hours_value = "".to_string();
                    self.add_step.mins_value = "".to_string();
                    self.add_step.secs_value = "".to_string();
                    self.add_step.hover = false;
                    self.add_step.wait = false;
                }
            }
            BuildMessage::AddStepMessage(msg) => self.add_step.update(msg),
            BuildMessage::RequiredInputTab => self.state = BuildState::RequiredInput,
            BuildMessage::StepsTab => self.state = BuildState::Steps,
            BuildMessage::BeforeRequiredInputMessage(i, RequiredInputMessage::Delete) => {
                self.unsaved = true;
                self.unsaved_tabs.borrow_mut().insert(TabState::Build, true);
                self.modified_before_inputs.remove(i);
            }
            BuildMessage::BeforeRequiredInputMessage(i, msg) => {
                self.modified_before_inputs[i].update(msg)
            }
            BuildMessage::AddInputBefore => {
                self.unsaved = true;
                self.unsaved_tabs.borrow_mut().insert(TabState::Build, true);
                self.modified_before_inputs
                    .push(RequiredInput::new("".to_string()));
            }
            BuildMessage::AfterRequiredInputMessage(i, RequiredInputMessage::Delete) => {
                self.unsaved = true;
                self.unsaved_tabs.borrow_mut().insert(TabState::Build, true);
                self.modified_after_inputs.remove(i);
            }
            BuildMessage::AfterRequiredInputMessage(i, msg) => {
                self.modified_after_inputs[i].update(msg)
            }
            BuildMessage::AddInputAfter => {
                self.unsaved = true;
                self.unsaved_tabs.borrow_mut().insert(TabState::Build, true);
                self.modified_after_inputs
                    .push(RequiredInput::new("".to_string()));
            }
            BuildMessage::UserChangedName(new_name) => self.recipe_name_value = new_name,
            BuildMessage::SaveMessage(SaveBarMessage::Cancel) => {
                self.modified_steps = self.steps.clone();
                self.modified_before_inputs = self.before_inputs.clone();
                self.modified_after_inputs = self.after_inputs.clone();
                self.unsaved = false;
                self.unsaved_tabs
                    .borrow_mut()
                    .insert(TabState::Build, false);
            }
            BuildMessage::SaveMessage(SaveBarMessage::Save) => {
                if self.recipe_name_value != "".to_string() {
                    self.modified_before_inputs
                        .retain(|input| input.value != "".to_string());
                    self.modified_after_inputs
                        .retain(|input| input.value != "".to_string());
                    let save_data = Recipe {
                        required_inputs: Input {
                            before: self.modified_before_inputs.iter().fold(
                                Vec::with_capacity(self.modified_before_inputs.len()),
                                |mut v, input| {
                                    v.push(input.value.clone());
                                    v
                                },
                            ),
                            after: self.modified_after_inputs.iter().fold(
                                Vec::with_capacity(self.modified_after_inputs.len()),
                                |mut v, input| {
                                    v.push(input.value.clone());
                                    v
                                },
                            ),
                        },
                        steps: self.modified_steps.iter().fold(
                            Vec::with_capacity(self.modified_steps.len()),
                            |mut v, step| {
                                v.push(Step {
                                    step_num: step.step_num.unwrap().to_string(),
                                    selected_destination: step
                                        .selected_destination
                                        .clone()
                                        .unwrap(),
                                    hover: step.hover,
                                    selected_action: step.selected_action.clone().unwrap(),
                                    secs_value: step.secs_value.clone(),
                                    mins_value: step.mins_value.clone(),
                                    hours_value: step.hours_value.clone(),
                                    wait: step.wait,
                                });
                                v
                            },
                        ),
                    };
                    // TODO: add error is unable to build toml
                    let old_recipe = fs::read_to_string(Path::new(&format!(
                        "./recipes/{}",
                        &self.recipe_name_value
                    )))
                    .unwrap_or(String::new());
                    let new_recipe = toml::to_string_pretty(&save_data).unwrap();
                    match OpenOptions::new().write(true).open(Path::new(&format!(
                        "./recipes/{}.toml",
                        &self.recipe_name_value
                    ))) {
                        Ok(mut file) => {
                            // file already exists, thus we need to log that recipe was changed
                            write!(file, "{}", new_recipe).unwrap();
                            self.logger.set_log_file(format!(
                                "{}| Build - Changed '{}'",
                                Local::now().to_rfc2822(),
                                self.recipe_name_value
                            ));
                            self.logger.send_line(String::new()).unwrap();
                            self.logger
                                .send_line("Recipe changed from:".to_string())
                                .unwrap();
                            self.logger.send_line(old_recipe).unwrap();
                            self.logger
                                .send_line("\n\n\nRecipe changed to:".to_string())
                                .unwrap();
                            self.logger.send_line(new_recipe).unwrap();
                        }
                        Err(_) => {
                            // new file thus new recipie, log should state created recipe
                            println!("{:?}", self.recipe_name_value);
                            let mut file = OpenOptions::new()
                                .create(true)
                                .write(true)
                                .open(Path::new(&format!(
                                    "./recipes/{}.toml",
                                    &self.recipe_name_value
                                )))
                                .unwrap();
                            write!(file, "{}", new_recipe).unwrap();
                            self.logger.set_log_file(format!(
                                "{}| Build - Created '{}'",
                                Local::now().to_rfc2822(),
                                self.recipe_name_value
                            ));
                            self.logger.send_line(String::new()).unwrap();
                            self.logger
                                .send_line(format!(
                                    "Created Recipe '{}' as:",
                                    self.recipe_name_value
                                ))
                                .unwrap();
                            self.logger.send_line(new_recipe).unwrap();
                        }
                    }
                    self.steps = self.modified_steps.clone();
                    self.before_inputs = self.modified_before_inputs.clone();
                    self.after_inputs = self.modified_after_inputs.clone();
                    self.unsaved = false;
                    self.unsaved_tabs
                        .borrow_mut()
                        .insert(TabState::Build, false);
                    // TODO: Have errors show to user if unable to save
                    // TODO: Have different logging if name is unique vs changed name
                    command = Command::perform(do_nothing(), BuildMessage::Saved);
                }
            }
            _ => {}
        }
        command
    }
    pub fn view(&mut self) -> Element<BuildMessage> {
        let save_bar = match self.unsaved {
            true => Column::new().align_items(Align::Center).push(
                self.save_bar
                    .view()
                    .map(move |msg| BuildMessage::SaveMessage(msg)),
            ),
            false => Column::new()
                .align_items(Align::Center)
                .push(Space::with_height(Length::Units(50))),
        };
        let tab_btns = Column::new().align_items(Align::Center).push(
            Row::new()
                .push(Space::with_width(Length::Fill))
                .push(
                    Button::new(
                        &mut self.steps_tab_btn,
                        Text::new("Steps")
                            .font(CQ_MONO)
                            .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .style(match self.state {
                        BuildState::Steps => Theme::BlueBorderOnly,
                        _ => Theme::Blue,
                    })
                    .padding(10)
                    .on_press(BuildMessage::StepsTab)
                    .width(Length::Units(200)),
                )
                .push(
                    Button::new(
                        &mut self.required_input_tab_btn,
                        Text::new("Required Input")
                            .font(CQ_MONO)
                            .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .style(match self.state {
                        BuildState::RequiredInput => Theme::BlueBorderOnly,
                        _ => Theme::Blue,
                    })
                    .padding(10)
                    .on_press(BuildMessage::RequiredInputTab)
                    .width(Length::Units(200)),
                )
                .push(Space::with_width(Length::Fill)),
        );
        let search = Row::new().height(Length::Units(40)).push(
            TextInput::new(
                &mut self.recipe_name,
                "Recipie Name",
                &self.recipe_name_value,
                BuildMessage::UserChangedName,
            )
            .style(Theme::Blue)
            .padding(10)
            .width(Length::Fill),
        );
        match self.state {
            BuildState::Steps => {
                let column_text = Row::new()
                    .push(Space::with_width(Length::Units(70)))
                    .push(
                        Text::new("Destination")
                            .width(Length::Units(125))
                            .font(CQ_MONO),
                    )
                    .push(Text::new("Action").width(Length::Units(120)).font(CQ_MONO));

                let add_step = Row::new().push(
                    self.add_step
                        .view()
                        .map(move |msg| BuildMessage::AddStepMessage(msg)),
                );

                let steps: Element<_> = self
                    .modified_steps
                    .iter_mut()
                    .enumerate()
                    .fold(Column::new().spacing(15), |column, (i, step)| {
                        column.push(
                            step.view()
                                .map(move |msg| BuildMessage::StepMessage(i, msg)),
                        )
                    })
                    .into();

                let content = Column::new()
                    .max_width(800)
                    .spacing(10)
                    .push(save_bar)
                    .push(search)
                    .push(tab_btns)
                    .push(column_text)
                    .push(steps)
                    .push(Space::with_height(Length::Units(25)))
                    .push(add_step);
                Scrollable::new(&mut self.scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
            BuildState::RequiredInput => {
                let before_title = Text::new("Before Run")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .font(CQ_MONO)
                    .size(40)
                    .width(Length::Fill);
                let before_required_inputs = self
                    .modified_before_inputs
                    .iter_mut()
                    .enumerate()
                    .fold(Column::new().spacing(5), |col, (i, input)| {
                        col.push(
                            input
                                .view()
                                .map(move |msg| BuildMessage::BeforeRequiredInputMessage(i, msg)),
                        )
                    });
                let add_input_before_btn = Row::new()
                    .push(Space::with_width(Length::Fill))
                    .push(
                        Button::new(
                            &mut self.add_input_before_btn,
                            Text::new("Add Input")
                                .font(CQ_MONO)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .style(Theme::Blue)
                        .on_press(BuildMessage::AddInputBefore)
                        .padding(10)
                        .width(Length::Units(400)),
                    )
                    .push(Space::with_width(Length::Fill));
                let after_title = Text::new("After Run")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .font(CQ_MONO)
                    .size(40)
                    .width(Length::Fill);
                let after_required_inputs = self.modified_after_inputs.iter_mut().enumerate().fold(
                    Column::new().spacing(5),
                    |col, (i, input)| {
                        col.push(
                            input
                                .view()
                                .map(move |msg| BuildMessage::AfterRequiredInputMessage(i, msg)),
                        )
                    },
                );
                let add_input_after_btn = Row::new()
                    .push(Space::with_width(Length::Fill))
                    .push(
                        Button::new(
                            &mut self.add_input_after_btn,
                            Text::new("Add Input")
                                .font(CQ_MONO)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .style(Theme::Blue)
                        .on_press(BuildMessage::AddInputAfter)
                        .padding(10)
                        .width(Length::Units(400)),
                    )
                    .push(Space::with_width(Length::Fill));
                let width_limit = Column::new()
                    .width(Length::Units(500))
                    .push(before_title)
                    .push(before_required_inputs)
                    .push(Space::with_height(Length::Units(5)))
                    .push(add_input_before_btn)
                    .push(after_title)
                    .push(after_required_inputs)
                    .push(Space::with_height(Length::Units(5)))
                    .push(add_input_after_btn);
                let content = Column::new()
                    .max_width(800)
                    .spacing(10)
                    .push(save_bar)
                    .push(search)
                    .push(tab_btns)
                    .push(
                        Row::new()
                            .push(Space::with_width(Length::Fill))
                            .push(width_limit)
                            .push(Space::with_width(Length::Fill)),
                    );
                Scrollable::new(&mut self.scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
        }
    }
}
#[derive(Serialize, Deserialize)]
// Step found in ./run.rs
pub struct Recipe {
    pub required_inputs: Input,
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize)]
pub struct Input {
    pub before: Vec<String>,
    pub after: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RequiredInput {
    state: text_input::State,
    delete_btn: button::State,
    value: String,
}

#[derive(Debug, Clone)]
pub enum RequiredInputMessage {
    InputChanged(String),
    Delete,
}

impl RequiredInput {
    fn new(value: String) -> Self {
        RequiredInput {
            state: text_input::State::new(),
            value,
            delete_btn: button::State::new(),
        }
    }

    fn update(&mut self, message: RequiredInputMessage) {
        match message {
            RequiredInputMessage::InputChanged(input) => self.value = input,
            _ => {}
        }
    }

    fn view(&mut self) -> Element<'_, RequiredInputMessage> {
        Row::new()
            .push(
                TextInput::new(
                    &mut self.state,
                    "Required Input",
                    &self.value[..],
                    RequiredInputMessage::InputChanged,
                )
                .style(Theme::Blue)
                .padding(10),
            )
            .push(
                Button::new(&mut self.delete_btn, delete_icon())
                    .width(Length::Units(50))
                    .padding(10)
                    .on_press(RequiredInputMessage::Delete)
                    .style(Theme::Red),
            )
            .into()
    }
}

#[derive(Debug, Clone)]
pub struct BuildStep {
    step_num: Option<usize>,
    steps_len: usize,
    nodes_ref: Rc<RefCell<Nodes>>,
    actions_ref: Rc<RefCell<Actions>>,
    selected_destination: Option<String>,
    hover: bool,
    selected_action: Option<String>,
    secs_value: String,
    mins_value: String,
    hours_value: String,
    wait: bool,
    state: StepState,
}

#[derive(Debug, Clone)]
pub enum StepState {
    Idle {
        edit_btn: button::State,
    },
    Editing {
        destination_state: pick_list::State<String>,
        actions_state: pick_list::State<String>,
        step_num_state: pick_list::State<usize>,
        okay_btn: button::State,
        delete_btn: button::State,
        secs_input: text_input::State,
        mins_input: text_input::State,
        hours_input: text_input::State,
    },
}

impl Default for StepState {
    fn default() -> Self {
        StepState::Idle {
            edit_btn: button::State::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StepMessage {
    NewDestination(String),
    NewAction(String),
    SecsChanged(String),
    MinsChanged(String),
    HoursChanged(String),
    NewNum(usize),
    ToggleHover(bool),
    ToggleWait(bool),
    HoursIncrement,
    HoursDecrement,
    MinsIncrement,
    MinsDecrement,
    SecsIncrement,
    SecsDecrement,
    Edit,
    Delete,
    Okay,
}

impl BuildStep {
    fn new(
        step_num: Option<usize>,
        steps_len: usize,
        nodes_ref: Rc<RefCell<Nodes>>,
        actions_ref: Rc<RefCell<Actions>>,
        selected_destination: Option<String>,
        hover: bool,
        selected_action: Option<String>,
        hours_value: String,
        mins_value: String,
        secs_value: String,
        wait: bool,
    ) -> Self {
        BuildStep {
            step_num,
            steps_len,
            nodes_ref,
            actions_ref,
            selected_destination,
            hover,
            selected_action,
            secs_value,
            mins_value,
            hours_value,
            wait,
            state: StepState::Idle {
                edit_btn: button::State::new(),
            },
        }
    }

    fn update(&mut self, message: StepMessage) {
        match message {
            StepMessage::NewDestination(destination) => {
                self.selected_destination = Some(destination)
            }
            StepMessage::NewAction(action) => self.selected_action = Some(action),
            StepMessage::ToggleHover(b) => self.hover = b,
            StepMessage::ToggleWait(b) => self.wait = b,
            StepMessage::HoursChanged(hours) => {
                let into_num = hours.parse::<usize>();
                if hours == "".to_string() {
                    self.hours_value = "".to_string()
                } else if into_num.is_ok() {
                    self.hours_value = into_num.unwrap().min(99).to_string();
                }
            }
            StepMessage::MinsChanged(mins) => {
                let into_num = mins.parse::<usize>();
                if mins == "".to_string() {
                    self.mins_value = "".to_string()
                } else if into_num.is_ok() {
                    self.mins_value = into_num.unwrap().min(59).to_string();
                }
            }
            StepMessage::SecsChanged(secs) => {
                let into_num = secs.parse::<usize>();
                if secs == "".to_string() {
                    self.secs_value = "".to_string()
                } else if into_num.is_ok() {
                    self.secs_value = into_num.unwrap().min(59).to_string();
                }
            }
            StepMessage::HoursIncrement => {
                self.hours_value = (self.hours_value.parse::<usize>().unwrap_or(0) + 1)
                    .min(99)
                    .to_string()
            }
            StepMessage::MinsIncrement => {
                self.mins_value = (self.mins_value.parse::<usize>().unwrap_or(0) + 1)
                    .min(59)
                    .to_string()
            }
            StepMessage::SecsIncrement => {
                self.secs_value = (self.secs_value.parse::<usize>().unwrap_or(0) + 1)
                    .min(59)
                    .to_string()
            }
            StepMessage::Delete => {}
            StepMessage::Okay => {
                self.state = StepState::Idle {
                    edit_btn: button::State::new(),
                }
            }
            StepMessage::Edit => {
                self.state = StepState::Editing {
                    destination_state: pick_list::State::default(),
                    actions_state: pick_list::State::default(),
                    step_num_state: pick_list::State::default(),
                    okay_btn: button::State::new(),
                    delete_btn: button::State::new(),
                    hours_input: text_input::State::new(),
                    mins_input: text_input::State::new(),
                    secs_input: text_input::State::new(),
                }
            }
            StepMessage::HoursDecrement => {
                if self.hours_value != 0.to_string()
                    && self.hours_value != 1.to_string()
                    && self.hours_value != "".to_string()
                {
                    self.hours_value =
                        (self.hours_value.parse::<usize>().unwrap_or(1) - 1).to_string();
                } else {
                    self.hours_value = "".to_string()
                }
            }
            StepMessage::MinsDecrement => {
                if self.mins_value != 0.to_string()
                    && self.mins_value != 1.to_string()
                    && self.mins_value != "".to_string()
                {
                    self.mins_value =
                        (self.mins_value.parse::<usize>().unwrap_or(1) - 1).to_string()
                } else {
                    self.mins_value = "".to_string()
                }
            }
            StepMessage::SecsDecrement => {
                if self.secs_value != 0.to_string()
                    && self.secs_value != 1.to_string()
                    && self.secs_value != "".to_string()
                {
                    self.secs_value =
                        (self.secs_value.parse::<usize>().unwrap_or(1) - 1).to_string()
                } else {
                    self.secs_value = "".to_string()
                }
            }
            StepMessage::NewNum(_) => {}
        }
    }

    fn view(&mut self) -> Element<StepMessage> {
        match &mut self.state {
            StepState::Editing {
                destination_state,
                step_num_state,
                actions_state,
                okay_btn,
                delete_btn,
                secs_input,
                mins_input,
                hours_input,
            } => {
                Column::new()
                    .push(
                        Row::new()
                            .push(
                                // Step num
                                Column::new().push(
                                    PickList::new(
                                        step_num_state,
                                        (1..self.steps_len + 1).collect::<Vec<usize>>(),
                                        self.step_num,
                                        StepMessage::NewNum,
                                    )
                                    .style(Theme::Blue)
                                    .padding(10)
                                    .width(Length::Shrink),
                                ),
                            )
                            .push(
                                // Destination
                                Column::new().push(
                                    PickList::new(
                                        destination_state,
                                        self.nodes_ref
                                            .borrow()
                                            .node
                                            .iter()
                                            .filter(|n| !n.name.contains("_hover") && !n.hide)
                                            .fold(Vec::new(), |mut v, n| {
                                                v.push(n.name.clone());
                                                v
                                            }),
                                        self.selected_destination.clone(),
                                        StepMessage::NewDestination,
                                    )
                                    .style(Theme::Blue)
                                    .padding(10)
                                    .width(Length::Shrink),
                                ),
                            )
                            .push(
                                // actions
                                Column::new().push(
                                    Row::new()
                                        .push(
                                            PickList::new(
                                                actions_state,
                                                self.actions_ref.borrow().action.iter().fold(
                                                    Vec::new(),
                                                    |mut v, a| {
                                                        v.push(a.name.clone());
                                                        v
                                                    },
                                                ),
                                                self.selected_action.clone(),
                                                StepMessage::NewAction,
                                            )
                                            .style(Theme::Blue)
                                            .padding(10)
                                            .width(Length::Shrink),
                                        )
                                        .push(
                                            TextInput::new(
                                                // hours
                                                hours_input,
                                                "Hours",
                                                &self.hours_value,
                                                StepMessage::HoursChanged,
                                            )
                                            .style(Theme::Blue)
                                            .on_scroll_up(StepMessage::HoursIncrement)
                                            .on_scroll_down(StepMessage::HoursDecrement)
                                            .padding(10)
                                            .width(Length::Fill),
                                        )
                                        .push(
                                            (TextInput::new(
                                                // mins
                                                mins_input,
                                                "Minutes",
                                                &self.mins_value,
                                                StepMessage::MinsChanged,
                                            )
                                            .style(Theme::Blue))
                                            .on_scroll_up(StepMessage::MinsIncrement)
                                            .on_scroll_down(StepMessage::MinsDecrement)
                                            .padding(10)
                                            .width(Length::Fill),
                                        )
                                        .push(
                                            TextInput::new(
                                                // secs
                                                secs_input,
                                                "Seconds",
                                                &self.secs_value,
                                                StepMessage::SecsChanged,
                                            )
                                            .style(Theme::Blue)
                                            .on_scroll_up(StepMessage::SecsIncrement)
                                            .on_scroll_down(StepMessage::SecsDecrement)
                                            .padding(10)
                                            .width(Length::Fill),
                                        )
                                        .push(
                                            Button::new(okay_btn, okay_icon())
                                                .on_press(StepMessage::Okay)
                                                .padding(10)
                                                .width(Length::Units(50))
                                                .style(Theme::Green),
                                        )
                                        .push(
                                            Button::new(delete_btn, delete_icon())
                                                .on_press(StepMessage::Delete)
                                                .padding(10)
                                                .width(Length::Units(50))
                                                .style(Theme::Red),
                                        ),
                                ),
                            ),
                    )
                    .push(
                        Row::new()
                            .push(Space::with_width(Length::Fill))
                            .push(
                                Column::new()
                                    .push(
                                        Checkbox::new(
                                            self.hover,
                                            "Hover Above",
                                            StepMessage::ToggleHover,
                                        )
                                        .style(Theme::Blue),
                                    )
                                    .padding(4)
                                    .width(Length::Shrink),
                            )
                            .push(Space::with_width(Length::Units(25)))
                            .push(
                                Column::new()
                                    .push(
                                        Checkbox::new(
                                            self.wait,
                                            "Require Input",
                                            StepMessage::ToggleWait,
                                        )
                                        .style(Theme::Blue),
                                    )
                                    .padding(4)
                                    .width(Length::Shrink),
                            )
                            .push(Space::with_width(Length::Fill)),
                    )
                    .into()
            }
            StepState::Idle { edit_btn } => {
                let e = "".to_string(); //empty
                let hover = match self.hover {
                    true => "Hover above\n",
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
                        format!(
                            "{}{} for 0 seconds",
                            ri,
                            self.selected_action
                                .as_ref()
                                .unwrap_or(&"*𝘚𝘵𝘦𝘱 𝘌𝘙𝘙𝘖𝘙*".to_string())
                        )
                    }
                    (h, m, s) if h == e && m == e => format!(
                        "{}{} for {} second{}",
                        ri,
                        self.selected_action
                            .as_ref()
                            .unwrap_or(&"*𝘚𝘵𝘦𝘱 𝘌𝘙𝘙𝘖𝘙*".to_string()),
                        s,
                        ns(&s)
                    ),
                    (h, m, s) if h == e && s == e => format!(
                        "{}{} for {} minute{}",
                        ri,
                        self.selected_action
                            .as_ref()
                            .unwrap_or(&"*𝘚𝘵𝘦𝘱 𝘌𝘙𝘙𝘖𝘙*".to_string()),
                        m,
                        ns(&m)
                    ),
                    (h, m, s) if m == e && s == e => {
                        format!(
                            "{}{} for {} hour{}",
                            ri,
                            self.selected_action
                                .as_ref()
                                .unwrap_or(&"*𝘚𝘵𝘦𝘱 𝘌𝘙𝘙𝘖𝘙*".to_string()),
                            h,
                            ns(&h)
                        )
                    }
                    (h, m, s) if h == e => format!(
                        "{}{} for {} minute{} and {} second{}",
                        ri,
                        self.selected_action
                            .as_ref()
                            .unwrap_or(&"*𝘚𝘵𝘦𝘱 𝘌𝘙𝘙𝘖𝘙*".to_string()),
                        m,
                        ns(&m),
                        s,
                        ns(&s)
                    ),
                    (h, m, s) if m == e => format!(
                        "{}{} for {} hour{} and {} second{}",
                        ri,
                        self.selected_action
                            .as_ref()
                            .unwrap_or(&"*𝘚𝘵𝘦𝘱 𝘌𝘙𝘙𝘖𝘙*".to_string()),
                        h,
                        ns(&h),
                        s,
                        ns(&s)
                    ),
                    (h, m, s) if s == e => format!(
                        "{}{} for {} hour{} and {} minute{}",
                        ri,
                        self.selected_action
                            .as_ref()
                            .unwrap_or(&"*𝘚𝘵𝘦𝘱 𝘌𝘙𝘙𝘖𝘙*".to_string()),
                        h,
                        ns(&h),
                        m,
                        ns(&m)
                    ),
                    (h, m, s) => format!(
                        "{}{} for {} hour{}, {} minute{} and {} second{}",
                        ri,
                        self.selected_action
                            .as_ref()
                            .unwrap_or(&"*𝘚𝘵𝘦𝘱 𝘌𝘙𝘙𝘖𝘙*".to_string()),
                        h,
                        ns(&h),
                        m,
                        ns(&m),
                        s,
                        ns(&s)
                    ),
                };
                Row::new()
                    .align_items(Align::Center)
                    .push(
                        Text::new(format!("{}", self.step_num.unwrap()))
                            .width(Length::Units(75))
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .font(CQ_MONO),
                    )
                    .push(
                        // Destination
                        Column::new().push(
                            Text::new(format!(
                                "{}{}",
                                hover,
                                self.selected_destination
                                    .as_ref()
                                    .unwrap_or(&"*𝘚𝘵𝘦𝘱 𝘌𝘙𝘙𝘖𝘙*".to_string()),
                            ))
                            .font(CQ_MONO)
                            .width(Length::Units(120))
                            .vertical_alignment(VerticalAlignment::Center),
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
                            .width(Length::Fill)
                            .align_items(Align::Start),
                    )
                    .push(
                        // edit button
                        Button::new(
                            edit_btn,
                            Text::new("Edit")
                                .vertical_alignment(VerticalAlignment::Center)
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .font(CQ_MONO),
                        )
                        .style(Theme::Blue)
                        .padding(10)
                        .on_press(StepMessage::Edit)
                        //.height(Length::Units(75))
                        .width(Length::Units(100)),
                    )
                    .into()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddStep {
    step_num: Option<usize>,
    steps_len: usize,
    nodes_ref: Rc<RefCell<Nodes>>,
    actions_ref: Rc<RefCell<Actions>>,
    destination_state: pick_list::State<String>,
    hover: bool,
    selected_destination: Option<String>,
    actions_state: pick_list::State<String>,
    step_num_state: pick_list::State<usize>,
    selected_action: Option<String>,
    secs_input: text_input::State,
    secs_value: String,
    mins_input: text_input::State,
    mins_value: String,
    hours_input: text_input::State,
    hours_value: String,
    wait: bool,
    add_btn: button::State,
}

#[derive(Debug, Clone)]
pub enum AddStepMessage {
    Add(
        Option<String>,
        bool,
        Option<String>,
        Option<usize>,
        String,
        String,
        String,
        bool,
    ),
    NewDestination(String),
    NewAction(String),
    SecsChanged(String),
    MinsChanged(String),
    HoursChanged(String),
    NewNum(usize),
    ToggleHover(bool),
    ToggleWait(bool),
    HoursIncrement,
    HoursDecrement,
    MinsIncrement,
    MinsDecrement,
    SecsIncrement,
    SecsDecrement,
}

impl AddStep {
    fn new(
        step_num: usize,
        steps_len: usize,
        nodes_ref: Rc<RefCell<Nodes>>,
        actions_ref: Rc<RefCell<Actions>>,
    ) -> AddStep {
        AddStep {
            step_num: Some(step_num),
            steps_len,
            nodes_ref,
            actions_ref: Rc::clone(&actions_ref),
            destination_state: pick_list::State::default(),
            selected_destination: None,
            hover: false,
            actions_state: pick_list::State::default(),
            step_num_state: pick_list::State::default(),
            selected_action: actions_ref
                .borrow()
                .action
                .first()
                .map_or(None, |a| Some(a.name.clone())),
            secs_input: text_input::State::new(),
            secs_value: "".to_string(),
            mins_input: text_input::State::new(),
            mins_value: "".to_string(),
            hours_input: text_input::State::new(),
            hours_value: "".to_string(),
            wait: false,
            add_btn: button::State::new(),
        }
    }

    fn update(&mut self, message: AddStepMessage) {
        match message {
            AddStepMessage::NewDestination(destination) => {
                self.selected_destination = Some(destination)
            }
            AddStepMessage::NewAction(action) => self.selected_action = Some(action),
            AddStepMessage::ToggleHover(b) => self.hover = b,
            AddStepMessage::ToggleWait(b) => self.wait = b,
            AddStepMessage::HoursChanged(hours) => {
                let into_num = hours.parse::<usize>();
                if hours == "".to_string() {
                    self.hours_value = "".to_string()
                } else if into_num.is_ok() {
                    self.hours_value = into_num.unwrap().min(99).to_string();
                }
            }
            AddStepMessage::MinsChanged(mins) => {
                let into_num = mins.parse::<usize>();
                if mins == "".to_string() {
                    self.mins_value = "".to_string()
                } else if into_num.is_ok() {
                    self.mins_value = into_num.unwrap().min(59).to_string();
                }
            }
            AddStepMessage::SecsChanged(secs) => {
                let into_num = secs.parse::<usize>();
                if secs == "".to_string() {
                    self.secs_value = "".to_string()
                } else if into_num.is_ok() {
                    self.secs_value = into_num.unwrap().min(59).to_string();
                }
            }
            AddStepMessage::HoursIncrement => {
                self.hours_value = (self.hours_value.parse::<usize>().unwrap_or(0) + 1)
                    .min(99)
                    .to_string()
            }
            AddStepMessage::MinsIncrement => {
                self.mins_value = (self.mins_value.parse::<usize>().unwrap_or(0) + 1)
                    .min(59)
                    .to_string()
            }
            AddStepMessage::SecsIncrement => {
                self.secs_value = (self.secs_value.parse::<usize>().unwrap_or(0) + 1)
                    .min(59)
                    .to_string()
            }
            AddStepMessage::Add(_, _, _, _, _, _, _, _) => {
                self.hours_value = "".to_string();
                self.mins_value = "".to_string();
                self.secs_value = "".to_string()
            }
            AddStepMessage::HoursDecrement => {
                if self.hours_value != 0.to_string()
                    && self.hours_value != 1.to_string()
                    && self.hours_value != "".to_string()
                {
                    self.hours_value =
                        (self.hours_value.parse::<usize>().unwrap_or(1) - 1).to_string();
                } else {
                    self.hours_value = "".to_string()
                }
            }
            AddStepMessage::MinsDecrement => {
                if self.mins_value != 0.to_string()
                    && self.mins_value != 1.to_string()
                    && self.mins_value != "".to_string()
                {
                    self.mins_value =
                        (self.mins_value.parse::<usize>().unwrap_or(1) - 1).to_string()
                } else {
                    self.mins_value = "".to_string()
                }
            }
            AddStepMessage::SecsDecrement => {
                if self.secs_value != 0.to_string()
                    && self.secs_value != 1.to_string()
                    && self.secs_value != "".to_string()
                {
                    self.secs_value =
                        (self.secs_value.parse::<usize>().unwrap_or(1) - 1).to_string()
                } else {
                    self.secs_value = "".to_string()
                }
            }
            AddStepMessage::NewNum(num) => {
                self.step_num = Some(num);
            }
        }
    }

    fn view(&mut self) -> Element<AddStepMessage> {
        Column::new()
            .push(
                Row::new()
                    .push(
                        // Step num
                        Column::new().push(
                            PickList::new(
                                &mut self.step_num_state,
                                (1..self.steps_len + 2).collect::<Vec<usize>>(),
                                self.step_num,
                                AddStepMessage::NewNum,
                            )
                            .style(Theme::Blue)
                            .padding(10)
                            .width(Length::Shrink),
                        ),
                    )
                    .push(
                        // Destination
                        Column::new().push(
                            PickList::new(
                                &mut self.destination_state,
                                self.nodes_ref
                                    .borrow()
                                    .node
                                    .iter()
                                    .filter(|n| !n.name.contains("_hover") && !n.hide)
                                    .fold(Vec::new(), |mut v, n| {
                                        v.push(n.name.clone());
                                        v
                                    }),
                                self.selected_destination.clone(),
                                AddStepMessage::NewDestination,
                            )
                            .style(Theme::Blue)
                            .padding(10)
                            .width(Length::Shrink),
                        ),
                    )
                    .push(
                        // actions
                        Column::new().push(
                            Row::new()
                                .push(
                                    PickList::new(
                                        &mut self.actions_state,
                                        self.actions_ref.borrow().action.iter().fold(
                                            Vec::new(),
                                            |mut v, a| {
                                                v.push(a.name.clone());
                                                v
                                            },
                                        ),
                                        self.selected_action.clone(),
                                        AddStepMessage::NewAction,
                                    )
                                    .style(Theme::Blue)
                                    .padding(10)
                                    .width(Length::Shrink),
                                )
                                .push(
                                    TextInput::new(
                                        // hours
                                        &mut self.hours_input,
                                        "Hours",
                                        &self.hours_value,
                                        AddStepMessage::HoursChanged,
                                    )
                                    .style(Theme::Blue)
                                    .on_scroll_up(AddStepMessage::HoursIncrement)
                                    .on_scroll_down(AddStepMessage::HoursDecrement)
                                    .padding(10)
                                    .width(Length::Fill),
                                )
                                .push(
                                    (TextInput::new(
                                        // mins
                                        &mut self.mins_input,
                                        "Minutes",
                                        &self.mins_value,
                                        AddStepMessage::MinsChanged,
                                    )
                                    .style(Theme::Blue))
                                    .on_scroll_up(AddStepMessage::MinsIncrement)
                                    .on_scroll_down(AddStepMessage::MinsDecrement)
                                    .padding(10)
                                    .width(Length::Fill),
                                )
                                .push(
                                    TextInput::new(
                                        // secs
                                        &mut self.secs_input,
                                        "Seconds",
                                        &self.secs_value,
                                        AddStepMessage::SecsChanged,
                                    )
                                    .style(Theme::Blue)
                                    .on_scroll_up(AddStepMessage::SecsIncrement)
                                    .on_scroll_down(AddStepMessage::SecsDecrement)
                                    .padding(10)
                                    .width(Length::Fill),
                                )
                                .push(
                                    Button::new(
                                        &mut self.add_btn,
                                        Text::new("Add Step")
                                            .horizontal_alignment(HorizontalAlignment::Center)
                                            .font(CQ_MONO),
                                    )
                                    .style(Theme::Blue)
                                    .on_press(AddStepMessage::Add(
                                        self.selected_destination.clone(),
                                        self.hover,
                                        self.selected_action.clone(),
                                        self.step_num,
                                        self.hours_value.clone(),
                                        self.mins_value.clone(),
                                        self.secs_value.clone(),
                                        self.wait,
                                    ))
                                    .padding(10)
                                    .width(Length::Units(100)),
                                ),
                        ),
                    ),
            )
            .push(
                Row::new()
                    .push(Space::with_width(Length::Fill))
                    .push(
                        Column::new()
                            .push(
                                Checkbox::new(
                                    self.hover,
                                    "Hover Above",
                                    AddStepMessage::ToggleHover,
                                )
                                .style(Theme::Blue),
                            )
                            .padding(4)
                            .width(Length::Shrink),
                    )
                    .push(Space::with_width(Length::Units(25)))
                    .push(
                        Column::new()
                            .push(
                                Checkbox::new(
                                    self.wait,
                                    "Wait for User",
                                    AddStepMessage::ToggleWait,
                                )
                                .style(Theme::Blue),
                            )
                            .padding(4)
                            .width(Length::Shrink),
                    )
                    .push(Space::with_width(Length::Fill)),
            )
            .into()
    }
}

pub fn ns(string: &String) -> String {
    // needs s ?
    if string.parse::<usize>().unwrap_or(0) > 1 {
        "s".to_string()
    } else {
        "".to_string()
    }
}

// Fonts
const ICONS_FONT: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../fonts/Icons.ttf"),
};

fn icon(unicode: char) -> Text {
    Text::new(&unicode.to_string())
        .font(ICONS_FONT)
        .width(Length::Units(20))
        .horizontal_alignment(HorizontalAlignment::Center)
        .size(20)
}

pub fn okay_icon() -> Text {
    icon('\u{E801}')
}

pub fn delete_icon() -> Text {
    icon('\u{E800}')
}

pub fn play_icon() -> Text {
    icon('\u{E804}')
}

pub fn pause_icon() -> Text {
    icon('\u{E805}')
}

pub fn attention_icon() -> Text {
    icon('\u{E806}')
}

pub fn close_icon() -> Text {
    icon('\u{E807}')
}

pub fn edit_icon() -> Text {
    icon('\u{E808}')
}

pub fn down_icon() -> Text {
    icon('\u{E802}')
}

pub fn right_icon() -> Text {
    icon('\u{E803}')
}
