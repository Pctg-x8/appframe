
#[macro_use] extern crate bitflags;
extern crate libc;

#[cfg(target_os = "macos")] #[macro_use] extern crate objc;
#[cfg(target_os = "macos")] #[macro_use] mod appkit;
#[cfg(target_os = "macos")] mod macos;
#[cfg(target_os = "macos")] pub use macos::{GUIApplication, NativeWindow};

pub trait GUIApplicationRunner
{
    fn run<F: FnMut()>(appname: &str, appcode: F) -> i32;
}
pub trait Window : Sized
{
    fn new(width: u16, height: u16, caption: &str) -> Option<Self>;
    fn show(&self);
}
