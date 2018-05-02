//! MacOS Runner

use appkit::*;
use objc::runtime::*;
use objc::declare::*;
use std::rc::*;
use {GUIApplicationRunner, WindowEventDelegate, EventDelegate, Window, WindowBuilder};
use std::marker::PhantomData;
use std::io::{Result as IOResult, Error as IOError, ErrorKind};
#[cfg_attr(not(feature = "with_bedrock"), allow(unused_imports))]
use std::ops::{Deref, DerefMut};
use std::mem::transmute;

#[cfg(feature = "with_bedrock")] use bedrock as br;

/*
#[link(name = "Foundation", kind = "framework")] extern "system"
{
    fn NSStringFromClass(class: &Class) -> *mut Object;
}
*/

/// Info.plistのCFBundleNameもしくはプロセス名
fn product_name() -> &'static NSString
{
    NSBundle::main().and_then(|b| b.object_for_info_dictionary_key("CFBundleName").ok_or(()))
        .unwrap_or_else(|_| NSProcessInfo::current().unwrap().name())
}

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
    (#Declaring($d: expr) $(#[$attr: meta])* - $($name: ident : ($aty: ty))+ = $fr: expr; $($rest: tt)*) =>
    {
        $(#[$attr])* unsafe { $d.add_method(sel!($($name :)+), $fr as extern fn(&Object, Sel $(, $aty)*)); }
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    // void noarg
    (#Declaring($d: expr) $(#[$attr: meta])* - $name: ident = $fr: expr; $($rest: tt)*) =>
    {
        $(#[$attr])* unsafe { $d.add_method(sel!($name), $fr as extern fn(&Object, Sel)); }
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    // void noarg mutable-this
    (#Declaring($d: expr) $(#[$attr: meta])* - mut $name: ident = $fr: expr; $($rest: tt)*) =>
    {
        $(#[$attr])* unsafe { $d.add_method(sel!($name), $fr as extern fn(&mut Object, Sel)); }
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
    (#Declaring($d: expr) $(#[$attr: meta])* ivar $name: ident: $vt: ty; $($rest: tt)*) =>
    {
        $(#[$attr])* { $d.add_ivar::<$vt>(stringify!($name)); }
        DeclareObjcClass!(#Declaring($d) $($rest)*);
    };
    (#Declaring($d: expr)) => {  }
}
/// Store boxed pointer into ivar in objc object
/*unsafe fn store_boxed_ptr<T>(obj: &mut Object, varname: &str, vbox: &Box<T>)
{
    obj.set_ivar(varname, &**vbox as *const _ as usize)
}*/
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

#[derive(ObjcObjectBase)]
struct AppDelegate<E: EventDelegate>(Object, PhantomData<Rc<GUIApplication<E>>>);
impl<E: EventDelegate> Deref for AppDelegate<E>
{
    type Target = NSObject; fn deref(&self) -> &NSObject { unsafe { transmute(self) } }
}
impl<E: EventDelegate + 'static> AppDelegate<E>
{
    fn new(caller: &Rc<GUIApplication<E>>) -> Result<CocoaObject<Self>, ()>
    {
        let class = DeclareObjcClass!{ class AppDelegate : NSObject
            {
                ivar appinstance: usize;
                - applicationDidFinishLaunching:(objc_id) = Self::did_finish_launching_cb;
                - applicationDidBecomeActive:(objc_id) = Self::become_active;
            }
        };
        let ptr: *mut Object = unsafe { msg_send![class, new] };
        if ptr.is_null() { return Err(()); }
        let caller = Box::new(caller.clone());
        unsafe { move_boxed_ptr(&mut *ptr, "appinstance", caller); }
        unsafe { CocoaObject::from_id(ptr) }
    }
    extern fn did_finish_launching_cb(this: &Object, _selector: Sel, _notify: objc_id)
    {
        let nsapp = NSApplication::shared().expect("retrieving shared NSApplication instance");
        Self::init_menu(&nsapp, product_name().to_str());

        let app: &Rc<GUIApplication<E>> = unsafe { retrieve_ptr(this, "appinstance") };
        app.0.postinit(&app);
        nsapp.activate_ignoring_other_apps();
    }
    extern fn become_active(this: &Object, _sel: Sel, _notification: objc_id)
    {
        let app: &Rc<GUIApplication<E>> = unsafe { retrieve_ptr(this, "appinstance") };
        app.0.on_activated(&app);
    }
    fn init_menu(nsapp: &NSApplication, appname: &str)
    {
        nsapp.set_main_menu(NSMenu::new().unwrap().add({
            NSMenuItem::new("", None, None).unwrap().set_submenu({
                let about_menu = NSMenuItem::new(&format!("About {}", appname), Some(sel!(orderFrontStandardAboutPanel:)), None).unwrap();
                let prefs = NSMenuItem::new("Preferences...", None, Some(&NSString::from_str(",").unwrap())).unwrap();
                let services = NSMenuItem::new("Services", None, None).unwrap();
                services.set_submenu(&NSMenu::new().unwrap());
                let hide = NSMenuItem::new(&format!("Hide {}", appname), Some(sel!(hide:)), Some(&NSString::from_str("h").unwrap())).unwrap();
                let hideother = NSMenuItem::new("Hide Others", Some(sel!(hideOtherApplications:)), None).unwrap();
                let showall = NSMenuItem::new("Show All", Some(sel!(unhideAllApplications:)), None).unwrap();
                let quit_menu = NSMenuItem::new(&format!("Quit {}", appname), Some(sel!(terminate:)), Some(&NSString::from_str("q").unwrap())).unwrap();

                NSMenu::new().unwrap()
                    .add(&about_menu).add(&NSMenuItem::separator().unwrap())
                    .add(&prefs).add(&NSMenuItem::separator().unwrap())
                    .add(&services).add(&NSMenuItem::separator().unwrap())
                    .add(&hide).add(hideother.set_accelerator(NSEventModifierFlags::COMMAND | NSEventModifierFlags::OPTION, "h"))
                    .add(&showall).add(&NSMenuItem::separator().unwrap())
                    .add(&quit_menu)
            })
        }))
    }
}

pub struct GUIApplication<E: EventDelegate>(E);
impl<E: EventDelegate + 'static> GUIApplicationRunner<E> for GUIApplication<E>
{
    fn run(delegate: E) -> i32
    {
        let app = Rc::new(GUIApplication(delegate));
        let appdelegate = AppDelegate::new(&app).unwrap();
        let nsapp = NSApplication::shared().expect("initializing shared NSApplication");
        nsapp.set_delegate(appdelegate.objid());
        nsapp.set_activation_policy(NSApplicationActivationPolicy::Regular);
        nsapp.run();
        0
    }
    fn event_delegate(&self) -> &E { &self.0 }
}
#[cfg(feature = "with_bedrock")]
impl<E: EventDelegate> ::BedrockRenderingServer for GUIApplication<E>
{
    fn presentation_support(&self, _adapter: &br::PhysicalDevice, _queue_family_index: u32) -> bool { true }
    fn create_surface<WE: WindowEventDelegate>(&self, w: &FeRenderableView<WE>, instance: &br::Instance)
        -> br::Result<br::Surface>
    {
        br::Surface::new_macos(instance, w as *const _ as _)
    }
}

pub struct NativeWindow<WE: WindowEventDelegate>(CocoaObject<NSWindow>, CocoaObject<NativeWindowController<WE>>);
impl<WE: WindowEventDelegate> Window for NativeWindow<WE>
{
    fn show(&self) { self.0.make_key_and_order_front(NSApplication::shared().unwrap().objid()); }
    #[cfg(feature = "with_bedrock")]
    fn mark_dirty(&self) {
        let _: () = unsafe { msg_send![transmute::<_, &Object>(self.1.view()), setNeedsDisplay: YES] };
    }
}

pub struct NativeWindowBuilder<'c>
{
    style: NSWindowStyleMask, width: u16, height: u16, caption: &'c str, transparency: bool
}
impl<'c> WindowBuilder<'c> for NativeWindowBuilder<'c>
{
    fn new(width: u16, height: u16, caption: &'c str) -> Self
    {
        NativeWindowBuilder
        {
            style: NSWindowStyleMask::TITLED | NSWindowStyleMask::CLOSABLE | NSWindowStyleMask::MINIATURIZABLE | NSWindowStyleMask::RESIZABLE,
            width, height, caption, transparency: false
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
    fn transparent(&mut self, c: bool) -> &mut Self { self.transparency = c; self }

    fn create<WE: WindowEventDelegate>(&self, _server: &Rc<GUIApplication<WE::ClientDelegate>>, event: &Rc<WE>)
        -> IOResult<NativeWindow<WE>> {
        let wc = NativeWindowController::new(self.caption, &self.client_rect(), event)?;
        unsafe {
            NSWindow::with_view_controller_ptr(wc.id()).map(|w| {
                if self.transparency {
                    w.set_background_color(NSColor::clear_color().unwrap());
                    w.set_opaque(false);
                    wc.view().layer().expect("No layer").set_opaque(false);
                }
                w.center(); return NativeWindow(w, wc);
            }).map_err(|_| IOError::new(ErrorKind::Other, "System I/O Error on creating NSWindow"))
        }
    }
    #[cfg(feature = "with_bedrock")]
    fn create_renderable<WE: WindowEventDelegate>(&self, server: &Rc<GUIApplication<WE::ClientDelegate>>, event: &Rc<WE>)
        -> IOResult<NativeWindow<WE>> where WE::ClientDelegate: 'static
    {
        self.create(server, event)
    }
}
impl<'c> NativeWindowBuilder<'c>
{
    fn client_rect(&self) -> NSRect
    {
        NSRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: self.width as _, height: self.height as _ } }
    }
}

pub struct FeRenderableView<WE: WindowEventDelegate>(Object, PhantomData<(Weak<GUIApplication<WE::ClientDelegate>>, Weak<WE>)>);
impl<WE: WindowEventDelegate> Deref for FeRenderableView<WE> {
    type Target = NSView;
    fn deref(&self) -> &NSView { unsafe { transmute(self) } }
}
impl<WE: WindowEventDelegate> DerefMut for FeRenderableView<WE> {
    fn deref_mut(&mut self) -> &mut NSView { unsafe { transmute(self) } }
}
impl<WE: WindowEventDelegate> ObjcObjectBase for FeRenderableView<WE> {
    fn objid(&self) -> &Object { &self.0 }
    fn objid_mut(&mut self) -> &mut Object { &mut self.0 }
}
impl<WE: WindowEventDelegate> FeRenderableView<WE> {
    fn class() -> &'static Class {
        extern fn yesman(_this: &Object, _sel: Sel) -> BOOL { YES }
        extern fn make_backing_layer(this: &Object, _sel: Sel) -> objc_id {
            let this: &NSView = unsafe { transmute(this) };
            let layer = if let Ok(l) = CAMetalLayer::layer() { l }
                else { println!("Failed to create CAMetalLayer"); panic!("Creating CAMetalLayer"); };
            let view_scale = this.convert_size_to_backing(&NSSize { width: 1.0, height: 1.0 });
            layer.set_contents_scale(view_scale.width.min(view_scale.height));
            // #[cfg(feature = "manual_rendering")] layer.set_needs_display_on_bounds_change(true);
            layer.into_id()
        }

        Class::get("FeRenderableView").unwrap_or_else(|| DeclareObjcClass! {
            class FeRenderableView : NSView {
                - (BOOL) wantsUpdateLayer = yesman;
                - (objc_id) makeBackingLayer = make_backing_layer;
                #[cfg(feature = "manual_rendering")]
                ivar event_delegate: usize;
                #[cfg(feature = "manual_rendering")]
                - mut dealloc = Self::dealloc;
                #[cfg(feature = "manual_rendering")]
                - displayLayer:(objc_id) = Self::display_layer;
                - setFrameSize:(NSSize) = Self::set_frame_size;
                - viewDidEndLiveResize = Self::did_end_live_resize;
            }
        })
    }
    fn new(d: Weak<WE>) -> Result<CocoaObject<Self>, ()> {
        let obj: objc_id = unsafe { msg_send![Self::class(), new] };
        if obj.is_null() { return Err(()); } else
        {
            unsafe {
                move_boxed_ptr(&mut *obj, "event_delegate", Box::new(d));
                return Ok(CocoaObject::from_id_unchecked(obj));
            }
        }
    }

    #[cfg(feature = "manual_rendering")]
    extern fn display_layer(this: &Object, _sel: Sel, _layer: objc_id)
    {
        // println!("DisplayLayer");
        let d: &Weak<WE> = unsafe { retrieve_ptr(this, "event_delegate") };
        d.upgrade().unwrap().render();
    }
    #[cfg(feature = "manual_rendering")]
    extern fn dealloc(this: &mut Object, _sel: Sel)
    {
        unsafe
        {
            drop(take_ptr::<Weak<WE>>(this, "event_delegate"));
            msg_send![super(this, Class::get("NSView").unwrap()), dealloc]
        }
    }
    extern fn set_frame_size(this: &Object, _sel: Sel, size: NSSize)
    {
        let _: () = unsafe { msg_send![super(this, Class::get("NSView").unwrap()), setFrameSize: size.clone()] };
        let layer: *mut Object = unsafe { msg_send![this, layer] };
        if !layer.is_null()
        {
            let rect = CGRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: size.clone() };
            unsafe {
                let _: () = msg_send![layer, setFrame: rect.clone()];
                let _: () = msg_send![layer, setBounds: rect];

                let is_in_live_resize: BOOL = msg_send![this, inLiveResize];

                retrieve_ptr::<Weak<WE>>(this, "event_delegate").upgrade().unwrap()
                    .resize(size.width as _, size.height as _, is_in_live_resize == YES);
            }
        }
    }
    extern fn did_end_live_resize(this: &Object, _sel: Sel) {
        unsafe {
            let _: () = msg_send![super(this, Class::get("NSView").unwrap()), viewDidEndLiveResize];
            let frame: NSRect = msg_send![this, frame];
            retrieve_ptr::<Weak<WE>>(this, "event_delegate").upgrade().unwrap()
                .resize(frame.size.width as _, frame.size.height as _, false);
        }
    }
}
#[cfg(feature = "with_bedrock")] pub type NativeView<E> = FeRenderableView<E>;
#[cfg(not(feature = "with_bedrock"))] pub type NativeView<E> = (NSView, PhantomData<E>);
#[cfg(feature = "manual_rendering")]
pub struct FeRenderableViewCtrlIvarShadowings<WE: WindowEventDelegate>
{
    _ev: Weak<WE>, _server: Weak<GUIApplication<WE::ClientDelegate>>,
    #[cfg(not(feature = "manual_rendering"))]
    _displaylink: CVDisplayLink
}
#[derive(ObjcObjectBase)]
pub struct NativeWindowController<WE: WindowEventDelegate>(Object, PhantomData<FeRenderableViewCtrlIvarShadowings<WE>>);
impl<WE: WindowEventDelegate> Deref for NativeWindowController<WE>
{
    type Target = NSViewController;
    fn deref(&self) -> &NSViewController { unsafe { transmute(self) } }
}
impl<WE: WindowEventDelegate> DerefMut for NativeWindowController<WE>
{
    fn deref_mut(&mut self) -> &mut NSViewController { unsafe { transmute(self) } }
}
impl<WE: WindowEventDelegate> NativeWindowController<WE>
{
    fn class() -> &'static Class
    {
        Class::get("NativeWindowController").unwrap_or_else(|| DeclareObjcClass!{
            class NativeWindowController : NSViewController
            {
                ivar event_delegate_ptr: usize;
                ivar initial_frame_size: NSRect;
                - mut loadView = Self::load_view;
                - mut viewDidLoad = Self::view_did_load;
                #[cfg(not(feature = "manual_rendering"))]
                ivar dp_link_instance: usize;
                #[cfg(not(feature = "manual_rendering"))]
                - viewDidAppear = Self::view_did_appear;
                #[cfg(not(feature = "manual_rendering"))]
                - viewWillDisappear = Self::view_will_disappear;

                - mut dealloc = Self::dealloc;
            }
        })
    }

    fn new(title: &str, initial_frame_size: &NSRect, ev: &Rc<WE>) -> IOResult<CocoaObject<Self>>
    {
        let title = NSString::from_str(title)
            .map_err(|_| IOError::new(ErrorKind::Other, "System I/O Error on creating NSString"))?;

        let obj: objc_id = unsafe { msg_send![Self::class(), new] };
        if obj.is_null()
        {
            return Err(IOError::new(ErrorKind::Other, "Failed to Alloc/Init of NativeWindowController"));
        }
        unsafe
        {
            move_boxed_ptr(&mut *obj, "event_delegate_ptr", Box::new(Rc::downgrade(ev)));
            (*obj).set_ivar("initial_frame_size", initial_frame_size.clone());
        }
        #[cfg(not(feature = "manual_rendering"))] unsafe
        {
            let displaylink = Box::new(CVDisplayLink::with_active_display()
                .ok_or_else(|| IOError::new(ErrorKind::Other, "System I/O Error on creating CVDisplayLink"))?);
            displaylink.set_callback(Some(Self::on_update_sync), obj as *mut _);
            move_boxed_ptr(&mut *obj, "dp_link_instance", displaylink);
        }
        let obj = unsafe
        {
            CocoaObject::<Self>::from_id(obj)
                .map_err(|_| IOError::new(ErrorKind::Other, "System I/O Error on creating NativeWindowController"))?
        };
        obj.set_title(&title); Ok(obj)
    }
    fn view(&self) -> &FeRenderableView<WE> { unsafe { transmute(self.deref().view().unwrap()) } }
    fn view_mut(&mut self) -> &mut FeRenderableView<WE> { unsafe { transmute(self.deref_mut().view_mut().unwrap()) } }
    extern fn load_view(this: &mut Object, _sel: Sel)
    {
        let fsize = unsafe { this.get_ivar::<NSRect>("initial_frame_size") };
        let ev: &Weak<WE> = unsafe { retrieve_ptr(this, "event_delegate_ptr") };
        let mut view = FeRenderableView::new(ev.clone()).expect("Failed to create Renderable View");

        view.set_frame(fsize);
        /*if let Some(l) = view.layer_mut() { l.set_frame(fsize.clone()); }
            else { println!("Null Layer"); }*/
        let _: () = unsafe { msg_send![this, setView: view.id()] };
    }
    extern fn view_did_load(this: &mut Object, _sel: Sel)
    {
        let ev: &Weak<WE> = unsafe { retrieve_ptr(this, "event_delegate_ptr") };
        let this: &mut Self = unsafe { transmute(this) };
        let v = this.view_mut();
        v.set_wants_layer(true);
        v.set_layer_contents_redraw_policy(2  /* NSViewLayerContentsRedrawDuringViewResize */);
        ev.upgrade().unwrap().init_view(&v);
    }
    #[cfg(not(feature = "manual_rendering"))]
    extern fn view_did_appear(this: &Object, _sel: Sel)
    {
        if let &Some(ref l) = unsafe { retrieve_ptr::<Option<CVDisplayLink>>(this, "dp_link_instance") } { l.start(); }
    }
    #[cfg(not(feature = "manual_rendering"))]
    extern fn view_will_disappear(this: &Object, _sel: Sel)
    {
        if let &Some(ref l) = unsafe { retrieve_ptr::<Option<CVDisplayLink>>(this, "dp_link_instance") } { l.stop(); }
    }
    #[cfg(not(feature = "manual_rendering"))]
    extern "system" fn on_update_sync(_link: CVDisplayLinkRef, _now: *const CVTimeStamp, _outtime: *const CVTimeStamp,
        _flags: CVOptionFlags, _flags_out: *mut CVOptionFlags, context: *mut ::libc::c_void) -> CVReturn
    {
        // println!("DPLINK... {}", unsafe { (*_now).hostTime });
        let e: &Weak<WE> = unsafe { retrieve_ptr(&*(context as *mut Object), "event_delegate_ptr") };
        e.render();
        return 0;
    }
    extern fn dealloc(this: &mut Object, _sel: Sel)
    {
        #[cfg(not(feature = "manual_rendering"))]
        unsafe { drop(take_ptr::<CVDisplayLink>("dp_link_instance")); }
        unsafe
        {
            drop(take_ptr::<Weak<WE>>(this, "event_delegate_ptr"));

            msg_send![super(this, Class::get("NSViewController").unwrap()), dealloc]
        }
    }
}
