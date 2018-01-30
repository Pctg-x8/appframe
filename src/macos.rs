//! MacOS Runner

use appkit::*;
use objc::runtime::*;
use objc::declare::*;
use std::marker::PhantomData;

struct AppDelegate<F: FnMut()>(*mut Object, PhantomData<F>);
impl<F: FnMut()> AppDelegate<F>
{
    fn new(callback: F, appname: &str) -> Option<Self>
    {
        let nsobject_class = Class::get("NSObject").expect("objc class NSObject");
        let mut classdecl = ClassDecl::new("AppDelegate", nsobject_class).expect("Beginning declaring AppDelegate");
        unsafe
        {
            classdecl.add_ivar::<usize>("callback");
            classdecl.add_ivar::<*mut Object>("appname");
            classdecl.add_method(sel!(applicationDidFinishLaunching:),
                Self::did_finish_launching_cb as extern fn(&Object, Sel, *mut Object));
        }
        let class = classdecl.register();
        let p: *mut Object = unsafe { msg_send![class, new] };
        if p.is_null() { return None; }
        let this = AppDelegate(p, PhantomData);
        unsafe
        {
            (*this.0).set_ivar("callback", Box::into_raw(Box::new(callback)) as usize);
            (*this.0).set_ivar("appname", NSString::new(appname).unwrap().leave_id());
        }
        Some(this)
    }
    extern fn did_finish_launching_cb(this: &Object, _selector: Sel, _notify: *mut Object)
    {
        let nsapp = NSApplication::shared().expect("retrieving shared NSApplication instance");
        let appname = unsafe { NSString::retain_id(*this.get_ivar("appname")) };
        Self::init_menu(&nsapp, appname.to_str());

        let mut cb = unsafe { Box::<F>::from_raw(*this.get_ivar::<usize>("callback") as *mut F) };
        cb();
        nsapp.activate_ignoring_other_apps();
    }
    fn init_menu(nsapp: &NSApplication, appname: &str)
    {
        nsapp.set_main_menu(NSMenu::new().unwrap().add_item({
            NSMenuItem::new("", None, None).unwrap().set_submenu({
                let about_menu = NSMenuItem::new(&format!("About {}", appname), None, None).unwrap();
                let prefs = NSMenuItem::new("Preferences...", None, None).unwrap();
                let services = NSMenuItem::new("Services", None, None).unwrap();
                let hide = NSMenuItem::new(&format!("Hide {}", appname), None, None).unwrap();
                let hideother = NSMenuItem::new("Hide Others", None, None).unwrap();
                let showall = NSMenuItem::new("Show All", None, None).unwrap();
                let quit_menu = NSMenuItem::new(&format!("Quit {}", appname), None, None).unwrap();

                NSMenu::new().unwrap()
                    .add_item(about_menu.set_action(sel!(orderFrontStandardAboutPanel:)))
                    .add_item(&NSMenuItem::separator().unwrap())
                    .add_item(prefs.set_key_equivalent(","))
                    .add_item(&NSMenuItem::separator().unwrap())
                    .add_item(services.set_submenu(&NSMenu::new().unwrap()))
                    .add_item(&NSMenuItem::separator().unwrap())
                    .add_item(hide.set_action(sel!(hide:)).set_key_equivalent("h"))
                    .add_item(hideother.set_action(sel!(hideOtherApplications:)).set_accelerator(NSEventModifierFlags::COMMAND | NSEventModifierFlags::OPTION, "h"))
                    .add_item(showall.set_action(sel!(unhideAllApplications:)))
                    .add_item(&NSMenuItem::separator().unwrap())
                    .add_item(quit_menu.set_action(sel!(terminate:)).set_key_equivalent("q"))
            })
        }))
    }
}

pub struct GUIApplication;
impl ::GUIApplicationRunner for GUIApplication
{
    fn run<F: FnMut()>(appname: &str, appcode: F) -> i32
    {
        let appdelegate = AppDelegate::new(appcode, appname).unwrap();
        let nsapp = NSApplication::shared().expect("initializing shared NSApplication");
        nsapp.set_delegate(appdelegate.0);
        nsapp.set_activation_policy(NSApplicationActivationPolicy::Regular);
        nsapp.run();
        0
    }
}

pub struct NativeWindow(NSWindow);
impl ::Window for NativeWindow
{
    fn new(width: u16, height: u16, caption: &str) -> Option<Self>
    {
        let w = NSWindow::new(NSRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: width as _, height: height as _ } },
            NSWindowStyleMask::TITLED | NSWindowStyleMask::CLOSABLE | NSWindowStyleMask::MINIATURIZABLE | NSWindowStyleMask::RESIZABLE);
        w.map(|w| { w.center(); w.set_title(caption); NativeWindow(w) })
    }
    fn show(&self) { self.0.make_key_and_order_front(NSApplication::shared().unwrap().0); }
}
