/// Re-export [`App`] so that wasm-bindgen can find the bindings in the library.
pub use hyperbolic::App;

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("could not initialize logger");
}
