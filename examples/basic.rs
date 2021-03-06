extern crate appframe;

use appframe::*;
use std::rc::Rc;
use std::cell::RefCell;

pub struct Application(RefCell<Option<NativeWindow<EmptyWindowEventDelegate<Application>>>>);
impl EventDelegate for Application
{
    fn postinit(&self, srv: &Rc<GUIApplication<Self>>)
    {
        let w = NativeWindowBuilder::new(640, 360, "AppFrame basic example")
            .resizable(false).transparent(true).create(&srv, &Rc::new(EmptyWindowEventDelegate::default())).expect("Creating MainWindow");
        *self.0.borrow_mut() = Some(w);
        self.0.borrow().as_ref().unwrap().show();
    }
}
impl Application
{
    fn new() -> Self { Application(RefCell::new(None)) }
}

fn main()
{
    GUIApplication::run(Application::new());
}
