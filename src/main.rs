pub use hyperbolic::App;

// #[cfg(target_arch = "wasm32")]
// pub fn start() {
//     #[cfg(target_arch = "wasm32")]
//     {
//         web_sys::window()
//             .and_then(|w| w.document())
//             .and_then(|d| {
//                 d.get_element_by_id("container")?
//                     .append_child(&web_sys::Element::from(window.canvas()))
//                     .ok()
//             })
//             .expect("could not append canvas to document body");
//         wasm_bindgen_futures::spawn_local(run(event_loop, window));
//     }
//     #[cfg(not(target_arch = "wasm32"))]
//     pollster::block_on(run(event_loop, window));
// }

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use winit::{event_loop::EventLoop, window::Window};

        env_logger::init();

        let event_loop = EventLoop::new();
        let window = Window::new(&event_loop).expect("could not create window");

        pollster::block_on(run(event_loop, window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
    }
}
