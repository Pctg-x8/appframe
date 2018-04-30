
extern crate libc;
#[cfg(feature = "with_bedrock")] extern crate bedrock;

#[cfg(target_os = "macos")] #[macro_use] extern crate objc;
#[cfg(target_os = "macos")] extern crate appkit;
#[cfg(target_os = "macos")] #[macro_use] extern crate appkit_derive;
#[cfg(target_os = "macos")] mod macos;
#[cfg(target_os = "macos")] pub use macos::{GUIApplication, NativeWindow, NativeView, NativeWindowBuilder};

#[cfg(windows)] extern crate winapi;
#[cfg(windows)] mod win32;
#[cfg(windows)] pub use win32::{GUIApplication, NativeWindow, NativeView, NativeWindowBuilder};

#[cfg(feature = "with_xcb")] mod rxcb;
#[cfg(feature = "with_xcb")] mod xcb;
#[cfg(feature = "with_xcb")] pub use xcb::{GUIApplication, NativeWindow, NativeView, NativeWindowBuilder};

use std::rc::Rc;
use std::io::Result as IOResult;

pub trait GUIApplicationRunner<E: EventDelegate>
{
    fn run(delegate: E) -> i32;
    fn event_delegate(&self) -> &E;
}
#[cfg(feature = "with_bedrock")]
pub trait BedrockRenderingServer
{
    fn presentation_support(&self, adapter: &bedrock::PhysicalDevice, rendered_qf: u32) -> bool;
    fn create_surface<WE: WindowEventDelegate>(&self, w: &NativeView<WE>, instance: &bedrock::Instance)
        -> bedrock::Result<bedrock::Surface>;
}
pub trait Window
{
    fn show(&self);
    #[cfg(feature = "with_bedrock")]
    fn mark_dirty(&self);
}
pub trait WindowBuilder<'c> : Sized
{
    fn new(width: u16, height: u16, caption: &'c str) -> Self;
    /// Set window as closable(if true passed, default) or unclosable(if false passed)
    fn closable(&mut self, c: bool) -> &mut Self;
    /// Set window as resizable(if true passed, default) or unresizable(if false passed)
    fn resizable(&mut self, c: bool) -> &mut Self;
    /// Set whether the window's background is transparent
    fn transparent(&mut self, c: bool) -> &mut Self;

    /// Create a window
    fn create<WE: WindowEventDelegate>(&self, server: &Rc<GUIApplication<WE::ClientDelegate>>, event: &Rc<WE>)
        -> IOResult<NativeWindow<WE>>;
    #[cfg(feature = "with_bedrock")]
    /// Create a Renderable window
    fn create_renderable<WE: WindowEventDelegate>(&self, server: &Rc<GUIApplication<WE::ClientDelegate>>, event: &Rc<WE>)
        -> IOResult<NativeWindow<WE>> where WE::ClientDelegate: 'static;
}

pub trait EventDelegate : Sized
{
    fn postinit(&self, _server: &Rc<GUIApplication<Self>>) { }
    fn on_activated(&self, _server: &Rc<GUIApplication<Self>>) { }

    /*
    #[cfg(feature = "with_bedrock")]
    fn on_init_view(&self, _server: &GUIApplication<Self>, _surface_onto: &NativeView<Self>) { }

    #[cfg(feature = "with_bedrock")]
    fn on_render_period(&self) {}
    */
}

pub trait WindowEventDelegate : Sized
{
    type ClientDelegate: EventDelegate;

    fn init_view(&self, _view: &NativeView<Self>) { }
    fn render(&self) { }
    fn resize(&self, _width: u32, _height: u32, _in_live_resize: bool) { }
}

pub struct EmptyWindowEventDelegate<E: EventDelegate>(std::marker::PhantomData<Rc<E>>);
impl<E: EventDelegate> Default for EmptyWindowEventDelegate<E>
{
    fn default() -> Self { EmptyWindowEventDelegate(std::marker::PhantomData) }
}
impl<E: EventDelegate> WindowEventDelegate for EmptyWindowEventDelegate<E>
{
    type ClientDelegate = E;
}
