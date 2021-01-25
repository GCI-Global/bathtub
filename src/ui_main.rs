#![feature(total_cmp)]
mod nodes;
use nodes::{Node, NodeGrid2d};

use iced::{button, Button, Scrollable, scrollable, Container, Command, HorizontalAlignment, Length ,Column, Row, Element, Application, Settings, Text};

pub fn main() -> iced::Result {
    Bathtub::run(Settings::default())
}

#[derive(Debug)]
enum Bathtub {
    Loading,
    Loaded(State)
}

#[derive(Debug)]
struct State {
    scroll: scrollable::State,
    //bath_btns: (NodeGrid2d, Vec<Vec<button::State>>),
    bath_btns: Vec<Vec<(Node, button::State)>>,
}

#[derive(Debug, Clone)]
struct LoadState {
    node_grid2d: NodeGrid2d,
}

#[derive(Debug, Clone)]
enum LoadError {
    Placeholder,
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<LoadState, LoadError>),
    ButtonPressed(String),
}

impl Application for Bathtub {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Bathtub, Command<Message>) {
        (
            Bathtub::Loading,
            Command::perform(LoadState::load(), Message::Loaded),
        )
    }

    fn title(&self) -> String {
        String::from("Bathtub")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match self {
            Bathtub::Loading => {
                match message {
                    Message::Loaded(Ok(state)) => {
                        *self = Bathtub::Loaded(State {
                            bath_btns: state.node_grid2d.grid.into_iter()
                                .fold(Vec::new(), |mut vec, axis| {
                                    vec.push(
                                        axis.into_iter()
                                            .fold(Vec::new(), |mut axis_vec, node| {
                                                axis_vec.push((node, button::State::new()));
                                                axis_vec
                                            })
                                    );
                                    vec
                                }),
                            scroll: scrollable::State::new()
                        });
                    }
                    Message::Loaded(Err(_)) => {
                        panic!("somehow loaded had an error")
                        // need to add correct fail state, following is from the Todos example
                        //*self = Bathtub::Loaded(State::default());
                    }
                    Message::ButtonPressed(btn_name) => {
                        println!("{} was pressed", btn_name);
                    }
                }
                Command::none()
            }
            Bathtub::Loaded(_state) => {
                // placeholder
                Command::none()
            }
        }
    }
    
    fn view(&mut self) -> Element<Message> {
        match self {
            Bathtub::Loading => loading_message(),
            Bathtub::Loaded(State {
                                scroll,
                                bath_btns,
            }) => {
                let title = Text::new("Bathtub")
                    .width(Length::Fill)
                    .size(100)
                    .color([0.5, 0.5, 0.5])
                    .horizontal_alignment(HorizontalAlignment::Center); 
               
                let button_grid = bath_btns.into_iter()
                    .fold(Column::new(), |column, grid| {
                        column.push(grid.into_iter()
                            .fold(Row::new(), |row, node_tup| {
                                row.push(
                                    Button::new(&mut node_tup.1, Text::new(&node_tup.0.name))
                                        .on_press(Message::ButtonPressed(node_tup.0.name.clone()))
                                )
                            })
                        )
                    });

                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .push(title)
                    .push(button_grid);

                Scrollable::new(scroll)
                    .padding(40)
                    .push(
                        Container::new(content).width(Length::Fill).center_x(),
                    )
                    .into()
            }
        }
    }
}

impl LoadState {
    fn new(node_grid2d: NodeGrid2d) -> LoadState {
        LoadState {
            node_grid2d,
        }
    }

    // This is just a placeholder. Will eventually read data from server
    async fn load() -> Result<LoadState, LoadError> {
        let nodes = nodes::gen_nodes();
        Ok(
            LoadState::new(NodeGrid2d::from_nodes(nodes.clone()))
        )
    }
}

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

/*
//Code for interacting with tubby. will make UI that will interact with this later
fn main() {
    
    let start = 15;
    let finish = 12;

    let nodes = nodes::gen_nodes();
    // gen node grid for UI
    let nodes = nodes::gen_nodes();
    let node_grid2d = NodeGrid2d::from_nodes(nodes.clone());
    for node_vec in node_grid2d.grid {
        for node in node_vec {
            println!("{}", node.name);
        }
        println!("------")
    }
    // stop gen node grid
    println!("From {} to {}", &nodes.node[start].name, &nodes.node[finish].name);
    let node_paths = paths::gen_node_paths(&nodes, &nodes.node[start], &nodes.node[finish]);
    for node in &node_paths.node {
        println!("{}", node.name);
    };
    let gcode_path = paths::gen_gcode_paths(&node_paths);

    // This is for interacting with Tubby, will get back to later
    let mut port = serial::open("/dev/ttyUSB0").expect("unable to find tty");
    interact(&mut port, &gcode_path).unwrap();
}
 //This sends data to tubby. Likely needs to placed in separate module. WIll uncomment after UI is finished.
fn interact<T: SerialPort>(port: &mut T, gcode_path: &Vec<String>) -> io::Result<()> {
    port.reconfigure(&|settings| {
        settings.set_baud_rate(serial::Baud115200).unwrap();
        settings.set_char_size(serial::Bits8);
        settings.set_parity(serial::ParityNone);
        settings.set_stop_bits(serial::Stop1);
        settings.set_flow_control(serial::FlowNone);

        Ok(())
    }).unwrap();
    port.set_timeout(Duration::from_secs(60)).unwrap();
    
    // Initialize GRBL
    let mut buf: Vec<u8> = "\r\n\r\n".as_bytes().to_owned(); //wake GRBL then wait for server to start
    port.write(&buf[..]).unwrap();
    thread::sleep(Duration::from_secs(2));
    port.flush().unwrap();
    buf = "$H\n".as_bytes().to_owned(); //Unlock head
    println!("{:?}", &buf[..]);
    port.write(&buf[..]).unwrap();
    port.read(&mut buf[..]).unwrap(); //Should be able to parse this in the future for sucess/fail messages

    //send to above rinse 1
    buf = "G90 X0 Y-13.5 Z0\n".as_bytes().to_owned();
    port.write(&buf[..]).unwrap();
    thread::sleep(Duration::from_secs(2));
    let mut output = String::from("");
    for gcode in gcode_path {
        println!("{}",gcode);
        buf = gcode.as_bytes().to_owned();
        port.write(&buf[..]).unwrap();
        while !output.contains("ok") {
            port.read(&mut buf[..]).unwrap();
            output = format!("{}{}", output, str::from_utf8(&buf[..]).unwrap());
            //println!("{}", output);
        }
        output.clear();
        port.flush().unwrap();
    }
    println!("{}", output);
    Ok(())
}
*/
// Ideas
// Things that should be in the config file
// 1. Add the path to the serial port (i think linux is /dev/ttyUSB0) not sure about windows yet
// 2. All usb settings should come from the config file
