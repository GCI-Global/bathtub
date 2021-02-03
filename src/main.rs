use iced::{Scrollable, HorizontalAlignment, VerticalAlignment, TextInput, Button, button, text_input, scrollable, PickList, Row, pick_list, Application, Container, Text, Element, Column, Command, Settings, Length};

enum App {
    Loading,
    Loaded(State)
}

struct State {
    scroll: scrollable::State,
    add_button: button::State,
    steps: Vec<Step>
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
    AddStep,
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
                        *self = App::Loaded(State{
                            // return a State struct to be viewed
                            scroll: scrollable::State::new(),
                            add_button: button::State::new(),
                            steps: vec![Step::new(1)],
                        })
                    },
                    Message::Loaded(Err(_)) => {
                        // what to do in the case async load fails
                    },
                    _ => (),
                }
            }
            App::Loaded(state) => {
                match message {
                    Message::StepMessage(i, StepMessage::Delete) => {
                        state.steps.remove(i);
                        for i in 0..state.steps.len() {
                            state.steps[i].step_num = i + 1;
                        }
                    },
                    Message::StepMessage(i, msg) => {
                        if let Some(step) = state.steps.get_mut(i) {
                            step.update(msg)
                        }
                    },
                    Message::AddStep => state.steps.push(Step::new(state.steps.len() + 1)),
                    _ => (),
                }
            }

        }
        Command::none()
    }
    fn view(&mut self) -> Element<Message> {
        match self {
            App::Loading => loading_message(),
            App::Loaded(State {
                    // have state variables to be accessable 
                    scroll,
                    add_button,
                    steps,
                    ..
            }) => {
                let title = Text::new("Recipie");

                let steps: Element<_> =
                    steps.iter_mut().enumerate()
                        .fold( Column::new().spacing(15), |column, (i, step)| {
                            column.push(step.view().map(move |msg| {
                                Message::StepMessage(i, msg)
                            }))
                        })
                        .into();

                let add_btn = Button::new(
                    add_button,
                    Text::new("+")
                ).padding(20).on_press(Message::AddStep);

                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .push(title)
                    .push(steps)
                    .push(add_btn);
                Scrollable::new(scroll)
                    .padding(40)
                    .push(
                        Container::new(content)
                            .width(Length::Fill)
                            .center_x()
                    )
                    .into()
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Step {
    step_num: usize,
    destination_state: pick_list::State<Baths>,
    selected_destination: Option<Baths>,
    actions_state: pick_list::State<Actions>,
    selected_action: Option<Actions>,
    input: text_input::State,
    input_value: String,
    units_state: pick_list::State<Units>,
    selected_units: Option<Units>,
    delete_btn: button::State,
}

#[derive(Debug, Clone)]
pub enum StepMessage {
    NewDestination(Baths),
    NewAction(Actions),
    NewUnit(Units),
    InputChanged(String),
    Delete,
}

impl Step {
    fn new(step_num: usize) -> Self {
        Step {
            step_num,
            destination_state: pick_list::State::default(),
            selected_destination: None,
            actions_state: pick_list::State::default(),
            selected_action: Some(Actions::default()),
            units_state: pick_list::State::default(),
            selected_units: Some(Units::default()),
            input_value: "".to_string(),
            input: text_input::State::new(),
            delete_btn: button::State::new(),
        }
    }

    fn update(&mut self, message: StepMessage) {
        match message {
            StepMessage::NewDestination(destination) => {
                self.selected_destination = Some(destination);
            },
            StepMessage::NewAction(action) => {
                self.selected_action = Some(action);
            },
            StepMessage::NewUnit(unit) => {
                self.selected_units = Some(unit);
            },
            StepMessage::InputChanged(input) => {
                self.input_value = input;
            },
            StepMessage::Delete => {}
        }
    }

    fn view(&mut self) -> Element<StepMessage> {
        Row::new()
            .push( // Step num
                Column::new()
                    .push(
                    Text::new(format!("{}", self.step_num))
                        .size(20)
                        .vertical_alignment(VerticalAlignment::Center)
                        .horizontal_alignment(HorizontalAlignment::Center)
                ).padding(10).width(Length::Units(55))
            )
            .push( // Destination
                Column::new()
                    .push(Text::new("Destination"))
                    .push(PickList::new(
                        &mut self.destination_state,
                        &Baths::ALL[..],
                        self.selected_destination,
                        StepMessage::NewDestination,
                    ).padding(10))
            ).push( // actions
                Column::new()
                    .push(Text::new("Action"))
                    .push(
                        Row::new()
                            .push(PickList::new(
                                &mut self.actions_state,
                                &Actions::ALL[..],
                                self.selected_action,
                                StepMessage::NewAction,
                            ).padding(10))
                            .push(TextInput::new(
                                &mut self.input,
                                "0 - 100",
                                &self.input_value,
                                StepMessage::InputChanged,
                            ).padding(10).width(Length::Units(100)))
                            .push(PickList::new(
                                &mut self.units_state,
                                &Units::ALL[..],
                                self.selected_units,
                                StepMessage::NewUnit,
                            ).padding(10))
                            .push(
                                Button::new(&mut self.delete_btn, Text::new("X"))
                                    .on_press(StepMessage::Delete)
                            )
                    )
        ).into()
        
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
        Baths::Rinse7
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
    const ALL: [Actions; 2] = [
        Actions::Rest,
        Actions::Swish,
    ];
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    Minutes,
    Seconds,
    Milliseconds,
}

impl Units {
    const ALL: [Units; 3] = [
        Units::Minutes,
        Units::Seconds,
        Units::Milliseconds,
    ];
}

impl Default for Units {
    fn default() -> Units {
        Units::Seconds
    }
}

impl std::fmt::Display for Units {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Units::Minutes => "Minutes",
                Units::Seconds => "Seconds",
                Units::Milliseconds => "Milliseconds",
            }
        )
    }
}

impl LoadState {
    // this is the function that is called to load data
    async fn load() -> Result<LoadState, LoadError> {
        Ok(LoadState{})
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
