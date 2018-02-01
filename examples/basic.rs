extern crate appframe;

use appframe::*;

pub struct Application(NativeWindow);
impl EventDelegate for Application
{
    fn postinit(&mut self)
    {
        let w = NativeWindowBuilder::new(640, 360, "AppFrame basic example")
            .resizable(false).create().expect("Creating MainWindow");
        std::mem::forget(std::mem::replace(&mut self.0, w));
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
