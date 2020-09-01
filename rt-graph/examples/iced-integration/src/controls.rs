use iced_wgpu::Renderer;
use iced_winit::{
    slider, Align, Color, Column, Command, Element, Length, Program, Row,
    Slider, Text,
};
use iced::{
    button,
    HorizontalAlignment,
    widget::{Button, Container, Space},
};

pub struct Controls {
    play_pause_btn: button::State,
    running: Running,
    zoom_in_btn: button::State,
    zoom_out_btn: button::State,
    zoom_x: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Running {
    Play,
    Pause,
}

#[derive(Clone, Debug)]
pub enum Message {
    PlayPause,
    ZoomIn,
    ZoomOut,
}

impl Controls {
    pub fn new() -> Controls {
        Controls {
            play_pause_btn: button::State::default(),
            running: Running::Play,
            zoom_in_btn: button::State::default(),
            zoom_out_btn: button::State::default(),
            zoom_x: crate::BASE_ZOOM_X,
        }
    }

    pub fn zoom_x(&self) -> f32 {
        self.zoom_x
    }

    pub fn running(&self) -> Running {
        self.running
    }
}

impl Program for Controls {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PlayPause => {
                self.running = match self.running {
                    Running::Play => Running::Pause,
                    Running::Pause => Running::Play,
                }
            },
            Message::ZoomIn => {
                self.zoom_x = self.zoom_x / 2.0;
            }
            Message::ZoomOut => {
                self.zoom_x = self.zoom_x * 2.0;
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        let content = Column::new()
            .spacing(20)
            .align_items(Align::Center)

            // Space for the graph.
            .push(Space::new(Length::Units(800), Length::Units(200)))

            .push(Text::new("My graph".to_owned()))
            .push(Row::new()
                  .push(Button::new(&mut self.play_pause_btn,
                                    Text::new(match self.running {
                                        Running::Play => "Running",
                                        Running::Pause => "Paused",
                                    }).horizontal_alignment(HorizontalAlignment::Center))
                        .on_press(Message::PlayPause)
                        .width(Length::Units(150)))
                  .push(Button::new(&mut self.zoom_in_btn,
                                    Text::new("Zoom in")
                                    .horizontal_alignment(HorizontalAlignment::Center))
                        .on_press(Message::ZoomIn)
                        .width(Length::Units(150)))
                  .push(Button::new(&mut self.zoom_out_btn,
                                    Text::new("Zoom out")
                                    .horizontal_alignment(HorizontalAlignment::Center))
                        .on_press(Message::ZoomOut)
                        .width(Length::Units(150)))
             );
        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .align_y(Align::Start)
            .into()
    }
}
