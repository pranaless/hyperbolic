use cgmath::Vector2;
use js_sys::Function;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle, WebDisplayHandle,
    WebWindowHandle,
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::Window;

#[wasm_bindgen]
pub struct AppWindow {
    redraw: Function,
    canvas: HtmlCanvasElement,
}
#[wasm_bindgen]
impl AppWindow {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement, redraw: Function) -> Self {
        canvas.dataset().set("rawHandle", "1").unwrap();
        AppWindow { redraw, canvas }
    }
}

unsafe impl HasRawWindowHandle for AppWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut handle = WebWindowHandle::empty();
        handle.id = self
            .canvas
            .dataset()
            .get("rawHandle")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap();
        debug_assert!(handle.id != 0);
        RawWindowHandle::Web(handle)
    }
}
unsafe impl HasRawDisplayHandle for AppWindow {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Web(WebDisplayHandle::empty())
    }
}
impl Window for AppWindow {
    fn size(&self) -> Vector2<u32> {
        Vector2::new(self.canvas.width(), self.canvas.height())
    }

    fn request_redraw(&self) {
        self.redraw.call0(&JsValue::NULL).unwrap();
    }
}
