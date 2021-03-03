use super::actions::Actions;
use super::nodes::Nodes;
use crate::CQ_MONO;
use iced::{
    button, pick_list, scrollable, text_input, Align, Button, Checkbox, Column, Container, Element,
    Font, HorizontalAlignment, Length, PickList, Row, Scrollable, Space, Text, TextInput,
    VerticalAlignment,
};
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;

pub struct Build {
    scroll: scrollable::State,
    nodes_ref: Rc<RefCell<Nodes>>,
    actions_ref: Rc<RefCell<Actions>>,
    steps: Vec<Step>,
    add_step: AddStep,
    recipie_name: text_input::State,
    recipie_name_value: String,
    save_button: button::State,
}

#[derive(Debug, Clone)]
pub enum BuildMessage {
    StepMessage(usize, StepMessage),
    AddStepMessage(AddStepMessage),
    UserChangedName(String),
    Save,
}

impl Build {
    pub fn new(nodes_ref: Rc<RefCell<Nodes>>, actions_ref: Rc<RefCell<Actions>>) -> Self {
        Build {
            scroll: scrollable::State::new(),
            add_step: AddStep::new(1, 0, Rc::clone(&nodes_ref), Rc::clone(&actions_ref)),
            nodes_ref,
            actions_ref,
            recipie_name: text_input::State::new(),
            recipie_name_value: "".to_string(),
            save_button: button::State::new(),
            steps: Vec::new(),
        }
    }

    pub fn update(&mut self, message: BuildMessage) {
        match message {
            BuildMessage::StepMessage(i, StepMessage::Delete) => {
                self.steps.remove(i);
                for i in 0..self.steps.len() {
                    self.steps[i].step_num = Some(i + 1);
                }
                for i in 0..self.steps.len() {
                    self.steps[i].steps_len = self.steps.len();
                }
                self.add_step.step_num = Some(self.steps.len() + 1);
                self.add_step.steps_len = self.steps.len();
            }
            BuildMessage::StepMessage(i, StepMessage::NewNum(num)) => {
                self.steps[i].step_num = Some(num);
                if num <= i {
                    for j in num - 1..i {
                        self.steps[j].step_num = Some(self.steps[j].step_num.unwrap() + 1);
                    }
                } else {
                    for j in i + 1..num {
                        self.steps[j].step_num = Some(self.steps[j].step_num.unwrap() - 1);
                    }
                }
                self.steps
                    .sort_by(|a, b| a.step_num.partial_cmp(&b.step_num).unwrap());
            }
            BuildMessage::StepMessage(i, msg) => {
                if let Some(step) = self.steps.get_mut(i) {
                    step.update(msg)
                }
            }
            BuildMessage::AddStepMessage(AddStepMessage::Add(
                dest,
                inbath,
                action,
                nodes,
                hours,
                mins,
                secs,
                req_input,
            )) => {
                if let Some(d) = dest {
                    for i in nodes.unwrap() - 1..self.steps.len() {
                        self.steps[i].step_num = Some(self.steps[i].step_num.unwrap() + 1);
                    }
                    self.steps.push(Step::new(
                        nodes,
                        self.steps.len(),
                        Rc::clone(&self.nodes_ref),
                        Rc::clone(&self.actions_ref),
                        Some(d),
                        inbath,
                        action,
                        hours,
                        mins,
                        secs,
                        req_input,
                    ));

                    self.steps
                        .sort_by(|a, b| a.step_num.partial_cmp(&b.step_num).unwrap());
                    for i in 0..self.steps.len() {
                        self.steps[i].steps_len = self.steps.len();
                    }
                    self.scroll.scroll_to_bottom();
                    self.add_step.step_num = Some(self.steps.len() + 1);
                    self.add_step.steps_len = self.steps.len();
                    self.add_step.hours_value = "".to_string();
                    self.add_step.mins_value = "".to_string();
                    self.add_step.secs_value = "".to_string();
                    self.add_step.in_bath = true;
                    self.add_step.require_input = false;
                }
            }
            BuildMessage::AddStepMessage(msg) => self.add_step.update(msg),
            BuildMessage::UserChangedName(new_name) => self.recipie_name_value = new_name,
            BuildMessage::Save => {
                if self.recipie_name_value != "".to_string() {
                    let mut recipie = csv::Writer::from_writer(
                        File::create(format!("./recipies/{}.recipie", &self.recipie_name_value))
                            .expect("unable to create file"),
                    );
                    recipie
                        .write_record(&[
                            "step_num",
                            "selected_destination",
                            "selected_action",
                            "hours_value",
                            "mins_value",
                            "secs_value",
                            "in_bath",
                            "require_input",
                        ])
                        .unwrap();
                    for step in self.steps.iter() {
                        recipie
                            .write_record(&[
                                step.step_num.unwrap().to_string(),
                                format!("{}", step.selected_destination.as_ref().unwrap()),
                                format!("{}", step.selected_action.as_ref().unwrap()),
                                (*step.hours_value).to_string(),
                                (*step.mins_value).to_string(),
                                (*step.secs_value).to_string(),
                                step.in_bath.to_string(),
                                step.require_input.to_string(),
                            ])
                            .unwrap();
                    }
                }
            }
        }
    }
    pub fn view(&mut self) -> Element<BuildMessage> {
        let name_and_save = Row::new()
            .push(Space::with_width(Length::Fill))
            .push(
                TextInput::new(
                    &mut self.recipie_name,
                    "Recipie Name",
                    &self.recipie_name_value,
                    BuildMessage::UserChangedName,
                )
                .padding(10)
                .width(Length::Units(300)),
            )
            .push(
                Button::new(
                    &mut self.save_button,
                    Text::new("Save").horizontal_alignment(HorizontalAlignment::Center),
                )
                .padding(10)
                .width(Length::Units(100))
                .on_press(BuildMessage::Save),
            )
            .push(Space::with_width(Length::Fill));

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
            .steps
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
            .spacing(20)
            .push(name_and_save)
            .push(column_text)
            .push(steps)
            .push(add_step);
        Scrollable::new(&mut self.scroll)
            .padding(40)
            .push(Container::new(content).width(Length::Fill).center_x())
            .into()
    }
}

