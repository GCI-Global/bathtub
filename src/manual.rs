use super::grbl::{Command as Cmd, Grbl};
use super::logger::Logger;
use super::nodes::{Node, Nodes};
use super::paths::gen_node_paths;
use super::style::style::Theme;
use crate::{NodeTracker, RecipeState, CQ_MONO};
use chrono::prelude::*;
use iced::{
    button, scrollable, text_input, tooltip, Button, Checkbox, Column, Command, Container, Element,
    HorizontalAlignment, Length, Row, Scrollable, Space, Text, TextInput, Tooltip,
};
use regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

pub struct Manual {
    pub scroll: scrollable::State,
    pub bath_btns: Vec<Vec<Option<(usize, button::State)>>>,
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
    ref_nodes: Rc<RefCell<Nodes>>,
    homing_required: Rc<RefCell<bool>>,
    pub grbl: Grbl,
    logger: Logger,
    recipe_state: Arc<(Mutex<RecipeState>, Condvar)>,
    node_tracker: Arc<Mutex<NodeTracker>>,
    pub unsaved_in_bathtub: bool,
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
    ThankYou(Option<Cmd>),
}

impl Manual {
    pub fn new(
        ref_nodes: Rc<RefCell<Nodes>>,
        grbl: Grbl,
        logger: Logger,
        homing_required: Rc<RefCell<bool>>,
        recipe_state: Arc<(Mutex<RecipeState>, Condvar)>,
        node_tracker: Arc<Mutex<NodeTracker>>,
    ) -> Self {
        Manual {
            scroll: scrollable::State::new(),
            bath_btns: get_grid_btns(Rc::clone(&ref_nodes)),
            status: "Click any button\nto start homing cycle".to_string(),
            stop_btn: button::State::new(),
            hover: true,
            ref_nodes,
            homing_required,
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
            logger,
            recipe_state,
            node_tracker,
            unsaved_in_bathtub: false,
        }
    }

    pub fn update(&mut self, message: ManualMessage) -> Command<ManualMessage> {
        match message {
            ManualMessage::ToggleBath(boolean) => self.hover = boolean,
            ManualMessage::TerminalTab => self.state = ManualState::Terminal,
            ManualMessage::GridTab => self.state = ManualState::Grid,
            ManualMessage::TerminalInputChanged(val) => self.terminal_input_value = val,
            ManualMessage::ThankYou(cmd) => match cmd {
                Some(cmd) => self.terminal_responses.insert(
                    0,
                    format!(
                        "{}; '{}' => {}",
                        cmd.response_time.unwrap().to_rfc2822(),
                        cmd.command,
                        cmd.result.unwrap()
                    ),
                ),
                None => {}
            },
            ManualMessage::TerminalInputSubmitted => {
                *self.homing_required.borrow_mut() = true;
                let val = self.terminal_input_value.replace("\n", "").to_uppercase();
                self.terminal_input_value = "".to_string();
                if val.contains("?") {
                    self.terminal_responses.insert(
                        0,
                        format!(
                            "{}; => '?' Status command not available. Look at GUI above ;)",
                            Local::now().to_rfc2822()
                        ),
                    )
                } else if val.contains("M")
                    || val.contains("P")
                    || val.contains("C")
                    || val == "$G".to_string()
                {
                    self.terminal_responses.insert(
                        0,
                        format!(
                            "{}; '{}' => Command not supported by Bathtub.",
                            Local::now().to_rfc2822(),
                            val
                        ),
                    )
                } else if &val[..] == "$$" || &val[..] == "$I" {
                    self.terminal_responses.insert(0,format!("{}; '{}' => View and edit settings withing Bathtub! Advanced Tab => Grbl ;)", Local::now().to_rfc2822(), val))
                } else {
                    self.logger.set_log_file(format!(
                        "{}; Manual (Terminal) - {}",
                        Local::now().to_rfc2822(),
                        val
                    ));
                    self.logger.send_line(String::new()).unwrap();
                    self.logger
                        .send_line(format!(
                            "{}; Manual (Terminal) - {}",
                            Local::now().to_rfc2822(),
                            val
                        ))
                        .unwrap();

                    self.grbl.push_command(Cmd::new(val));
                    return Command::perform(
                        command_please(self.grbl.clone()),
                        ManualMessage::ThankYou,
                    );
                }
            }
            _ => {}
        };
        Command::none()
    }

