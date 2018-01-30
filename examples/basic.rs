extern crate appframe;

use appframe::*;

fn main()
{
    let mut mainwnd = None;
    let e = GUIApplication::run("basic", ||
    {
        mainwnd = Some(NativeWindow::new(320, 240, "appframe basic example").unwrap());
        mainwnd.as_ref().unwrap().show();
    });
    std::process::exit(e);
}
