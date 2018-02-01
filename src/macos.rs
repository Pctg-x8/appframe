//! MacOS Runner

use appkit::*;
use objc::runtime::*;
use objc::declare::*;
use std::marker::PhantomData;

macro_rules! DeclareObjcClass
{
    (class $t: ident : $p: ident { $($content: tt)* }) =>
    {{
        let parent = Class::get(stringify!($p)).expect(concat!("objc class ", stringify!($p), "not found"));
        let mut d = ClassDecl::new(stringify!($t), parent).expect(concat!("Beginning declaring ", stringify!($t)));
        DeclareObjcClass!(#Declaring(d) $($content)*);
        d.register()
    }};
    (#Declaring($d: expr) - $($name: ident : ($aty: ty))+ = $fr: expr; $($rest: tt)*) =>
    {
        unsafe { $d.add_method(sel!($($name :)+), $fr as extern fn(&Object, Sel $(, $aty)*)); }
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    (#Declaring($d: expr) ivar $name: ident: $vt: ty; $($rest: tt)*) =>
    {
        $d.add_ivar::<$vt>(stringify!($name));
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    (#Declaring($d: expr)) => {  }
}

struct AppDelegate<'d, E: ::EventDelegate + 'd>(*mut Object, PhantomData<&'d mut E>);
impl<'d, E: ::EventDelegate + 'd> AppDelegate<'d, E>
{
    fn new(delegate: &'d mut E, appname: &str) -> Option<Self>
    {
        let class = DeclareObjcClass! { class AppDelegate : NSObject
            {
                ivar delegate: usize;
                ivar appname: *mut Object;
                - applicationDidFinishLaunching:(*mut Object) = Self::did_finish_launching_cb;
            }
        };
        let p: *mut Object = unsafe { msg_send![class, new] };
        if p.is_null() { return None; }
        let this = AppDelegate(p, PhantomData);
        unsafe
        {
            (*this.0).set_ivar("delegate", delegate as *mut _ as usize);
            (*this.0).set_ivar("appname", NSString::new(appname).unwrap().leave_id());
        }
        Some(this)
    }
    extern fn did_finish_launching_cb(this: &Object, _selector: Sel, _notify: *mut Object)
    {
        let nsapp = NSApplication::shared().expect("retrieving shared NSApplication instance");
        let appname = unsafe { NSString::retain_id(*this.get_ivar("appname")) };
        Self::init_menu(&nsapp, appname.to_str());

        let d = unsafe { &mut *(*this.get_ivar::<usize>("delegate") as *mut E) };
        d.postinit();
        nsapp.activate_ignoring_other_apps();
    }
    fn init_menu(nsapp: &NSApplication, appname: &str)
    {
        nsapp.set_main_menu(NSMenu::new().unwrap().add_item({
            NSMenuItem::new("", None, None).unwrap().set_submenu({
                let about_menu = NSMenuItem::new(&format!("About {}", appname), Some(sel!(orderFrontStandardAboutPanel:)), None).unwrap();
                let prefs = NSMenuItem::new("Preferences...", None, Some(&NSString::new(",").unwrap())).unwrap();
                let services = NSMenuItem::new("Services", None, None).unwrap(); services.set_submenu(&NSMenu::new().unwrap());
                let hide = NSMenuItem::new(&format!("Hide {}", appname), Some(sel!(hide:)), Some(&NSString::new("h").unwrap())).unwrap();
                let hideother = NSMenuItem::new("Hide Others", Some(sel!(hideOtherApplications:)), None).unwrap();
                let showall = NSMenuItem::new("Show All", Some(sel!(unhideAllApplications:)), None).unwrap();
                let quit_menu = NSMenuItem::new(&format!("Quit {}", appname), Some(sel!(terminate:)), Some(&NSString::new("q").unwrap())).unwrap();

                NSMenu::new().unwrap()
                    .add_item(&about_menu).add_separator()
                    .add_item(&prefs).add_separator()
                    .add_item(&services).add_separator()
                    .add_item(&hide).add_item(hideother.set_accelerator(NSEventModifierFlags::COMMAND | NSEventModifierFlags::OPTION, "h"))
                    .add_item(&showall).add_separator()
                    .add_item(&quit_menu)
            })
        }))
    }
}

pub struct GUIApplication;
impl ::GUIApplicationRunner for GUIApplication
{
    fn run<E: ::EventDelegate>(appname: &str, delegate: &mut E) -> i32
    {
        let appdelegate = AppDelegate::new(delegate, appname).unwrap();
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
    fn show(&self) { self.0.make_key_and_order_front(NSApplication::shared().unwrap().0); }
}

pub struct NativeWindowBuilder<'c>
{
    style: NSWindowStyleMask, width: u16, height: u16, caption: &'c str
}
impl<'c> ::WindowBuilder<'c> for NativeWindowBuilder<'c>
{
    fn new(width: u16, height: u16, caption: &'c str) -> Self
    {
        NativeWindowBuilder
        {
            style: NSWindowStyleMask::TITLED | NSWindowStyleMask::CLOSABLE | NSWindowStyleMask::MINIATURIZABLE | NSWindowStyleMask::RESIZABLE,
            width, height, caption
        }
    }
    fn closable(&mut self, c: bool) -> &mut Self
    {
        if c { self.style |= NSWindowStyleMask::CLOSABLE; } else { self.style &= !NSWindowStyleMask::CLOSABLE; } self
    }
    fn resizable(&mut self, c: bool) -> &mut Self
    {
        if c { self.style |= NSWindowStyleMask::RESIZABLE } else { self.style &= !NSWindowStyleMask::RESIZABLE; } self
    }

    type WindowTy = NativeWindow;
    fn create(&self) -> Option<NativeWindow>
    {
        let r = NSRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: self.width as _, height: self.height as _ } };
        NSWindow::new(r, self.style).map(|w| { w.center(); w.set_title(self.caption); NativeWindow(w) })
    }
}
