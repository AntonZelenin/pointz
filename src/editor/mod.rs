use crate::widgets::fps;

use iced::alignment::{self, Alignment};
use iced_wgpu::{Backend, Renderer, Settings, wgpu};
use iced::widget::{button, column, horizontal_space, vertical_space, row, text};
use iced_winit::{Color, Command, Element, Length, Program, program, winit, Debug, Size};
use iced_winit::winit::dpi::PhysicalPosition;
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
        let mut renderer = Renderer::new(Backend::new(device, Settings::default(), texture_format));
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
    // buttons: [State; 1],
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
            // buttons: Default::default(),
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

    fn view(&self) -> Element<Message, Renderer> {
        column![
            row![
                // text("1").style(Color::from([1.0, 1.0, 1.0])),
                horizontal_space(Length::Fill),
                text(self.fps.to_string()).style(Color::from([1.0, 1.0, 1.0])),
            ],
            vertical_space(Length::Fill),
            row![
                text(self.debug_info.clone())
                    .style(Color::from([1.0, 1.0, 1.0]))
                    .vertical_alignment(alignment::Vertical::Center),
                horizontal_space(Length::Fill),
                button("Change background").on_press(Message::ChangeBackgroundColor),
            ]
        ]
            .width(Length::Fill)
            .padding(5)
            .into()
    }
}
