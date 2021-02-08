use iced::{
    button, pick_list, Font, scrollable, text_input, Application, Button, Column, Command, Container,
    Element, HorizontalAlignment, Length, PickList, Row, Scrollable, Settings, Space, Text,
    TextInput, VerticalAlignment,
};

enum App {
    Loading,
    Loaded(State),
}

struct State {
    scroll: scrollable::State,
    steps: Vec<Step>,
    add_step: AddStep,
    recipie_name: text_input::State,
    recipie_name_value: String,
    save_button: button::State,
}

#[derive(Debug, Clone)]
struct LoadState {
    // fill with async loaded things
}

#[derive(Debug, Clone)]
enum LoadError {
    // Placeholder for if async load fails
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<LoadState, LoadError>),
    StepMessage(usize, StepMessage),
    AddStepMessage(AddStepMessage),
    UserChangedName(String),
    Save,
}

pub fn main() -> iced::Result {
    App::run(Settings::default())
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (App, Command<Message>) {
        (
            App::Loading,
            Command::perform(LoadState::load(), Message::Loaded),
        )
    }

    fn title(&self) -> String {
        String::from("Application Boiler Plate")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match self {
            App::Loading => {
                match message {
                    Message::Loaded(Ok(_load_state)) => {
                        *self = App::Loaded(State {
                            // return a State struct to be viewed
                            scroll: scrollable::State::new(),
                            recipie_name: text_input::State::new(),
                            recipie_name_value: "".to_string(),
                            save_button: button::State::new(),
                            steps: Vec::new(),
                            add_step: AddStep::new(1),
                        })
                    }
                    Message::Loaded(Err(_)) => {
                        // what to do in the case async load fails
                    }
                    _ => (),
                }
            }
            App::Loaded(state) => match message {
                Message::StepMessage(i, StepMessage::Delete) => {
                    state.steps.remove(i);
                    for i in 0..state.steps.len() {
                        state.steps[i].step_num = Some(i + 1);
                    }
                    for i in 0..state.steps.len() {
                        state.steps[i].steps_len = state.steps.len();
                    }
                    state.add_step.step_num = state.steps.len() + 1
                }
                Message::StepMessage(i, StepMessage::NewNum(num)) => {
                    state.steps[i].step_num = Some(num);
                    if num <= i {
                        for j in num-1..i {
                            state.steps[j].step_num = Some(state.steps[j].step_num.unwrap() + 1);
                        }
                    } else {
                        for j in i+1..num {
                            state.steps[j].step_num = Some(state.steps[j].step_num.unwrap() - 1);
                        }
                    }
                    state.steps.sort_by(|a, b| a.step_num.partial_cmp(&b.step_num).unwrap());
                }
                Message::StepMessage(i, msg) => {
                    if let Some(step) = state.steps.get_mut(i) {
                        step.update(msg)
                    }
                }
                Message::AddStepMessage(AddStepMessage::Add(d, a, h, m, s)) => {
                    if let Some(d) = d {
                        state
                            .steps
                            .push(Step::new(Some(state.steps.len() + 1), state.steps.len(), Some(d), a, h, m, s));
                        for i in 0..state.steps.len() {
                            state.steps[i].steps_len = state.steps.len();
                        }
                        state.scroll.scroll_to_bottom();
                        state.add_step.step_num = state.steps.len() + 1;
                    }
                }
                Message::AddStepMessage(msg) => state.add_step.update(msg),
                Message::UserChangedName(new_name) => state.recipie_name_value = new_name,
                _ => (),
            },
        }
        Command::none()
    }
    fn view(&mut self) -> Element<Message> {
        match self {
            App::Loading => loading_message(),
            App::Loaded(State {
                // have state variables to be accessable
                scroll,
                recipie_name,
                recipie_name_value,
                save_button,
                steps,
                add_step,
                ..
            }) => {
                let name_and_save = Row::new()
                    .push(Space::with_width(Length::Fill))
                    .push(
                        TextInput::new(
                            recipie_name,
                            "Recipie Name",
                            recipie_name_value,
                            Message::UserChangedName,
                        )
                        .padding(10)
                        .width(Length::Units(300)),
                    )
                    .push(
                        Button::new(
                            save_button,
                            Text::new("Save").horizontal_alignment(HorizontalAlignment::Center),
                        )
                        //.height(Length::Units(20))
                        .padding(10)
                        .width(Length::Units(100))
                        .on_press(Message::Save),
                    )
                    .push(Space::with_width(Length::Fill));

                let add_step =
                    Row::new().push(add_step.view().map(move |msg| Message::AddStepMessage(msg)));

                let steps: Element<_> = steps
                    .iter_mut()
                    .enumerate()
                    .fold(Column::new().spacing(15), |column, (i, step)| {
                        column.push(step.view().map(move |msg| Message::StepMessage(i, msg)))
                    })
                    .into();

                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .push(name_and_save)
                    .push(steps)
                    .push(add_step);
                Scrollable::new(scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Step {
    step_num: Option<usize>,
    steps_len: usize,
    selected_destination: Option<Baths>,
    selected_action: Option<Actions>,
    secs_value: String,
    mins_value: String,
    hours_value: String,
    state: StepState,
}

#[derive(Debug, Clone)]
pub enum StepState {
    Idle {
        edit_btn: button::State,
    },
    Editing {
        destination_state: pick_list::State<Baths>,
        actions_state: pick_list::State<Actions>,
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
    NewDestination(Baths),
    NewAction(Actions),
    SecsChanged(String),
    MinsChanged(String),
    HoursChanged(String),
    NewNum(usize),
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

        selected_destination: Option<Baths>,
        selected_action: Option<Actions>,
        hours_value: String,
        mins_value: String,
        secs_value: String,
    ) -> Self {
        Step {
            step_num,
            steps_len,
            selected_destination,
            selected_action,
            secs_value,
            mins_value,
            hours_value,
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
                Row::new()
                    .push(
                        // Step num
                        Column::new()
                            .push(
                                PickList::new(
                                    step_num_state,
                                    (1..self.steps_len +1 ).collect::<Vec<usize>>(),
                                    self.step_num,
                                    StepMessage::NewNum,
                                ).padding(10).width(Length::Units(55))
                            )
                    )
                    .push(
                        // Destination
                        Column::new().push(Text::new("Destination")).push(
                            PickList::new(
                                destination_state,
                                &Baths::ALL[..],
                                self.selected_destination,
                                StepMessage::NewDestination,
                            )
                            .padding(10)
                        ),
                    )
                    .push(
                        // actions
                        Column::new().push(Text::new("Action")).push(
                            Row::new()
                                .push(
                                    PickList::new(
                                        actions_state,
                                        &Actions::ALL[..],
                                        self.selected_action,
                                        StepMessage::NewAction,
                                    )
                                    .padding(10),
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
                                    .width(Length::Units(100)),
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
                                    .width(Length::Units(100)),
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
                                    .width(Length::Units(100)),
                                )
                                .push(
                                    Button::new(
                                        okay_btn,
                                        okay_icon(),
                                    )
                                    .on_press(StepMessage::Okay)
                                    .padding(10)
                                    .width(Length::Units(50)),
                                )
                                .push(
                                    Button::new(
                                        delete_btn,
                                        delete_icon(),
                                            //.horizontal_alignment(HorizontalAlignment::Center),
                                    )
                                    .on_press(StepMessage::Delete)
                                    .padding(10)
                                    .width(Length::Units(50)),
                                ),
                        ),
                    )
                    .into()
            }
            StepState::Idle { edit_btn } => {
                let e = "".to_string(); //empty
                let step_time_text = match (self.hours_value.clone(), self.mins_value.clone(), self.secs_value.clone()) {
                (h, m, s) if h == e && m == e && s == e=> format!("{} for 0 seconds", self.selected_action.unwrap()),
                (h, m, s) if h == e && m == e => format!("{} for {} second{}", self.selected_action.unwrap(), s, ns(&s)),
                (h, m, s) if h == e && s == e => format!("{} for {} minute{}", self.selected_action.unwrap(), m, ns(&m)),
                (h, m, s) if m == e && s == e => format!("{} for {} hour{}", self.selected_action.unwrap(), h, ns(&h)),
                (h, m, s) if h == e => format!("{} for {} minute{} and {} second{}", self.selected_action.unwrap(), m, ns(&m), s, ns(&s)),
                (h, m, s) if m == e => format!("{} for {} hour{} and {} second{}", self.selected_action.unwrap(), h, ns(&h), s, ns(&s)),
                (h, m, s) if s == e => format!("{} for {} hour{} and {} minute{}", self.selected_action.unwrap(), h, ns(&h), m, ns(&m)),
                (h, m, s) => format!("{} for {} hour{}, {} minute{} and {} second{}", self.selected_action.unwrap(), h, ns(&h), m, ns(&m), s, ns(&s)),
                };
                Row::new()
                    .height(Length::Units(55))
                    .push(
                        // Step num
                        Column::new()
                            .push(
                                Text::new(format!("{}", self.step_num.unwrap()))
                                    .size(20)
                                    .vertical_alignment(VerticalAlignment::Center)
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .padding(10)
                            .width(Length::Units(50)),
                    )
                    .push(
                        // Destination
                        Column::new()
                            .push(Text::new("Destination"))
                            .push(
                                Text::new(format!("{:?}", self.selected_destination.unwrap()))
                                    .height(Length::Units(55))
                                    .width(Length::Units(120)),
                            )
                            .height(Length::Fill),
                    )
                    .push(
                        // actions
                        Column::new()
                            .height(Length::Fill)
                            .push(Text::new("Action"))
                            .push(
                                /*
                                Row::new().height(Length::Units(100)).push(
                                    Text::new(format!(
                                        "{:?} for {} hours : {} minutes : {} seconds",
                                        self.selected_action.unwrap(),
                                        self.hours_value,
                                        self.mins_value,
                                        self.secs_value
                                    ))
                                    //.height(Length::Units(300))
                                    .width(Length::Units(455)),
                                )
                                */
                                Row::new().push(
                                Text::new(step_time_text).width(Length::Units(455)))
                            
                    
                    .push(
                        Button::new(
                            edit_btn,
                            Text::new("Edit").vertical_alignment(VerticalAlignment::Center).horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .padding(10)
                        .on_press(StepMessage::Edit)
                        //.height(Length::Units(75))
                        .width(Length::Units(100)),
                    )))
                    .into()
            }
        }
    }
}

#[derive(Debug, Clone)]
struct AddStep {
    step_num: usize,
    destination_state: pick_list::State<Baths>,
    selected_destination: Option<Baths>,
    actions_state: pick_list::State<Actions>,
    selected_action: Option<Actions>,
    secs: text_input::State,
    secs_value: String,
    mins: text_input::State,
    mins_value: String,
    hours: text_input::State,
    hours_value: String,
    add_btn: button::State,
}

#[derive(Debug, Clone)]
pub enum AddStepMessage {
    Add(Option<Baths>, Option<Actions>, String, String, String),
    NewDestination(Baths),
    NewAction(Actions),
    SecsChanged(String),
    MinsChanged(String),
    HoursChanged(String),
    HoursIncrement,
    HoursDecrement,
    MinsIncrement,
    MinsDecrement,
    SecsIncrement,
    SecsDecrement,
}

impl AddStep {
    fn new(step_num: usize) -> AddStep {
        AddStep {
            step_num,
            destination_state: pick_list::State::default(),
            selected_destination: None,
            actions_state: pick_list::State::default(),
            selected_action: Some(Actions::default()),
            secs: text_input::State::new(),
            secs_value: "".to_string(),
            mins: text_input::State::new(),
            mins_value: "".to_string(),
            hours: text_input::State::new(),
            hours_value: "".to_string(),
            add_btn: button::State::new(),
        }
    }

    fn update(&mut self, message: AddStepMessage) {
        match message {
            AddStepMessage::NewDestination(destination) => {
                self.selected_destination = Some(destination)
            }
            AddStepMessage::NewAction(action) => self.selected_action = Some(action),
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
            AddStepMessage::Add(_, _, _, _, _) => {}
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
        }
    }

    fn view(&mut self) -> Element<AddStepMessage> {
        Row::new()
            .push(
                // Step num
                Column::new()
                    .push(
                        Text::new(format!("{}", self.step_num))
                            .size(20)
                            .vertical_alignment(VerticalAlignment::Center)
                            .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .padding(10)
                    .width(Length::Units(55)),
            )
            .push(
                // Destination
                Column::new().push(Text::new("Destination")).push(
                    PickList::new(
                        &mut self.destination_state,
                        &Baths::ALL[..],
                        self.selected_destination,
                        AddStepMessage::NewDestination,
                    )
                    .padding(10),
                ),
            )
            .push(
                // actions
                Column::new().push(Text::new("Action")).push(
                    Row::new()
                        .push(
                            PickList::new(
                                &mut self.actions_state,
                                &Actions::ALL[..],
                                self.selected_action,
                                AddStepMessage::NewAction,
                            )
                            .padding(10),
                        )
                        .push(
                            TextInput::new(
                                // hours
                                &mut self.hours,
                                "Hours",
                                &self.hours_value,
                                AddStepMessage::HoursChanged,
                            )
                            .on_scroll_up(AddStepMessage::HoursIncrement)
                            .on_scroll_down(AddStepMessage::HoursDecrement)
                            .padding(10)
                            .width(Length::Units(100)),
                        )
                        .push(
                            (TextInput::new(
                                // mins
                                &mut self.mins,
                                "Minutes",
                                &self.mins_value,
                                AddStepMessage::MinsChanged,
                            ))
                            .on_scroll_up(AddStepMessage::MinsIncrement)
                            .on_scroll_down(AddStepMessage::MinsDecrement)
                            .padding(10)
                            .width(Length::Units(100)),
                        )
                        .push(
                            TextInput::new(
                                // secs
                                &mut self.secs,
                                "Seconds",
                                &self.secs_value,
                                AddStepMessage::SecsChanged,
                            )
                            .on_scroll_up(AddStepMessage::SecsIncrement)
                            .on_scroll_down(AddStepMessage::SecsDecrement)
                            .padding(10)
                            .width(Length::Units(100)),
                        )
                        .push(
                            Button::new(
                                &mut self.add_btn,
                                Text::new("Add Step")
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            .on_press(AddStepMessage::Add(
                                self.selected_destination,
                                self.selected_action,
                                self.hours_value.clone(),
                                self.mins_value.clone(),
                                self.secs_value.clone(),
                            ))
                            .padding(10)
                            .width(Length::Units(100)),
                        ),
                ),
            )
            .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Baths {
    Mcl16,
    HNO3,
    Zn,
    HF,
    Ni,
    Pd,
    Au,
    Rinse1,
    Rinse2,
    Rinse3,
    Rinse4,
    Rinse5,
    Rinse6,
    Rinse7,
}

impl Baths {
    const ALL: [Baths; 14] = [
        Baths::Mcl16,
        Baths::HNO3,
        Baths::Zn,
        Baths::HF,
        Baths::Ni,
        Baths::Pd,
        Baths::Au,
        Baths::Rinse1,
        Baths::Rinse2,
        Baths::Rinse3,
        Baths::Rinse4,
        Baths::Rinse5,
        Baths::Rinse6,
        Baths::Rinse7,
    ];
}

impl Default for Baths {
    fn default() -> Baths {
        Baths::Mcl16
    }
}

impl std::fmt::Display for Baths {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Baths::Mcl16 => "MCL-16",
                Baths::HNO3 => "HNO₃",
                Baths::Zn => "Zn",
                Baths::HF => "HF",
                Baths::Ni => "Ni",
                Baths::Pd => "Pd",
                Baths::Au => "Au",
                Baths::Rinse1 => "Rinse 1",
                Baths::Rinse2 => "Rinse 2",
                Baths::Rinse3 => "Rinse 3",
                Baths::Rinse4 => "Rinse 4",
                Baths::Rinse5 => "Rinse 5",
                Baths::Rinse6 => "Rinse 6",
                Baths::Rinse7 => "Rinse 7",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actions {
    Rest,
    Swish,
}

impl Actions {
    const ALL: [Actions; 2] = [Actions::Rest, Actions::Swish];
}

impl Default for Actions {
    fn default() -> Actions {
        Actions::Rest
    }
}

impl std::fmt::Display for Actions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Actions::Rest => "Rest",
                Actions::Swish => "Swish",
            }
        )
    }
}

impl LoadState {
    // this is the function that is called to load data
    async fn load() -> Result<LoadState, LoadError> {
        Ok(LoadState {})
    }
}

// what is displayed while waiting for the `async fn load()`
fn loading_message<'a>() -> Element<'a, Message> {
    Container::new(
        Text::new("Loading...")
            .horizontal_alignment(HorizontalAlignment::Center)
            .size(50),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_y()
    .into()
}

fn ns(string: &String) -> String { // needs s ?
    if string.parse::<usize>().unwrap_or(0) > 1 {"s".to_string()} else {"".to_string()}
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

fn okay_icon() -> Text {
    icon('\u{F00C}')
}

fn delete_icon() -> Text {
    icon('\u{F1F8}')
}
