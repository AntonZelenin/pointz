use iced_winit::{Alignment, Color, Command, Element, Length, Program, program, winit, Debug, Size};
use crate::widgets::fps;
use iced_winit::winit::dpi::PhysicalPosition;
use iced_wgpu::{Backend, Renderer, Settings, wgpu};
use iced_winit::widget::{button, Button, Column, Row, Text};
use winit::dpi::PhysicalSize;

pub struct GUI {
    pub renderer: Renderer,
    pub program_state: program::State<GUIState>,
    // todo keep a list of widgets, come up with a normal design
    pub(crate) fps_meter: fps::Meter,
    pub cursor_position: PhysicalPosition<f64>,
    pub debug: Debug,

}

impl GUI {
    pub fn new(device: &wgpu::Device, scale_factor: f64, size: PhysicalSize<u32>, texture_format: wgpu::TextureFormat) -> GUI {
        let mut renderer = iced_wgpu::Renderer::new(Backend::new(device, Settings::default(), texture_format));
        let mut debug = Debug::new();
        let program_state = program::State::new(
            GUIState::new(),
            Size::new(size.width as f32, size.height as f32),
            // conversion::cursor_position(PhysicalPosition::new(-1.0, -1.0), scale_factor),
            &mut renderer,
            &mut debug,
        );
        GUI {
            renderer,
            program_state,
            fps_meter: fps::Meter::new(),
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            debug,
        }
    }
}

pub struct GUIState {
    background_color: Color,
    buttons: [button::State; 1],
    fps: i32,
    debug_info: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    ChangeBackgroundColor,
    UpdateFps(i32),
    DebugInfo(String),
}

impl GUIState {
    pub fn new() -> GUIState {
        GUIState {
            background_color: Color::BLACK,
            buttons: Default::default(),
            fps: 0,
            debug_info: "".to_string(),
        }
    }

    pub fn background_color(&self) -> Color {
        self.background_color
    }
}

impl Program for GUIState {
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
            Message::UpdateFps(val) => {
                self.fps = val;
            }
            Message::DebugInfo(s) => {
                self.debug_info = s;
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        Row::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::End)
            .push(
                Column::new()
                    .width(Length::Shrink)
                    .align_items(Alignment::Start)
                    .push(
                        Text::new(self.fps.to_string())
                            .size(30)
                            .color([0.8, 0.8, 0.8]),
                    )
                    .push(
                        Text::new(format!("{}", self.debug_info))
                            .size(20)
                            .color([0.8, 0.8, 0.8]),
                    ),
            )
            .push(
                Column::new()
                    .width(Length::Fill)
                    .align_items(Alignment::End)
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
