use std::ops::Deref;

use wgpu::Device;

use crate::camera::CameraBindGroupLayout;

pub struct PipelineLayout {
    pub pipeline: wgpu::PipelineLayout,
    pub camera: CameraBindGroupLayout,
}
impl PipelineLayout {
    pub fn new(device: &Device) -> Self {
        let camera = CameraBindGroupLayout::new(device);
        PipelineLayout {
            pipeline: device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&camera],
                push_constant_ranges: &[],
            }),
            camera,
        }
    }
}

pub struct Pipeline {
    pub layout: PipelineLayout,
    inner: wgpu::RenderPipeline,
}
impl Pipeline {
    pub fn new(device: &Device, swapchain_format: wgpu::TextureFormat) -> Self {
        let layout = PipelineLayout::new(device);
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        Pipeline {
            inner: device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout.pipeline),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[super::Vertex::LAYOUT],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(swapchain_format.into())],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            }),
            layout,
        }
    }
}
impl Deref for Pipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
