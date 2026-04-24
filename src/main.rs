use std::num::NonZeroU32;
use std::rc::Rc;

use glam::Vec3;
use softbuffer::{Context, Pixel, Surface};
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowId};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let context = Context::new(event_loop.owned_display_handle()).unwrap();
    let mut app = App {
        context,
        state: AppState::Initial,
    };
    event_loop.run_app(&mut app).unwrap();
}

#[derive(Debug)]
struct App {
    context: Context<OwnedDisplayHandle>,
    state: AppState,
}

#[derive(Debug)]
enum AppState {
    Initial,
    Suspended {
        window: Rc<Window>,
    },
    Running {
        surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    },
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::Init = cause {
            // Create window on startup.
            let window_attrs = Window::default_attributes();
            let window = event_loop
                .create_window(window_attrs)
                .expect("failed creating window");
            self.state = AppState::Suspended {
                window: Rc::new(window),
            };
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Create or re-create the surface.
        let AppState::Suspended { window } = &mut self.state else {
            unreachable!("got resumed event while not suspended");
        };
        let mut surface =
            Surface::new(&self.context, window.clone()).expect("failed creating surface");

        // TODO: https://github.com/rust-windowing/softbuffer/issues/106
        let size = window.inner_size();
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            // Resize surface
            surface.resize(width, height).unwrap();
        }

        self.state = AppState::Running { surface };
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Drop the surface.
        let AppState::Running { surface } = &mut self.state else {
            unreachable!("got resumed event while not running");
        };
        let window = surface.window().clone();
        self.state = AppState::Suspended { window };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let AppState::Running { surface } = &mut self.state else {
            unreachable!("got window event while suspended");
        };

        if surface.window().id() != window_id {
            return;
        }

        match event {
            WindowEvent::Resized(size) => {
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    // Resize surface
                    surface.resize(width, height).unwrap();
                }
            }
            WindowEvent::RedrawRequested => {
                // Get the next buffer.
                let mut buffer = surface.next_buffer().unwrap();
                let width = buffer.width().get() as f32;
                let height = buffer.height().get() as f32;

                // Render into the buffer.
                for (x, y, pixel) in buffer.pixels_iter() {
                    for vertex in TRIANGLE_VERTICES.chunks(3) {
                        let u = x as f32 / width;
                        let v = y as f32 / height;

                        fn same_side(p: Vec3, p2: Vec3, a: Vec3, b: Vec3) -> bool {
                            let cp1 = (b - a).cross(p - a);
                            let cp2 = (b - a).cross(p2 - a);
                            cp1.dot(cp2) >= 0.0
                        }

                        fn point_in_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> bool {
                            same_side(p, a, b, c) && same_side(p, b, a, c) && same_side(p, c, a, b)
                        }

                        let p = Vec3::new(u, 1.0 - v, 0.0) * 2.0 - 1.0;
                        if point_in_triangle(p, vertex[0], vertex[1], vertex[2]) {
                            *pixel = Pixel::new_rgb(255, 0, 0);
                        } else {
                            *pixel = Pixel::new_rgb(0, 0, 0);
                        }
                    }
                }

                // Send the buffer to the compositor.
                buffer.present().unwrap();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }
}

const TRIANGLE_VERTICES: [Vec3; 3] = [
    Vec3::new(0.0, 0.5, 0.0),
    Vec3::new(-0.5, -0.5, 0.0),
    Vec3::new(0.5, -0.5, 0.0),
];
