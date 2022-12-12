use std::num::ParseIntError;
use std::str::FromStr;

use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, Matrix3, Matrix4, Vector2, Vector3};
use tiling::TilingGenerator;
use wgpu::util::DeviceExt;
use wgpu::Device;
use winit::event::{ElementState, Event, KeyboardInput, MouseButton, WindowEvent};
use winit::{event_loop::EventLoop, window::Window};

pub mod camera;
pub mod pipeline;
pub mod tiling;

use camera::{Camera, CameraBindGroup};
use pipeline::Pipeline;

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

pub struct CameraController {
    track: bool,
    value: Option<Vector2<f64>>,
}
impl CameraController {
    pub fn new() -> Self {
        CameraController {
            track: false,
            value: None,
        }
    }

    pub fn update(&mut self, pos: Vector2<f64>) -> Option<Vector2<f64>> {
        match self.value {
            Some(ref mut value) if self.track => {
                let delta = pos - *value;
                *value = pos;
                Some(delta)
            }
            None if self.track => {
                self.value = Some(pos);
                None
            }
            _ => None,
        }
    }

    pub fn set_track(&mut self, track: bool) {
        self.track = track;
        if !track {
            self.value = None;
        }
    }
}
impl Default for CameraController {
    fn default() -> Self {
        Self::new()
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

pub async fn run(event_loop: EventLoop<()>, window: Window) {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("failed to create device");

    let swapchain_format = surface.get_supported_formats(&adapter)[0];
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface.get_supported_alpha_modes(&adapter)[0],
    };
    surface.configure(&device, &config);

    let pipeline = Pipeline::new(&device, swapchain_format);

    let mut camera_controller = CameraController::new();
    let mut camera = Camera::new(size.width as f64 / size.height as f64);
    let mut camera_bind_group = CameraBindGroup::new(&device, &pipeline.layout.camera, &camera);

    let mut depth = 5;
    #[cfg(not(target_arch = "wasm32"))]
    let mut tiling = TilingGenerator::open("4,5-tiling").unwrap();
    #[cfg(target_arch = "wasm32")]
    let tiling = TilingGenerator::new(include_str!("4,5-tiling.txt"));
    let mut mesh = Mesh::new(&device, tiling.generate(depth));

    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter, &pipeline);

        *control_flow = winit::event_loop::ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                config.width = size.width;
                config.height = size.height;
                surface.configure(&device, &config);
                camera.update_viewport(size.width as f64 / size.height as f64);
                camera_bind_group.update(&queue, &camera);
                window.request_redraw();
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                scancode, state, ..
                            },
                        ..
                    },
                ..
            } => {
                if matches!(state, ElementState::Pressed) {
                    match scancode {
                        #[cfg(not(target_arch = "wasm32"))]
                        19 /* R */ => {
                            tiling = TilingGenerator::open("4,5-tiling").unwrap();
                            mesh = Mesh::new(&device, tiling.generate(depth));
                            window.request_redraw();
                        }
                        103 /* Up */ => {
                            depth += 1;
                            mesh = Mesh::new(&device, tiling.generate(depth));
                            window.request_redraw();
                        }
                        108 /* Up */ => {
                            depth -= 1;
                            mesh = Mesh::new(&device, tiling.generate(depth));
                            window.request_redraw();
                        }
                        _ => {}
                    }
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
            } => camera_controller.set_track(ElementState::Pressed == state),
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                if let Some(delta) = camera_controller.update(Vector2::new(position.x, -position.y))
                {
                    camera.transform =
                        Matrix4::from(translation(delta * 2.0 / config.height as f64))
                            * camera.transform;
                    camera_bind_group.update(&queue, &camera);
                    window.request_redraw();
                }
            }
            Event::RedrawRequested(_) => {
                let frame = surface
                    .get_current_texture()
                    .expect("failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
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
                    rpass.set_pipeline(&pipeline);
                    rpass.set_bind_group(0, &camera_bind_group, &[]);
                    rpass.set_vertex_buffer(0, mesh.vertex.slice(..));
                    rpass.set_index_buffer(mesh.index.slice(..), wgpu::IndexFormat::Uint32);
                    rpass.draw_indexed(0..(mesh.index.size() / 4) as _, 0, 0..1);
                }
                queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = winit::event_loop::ControlFlow::Exit,
            _ => {}
        }
    })
}