#[derive(Debug)]
pub struct Step {
    step_num: Option<usize>,
    steps_len: usize,
    nodes_ref: Rc<RefCell<Nodes>>,
    actions_ref: Rc<RefCell<Actions>>,
    selected_destination: Option<String>,
    in_bath: bool,
    selected_action: Option<String>,
    secs_value: String,
    mins_value: String,
    hours_value: String,
    require_input: bool,
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
    ToggleBath(bool),
    ToggleInput(bool),
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

impl Step {
    fn new(
        step_num: Option<usize>,
        steps_len: usize,
        nodes_ref: Rc<RefCell<Nodes>>,
        actions_ref: Rc<RefCell<Actions>>,
        selected_destination: Option<String>,
        in_bath: bool,
        selected_action: Option<String>,
        hours_value: String,
        mins_value: String,
        secs_value: String,
        require_input: bool,
    ) -> Self {
        Step {
            step_num,
            steps_len,
            nodes_ref,
            actions_ref,
            selected_destination,
            in_bath,
            selected_action,
            secs_value,
            mins_value,
            hours_value,
            require_input,
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
            StepMessage::ToggleBath(b) => self.in_bath = b,
            StepMessage::ToggleInput(b) => self.require_input = b,
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
                                        (1..self.steps_len + 2).collect::<Vec<usize>>(),
                                        self.step_num,
                                        StepMessage::NewNum,
                                    )
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
                                            .filter(|n| !n.name.contains("_inBath") && !n.hide)
                                            .fold(Vec::new(), |mut v, n| {
                                                v.push(n.name.clone());
                                                v
                                            }),
                                        self.selected_destination.clone(),
                                        StepMessage::NewDestination,
                                    )
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
                                            ))
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
                                            .on_scroll_up(StepMessage::SecsIncrement)
                                            .on_scroll_down(StepMessage::SecsDecrement)
                                            .padding(10)
                                            .width(Length::Fill),
                                        )
                                        .push(
                                            Button::new(okay_btn, okay_icon())
                                                .on_press(StepMessage::Okay)
                                                .padding(10)
                                                .width(Length::Units(50)),
                                        )
                                        .push(
                                            Button::new(delete_btn, delete_icon())
                                                .on_press(StepMessage::Delete)
                                                .padding(10)
                                                .width(Length::Units(50)),
                                        ),
                                ),
                            ),
                    )
                    .push(
                        Row::new()
                            .push(Space::with_width(Length::Fill))
                            .push(
                                Column::new()
                                    .push(Checkbox::new(
                                        self.in_bath,
                                        "Enter Bath",
                                        StepMessage::ToggleBath,
                                    ))
                                    .padding(4)
                                    .width(Length::Shrink),
                            )
                            .push(Space::with_width(Length::Units(25)))
                            .push(
                                Column::new()
                                    .push(Checkbox::new(
                                        self.require_input,
                                        "Require Input",
                                        StepMessage::ToggleInput,
                                    ))
                                    .padding(4)
                                    .width(Length::Shrink),
                            )
                            .push(Space::with_width(Length::Fill)),
                    )
                    .into()
            }
            StepState::Idle { edit_btn } => {
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
                                self.selected_destination
                                    .as_ref()
                                    .unwrap_or(&"*𝘚𝘵𝘦𝘱 𝘌𝘙𝘙𝘖𝘙*".to_string()),
                                eb
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
    in_bath: bool,
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
    require_input: bool,
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
    ToggleBath(bool),
    ToggleInput(bool),
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
            in_bath: true,
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
            require_input: false,
            add_btn: button::State::new(),
        }
    }

    fn update(&mut self, message: AddStepMessage) {
        match message {
            AddStepMessage::NewDestination(destination) => {
                self.selected_destination = Some(destination)
            }
            AddStepMessage::NewAction(action) => self.selected_action = Some(action),
            AddStepMessage::ToggleBath(b) => self.in_bath = b,
            AddStepMessage::ToggleInput(b) => self.require_input = b,
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
                                    .filter(|n| !n.name.contains("_inBath") && !n.hide)
                                    .fold(Vec::new(), |mut v, n| {
                                        v.push(n.name.clone());
                                        v
                                    }),
                                self.selected_destination.clone(),
                                AddStepMessage::NewDestination,
                            )
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
                                    ))
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
                                    .on_press(AddStepMessage::Add(
                                        self.selected_destination.clone(),
                                        self.in_bath,
                                        self.selected_action.clone(),
                                        self.step_num,
                                        self.hours_value.clone(),
                                        self.mins_value.clone(),
                                        self.secs_value.clone(),
                                        self.require_input,
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
                            .push(Checkbox::new(
                                self.in_bath,
                                "Enter Bath",
                                AddStepMessage::ToggleBath,
                            ))
                            .padding(4)
                            .width(Length::Shrink),
                    )
                    .push(Space::with_width(Length::Units(25)))
                    .push(
                        Column::new()
                            .push(Checkbox::new(
                                self.require_input,
                                "Require Input",
                                AddStepMessage::ToggleInput,
                            ))
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
    bytes: include_bytes!("../fonts/icons.ttf"),
};

fn icon(unicode: char) -> Text {
    Text::new(&unicode.to_string())
        .font(ICONS_FONT)
        .width(Length::Units(20))
        .horizontal_alignment(HorizontalAlignment::Center)
        .size(20)
}

pub fn okay_icon() -> Text {
    icon('\u{F00C}')
}

pub fn delete_icon() -> Text {
    icon('\u{F1F8}')
}
