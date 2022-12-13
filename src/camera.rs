use std::ops::Deref;

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, One, Vector2};
use wgpu::{util::DeviceExt, Device, Queue};

use crate::{translation, window::Window, Surface};

pub struct Camera {
    pub bind_group: CameraBindGroup,
    controller: CameraController,
    tracker: CameraTracker,
}
impl Camera {
    pub fn new(device: &Device, layout: &CameraBindGroupLayout, aspect_ratio: f64) -> Self {
        let tracker = CameraTracker::new(aspect_ratio);
        let bind_group = CameraBindGroup::new(device, layout, &tracker);
        Camera {
            tracker,
            bind_group,
            controller: CameraController::new(),
        }
    }

    pub fn update_viewport(&mut self, queue: &Queue, aspect_ratio: f64) {
        self.tracker.update_viewport(aspect_ratio);
        self.bind_group.update(queue, &self.tracker);
    }

    pub fn update_delta<W: Window>(
        &mut self,
        queue: &Queue,
        surface: &Surface<W>,
        pos: Vector2<f64>,
    ) {
        if let Some(delta) = self.controller.update(pos) {
            self.tracker.translate(delta * 2.0 / surface.size().y);
            self.bind_group.update(queue, &self.tracker);
            surface.window.request_redraw();
        }
    }

    pub fn reset_delta(&mut self) {
        self.controller.reset();
    }
}

pub struct CameraController {
    value: Option<Vector2<f64>>,
}
impl CameraController {
    pub fn new() -> Self {
        CameraController { value: None }
    }

    pub fn update(&mut self, pos: Vector2<f64>) -> Option<Vector2<f64>> {
        self.value.replace(pos).map(|old| pos - old)
    }

    pub fn reset(&mut self) {
        self.value = None;
    }
}
impl Default for CameraController {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CameraTracker {
    viewport: Matrix4<f64>,
    pub transform: Matrix4<f64>,
}
impl CameraTracker {
    #[rustfmt::skip]
    fn ortho(aspect: f64) -> Matrix4<f64> {
        Matrix4::new(
            1.0 / aspect, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, -0.5, 0.0,
            0.0, 0.0, 0.5, 1.0,
        )
    }

    pub fn new(aspect: f64) -> Self {
        CameraTracker {
            viewport: Self::ortho(aspect),
            transform: Matrix4::one(),
        }
    }

    pub fn update_viewport(&mut self, aspect: f64) {
        self.viewport = Self::ortho(aspect);
    }

    pub fn translate(&mut self, delta: Vector2<f64>) {
        self.transform = Matrix4::from(translation(delta)) * self.transform;
    }
}

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct CameraUniform {
    viewport: [f32; 16],
    transform: [f32; 16],
}
impl CameraUniform {
    pub fn new(camera: &CameraTracker) -> Self {
        CameraUniform {
            viewport: *camera.viewport.cast().unwrap().as_ref(),
            transform: *camera.transform.cast().unwrap().as_ref(),
        }
    }
}

pub struct CameraBindGroupLayout {
    inner: wgpu::BindGroupLayout,
}
impl CameraBindGroupLayout {
    pub fn new(device: &Device) -> Self {
        CameraBindGroupLayout {
            inner: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            }),
        }
    }
}
impl Deref for CameraBindGroupLayout {
    type Target = wgpu::BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct CameraBindGroup {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}
impl CameraBindGroup {
    pub fn new(device: &Device, layout: &CameraBindGroupLayout, camera: &CameraTracker) -> Self {
        let uniform = CameraUniform::new(camera);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&uniform),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        CameraBindGroup { buffer, bind_group }
    }

    pub fn update(&mut self, queue: &Queue, camera: &CameraTracker) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::bytes_of(&CameraUniform::new(camera)),
        );
    }
}
impl Deref for CameraBindGroup {
    type Target = wgpu::BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.bind_group
    }
}