    pub fn view(&mut self) -> Element<ManualMessage> {
        let ref_nodes = self.ref_nodes.borrow();
        let homing_required = self.homing_required.borrow();
        let (recipe_state, _) = &*self.recipe_state;
        let recipe_state = recipe_state.lock().unwrap();
        let node_tracker = self.node_tracker.lock().unwrap();
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
                    .style(match self.state {
                        ManualState::Grid => Theme::BlueBorderOnly,
                        _ => Theme::Blue,
                    })
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
                    .style(match self.state {
                        ManualState::Terminal => Theme::BlueBorderOnly,
                        _ => Theme::Blue,
                    })
                    .padding(10)
                    .on_press(ManualMessage::TerminalTab)
                    .width(Length::Units(200)),
                )
                .push(Space::with_width(Length::Fill)),
        );
        let paths_r_safe: bool;
        {
            paths_r_safe = !ref_nodes.node.iter().any(|n| {
                if n.hide {
                    false
                } else {
                    gen_node_paths(&*ref_nodes, &node_tracker.current, n).is_err()
                }
            });
        }
        match self.state {
            ManualState::Grid => {
                let paths_warning = Container::new(
                    Row::with_children(vec![
                        Space::with_width(Length::Fill).into(),
                        Text::new(
                            "Not all paths from the current node are safe!\n\
                      This is an isssue with the neighbors of each node.\n\
                      Neighbors are configurable one way safe paths between nodes.\n\
                      Paths are built while the recipe is running,\n\
                      and will traverse the fewest neighbors between steps.\n\
                      Change the neighbors in 'Advanced' -> 'Nodes'",
                        )
                        .into(),
                        Space::with_width(Length::Fill).into(),
                    ])
                    .padding(10),
                )
                .style(Theme::Red);
                let button_grid = self
                    .bath_btns
                    .iter_mut()
                    .fold(Column::new(), |column, grid| {
                        column.push(
                            grid.into_iter()
                                .fold(Row::new(), |row, node_tup| {
                                    if let Some(nt) = node_tup {
                                        row.push(
                                            Tooltip::new(
                                                Button::new(
                                                    &mut nt.1,
                                                    Text::new(ref_nodes.node[nt.0].name.clone())
                                                        .horizontal_alignment(
                                                            HorizontalAlignment::Center,
                                                        )
                                                        .font(CQ_MONO),
                                                )
                                                .style(match *recipe_state {
                                                    RecipeState::ManualRunning => {
                                                        if node_tracker.current.name
                                                            == ref_nodes.node[nt.0].name
                                                            || node_tracker.current.name
                                                                == format!(
                                                                    "{}_hover",
                                                                    ref_nodes.node[nt.0].name
                                                                )
                                                        {
                                                            Theme::BlueDisabledBright
                                                        } else {
                                                            Theme::BlueDisabled
                                                        }
                                                    }
                                                    RecipeState::HomingManual => {
                                                        Theme::BlueDisabled
                                                    }
                                                    _ => Theme::Blue,
                                                })
                                                .padding(15)
                                                .width(Length::Fill)
                                                .on_press(ManualMessage::ButtonPressed(
                                                    ref_nodes.node[nt.0].name.clone(),
                                                )),
                                                if *homing_required {
                                                    "Will run homing cycle first!"
                                                } else {
                                                    ""
                                                },
                                                tooltip::Position::FollowCursor,
                                            )
                                            .size(25)
                                            .padding(5)
                                            .style(
                                                if *homing_required {
                                                    Theme::Active
                                                } else {
                                                    Theme::Disabled
                                                },
                                            ),
                                        )
                                    } else {
                                        row.push(Space::with_width(Length::Fill))
                                    }
                                })
                                .padding(3),
                        )
                    });

                let modifiers = Row::new()
                    .push(
                        Column::new()
                            .push(Space::with_height(Length::Units(10)))
                            .push(
                                Checkbox::new(self.hover, "Hover Above", ManualMessage::ToggleBath)
                                    .style(Theme::Blue),
                            ),
                    )
                    .push(Space::with_width(Length::Units(25)))
                    .push(
                        Tooltip::new(
                            Button::new(
                                &mut self.stop_btn,
                                Text::new("STOP")
                                    .horizontal_alignment(HorizontalAlignment::Center)
                                    .font(CQ_MONO)
                                    .size(30),
                            )
                            .style(match *recipe_state {
                                RecipeState::HomingManual => Theme::RedDisabled,
                                _ => Theme::Red,
                            })
                            .padding(10)
                            .width(Length::Fill)
                            .on_press(ManualMessage::Stop),
                            match *recipe_state {
                                RecipeState::HomingManual => "Unavailable while homing!",
                                _ => "",
                            },
                            tooltip::Position::FollowCursor,
                        )
                        .size(25)
                        .padding(5)
                        .style(match *recipe_state {
                            RecipeState::HomingManual => Theme::Red,
                            _ => Theme::Disabled,
                        }),
                    );

                let content = if paths_r_safe {
                    Column::new()
                        .max_width(800)
                        .spacing(20)
                        .push(title)
                        .push(tab_btns)
                        .push(button_grid)
                        .push(modifiers)
                } else {
                    Column::new()
                        .max_width(800)
                        .spacing(20)
                        .push(title)
                        .push(tab_btns)
                        .push(paths_warning)
                        .push(modifiers)
                };

                Scrollable::new(&mut self.scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
            ManualState::Terminal => {
                let warning = Container::new(Row::with_children(vec![Space::with_width(Length::Fill).into(), Text::new("Advanced usage only! Bathtub does not check if these commands are safe!").into(), Space::with_width(Length::Fill).into()]).padding(10)).style(Theme::Red);

                let terminal_input = TextInput::new(
                    &mut self.terminal_input_state,
                    "GRBL command",
                    &self.terminal_input_value,
                    ManualMessage::TerminalInputChanged,
                )
                .style(Theme::Blue)
                .on_submit(ManualMessage::TerminalInputSubmitted)
                .padding(10);

                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .push(title)
                    .push(tab_btns)
                    .push(warning)
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

    pub fn update_grid(&mut self) {
        self.bath_btns = get_grid_btns(Rc::clone(&self.ref_nodes));
    }
}

async fn command_please(grbl: Grbl) -> Option<Cmd> {
    thread::sleep(Duration::from_secs(1));
    grbl.pop_command()
}

// given (x, y) coord get name or none
fn grid(rn: &Nodes) -> Vec<Vec<Option<usize>>> {
    //let rn = ref_nodes.borrow();
    let mut nodes = rn.node.iter().enumerate().fold(Vec::new(), |mut v, n| {
        v.push(n);
        v
    });
    nodes.retain(|n| !n.1.name.contains("_hover") && !n.1.hide);
    nodes.sort_by(|a, b| (b.1.y).total_cmp(&a.1.y));
    let mut test_value = nodes[0].1.y;
    let mut push_vec: usize = 0;
    // break into separate row on significant change in x values
    let mut build_grid = nodes.into_iter().fold(
        vec![Vec::new()],
        |mut v: Vec<Vec<Option<(usize, &Node)>>>, n| {
            if (n.1.y - test_value).abs() < 1.0 {
                v[push_vec].push(Some(n));
            } else {
                push_vec += 1;
                test_value = n.1.y;
                v.push(vec![Some(n)])
            };
            v
        },
    );
    // sort rows by x values
    for row in &mut build_grid {
        row.sort_by(|a, b| (b.as_ref().unwrap().1.x).total_cmp(&a.as_ref().unwrap().1.x));
    }
    // assign each node a point on x-axis
    let max_x = build_grid
        .iter()
        .map(|row| row.last().unwrap())
        .max_by(|a, b| b.as_ref().unwrap().1.x.total_cmp(&a.as_ref().unwrap().1.x))
        .unwrap()
        .unwrap()
        .1
        .x
        .abs()
        .ceil();
    let mut grid = build_grid.into_iter().fold(Vec::new(), |mut v, row| {
        let mut new_row = Vec::with_capacity(max_x as usize);
        let mut row_index = 0;
        for i in 0..(max_x + 2.0) as usize {
            if row_index >= row.len()
                || i as f32 - row[row_index].as_ref().unwrap().1.x.abs() <= 1.0
            {
                new_row.push(None);
            } else {
                new_row.push(Some(row[row_index].unwrap().0));
                row_index += 1;
            }
        }
        v.push(new_row);
        v
    });
    // filter empty rows
    let mut index = 0;
    while index < grid[0].len() as usize {
        if grid.iter().all(|row| row[index] == None) {
            for row in &mut grid {
                row.remove(index);
            }
        } else {
            index += 1
        }
    }
    grid
}

fn get_grid_btns(ref_nodes: Rc<RefCell<Nodes>>) -> Vec<Vec<Option<(usize, button::State)>>> {
    grid(&ref_nodes.borrow())
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
        })
}
