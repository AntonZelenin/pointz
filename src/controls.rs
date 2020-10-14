use iced_wgpu::Renderer;
use iced_winit::{
    button, Align, Button, Color, Column, Command, Element, Length, Program, Row, Text,
};
use iced_winit::winit::dpi::PhysicalPosition;

pub struct GUI {
    background_color: Color,
    buttons: [button::State; 1],
    pub cursor_position: PhysicalPosition<f64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    ChangeBackgroundColor,
}

impl GUI {
    pub fn new() -> GUI {
        GUI {
            background_color: Color::BLACK,
            buttons: Default::default(),
            cursor_position: PhysicalPosition::new(0.0, 0.0),
        }
    }

    pub fn background_color(&self) -> Color {
        self.background_color
    }
}

impl Program for GUI {
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
                    .width(Length::Shrink)
                    .align_items(Align::Start)
                    .push(
                        Text::new(format!(
                            "Cursor position: x = {}, y = {}",
                            self.cursor_position.x, self.cursor_position.y
                        ))
                        .size(15)
                        .color([0.8, 0.8, 0.8]),
                    ),
            )
            .push(
                Column::new()
                    .width(Length::Fill)
                    .align_items(Align::End)
                    .push(
                        Column::new().padding(10).spacing(10).push(
                            Button::new(&mut self.buttons[0], Text::new("Change background"))
                                .on_press(Message::ChangeBackgroundColor),
                        ),
                    ),
            )
            .into()
    }
}
