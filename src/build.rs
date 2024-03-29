use super::actions::Actions;
use super::advanced::{validate_nums, SaveBar, SaveBarMessage, ValidateNums};
use super::logger::{replace_os_char, Logger};
use super::nodes::Nodes;
use super::run::{do_nothing, Step};
use super::style::style::Theme;
use crate::{TabState, CQ_MONO};
use chrono::prelude::*;
use iced::{
    button, pick_list, scrollable, text_input, tooltip, Align, Button, Checkbox, Column, Command,
    Container, Element, Font, HorizontalAlignment, Length, PickList, Row, Scrollable, Space, Text,
    TextInput, Tooltip, VerticalAlignment,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;
//use std::thread;
//use std::time::Duration;

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
    search_options: Vec<String>,
    search_state: pick_list::State<String>,
    search_value: Option<String>,
    required_input_tab_btn: button::State,
    steps_tab_btn: button::State,
    add_input_before_btn: button::State,
    add_input_after_btn: button::State,
    cancel_btn: button::State,
    overwrite_confirmed_btn: button::State,
    delete_confirmed_btn: button::State,
    confirm_delete_btn: button::State,
    save_with_name_btn: button::State,
    name_entry_value: String,
    name_entry_state: text_input::State,
    state: BuildState,
    logger: Logger,
    unsaved_tabs: Rc<RefCell<HashMap<TabState, bool>>>,
    recipe_regex: Regex,
}

enum BuildState {
    Steps,
    RequiredInput,
    DeleteConfirm,
    EnterName,
    OverwriteConfirm,
}

