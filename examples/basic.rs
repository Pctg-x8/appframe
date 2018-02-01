extern crate appframe;

use appframe::*;

pub struct Application(NativeWindow);
impl EventDelegate for Application
{
    fn postinit(&mut self)
    {
        std::mem::forget(std::mem::replace(&mut self.0, NativeWindow::new(320, 240, "appframe basic example").unwrap()));
        self.0.show();
    }
}
impl Application
{
    fn new() -> Self { Application(unsafe { std::mem::uninitialized() }) }
}

fn main()
{
    let e = GUIApplication::run("basic", &mut Application::new());
    std::process::exit(e);
}
