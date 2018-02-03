extern crate appframe;
extern crate ferrite;

use appframe::*;
use ferrite as fe;

struct App(NativeWindow);
impl App
{
    fn new() -> Self { App(unsafe { std::mem::zeroed() }) }
}
impl EventDelegate for App
{
    fn postinit(&mut self)
    {
        let w = NativeWindowBuilder::new(640, 360, "Ferrite integration").create().unwrap();
        std::mem::forget(std::mem::replace(&mut self.0, w));
        self.0.show();

        let fi = fe::InstanceBuilder::new("appframe_integ", (0, 1, 0), "Ferrite", (0, 1, 0))
            .add_extensions(vec!["VK_KHR_surface", "VK_MVK_macos_surface", "VK_EXT_debug_report"])
            .add_layer("VK_LAYER_LUNARG_standard_validation")
            .create().unwrap();
        let adapter = fi.enumerate_physical_devices().unwrap().remove(0);
        println!("Vulkan AdapterName: {}", unsafe { std::ffi::CStr::from_ptr(adapter.properties().deviceName.as_ptr()).to_str().unwrap() });
    }
}

fn main() { std::process::exit(GUIApplication::run("ferrite", &mut App::new())); }
