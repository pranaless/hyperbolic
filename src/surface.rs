use std::ops::Deref;

use cgmath::Vector2;
use parking_lot::Mutex;
use wgpu::{Device, Queue};

use crate::Window;

pub struct State {
    pub device: Device,
    pub queue: Queue,
}

pub struct Surface<W> {
    pub window: W,
    surface: wgpu::Surface,
    config: Mutex<wgpu::SurfaceConfiguration>,
    pub swapchain_format: wgpu::TextureFormat,
}
impl<W: Window> Surface<W> {
    pub async fn new(window: W) -> (State, Self) {
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
        let size = window.size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.x,
            height: size.y,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface.get_supported_alpha_modes(&adapter)[0],
        };
        surface.configure(&device, &config);

        (
            State { device, queue },
            Surface {
                window,
                surface,
                config: Mutex::new(config),
                swapchain_format,
            },
        )
    }

    pub fn resize(&self, state: &State, size: Vector2<u32>) {
        let mut config = self.config.lock();
        config.width = size.x;
        config.height = size.y;
        self.surface.configure(&state.device, &config);
    }

    pub fn aspect_ratio(&self) -> f64 {
        let config = self.config.lock();
        config.width as f64 / config.height as f64
    }

    pub fn size(&self) -> Vector2<f64> {
        let config = self.config.lock();
        Vector2::new(config.width as f64, config.height as f64)
    }
}
impl<W> Deref for Surface<W> {
    type Target = wgpu::Surface;

    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}
