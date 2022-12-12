use std::num::ParseIntError;
use std::str::FromStr;

use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, Matrix3, Vector2, Vector3};
use parking_lot::Mutex;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use wasm_bindgen::prelude::wasm_bindgen;
use wgpu::util::DeviceExt;
use wgpu::Device;

pub mod camera;
pub mod pipeline;
pub mod surface;
pub mod tiling;

#[cfg_attr(target_arch = "wasm32", path = "window/web.rs")]
#[cfg_attr(not(target_arch = "wasm32"), path = "window/desktop.rs")]
pub mod window;

use camera::Camera;
use pipeline::Pipeline;
use surface::{State, Surface};
use tiling::TilingGenerator;
use window::AppWindow;

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}
impl FromStr for Color {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let n = u32::from_str_radix(s, 16)?;
        Ok(Color {
            r: (n >> 16) as _,
            g: (n >> 8) as _,
            b: n as _,
        })
    }
}
impl From<Color> for [f32; 3] {
    fn from(color: Color) -> Self {
        [
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
        ]
    }
}

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}
impl Vertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as _,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 3 * 4,
                shader_location: 1,
            },
        ],
    };
}

pub struct Mesh {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
}
impl Mesh {
    pub fn new(device: &Device, (vertex, index): (Vec<Vertex>, Vec<u32>)) -> Self {
        let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&vertex),
        });

        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(&index),
        });
        Mesh { vertex, index }
    }
}

fn hyperpoint(x: f32, y: f32) -> Vector3<f32> {
    let w = (1.0 + x * x + y * y).sqrt();
    Vector3::new(x, y, w)
}

pub fn translation(pos: Vector2<f64>) -> Matrix3<f64> {
    let w = (1.0 + pos.magnitude2()).sqrt();
    let col = (pos / (w + 1.0)).extend(1.0);
    Matrix3::from_cols(
        col * pos.x + Vector3::unit_x(),
        col * pos.y + Vector3::unit_y(),
        pos.extend(w),
    )
}

pub trait Window: HasRawWindowHandle + HasRawDisplayHandle {
    fn size(&self) -> Vector2<u32>;
    fn request_redraw(&self);
}

#[wasm_bindgen]
pub struct App {
    state: State,
    surface: Surface<AppWindow>,
    pipeline: Pipeline,
    camera: Mutex<Camera>,

    tiling: TilingGenerator,
    mesh: Mutex<Mesh>,
}
#[wasm_bindgen]
impl App {
    #[wasm_bindgen(constructor)]
    pub async fn new(window: AppWindow) -> Self {
        let (state, surface) = Surface::new(window).await;
        let pipeline = Pipeline::new(&state.device, surface.swapchain_format);
        let camera = Camera::new(
            &state.device,
            &pipeline.layout.camera,
            surface.aspect_ratio(),
        );

        let tiling = TilingGenerator::new(include_str!("4,5-tiling.txt"));
        let mesh = Mesh::new(&state.device, tiling.generate(5));

        App {
            state,
            surface,
            pipeline,
            camera: Mutex::new(camera),
            tiling,
            mesh: Mutex::new(mesh),
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        let aspect_ratio = width as f64 / height as f64;
        self.camera
            .lock()
            .update_viewport(&self.state.queue, aspect_ratio);
        self.surface
            .resize(&self.state, Vector2::new(width, height));
        self.surface.window.request_redraw();
    }

    pub fn update_delta(&self, x: f64, y: f64) {
        self.camera
            .lock()
            .update_delta(&self.state.queue, &self.surface, Vector2::new(x, -y));
    }

    pub fn reset_delta(&self) {
        self.camera.lock().reset_delta();
    }

    pub fn set_depth(&self, depth: usize) {
        let mut mesh = self.mesh.lock();
        *mesh = Mesh::new(&self.state.device, self.tiling.generate(depth));
        self.surface.window.request_redraw();
    }

    pub fn draw(&self) {
        let frame = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let camera = self.camera.lock();
            let mesh = self.mesh.lock();

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &camera.bind_group, &[]);
            rpass.set_vertex_buffer(0, mesh.vertex.slice(..));
            rpass.set_index_buffer(mesh.index.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..(mesh.index.size() / 4) as _, 0, 0..1);
        }
        self.state.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn run(event_loop: EventLoop<()>, window: AppWindow) {
    use winit::event::{ElementState, Event, KeyboardInput, MouseButton, WindowEvent};
    use winit::event_loop::EventLoop;

    let app = App::new(window).await;

    let mut track = false;
    let mut depth = 5;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => app.resize(Vector2::new(size.width, size.height)),
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                scancode,
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    },
                ..
            } => {
                match scancode {
                    103 /* Up */ => {
                        depth += 1;
                        app.set_depth(depth);
                    }
                    108 /* Up */ => {
                        depth -= 1;
                        app.set_depth(depth);
                    }
                    _ => {}
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state,
                        button: MouseButton::Left,
                        ..
                    },
                ..
            } => track = ElementState::Pressed == state,
            Event::WindowEvent {
                event:
                    WindowEvent::CursorMoved {
                        position: winit::dpi::PhysicalPosition { x, y },
                        ..
                    },
                ..
            } if track => app
                .camera
                .lock()
                .update_delta(&app.surface, Vector2::new(x, -y)),
            Event::RedrawRequested(_) => app.draw(),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = winit::event_loop::ControlFlow::Exit,
            _ => {}
        }
    })
}
