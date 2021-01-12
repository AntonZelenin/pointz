use crate::scene::App;
use iced_wgpu::Viewport;
use iced_winit::winit::event::{
    DeviceEvent, ElementState, Event, KeyboardInput, ModifiersState, MouseButton, WindowEvent,
};
use iced_winit::winit::event_loop::ControlFlow;
use iced_winit::{conversion, Size};

const KEEP_CURSOR_POS_FOR_NUM_FRAMES: usize = 3;

pub fn process_events(app: &mut App, event: &Event<()>, control_flow: &mut ControlFlow) {
    match event {
        Event::WindowEvent { event, .. } => {
            let mut modifiers = ModifiersState::default();
            match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(key),
                            state,
                            ..
                        },
                    ..
                } => {
                    app.camera_state
                        .camera_controller
                        .process_keyboard(*key, *state);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    app.camera_state.camera_controller.process_scroll(delta);
                }
                WindowEvent::MouseInput {
                    button: MouseButton::Right,
                    state,
                    ..
                } => {
                    app.camera_state.camera_mode = *state == ElementState::Pressed;
                    app.window
                        .window
                        .set_cursor_visible(!app.camera_state.camera_mode);
                }
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => {
                    if *state == ElementState::Pressed {
                        app.process_left_click();
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if app.camera_state.camera_mode {
                        // make cursor stay at the same place
                        app.window
                            .window
                            .set_cursor_position(app.gui.cursor_position)
                            .unwrap();
                    } else {
                        app.gui.cursor_position = *position;
                    }
                }
                WindowEvent::ModifiersChanged(new_modifiers) => {
                    modifiers = *new_modifiers;
                }
                WindowEvent::Resized(new_size) => {
                    app.window.viewport = Viewport::with_physical_size(
                        Size::new(new_size.width, new_size.height),
                        app.window.window.scale_factor(),
                    );
                    // todo add events to queue and call them only when redraw requested?
                    app.window.resized = true;
                }
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            }
            if let Some(event) = iced_winit::conversion::window_event(
                &event,
                app.window.window.scale_factor(),
                modifiers,
            ) {
                app.gui.program_state.queue_event(event);
            }
        }
        Event::MainEventsCleared => {
            if !app.gui.program_state.is_queue_empty() {
                let _ = app.gui.program_state.update(
                    app.window.viewport.logical_size(),
                    conversion::cursor_position(
                        app.gui.cursor_position,
                        app.window.viewport.scale_factor(),
                    ),
                    None,
                    &mut app.gui.renderer,
                    &mut app.gui.debug,
                );
            }
            app.window.window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            let now = std::time::Instant::now();
            // todo can be moved
            let dt = now - app.rendering.last_render_time;
            app.rendering.last_render_time = now;
            app.update(dt);
            if app.window.resized {
                app.resize(app.window.window.inner_size());
                app.window.resized = false;
            }
            app.rendering.render(app);
        }
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::MouseMotion { delta } => {
                // todo too long, seems will move to a method or sort of
                if app
                    .camera_state
                    .cursor_watcher
                    .last_frames_cursor_deltas
                    .len()
                    > KEEP_CURSOR_POS_FOR_NUM_FRAMES
                {
                    app.camera_state
                        .cursor_watcher
                        .last_frames_cursor_deltas
                        .drain(..1);
                }
                app.camera_state
                    .cursor_watcher
                    .last_frames_cursor_deltas
                    .push(*delta);
                let (mouse_dx, mouse_dy) = app.camera_state.cursor_watcher.get_avg_cursor_pos();
                if app.camera_state.camera_mode {
                    app.camera_state
                        .camera_controller
                        .process_mouse(mouse_dx / 2.0, mouse_dy / 2.0);
                }
            }
            _ => {}
        },
        _ => {}
    };
}
