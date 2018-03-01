
#[macro_use] extern crate bitflags;
extern crate libc;
#[cfg(feature = "with_ferrite")] extern crate ferrite;

#[cfg(target_os = "macos")] #[macro_use] extern crate objc;
#[cfg(target_os = "macos")] #[macro_use] mod appkit;
#[cfg(target_os = "macos")] mod macos;
#[cfg(target_os = "macos")] pub use macos::{GUIApplication, NativeWindow, NativeWindowBuilder};

#[cfg(windows)] extern crate winapi;
#[cfg(windows)] mod win32;
#[cfg(windows)] pub use win32::{GUIApplication, NativeWindow, NativeWindowBuilder};

use std::rc::Rc;

pub trait GUIApplicationRunner<E: EventDelegate>
{
    fn run(appname: &str, delegate: E) -> i32;
    fn event_delegate(&self) -> &Rc<E>;
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
}
pub trait WindowBuilder<'c> : Sized
{
    type WindowTy : Window;

    fn new(width: u16, height: u16, caption: &'c str) -> Self;
    /// Set window as closable(if true passed, default) or unclosable(if false passed)
    fn closable(&mut self, c: bool) -> &mut Self;
    /// Set window as resizable(if true passed, default) or unresizable(if false passed)
    fn resizable(&mut self, c: bool) -> &mut Self;

    /// Create a window
    fn create(&self) -> Option<Self::WindowTy>;
    #[cfg(feature = "with_ferrite")]
    /// Create a Renderable window
    fn create_renderable<E, S>(&self, server: &Rc<S>) -> Option<Self::WindowTy> where
        E: EventDelegate, S: FerriteRenderingServer + GUIApplicationRunner<E>;
}

pub trait EventDelegate : Sized
{
    #[cfg(feature = "with_ferrite")]
    fn postinit<S: FerriteRenderingServer + GUIApplicationRunner<Self>>(&self, _server: &Rc<S>) { }
    #[cfg(not(feature = "with_ferrite"))]
    fn postinit(&self) { }

    #[cfg(feature = "with_ferrite")]
    fn on_activated<S: FerriteRenderingServer + GUIApplicationRunner<Self>>(&self, _server: &Rc<S>) { }
    #[cfg(not(feature = "with_ferrite"))]
    fn on_activated(&self) {  }

    #[cfg(feature = "with_ferrite")]
    fn on_init_view<S: FerriteRenderingServer>(&self, _server: &S, _surface_onto: &<S as FerriteRenderingServer>::SurfaceSource) { }

    #[cfg(feature = "with_ferrite")]
    fn on_render_period(&self) {}
}
