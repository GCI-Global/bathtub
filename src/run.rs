use iced::{
    button, pick_list, scrollable, Button, Column, Container, Element, Length, PickList,
    Scrollable, Text,
};

pub struct Run {
    scroll: scrollable::State,
    run_btn: button::State,
    search: Vec<String>,
    search_state: pick_list::State<String>,
    search_value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum RunMessage {
    RecipieChanged(String),
}

impl Run {
    pub fn new() -> Self {
        Run {
            scroll: scrollable::State::new(),
            run_btn: button::State::new(),
            search: Vec::new(),
            search_state: pick_list::State::default(),
            search_value: Some("".to_string()),
        }
    }

    pub fn update(&mut self, message: RunMessage) {
        match message {
            RunMessage::RecipieChanged(recipie) => self.search_value = Some(recipie),
        }
    }

    pub fn view(&mut self) -> Element<RunMessage> {
        let search = PickList::new(
            &mut self.search_state,
            &self.search[..],
            self.search_value.clone(),
            RunMessage::RecipieChanged,
        )
        .padding(10)
        .width(Length::Units(400));

        let run = Button::new(&mut self.run_btn, Text::new("Run").size(30));

        let content = Column::new()
            .max_width(800)
            .spacing(20)
            .push(search)
            .push(run);

        Scrollable::new(&mut self.scroll)
            .padding(40)
            .push(Container::new(content).width(Length::Fill).center_x())
            .into()
    }
}
