use std::ops::Deref;

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, One};
use wgpu::{util::DeviceExt, Device};

pub struct Camera {
    viewport: Matrix4<f64>,
    pub transform: Matrix4<f64>,
}
impl Camera {
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
        Camera {
            viewport: Self::ortho(aspect),
            transform: Matrix4::one(),
        }
    }

    pub fn update_viewport(&mut self, aspect: f64) {
        self.viewport = Self::ortho(aspect);
    }
}

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct CameraUniform {
    viewport: [f32; 16],
    transform: [f32; 16],
}
impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
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
    pub fn new(device: &Device, layout: &CameraBindGroupLayout, camera: &Camera) -> Self {
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

    pub fn update(&mut self, queue: &wgpu::Queue, camera: &Camera) {
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
