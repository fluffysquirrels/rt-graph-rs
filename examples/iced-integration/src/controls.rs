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
    pp_btn: button::State,
    running: Running,
}

enum Running {
    Play,
    Pause,
}

#[derive(Clone, Debug)]
pub enum Message {
    PlayPause,
}

impl Controls {
    pub fn new() -> Controls {
        Controls {
            running: Running::Play,
            pp_btn: button::State::default(),
        }
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
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
            let content = Column::new()
            .spacing(20)
            .align_items(Align::Center)
            .push(Space::new(Length::Units(800), Length::Units(200)))
            .push(Text::new("My graph".to_owned()))
            .push(Button::new(&mut self.pp_btn,
                              Text::new(match self.running {
                                  Running::Play => "Running",
                                  Running::Pause => "Paused",
                              })
                                  .horizontal_alignment(HorizontalAlignment::Center))
                  .on_press(Message::PlayPause)
                  .width(Length::Units(150)));
        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}
