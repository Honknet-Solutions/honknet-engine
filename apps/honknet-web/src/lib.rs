use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, MessageEvent, WebSocket};
#[wasm_bindgen]
pub struct WebRuntime {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    socket: Option<WebSocket>,
    x: f64,
    y: f64,
}

#[wasm_bindgen]
impl WebRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Result<Self, JsValue> {
        console_error_panic_hook::set_once();
        let d = web_sys::window().unwrap().document().unwrap();
        let canvas: HtmlCanvasElement = d
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("canvas not found"))?
            .dyn_into()?;
        let ctx: CanvasRenderingContext2d = canvas.get_context("2d")?.unwrap().dyn_into()?;
        Ok(Self {
            canvas,
            ctx,
            socket: None,
            x: 64.,
            y: 64.,
        })
    }
    pub fn connect(&mut self, url: &str) -> Result<(), JsValue> {
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        let cb = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            let _ = e.data();
        });
        ws.set_onmessage(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
        self.socket = Some(ws);
        Ok(())
    }
    pub fn frame(&mut self, dt: f64) {
        self.x = (self.x + dt * 40.) % (self.canvas.width() as f64);
        self.ctx.set_fill_style_str("#050811");
        self.ctx.fill_rect(
            0.,
            0.,
            self.canvas.width() as f64,
            self.canvas.height() as f64,
        );
        self.ctx.set_fill_style_str("#21e6c1");
        self.ctx.fill_rect(self.x, self.y, 24., 24.);
    }
    pub fn send_binary(&self, data: &[u8]) -> Result<(), JsValue> {
        self.socket
            .as_ref()
            .ok_or_else(|| JsValue::from_str("not connected"))?
            .send_with_u8_array(data)
    }
}
