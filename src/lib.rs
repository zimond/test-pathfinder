use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_renderer::concurrent::executor::SequentialExecutor;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::gpu_data::RenderCommand;
use pathfinder_renderer::options::{BuildOptions, RenderCommandListener, RenderTransform};
use pathfinder_resources::embedded::EmbeddedResourceLoader;
use pathfinder_svg::BuiltSVG;
use pathfinder_webgl::WebGlDevice;
use std::cell::RefCell;
use std::rc::Rc;
use usvg::Tree;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, HtmlCanvasElement, WebGl2RenderingContext};

struct Listener<F>(RefCell<F>);
impl<F: FnMut(RenderCommand)> RenderCommandListener for Listener<F> {
    fn send(&self, command: RenderCommand) {
        let mut guard = self.0.borrow_mut();
        let f = &mut *guard;
        f(command)
    }
}
impl<F: FnMut(RenderCommand)> Listener<F> {
    fn new(f: F) -> Self {
        Listener(RefCell::new(f))
    }
}

// we don't have threads on wasm.
unsafe impl<F: FnMut(RenderCommand)> Send for Listener<F> {}
unsafe impl<F: FnMut(RenderCommand)> Sync for Listener<F> {}

const SVG: &str = include_str!("test.svg");
// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    let tree = Tree::from_str(SVG, &usvg::Options::default()).unwrap();
    let mut svg = BuiltSVG::from_tree(&tree);
    svg.scene.set_view_box(svg.scene.view_box().scale(2.0));

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas: HtmlCanvasElement = document
        .get_element_by_id("stage")
        .unwrap()
        .dyn_into()
        .unwrap();
    let context: WebGl2RenderingContext = canvas
        .get_context("webgl2")
        .unwrap()
        .expect("failed to get WebGl2 context")
        .dyn_into()
        .unwrap();
    let device = WebGlDevice::new(context);
    let renderer = Rc::new(RefCell::new(Renderer::new(
        device,
        &EmbeddedResourceLoader::new(),
        DestFramebuffer::full_window(Vector2I::new(1139, 774)),
        RendererOptions {
            background_color: None,
        },
    )));
    renderer.borrow_mut().begin_scene();
    let tr = Transform2F::from_scale(Vector2F::new(2.0, -2.0))
        * Transform2F::from_translation(Vector2F::new(0.0, -774.0));
    let opts = BuildOptions {
        transform: RenderTransform::Transform2D(tr),
        dilation: Vector2F::default(),
        subpixel_aa_enabled: false,
    };
    let r_c = renderer.clone();
    svg.scene.build(
        opts,
        Box::new(Listener::new(move |cmd| {
            r_c.borrow_mut().render_command(&cmd);
        })) as Box<dyn RenderCommandListener>,
        &SequentialExecutor,
    );
    renderer.borrow_mut().end_scene();
    // Your code goes here!
    console::log_1(&JsValue::from_str("Hello world!"));

    Ok(())
}
