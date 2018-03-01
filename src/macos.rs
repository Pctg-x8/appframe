//! MacOS Runner

use appkit::*;
use objc::runtime::*;
use objc::declare::*;
use std::rc::*;
use {GUIApplicationRunner, EventDelegate};
#[cfg(feature = "with_ferrite")]
use std::marker::PhantomData;
#[cfg(feature = "with_ferrite")]
use std::mem::transmute;

#[cfg(feature = "with_ferrite")] use ferrite as fe;

/*
#[link(name = "Foundation", kind = "framework")] extern "system"
{
    fn NSStringFromClass(class: &Class) -> *mut Object;
}
*/

#[allow(non_camel_case_types)] pub type objc_id = *mut Object;
macro_rules! DeclareObjcClass
{
    (class $t: ident : $p: ident { $($content: tt)* }) =>
    {{
        let parent = Class::get(stringify!($p)).expect(concat!("objc class ", stringify!($p), "not found"));
        let mut d = ClassDecl::new(stringify!($t), parent).expect(concat!("Beginning declaring ", stringify!($t)));
        DeclareObjcClass!(#Declaring(d) $($content)*);
        d.register()
    }};
    // void with arg
    (#Declaring($d: expr) - $($name: ident : ($aty: ty))+ = $fr: expr; $($rest: tt)*) =>
    {
        unsafe { $d.add_method(sel!($($name :)+), $fr as extern fn(&Object, Sel $(, $aty)*)); }
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    // void noarg
    (#Declaring($d: expr) - $name: ident = $fr: expr; $($rest: tt)*) =>
    {
        unsafe { $d.add_method(sel!($name), $fr as extern fn(&Object, Sel)); }
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    // void noarg mutable-this
    (#Declaring($d: expr) - mut $name: ident = $fr: expr; $($rest: tt)*) =>
    {
        unsafe { $d.add_method(sel!($name), $fr as extern fn(&mut Object, Sel)); }
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    // full
    (#Declaring($d: expr) - ($rty: ty) $($name: ident : ($aty: ty))+ = $fr: expr; $($rest: tt)*) =>
    {
        unsafe { $d.add_method(sel!($($name :)+), $fr as extern fn(&Object, Sel $(, $aty)*) -> $rty); }
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    // noarg
    (#Declaring($d: expr) - ($rty: ty) $name: ident = $fr: expr; $($rest: tt)*) =>
    {
        unsafe { $d.add_method(sel!($name), $fr as extern fn(&Object, Sel) -> $rty); }
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    (#Declaring($d: expr) ivar $name: ident: $vt: ty; $($rest: tt)*) =>
    {
        $d.add_ivar::<$vt>(stringify!($name));
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    (#Declaring($d: expr)) => {  }
}
/// Store boxed pointer into ivar in objc object
unsafe fn store_boxed_ptr<T>(obj: &mut Object, varname: &str, vbox: &Box<T>)
{
    obj.set_ivar(varname, &**vbox as *const _ as usize)
}
#[allow(dead_code)]
/// Store boxed pointer into ivar in objc object by transferring pointer's ownership
unsafe fn move_boxed_ptr<T>(obj: &mut Object, varname: &str, vbox: Box<T>)
{
    obj.set_ivar(varname, Box::into_raw(vbox) as usize)
}
/// Extract boxed pointer from ivar in objc object
unsafe fn retrieve_ptr<'a, T>(obj: &Object, varname: &str) -> &'a T
{
    &*(*obj.get_ivar::<usize>(varname) as *const _)
}
#[allow(dead_code)]
/// Extract boxed pointer ownership from ivar in objc object
unsafe fn take_ptr<T>(obj: &mut Object, varname: &str) -> Box<T>
{
    Box::from_raw((*obj.get_ivar::<usize>(varname)) as _)
}

struct AppDelegate<E: EventDelegate>
{
    ptr: objc_id, caller: Box<Weak<GUIApplication<E>>>
}
impl<E: EventDelegate> AppDelegate<E>
{
    fn new(caller: &Rc<GUIApplication<E>>, appname: &str) -> Option<Self>
    {
        let class = DeclareObjcClass!{ class AppDelegate : NSObject
            {
                ivar appinstance: usize;
                ivar appname: objc_id;
                - applicationDidFinishLaunching:(objc_id) = Self::did_finish_launching_cb;
                - applicationDidBecomeActive:(objc_id) = Self::become_active;
            }
        };
        let ptr: *mut Object = unsafe { msg_send![class, new] };
        if ptr.is_null() { return None; }
        let caller = Box::new(Rc::downgrade(caller));
        unsafe
        {
            store_boxed_ptr(&mut *ptr, "appinstance", &caller);
            (*ptr).set_ivar("appname", NSString::new(appname).unwrap().leave_id());
        }
        Some(AppDelegate { ptr, caller })
    }
    extern fn did_finish_launching_cb(this: &Object, _selector: Sel, _notify: objc_id)
    {
        let nsapp = NSApplication::shared().expect("retrieving shared NSApplication instance");
        let appname = unsafe { NSString::retain_id(*this.get_ivar("appname")) };
        Self::init_menu(&nsapp, appname.to_str());

        let app: &Weak<GUIApplication<E>> = unsafe { retrieve_ptr(this, "appinstance") };
        if let Some(app) = app.upgrade()
        {
            #[cfg(feature = "with_ferrite")] app.event_delegate().postinit(&app);
            #[cfg(not(feature = "with_ferrite"))] app.event_delegate().postinit();
        }
        nsapp.activate_ignoring_other_apps();
    }
    extern fn become_active(this: &Object, _sel: Sel, _notification: objc_id)
    {
        let app: &Weak<GUIApplication<E>> = unsafe { retrieve_ptr(this, "appinstance") };
        if let Some(app) = app.upgrade()
        {
            #[cfg(feature = "with_ferrite")] app.event_delegate().on_activated(&app);
            #[cfg(not(feature = "with_ferrite"))] app.event_delegate().on_activated();
        }
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

pub struct GUIApplication<E: EventDelegate>(Rc<E>);
impl<E: EventDelegate> GUIApplicationRunner<E> for GUIApplication<E>
{
    fn run(appname: &str, delegate: E) -> i32
    {
        let dg = Rc::new(delegate);
        let gapp = Rc::new(GUIApplication(dg));
        let appdelegate = AppDelegate::new(&gapp, appname).unwrap();
        let nsapp = NSApplication::shared().expect("initializing shared NSApplication");
        nsapp.set_delegate(appdelegate.ptr);
        nsapp.set_activation_policy(NSApplicationActivationPolicy::Regular);
        nsapp.run();
        0
    }
    fn event_delegate(&self) -> &Rc<E> { &self.0 }
}
#[cfg(feature = "with_ferrite")]
impl<E: EventDelegate> ::FerriteRenderingServer for GUIApplication<E>
{
    type SurfaceSource = objc_id;

    fn presentation_support(&self, _adapter: &fe::PhysicalDevice, _queue_family_index: u32) -> bool { true }
    fn create_surface(&self, w: &objc_id, instance: &fe::Instance) -> fe::Result<fe::Surface>
    {
        fe::Surface::new_macos(instance, (*w) as *const _)
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
        NSWindow::new(self.client_rect(), self.style).map(|w|
        {
            w.center(); w.set_title(self.caption); NativeWindow(w)
        })
    }
    #[cfg(feature = "with_ferrite")]
    fn create_renderable<E, S>(&self, server: &Rc<S>) -> Option<NativeWindow> where
        E: EventDelegate, S: ::FerriteRenderingServer + GUIApplicationRunner<E>
    {
        let vc = if let Some(v) = FeRenderableViewController::new(self.caption, &self.client_rect(), server) { v }
            else { return None; };
        unsafe { NSWindow::with_view_controller_ptr(vc.ptr).map(|w| { w.center(); NativeWindow(w) }) }
    }
}
impl<'c> NativeWindowBuilder<'c>
{
    fn client_rect(&self) -> NSRect
    {
        NSRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: self.width as _, height: self.height as _ } }
    }
}

#[cfg(feature = "with_ferrite")]
pub struct FeRenderableView(objc_id);
#[cfg(feature = "with_ferrite")]
impl FeRenderableView
{
    fn class() -> &'static Class
    {
        extern fn yesman(_this: &Object, _sel: Sel) -> BOOL { YES }
        extern fn make_backing_layer(this: &Object, _sel: Sel) -> objc_id
        {
            unsafe
            {
                let layer = CAMetalLayer::layer().expect("Creating CAMetalLayer");
                let view_scale: CGSize = msg_send![this, convertSizeToBacking: CGSize { width: 1.0, height: 1.0 }];
                layer.set_contents_scale(view_scale.width.min(view_scale.height));
                layer.leave_id()
            }
        }

        Class::get("FeRenderableView").unwrap_or_else(|| DeclareObjcClass!{
            class FeRenderableView : NSView
            {
                - (BOOL) wantsUpdateLayer = yesman;
                - (objc_id) makeBackingLayer = make_backing_layer;
            }
        })
    }
    fn new() -> Option<Self>
    {
        let obj: objc_id = unsafe { msg_send![Self::class(), new] };
        if obj.is_null() { None } else { Some(FeRenderableView(obj)) }
    }
    fn set_frame(&self, f: &NSRect) { unsafe { msg_send![self.0, setFrame: f.clone()] } }
    fn layer_ptr(&self) -> objc_id { unsafe { msg_send![self.0, layer] } }
}
#[cfg(feature = "with_ferrite")]
pub struct FeRenderableViewController<E: EventDelegate, S: ::FerriteRenderingServer + GUIApplicationRunner<E>>
{
    // ph: S(GUIApplication)がE(EventDelegate)を弱参照していることを表明する
    ptr: objc_id, ph: PhantomData<(Weak<S>, Weak<E>)>
}
#[cfg(feature = "with_ferrite")]
impl<E: EventDelegate, S: ::FerriteRenderingServer + GUIApplicationRunner<E>> FeRenderableViewController<E, S>
{
    fn class() -> &'static Class
    {
        extern fn load_view(this: &Object, _sel: Sel)
        {
            let fsize = unsafe { this.get_ivar::<NSRect>("initial_frame_size") };
            let view = FeRenderableView::new().expect("Failed to create Renderable View");
            view.set_frame(fsize);
            unsafe { msg_send![view.layer_ptr(), setFrame: fsize.clone()]; }
            unsafe { msg_send![this, setView: view.0]; }
        }
        Class::get("FeRenderableViewController").unwrap_or_else(|| DeclareObjcClass!{
            class FeRenderableViewController : NSViewController
            {
                ivar server_ptr: usize;
                ivar dp_link_instance: usize;
                ivar initial_frame_size: NSRect;
                - loadView = load_view;
                - viewDidLoad = Self::view_did_load;
                - viewDidAppear = Self::view_did_appear;
                - viewWillDisappear = Self::view_will_disappear;
                - mut dealloc = Self::dealloc;
            }
        })
    }

    fn new(title: &str, initial_frame_size: &NSRect, server: &Rc<S>) -> Option<Self>
    {
        let title = if let Some(v) = NSString::new(title) { v } else { return None; };

        let obj: objc_id = unsafe { msg_send![Self::class(), new] };
        if obj.is_null() { return None; }
        let server = Box::new(Rc::downgrade(server));
        let link = if let Some(v) = CVDisplayLink::with_active_display() { v } else { return None; };
        link.set_callback(Some(Self::on_update_sync), obj as *mut _);
        unsafe
        {
            move_boxed_ptr(&mut *obj, "server_ptr", server);
            move_boxed_ptr(&mut *obj, "dp_link_instance", Box::new(link));
            (*obj).set_ivar("initial_frame_size", initial_frame_size.clone());

            msg_send![obj, setTitle: title.raw()];
        }
        Some(FeRenderableViewController { ptr: obj, ph: PhantomData })
    }
    extern fn view_did_load(this: &Object, _sel: Sel)
    {
        let v: objc_id = unsafe { msg_send![this, view] };
        unsafe { msg_send![v, setWantsLayer: YES] };
        let srv_weak: &Weak<S> = unsafe { retrieve_ptr(this, "server_ptr") };
        if let Some(srv) = srv_weak.upgrade() { srv.event_delegate().on_init_view::<S>(&srv, unsafe { transmute(&v) }); }
    }
    extern fn view_did_appear(this: &Object, _sel: Sel)
    {
        unsafe { retrieve_ptr::<CVDisplayLink>(this, "dp_link_instance").start() };
    }
    extern fn view_will_disappear(this: &Object, _sel: Sel)
    {
        unsafe { retrieve_ptr::<CVDisplayLink>(this, "dp_link_instance").stop() };
    }
    extern "system" fn on_update_sync(_link: CVDisplayLinkRef, _now: *const CVTimeStamp, _outtime: *const CVTimeStamp,
        _flags: CVOptionFlags, _flags_out: *mut CVOptionFlags, context: *mut ::libc::c_void) -> CVReturn
    {
        // println!("DPLINK... {}", unsafe { (*_now).hostTime });
        let srv = unsafe { retrieve_ptr::<Weak<S>>(&*(context as *mut Object), "server_ptr") };
        if let Some(srv) = srv.upgrade() { srv.event_delegate().on_render_period(); }
        0
    }
    extern fn dealloc(this: &mut Object, _sel: Sel)
    {
        // println!("deallocing viewcontroller");
        drop(unsafe { take_ptr::<Weak<S>>(this, "server_ptr") });
        drop(unsafe { take_ptr::<CVDisplayLink>(this, "dp_link_instance") });
        unsafe { msg_send![super(this, Class::get("NSViewController").unwrap()), dealloc]; }
    }
}