#[derive(Debug, Clone)]
pub enum BuildMessage {
    StepMessage(usize, StepMessage),
    BeforeRequiredInputMessage(usize, RequiredInputMessage),
    AfterRequiredInputMessage(usize, RequiredInputMessage),
    AddStepMessage(AddStepMessage),
    SearchChanged(String),
    UpdateSearch,
    SaveMessage(SaveBarMessage),
    StepsTab,
    RequiredInputTab,
    AddInputBefore,
    AddInputAfter,
    Saved(()),
    Cancel,
    ConfirmOverWrite,
    DeleteConfirmed,
    ConfirmDelete,
    SaveWithName,
    NameEntryChanged(String),
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
            save_bar: SaveBar::new_as(),
            scroll: scrollable::State::new(),
            add_step: AddStep::new(1, 0, Rc::clone(&nodes_ref), Rc::clone(&actions_ref)),
            nodes_ref,
            actions_ref,
            search_options: Vec::new(),
            search_state: pick_list::State::default(),
            search_value: None,
            required_input_tab_btn: button::State::new(),
            steps_tab_btn: button::State::new(),
            add_input_before_btn: button::State::new(),
            add_input_after_btn: button::State::new(),
            cancel_btn: button::State::new(),
            save_with_name_btn: button::State::new(),
            confirm_delete_btn: button::State::new(),
            delete_confirmed_btn: button::State::new(),
            overwrite_confirmed_btn: button::State::new(),
            name_entry_state: text_input::State::new(),
            name_entry_value: String::new(),
            steps: Vec::new(),
            before_inputs: Vec::new(),
            after_inputs: Vec::new(),
            modified_steps: Vec::new(),
            modified_before_inputs: Vec::new(),
            modified_after_inputs: Vec::new(),
            state: BuildState::Steps,
            logger,
            unsaved_tabs,
            recipe_regex: Regex::new(r"^[^.]+").unwrap(),
        }
    }

    pub fn update(&mut self, message: BuildMessage) -> Command<BuildMessage> {
        let mut command = Command::none();
        match message {
            BuildMessage::Cancel => {
                self.name_entry_value = String::new();
                self.state = BuildState::Steps;
            }
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
                } else {
                    self.add_step.destination_style = Theme::Red;
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
            BuildMessage::SearchChanged(recipe) => {
                if self.unsaved {
                    self.save_bar.message = "Save or Cancel before changing recipe!".to_string();
                } else {
                    self.search_value = Some(recipe);
                    update_recipe(self);
                }
            }
            BuildMessage::ConfirmDelete => {
                self.state = BuildState::DeleteConfirm;
            }
            BuildMessage::DeleteConfirmed => {
                std::fs::remove_file(format!(
                    "./recipes/{}.toml",
                    self.search_value.take().unwrap()
                ))
                .unwrap();
                update_search(self);
                update_recipe(self);
                self.state = BuildState::Steps;
            }
            BuildMessage::UpdateSearch => {
                // check for and update with new recipe files
                update_search(self);
                // update the ui with recipie if it was changed
                if let Some(selection) = self.search_value.as_ref() {
                    if self.search_options.iter().any(|o| o == selection) {
                        update_recipe(self);
                    }
                } else {
                    self.steps = Vec::new();
                    self.before_inputs = Vec::new();
                    self.after_inputs = Vec::new();
                    self.modified_steps = Vec::new();
                    self.modified_before_inputs = Vec::new();
                    self.modified_after_inputs = Vec::new();
                }
            }
            BuildMessage::SaveMessage(SaveBarMessage::Cancel) => {
                self.save_bar.message = "Unsaved Changes!".to_string();
                self.modified_steps = self.steps.clone();
                self.modified_before_inputs = self.before_inputs.clone();
                self.modified_after_inputs = self.after_inputs.clone();
                self.unsaved = false;
                self.unsaved_tabs
                    .borrow_mut()
                    .insert(TabState::Build, false);
            }
            BuildMessage::NameEntryChanged(val) => {
                // unsfe windows chars
                if ![r"/", r"\", r":", r"*", r"?", "\"", r"<", r">", r"|"]
                    .iter()
                    .any(|c| val.contains(c))
                {
                    self.name_entry_value = val
                }
            }
            BuildMessage::SaveMessage(SaveBarMessage::Save) => {
                if self.modified_steps.iter().any(|s| match s.state {
                    StepState::Idle { .. } => false,
                    StepState::Editing { .. } => true,
                }) && self.modified_steps.len() > 0
                {
                    self.save_bar.message = "'Ok' all steps before saving".to_string();
                } else {
                    self.save_bar.message = "Unsaved Changes!".to_string();
                    self.name_entry_value = self.search_value.clone().unwrap_or(String::new());
                    self.state = BuildState::EnterName;
                }
            }
            BuildMessage::ConfirmOverWrite => {
                save(self);
                command = Command::perform(do_nothing(), BuildMessage::Saved);
            }
            BuildMessage::SaveWithName => {
                if !Path::new(&format!("./recipes/{}.toml", self.name_entry_value)).exists() {
                    save(self);
                    command = Command::perform(do_nothing(), BuildMessage::Saved);
                } else {
                    self.state = BuildState::OverwriteConfirm
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
        let search = Row::new()
            .height(Length::Units(40))
            .push(
                PickList::new(
                    &mut self.search_state,
                    &self.search_options[..],
                    self.search_value.clone(),
                    BuildMessage::SearchChanged,
                )
                .style(Theme::Blue)
                .padding(10)
                .width(Length::Fill),
            )
            .push(match self.search_value {
                Some(_) => Row::with_children(vec![Button::new(
                    &mut self.confirm_delete_btn,
                    Text::new("Delete Recipe").font(CQ_MONO).size(20),
                )
                .padding(10)
                .style(Theme::Red)
                .on_press(BuildMessage::ConfirmDelete)
                .into()]),
                None => Row::new(),
            });
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
                    .fold(Column::new(), |column, (i, step)| {
                        step.set_style(if i % 2 == 0 {
                            Theme::LightGray
                        } else {
                            Theme::LighterGray
                        });
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
            BuildState::EnterName => {
                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(Text::new("SAVE").font(CQ_MONO).size(40))
                    .push(
                        Text::new("What should this recipe be saved as?")
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .size(30),
                    )
                    .push(
                        TextInput::new(
                            &mut self.name_entry_state,
                            "Name (Required)",
                            &self.name_entry_value,
                            BuildMessage::NameEntryChanged,
                        )
                        .padding(10),
                    )
                    .push(Row::with_children(vec![
                        Space::with_width(Length::Fill).into(),
                        if self.name_entry_value.is_empty() {
                            Tooltip::new(
                                Button::new(
                                    &mut self.save_with_name_btn,
                                    Text::new(format!("Save as\n'{}'", self.name_entry_value))
                                        .font(CQ_MONO)
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                )
                                .style(Theme::GreenDisabled)
                                .padding(10)
                                .width(Length::Units(200)),
                                "Name Required",
                                tooltip::Position::FollowCursor,
                            )
                            .style(Theme::Red)
                            .into()
                        } else {
                            Tooltip::new(
                                Button::new(
                                    &mut self.save_with_name_btn,
                                    Text::new(format!("Save as\n'{}'", self.name_entry_value))
                                        .font(CQ_MONO)
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                )
                                .style(Theme::Green)
                                .on_press(BuildMessage::SaveWithName)
                                .padding(10)
                                .width(Length::Units(200)),
                                "",
                                tooltip::Position::Top,
                            )
                        }
                        .into(),
                        Space::with_width(Length::Units(100)).into(),
                        Button::new(
                            &mut self.cancel_btn,
                            Text::new("No.\nDon't save.")
                                .font(CQ_MONO)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .style(Theme::Red)
                        .on_press(BuildMessage::Cancel)
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
            BuildState::DeleteConfirm => {
                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(Text::new("DELETE").font(CQ_MONO).size(40))
                    .push(
                        Text::new(format!(
                            "Delete '{}'?\nThis CANNOT be undone!",
                            self.search_value.as_ref().unwrap_or(&String::new())
                        ))
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .size(30),
                    )
                    .push(Row::with_children(vec![
                        Space::with_width(Length::Fill).into(),
                        Button::new(
                            &mut self.delete_confirmed_btn,
                            Text::new(format!("DELETE"))
                                .font(CQ_MONO)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .style(Theme::Red)
                        .on_press(BuildMessage::DeleteConfirmed)
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
                        .style(Theme::Blue)
                        .on_press(BuildMessage::Cancel)
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
            BuildState::OverwriteConfirm => {
                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(
                        Text::new(format!("'{}' already exists!", self.name_entry_value)).font(CQ_MONO).size(40)    
                    )
                    .push(Text::new(format!("This will delete the old '{}' and replace it with the new one.\nThis CANNOT be undone!", self.search_value.as_ref().unwrap_or(&String::new()))).horizontal_alignment(HorizontalAlignment::Center).size(30))
                            .push(Row::with_children(vec![
                                Space::with_width(Length::Fill).into(),
                                Button::new(
                                    &mut self.overwrite_confirmed_btn,
                                    Text::new(format!("Yes, replace\n'{}'", self.name_entry_value))
                                        .font(CQ_MONO)
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                ).style(Theme::Red)
                                .on_press(BuildMessage::ConfirmOverWrite)
                                .padding(10)
                                .width(Length::Units(200))
                                .into(),
                                Space::with_width(Length::Units(100)).into(),
                                Button::new(
                                    &mut self.cancel_btn,
                                    Text::new("Cancel\n ")
                                        .font(CQ_MONO)
                                        .horizontal_alignment(HorizontalAlignment::Center),
                                ).style(Theme::Blue)
                                .on_press(BuildMessage::Cancel)
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
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
// Step found in ./run.rs
pub struct Recipe {
    pub required_inputs: Input,
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub before: Vec<String>,
    pub after: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
// Step found in ./run.rs
pub struct SaveRecipe {
    pub required_inputs: SaveInput,
    pub steps: Option<Vec<Step>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SaveInput {
    pub before: Option<Vec<String>>,
    pub after: Option<Vec<String>>,
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
    style: Theme,
    errors: BuildStepErrors,
    error_message: Option<String>,
}

#[derive(Debug, Clone)]
struct BuildStepErrors {
    destination: bool,
    action: bool,
    time: bool,
}

impl BuildStepErrors {
    fn all(&self) -> Vec<bool> {
        vec![self.destination, self.action, self.time]
    }
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
        let dest_bool = !nodes_ref.borrow().node.iter().any(|n| {
            if let Some(dest) = &selected_destination {
                &n.name == dest
            } else {
                false
            }
        });
        let act_bool = !actions_ref.borrow().action.iter().any(|a| {
            if let Some(act) = &selected_action {
                &a.name == act
            } else {
                false
            }
        });
        let time_bool = match validate_nums(vec![&hours_value, &mins_value, &secs_value], 0) {
            ValidateNums::Okay => false,
            _ => true,
        };
        BuildStep {
            step_num,
            errors: BuildStepErrors {
                destination: dest_bool,
                action: act_bool,
                time: time_bool,
            },
            error_message: None,
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
            style: Theme::LightGray,
        }
    }
    fn set_style(&mut self, style: Theme) {
        self.style = style;
    }
    fn set_error(&mut self, msg: impl ToString) {
        self.error_message = Some(msg.to_string());
    }
    fn clear_error(&mut self) {
        self.error_message = None;
    }

    fn update(&mut self, message: StepMessage) {
        match message {
            StepMessage::NewDestination(destination) => {
                self.selected_destination = Some(destination);
                if self.nodes_ref.borrow().node.iter().any(|n| {
                    if let Some(dest) = &self.selected_destination {
                        &n.name == dest
                    } else {
                        false
                    }
                }) {
                    self.errors.destination = false;
                } else {
                    self.errors.destination = true;
                }
            }
            StepMessage::NewAction(action) => {
                self.selected_action = Some(action);
                if self.actions_ref.borrow().action.iter().any(|a| {
                    if let Some(act) = &self.selected_action {
                        &a.name == act
                    } else {
                        false
                    }
                }) {
                    self.errors.action = false;
                } else {
                    self.errors.action = true;
                }
            }
            StepMessage::ToggleHover(b) => self.hover = b,
            StepMessage::ToggleWait(b) => self.wait = b,
            StepMessage::HoursChanged(hours) => {
                let into_num = hours.parse::<usize>();
                if hours.is_empty() {
                    self.hours_value = String::new()
                } else if into_num.is_ok() {
                    self.hours_value = into_num.unwrap().min(99).to_string();
                }
                self.errors.time = match validate_nums(
                    vec![&self.hours_value, &self.mins_value, &self.secs_value],
                    0,
                ) {
                    ValidateNums::Okay => false,
                    _ => true,
                };
            }
            StepMessage::MinsChanged(mins) => {
                let into_num = mins.parse::<usize>();
                if mins.is_empty() {
                    self.mins_value = String::new()
                } else if into_num.is_ok() {
                    self.mins_value = into_num.unwrap().min(59).to_string();
                }
                self.errors.time = match validate_nums(
                    vec![&self.hours_value, &self.mins_value, &self.secs_value],
                    0,
                ) {
                    ValidateNums::Okay => false,
                    _ => true,
                };
            }
            StepMessage::SecsChanged(secs) => {
                let into_num = secs.parse::<usize>();
                if secs.is_empty() {
                    self.secs_value = String::new()
                } else if into_num.is_ok() {
                    self.secs_value = into_num.unwrap().min(59).to_string();
                }
                self.errors.time = match validate_nums(
                    vec![&self.hours_value, &self.mins_value, &self.secs_value],
                    0,
                ) {
                    ValidateNums::Okay => false,
                    _ => true,
                };
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
        if self.errors.destination {
            if let Some(_dest) = &self.selected_destination {
                self.set_error(format!("Selected destination not found in Nodes.\nEither select another destination, or go to\nAdvanced -> Nodes and rename/add a node with the name '{}'.", self.selected_destination.as_ref().unwrap()));
            } else {
                self.set_error("Select a destination.");
            }
        } else if self.errors.action {
            if let Some(_act) = &self.selected_action {
                self.set_error(format!("Selected action not found.\nEither select another action, or go to\nAdvanced -> Actions and rename/add an action with the name '{}'.", self.selected_action.as_ref().unwrap()));
            } else {
                self.set_error("Select an action.");
            }
        } else if self.errors.time {
            self.set_error("Time entries must be numbers with no decimals or fractions.");
        } else {
            self.clear_error();
        }
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
                Container::new(
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
                            .style(Theme::Red)
                        } else {
                            Container::new(Space::with_height(Length::Shrink))
                        })
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
                                        .style(if self.errors.destination {
                                            Theme::Red
                                        } else {
                                            Theme::Blue
                                        })
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
                                                .style(if self.errors.action {
                                                    Theme::Red
                                                } else {
                                                    Theme::Blue
                                                })
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
                                                .style(if self.errors.time {
                                                    Theme::Red
                                                } else {
                                                    Theme::Blue
                                                })
                                                .on_scroll_up(StepMessage::HoursIncrement)
                                                .on_scroll_down(StepMessage::HoursDecrement)
                                                .padding(10)
                                                .width(Length::Fill),
                                            )
                                            .push(
                                                TextInput::new(
                                                    // mins
                                                    mins_input,
                                                    "Minutes",
                                                    &self.mins_value,
                                                    StepMessage::MinsChanged,
                                                )
                                                .style(if self.errors.time {
                                                    Theme::Red
                                                } else {
                                                    Theme::Blue
                                                })
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
                                                .style(if self.errors.time {
                                                    Theme::Red
                                                } else {
                                                    Theme::Blue
                                                })
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
                        ),
                )
                .style(if self.errors.all().iter().any(|e| *e) {
                    Theme::Red
                } else {
                    self.style
                })
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
                Container::new(
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
                            .padding(15)
                            .on_press(StepMessage::Edit)
                            //.height(Length::Units(75))
                            .width(Length::Units(100)),
                        ),
                )
                .style(if self.errors.all().iter().any(|e| *e) {
                    Theme::Red
                } else {
                    self.style
                })
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
    destination_style: Theme,
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
            destination_style: Theme::Blue,
        }
    }

    fn update(&mut self, message: AddStepMessage) {
        match message {
            AddStepMessage::NewDestination(destination) => {
                self.destination_style = Theme::Blue;
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
                            .style(self.destination_style)
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

pub fn down_icon() -> Text {
    icon('\u{E802}')
}

pub fn right_icon() -> Text {
    icon('\u{E803}')
}
fn update_recipe(tab: &mut Build) {
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
            tab.before_inputs = rec.required_inputs.before.iter().fold(
                Vec::with_capacity(rec.required_inputs.before.len()),
                |mut v, input| {
                    v.push(RequiredInput::new(input.clone()));
                    v
                },
            );
            tab.after_inputs = rec.required_inputs.after.iter().fold(
                Vec::with_capacity(rec.required_inputs.after.len()),
                |mut v, input| {
                    v.push(RequiredInput::new(input.clone()));
                    v
                },
            );
            let steps_len = rec.steps.len();
            tab.steps = rec.steps.into_iter().enumerate().fold(
                Vec::with_capacity(steps_len),
                |mut v, (i, step)| {
                    v.push(BuildStep::new(
                        Some(i + 1),
                        steps_len,
                        Rc::clone(&tab.nodes_ref),
                        Rc::clone(&tab.actions_ref),
                        Some(step.selected_destination),
                        step.hover,
                        Some(step.selected_action),
                        step.hours_value,
                        step.mins_value,
                        step.secs_value,
                        step.wait,
                    ));
                    v
                },
            );
            tab.add_step.step_num = Some(steps_len + 1);
            tab.add_step.steps_len = steps_len;
            tab.modified_steps = tab.steps.clone();
            tab.modified_after_inputs = tab.after_inputs.clone();
            tab.modified_before_inputs = tab.before_inputs.clone();
        }
        // TODO: Display Error when unable to read file
        Err(_err) => {
            tab.modified_steps = Vec::new();
            tab.modified_before_inputs = Vec::new();
            tab.modified_after_inputs = Vec::new();
            tab.steps = Vec::new();
            tab.before_inputs = Vec::new();
            tab.after_inputs = Vec::new();
        }
    }
}

fn save(tab: &mut Build) {
    tab.save_bar.message = "Unsaved Changes!".to_string();
    if tab.name_entry_value != String::new() {
        tab.modified_before_inputs
            .retain(|input| input.value != "".to_string());
        tab.modified_after_inputs
            .retain(|input| input.value != "".to_string());
        let save_data = SaveRecipe {
            required_inputs: SaveInput {
                before: if tab.modified_before_inputs.len() > 0 {
                    Some(tab.modified_before_inputs.iter().fold(
                        Vec::with_capacity(tab.modified_before_inputs.len()),
                        |mut v, input| {
                            v.push(input.value.clone());
                            v
                        },
                    ))
                } else {
                    None
                },
                after: if tab.modified_after_inputs.len() > 0 {
                    Some(tab.modified_after_inputs.iter().fold(
                        Vec::with_capacity(tab.modified_after_inputs.len()),
                        |mut v, input| {
                            v.push(input.value.clone());
                            v
                        },
                    ))
                } else {
                    None
                },
            },
            steps: if tab.modified_steps.len() > 0 {
                Some(tab.modified_steps.iter().fold(
                    Vec::with_capacity(tab.modified_steps.len()),
                    |mut v, step| {
                        v.push(Step {
                            step_num: step.step_num.unwrap().to_string(),
                            selected_destination: step.selected_destination.clone().unwrap(),
                            hover: step.hover,
                            selected_action: step.selected_action.clone().unwrap(),
                            secs_value: step.secs_value.clone(),
                            mins_value: step.mins_value.clone(),
                            hours_value: step.hours_value.clone(),
                            wait: step.wait,
                        });
                        v
                    },
                ))
            } else {
                None
            },
        };
        // TODO: add error is unable to build toml
        let old_recipe = fs::read_to_string(Path::new(&format!(
            "./recipes/{}.toml",
            &tab.search_value.as_ref().unwrap_or(&String::new()),
        )))
        .unwrap_or(format!("No old recipe '{}' is new", tab.name_entry_value));
        tab.name_entry_value = replace_os_char(tab.name_entry_value.clone());
        let new_recipe = toml::to_string_pretty(&save_data).unwrap();
        match OpenOptions::new().write(true).open(Path::new(&format!(
            "./recipes/{}.toml",
            &tab.name_entry_value.replace(" ", ""),
        ))) {
            Ok(_file) => {
                // file already exists, thus we need to log that recipe was changed
                // write!(file, "{}", new_recipe).unwrap();
                fs::write(
                    format!(
                        "./recipes/{}.toml",
                        replace_os_char(tab.name_entry_value.clone())
                    ),
                    new_recipe.clone(),
                )
                .unwrap();
                tab.logger.set_log_file(format!(
                    "{}; Build - Changed '{}'",
                    Local::now().to_rfc2822(),
                    tab.name_entry_value,
                ));
                tab.logger.send_line(String::new()).unwrap();
                tab.logger
                    .send_line("Recipe changed from:".to_string())
                    .unwrap();
                tab.logger.send_line(old_recipe).unwrap();
                tab.logger
                    .send_line("\n\n\nRecipe changed to:".to_string())
                    .unwrap();
                tab.logger.send_line(new_recipe).unwrap();
            }
            Err(_) => {
                // new file thus new recip/e, log should state created recipe
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(Path::new(&format!(
                        "./recipes/{}.toml",
                        &tab.name_entry_value
                    )))
                    .unwrap();
                write!(file, "{}", new_recipe).unwrap();
                tab.logger.set_log_file(format!(
                    "{}; Build - Created '{}'",
                    Local::now().to_rfc2822(),
                    tab.name_entry_value
                ));
                tab.logger.send_line(String::new()).unwrap();
                tab.logger
                    .send_line(format!("Created Recipe '{}' as:", tab.name_entry_value))
                    .unwrap();
                tab.logger.send_line(new_recipe).unwrap();
            }
        }
        tab.steps = tab.modified_steps.clone();
        tab.before_inputs = tab.modified_before_inputs.clone();
        tab.after_inputs = tab.modified_after_inputs.clone();
        tab.unsaved = false;
        tab.unsaved_tabs.borrow_mut().insert(TabState::Build, false);
        tab.search_value = Some(tab.name_entry_value.clone());
        update_search(tab);
        update_recipe(tab);
        tab.state = BuildState::Steps;
        // TODO: Have errors show to user if unable to save
    } else {
        tab.name_entry_value = String::new();
        tab.steps = Vec::new();
        tab.before_inputs = Vec::new();
        tab.after_inputs = Vec::new();
        tab.modified_steps = Vec::new();
        tab.modified_before_inputs = Vec::new();
        tab.modified_after_inputs = Vec::new();
        update_search(tab);
        update_recipe(tab);
    }
}
fn update_search(tab: &mut Build) {
    tab.search_options = fs::read_dir("./recipes")
        .unwrap()
        .fold(Vec::new(), |mut rec, file| {
            if let Some(caps) = tab
                .recipe_regex
                .captures(&file.unwrap().file_name().to_str().unwrap())
            {
                rec.push(caps[0].to_string());
            }
            rec
        });
    tab.search_options.sort();
}
