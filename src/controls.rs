use iced_wgpu::Renderer;
use iced_winit::{
    button, Align, Color, Column, Command, Element, Length, Program, Row,
    Button, Text,
};

pub struct Controls {
    background_color: Color,
    buttons: [button::State; 1],
}

#[derive(Debug, Clone)]
pub enum Message {
    ChangeBackgroundColor,
}

impl Controls {
    pub fn new() -> Controls {
        Controls {
            background_color: Color::BLACK,
            buttons: Default::default(),
        }
    }

    pub fn background_color(&self) -> Color {
        self.background_color
    }
}

impl Program for Controls {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ChangeBackgroundColor => {
                self.background_color = if self.background_color == Color::BLACK {
                    Color::WHITE
                } else {
                    Color::BLACK
                };
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        Row::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Align::End)
            .push(
                Column::new()
                    .width(Length::Fill)
                    .align_items(Align::End)
                    .push(
                        Column::new()
                            .padding(10)
                            .spacing(10)
                            .push(Button::new(&mut self.buttons[0], Text::new("Change background"))
                                .on_press(Message::ChangeBackgroundColor)
                            ),
                    ),
            )
            .into()
    }
}
