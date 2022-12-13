use std::{ops::Deref, sync::Arc};

use wgpu::Device;

use crate::camera::CameraBindGroupLayout;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Projection {
    Poincare,
    Klein,
    Hyperboloid,
}
impl Projection {
    pub fn shader_source(&self) -> wgpu::ShaderModuleDescriptor {
        match self {
            Projection::Poincare => wgpu::include_wgsl!("poincare.wgsl"),
            Projection::Klein => wgpu::include_wgsl!("klein.wgsl"),
            Projection::Hyperboloid => wgpu::include_wgsl!("hyperboloid.wgsl"),
        }
    }
}

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
    pub layout: Arc<PipelineLayout>,
    inner: wgpu::RenderPipeline,
}
impl Pipeline {
    pub fn new(
        device: &Device,
        projection: Projection,
        swapchain_format: wgpu::TextureFormat,
    ) -> Self {
        let layout = Arc::new(PipelineLayout::new(device));
        Self::with_layout(device, layout, projection, swapchain_format)
    }

    pub fn with_layout(
        device: &Device,
        layout: Arc<PipelineLayout>,
        projection: Projection,
        swapchain_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(projection.shader_source());
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
