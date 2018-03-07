
#[cfg(target_os = "macos")] #[macro_use] extern crate bitflags;
extern crate libc;
#[cfg(feature = "with_ferrite")] extern crate ferrite;

#[cfg(target_os = "macos")] #[macro_use] extern crate objc;
#[cfg(target_os = "macos")] #[macro_use] mod appkit;
#[cfg(target_os = "macos")] mod macos;
#[cfg(target_os = "macos")] pub use macos::{GUIApplication, NativeWindow, NativeWindowBuilder};

#[cfg(windows)] extern crate winapi;
#[cfg(windows)] mod win32;
#[cfg(windows)] pub use win32::{GUIApplication, NativeWindow, NativeWindowBuilder};

#[cfg(feature = "with_xcb")] mod rxcb;
#[cfg(feature = "with_xcb")] mod xcb;
#[cfg(feature = "with_xcb")] pub use xcb::{GUIApplication, NativeWindow, NativeWindowBuilder};

use std::rc::Rc;
use std::io::Result as IOResult;

pub trait GUIApplicationRunner<E: EventDelegate>
{
    fn run(delegate: E) -> i32;
}
#[cfg(feature = "with_ferrite")]
pub trait FerriteRenderingServer
{
    type SurfaceSource;

    fn presentation_support(&self, adapter: &ferrite::PhysicalDevice, rendered_qf: u32) -> bool;
    fn create_surface(&self, w: &Self::SurfaceSource, instance: &ferrite::Instance) -> ferrite::Result<ferrite::Surface>;
}
pub trait Window
{
    fn show(&self);
    #[cfg(feature = "with_ferrite")]
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
    fn create<E: EventDelegate>(&self, server: &Rc<GUIApplication<E>>) -> IOResult<NativeWindow<E>>;
    #[cfg(feature = "with_ferrite")]
    /// Create a Renderable window
    fn create_renderable<E: EventDelegate + 'static>(&self, server: &Rc<GUIApplication<E>>) -> IOResult<NativeWindow<E>>;
}

pub trait EventDelegate : Sized
{
    fn postinit(&self, _server: &Rc<GUIApplication<Self>>) { }
    fn on_activated(&self, _server: &Rc<GUIApplication<Self>>) { }

    #[cfg(feature = "with_ferrite")]
    fn on_init_view(&self, _server: &GUIApplication<Self>, _surface_onto: &<GUIApplication<Self> as FerriteRenderingServer>::SurfaceSource) { }

    #[cfg(feature = "with_ferrite")]
    fn on_render_period(&self) {}
}
