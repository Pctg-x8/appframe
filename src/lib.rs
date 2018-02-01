
#[macro_use] extern crate bitflags;
extern crate libc;

#[cfg(target_os = "macos")] #[macro_use] extern crate objc;
#[cfg(target_os = "macos")] #[macro_use] mod appkit;
#[cfg(target_os = "macos")] mod macos;
#[cfg(target_os = "macos")] pub use macos::{GUIApplication, NativeWindow, NativeWindowBuilder};

pub trait GUIApplicationRunner
{
    fn run<E: EventDelegate>(appname: &str, delegate: &mut E) -> i32;
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
}

pub trait EventDelegate
{
    fn postinit(&mut self) {  }
}
