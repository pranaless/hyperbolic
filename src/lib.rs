use std::num::ParseIntError;
use std::str::FromStr;

use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, Matrix3, Vector2, Vector3};
use log::warn;
use parking_lot::Mutex;
use wasm_bindgen::prelude::wasm_bindgen;
use wgpu::util::DeviceExt;
use wgpu::Device;

pub mod camera;
pub mod pipeline;
pub mod surface;
pub mod tiling;

pub mod window;

use camera::Camera;
use pipeline::{Pipeline, Projection};
use surface::{State, Surface};
use tiling::TilingGenerator;
use window::{AppWindow, Window};

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

pub fn translation(pos: Vector2<f64>) -> Matrix3<f64> {
    let w = (1.0 + pos.magnitude2()).sqrt();
    let col = (pos / (w + 1.0)).extend(1.0);
    Matrix3::from_cols(
        col * pos.x + Vector3::unit_x(),
        col * pos.y + Vector3::unit_y(),
        pos.extend(w),
    )
}

#[rustfmt::skip]
const COLORS: &[Color] = &[
    Color { r: 255, g:   0, b:   0 },
    Color { r: 176, g: 196, b: 222 },
    Color { r:  48, g: 191, b: 190 },
    Color { r: 141, g: 217, b: 205 },
    Color { r:  13, g: 152, b: 187 },
    Color { r:  71, g: 171, b: 205 },
    Color { r:  17, g: 100, b: 179 },
];

#[wasm_bindgen]
pub struct App {
    state: State,
    surface: Surface<AppWindow>,
    pipeline: Pipeline,
    camera: Mutex<Camera>,

    tiling: TilingGenerator,
    mesh: Mesh,
}
#[wasm_bindgen]
impl App {
    #[wasm_bindgen(constructor)]
    pub async fn new(tiling: TilingGenerator, window: AppWindow) -> Self {
        let (state, surface) = Surface::new(window).await;
        let pipeline = Pipeline::new(
            &state.device,
            Projection::Poincare,
            surface.swapchain_format,
        );
        let camera = Camera::new(
            &state.device,
            &pipeline.layout.camera,
            surface.aspect_ratio(),
        );

        let mesh = Mesh::new(&state.device, tiling.generate(COLORS, 5));

        App {
            state,
            surface,
            pipeline,
            camera: Mutex::new(camera),
            tiling,
            mesh,
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

    pub fn set_tiling(&mut self, tiling: TilingGenerator, depth: usize) {
        self.tiling = tiling;
        self.mesh = Mesh::new(&self.state.device, self.tiling.generate(COLORS, depth));
        self.surface.window.request_redraw();
    }

    pub fn set_depth(&mut self, depth: usize) {
        self.mesh = Mesh::new(&self.state.device, self.tiling.generate(COLORS, depth));
        self.surface.window.request_redraw();
    }

    pub fn set_projection(&mut self, name: &str) {
        let projection = match name {
            "poincare" => Projection::Poincare,
            "klein" => Projection::Klein,
            "hyperboloid" => Projection::Hyperboloid,
            _ => {
                warn!("{} is not a valid projection", name);
                return;
            }
        };
        self.pipeline = Pipeline::with_layout(
            &self.state.device,
            self.pipeline.layout.clone(),
            projection,
            self.surface.swapchain_format,
        );
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
            rpass.set_vertex_buffer(0, self.mesh.vertex.slice(..));
            rpass.set_index_buffer(self.mesh.index.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..(self.mesh.index.size() / 4) as _, 0, 0..1);
        }
        self.state.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
