//! AppKit bindings

use libc::*;
use objc::runtime::*;
use std::mem::zeroed;
use std::borrow::*;
use std::mem::forget;
use objc::{Encode, Encoding};

/*#[cfg(feature = "with_ferrite")]
type NSRunLoopMode = *mut Object;*/
#[cfg(feature = "with_ferrite")]
pub enum __CVDisplayLink {}
#[cfg(feature = "with_ferrite")] #[cfg(not(feature = "manual_rendering"))]
pub type CVDisplayLinkRef = *mut __CVDisplayLink;
#[cfg(feature = "with_ferrite")] #[cfg(not(feature = "manual_rendering"))]
pub type CVReturn = i32;
#[cfg(feature = "with_ferrite")] #[cfg(not(feature = "manual_rendering"))]
pub type CVDisplayLinkOutputCallback = Option<extern "system" fn(
    link: CVDisplayLinkRef, in_now: *const CVTimeStamp, in_output_time: *const CVTimeStamp,
    in_flags: CVOptionFlags, out_flags: *mut CVOptionFlags, context: *mut c_void) -> CVReturn>;
#[cfg(feature = "with_ferrite")] #[cfg(not(feature = "manual_rendering"))]
#[repr(C)] #[allow(non_snake_case)]
pub struct CVTimeStamp
{
    pub version: u32, pub videoTimeScale: i32, pub videoTime: i64,
    pub hostTime: u64, pub rateScalar: c_double, pub videoRefreshPeriod: i64,
    pub smpteTime: CVSMPTETime, pub flags: u64, pub reserved: u64
}
#[cfg(feature = "with_ferrite")] #[cfg(not(feature = "manual_rendering"))]
#[repr(C)] #[allow(non_snake_case)]
pub struct CVSMPTETime
{
    pub subframes: i16, pub subframeDivisor: i16, pub counter: u32,
    pub type_: u32, pub flags: u32, pub hours: i16, pub minutes: i16, pub seconds: i16, pub frames: i16
}
#[cfg(feature = "with_ferrite")] #[cfg(not(feature = "manual_rendering"))] pub type CVOptionFlags = u64;
#[link(name = "AppKit", kind = "framework")] extern {}
#[cfg(all(feature = "with_ferrite", not(feature = "manual_rendering")))]
#[link(name = "QuartzCore", kind = "framework")] extern "system"
{
    fn CVDisplayLinkCreateWithActiveCGDisplays(displayLinkOut: *mut CVDisplayLinkRef) -> CVReturn;
    fn CVDisplayLinkSetOutputCallback(link: CVDisplayLinkRef, callback: CVDisplayLinkOutputCallback, userinfo: *mut c_void)
        -> CVReturn;
    fn CVDisplayLinkStart(link: CVDisplayLinkRef) -> CVReturn;
    fn CVDisplayLinkStop(link: CVDisplayLinkRef) -> CVReturn;
    fn CVDisplayLinkRelease(link: CVDisplayLinkRef);
}
/*#[cfg(feature = "with_ferrite")]
#[link(name = "Foundation", kind = "framework")] extern "system"
{
    pub static NSDefaultRunLoopMode: NSRunLoopMode;
}*/

#[cfg(target_pointer_width = "64")] pub type CGFloat = f64;
#[cfg(target_pointer_width = "64")] pub type NSInteger = i64;
#[cfg(target_pointer_width = "64")] pub type NSUInteger = u64;
#[cfg(not(target_pointer_width = "64"))] pub type CGFloat = f32;
#[cfg(not(target_pointer_width = "64"))] pub type NSInteger = i32;
#[cfg(not(target_pointer_width = "64"))] pub type NSUInteger = u32;
#[repr(C)] #[derive(Debug, Clone, PartialEq)] pub struct CGPoint { pub x: CGFloat, pub y: CGFloat }
#[repr(C)] #[derive(Debug, Clone, PartialEq)] pub struct CGSize  { pub width: CGFloat, pub height: CGFloat }
#[repr(C)] #[derive(Debug, Clone, PartialEq)] pub struct CGRect  { pub origin: CGPoint, pub size: CGSize }
pub type NSSize = CGSize;
pub type NSRect = CGRect;

unsafe impl Encode for CGPoint
{
    fn encode() -> Encoding
    {
        unsafe
        {
            Encoding::from_str(&format!("{{CGPoint={}{}}}", CGFloat::encode().as_str(), CGFloat::encode().as_str()))
        }
    }
}
unsafe impl Encode for CGSize
{
    fn encode() -> Encoding
    {
        unsafe
        {
            Encoding::from_str(&format!("{{CGSize={}{}}}", CGFloat::encode().as_str(), CGFloat::encode().as_str()))
        }
    }
}
unsafe impl Encode for CGRect
{
    fn encode() -> Encoding
    {
        unsafe
        {
            Encoding::from_str(&format!("{{CGRect={}{}}}", CGPoint::encode().as_str(), CGSize::encode().as_str()))
        }
    }
}

#[repr(C)] #[allow(dead_code)]
pub enum NSApplicationActivationPolicy { Regular, Accessory, Prohibited }
bitflags! {
    pub struct NSWindowStyleMask: NSUInteger
    {
        const BORDERLESS = 0;
        const TITLED = 1 << 0;
        const CLOSABLE = 1 << 1;
        const MINIATURIZABLE = 1 << 2;
        const RESIZABLE = 1 << 3;

        const TEXTURED_BACKGROUND = 1 << 8;
        const UNIFIED_TITLE_AND_TOOLBAR = 1 << 12;
        // >= OS X 10.7
        const FULLSCREEN = 1 << 14;
        // >= OS X 10.10
        const FULLSIZE_CONTENT_VIEW = 1 << 15;

        const UTILITY_WINDOW = 1 << 4;
        const DOC_MODAL_WINDOW = 1 << 6;
        const NONACTIVATING_PANEL = 1 << 7;
        // >= OS X 10.6
        const HUD_WINDOW = 1 << 13;
    }
}
bitflags! {
    pub struct NSEventModifierFlags : NSUInteger
    {
        const COMMAND = 1 << 20;
        const OPTION = 1 << 19;
        const SHIFT = 1 << 17;
    }
}

use std::ops::Deref;
use std::ptr::null_mut;
use std::mem::transmute;
pub trait NSRefCounted { fn as_object(&self) -> &NSObject; }
pub struct NSObject(Object);
impl NSObject
{
    pub fn retain(&self) -> *mut Self { unsafe { msg_send![&self.0, retain] } }
    pub fn release(&self) { unsafe { msg_send![&self.0, release] } }
}
impl NSRefCounted for NSObject { fn as_object(&self) -> &NSObject { self } }
impl<T: Deref<Target = NSObject>> NSRefCounted for T { fn as_object(&self) -> &NSObject { self.deref() } }
impl AsRef<Object> for NSObject { fn as_ref(&self) -> &Object { &self.0 } }
pub struct AutoreleaseBox<T: NSRefCounted>(*mut T);
impl<T: NSRefCounted> Drop for AutoreleaseBox<T>
{
    fn drop(&mut self)
    {
        if let Some(p) = unsafe { self.0.as_ref() } { p.as_object().release(); self.0 = null_mut(); }
    }
}
impl<T: NSRefCounted + 'static> AutoreleaseBox<T>
{
    // pub unsafe fn from_raw(p: *mut T) -> Self { AutoreleaseBox(p) }
    pub unsafe fn from_id(p: *mut Object) -> Self { AutoreleaseBox(p as *mut _) }
    pub unsafe fn retain_id(p: *mut Object) -> Self { Self::from_id(msg_send![p, retain]) }

    pub fn id(&self) -> *mut Object { self.0 as _ }
    pub fn into_id(self) -> *mut Object { let p = self.0; forget(self); p as *mut _ }
}
impl<T: NSRefCounted + 'static> Clone for AutoreleaseBox<T>
{
    fn clone(&self) -> Self { AutoreleaseBox(self.as_object().retain() as *mut _) }
}
impl<T: NSRefCounted + 'static> Deref for AutoreleaseBox<T>
{
    type Target = T; fn deref(&self) -> &T { unsafe { &*self.0 } }
}
impl<T: NSRefCounted + 'static> Borrow<T> for AutoreleaseBox<T>
{
    fn borrow(&self) -> &T { self.deref() }
}
macro_rules! DeclareClassDerivative
{
    ($t: ty : $o: ty) =>
    {
        impl Deref for $t { type Target = $o; fn deref(&self) -> &$o { unsafe { transmute(self) } } }
    }
}

pub struct NSApplication(Object); DeclareClassDerivative!(NSApplication : NSObject);
impl NSApplication
{
    pub fn shared() -> Option<&'static Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSApplication").unwrap(), sharedApplication] };
        unsafe { (p as *const Self).as_ref() }
    }

    pub fn set_activation_policy(&self, policy: NSApplicationActivationPolicy) -> bool
    {
        let b: BOOL = unsafe { msg_send![&self.0, setActivationPolicy: policy as NSInteger] };
        b == YES
    }
    pub fn run(&self) { unsafe { msg_send![&self.0, run] } }
    pub fn activate_ignoring_other_apps(&self)
    {
        unsafe { msg_send![&self.0, activateIgnoringOtherApps: YES] }
    }
    pub fn set_delegate(&self, delegate: &Object)
    {
        unsafe { msg_send![&self.0, setDelegate: delegate] }
    }
    pub fn set_main_menu(&self, menu: &NSMenu)
    {
        unsafe { msg_send![&self.0, setMainMenu: menu] }
    }
}
pub struct NSWindow(Object); DeclareClassDerivative!(NSWindow : NSObject);
impl NSWindow
{
    pub fn new(content_rect: NSRect, style_mask: NSWindowStyleMask) -> Option<AutoreleaseBox<Self>>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSWindow").unwrap(), alloc] };
        if p.is_null() { return None; }
        let p: *mut Object = unsafe
        {
            msg_send![p, initWithContentRect: content_rect styleMask: style_mask backing: 2 defer: YES]
        };
        if p.is_null() { None } else { Some(unsafe { AutoreleaseBox::from_id(p) }) }
    }
    #[cfg(feature = "with_ferrite")]
    pub unsafe fn with_view_controller_ptr(vc: *mut Object) -> Option<AutoreleaseBox<Self>>
    {
        let p: *mut Object = msg_send![Class::get("NSWindow").unwrap(), windowWithContentViewController: vc];
        if p.is_null() { None } else { Some(AutoreleaseBox::from_id(p)) }
    }

    pub fn center(&self) { unsafe { msg_send![&self.0, center] } }
    pub fn make_key_and_order_front(&self, sender: &Object)
    {
        unsafe { msg_send![&self.0, makeKeyAndOrderFront: sender] }
    }
    pub fn set_title<Title: CocoaString + ?Sized>(&self, title: &Title)
    {
        unsafe { msg_send![&self.0, setTitle: title.to_nsstring().id()] }
    }
    pub fn set_alpha_value(&self, a: CGFloat) { unsafe { msg_send![&self.0, setAlphaValue: a] } }
    pub fn set_background_color(&self, bg: &NSColor)
    {
        unsafe { msg_send![&self.0, setBackgroundColor: bg as *const _] }
    }
    pub fn set_opaque(&self, op: bool)
    {
        unsafe { msg_send![&self.0, setOpaque: if op { YES } else { NO }] }
    }
}
pub struct NSMenu(Object); DeclareClassDerivative!(NSMenu : NSObject);
impl NSMenu
{
    pub fn new() -> Option<AutoreleaseBox<Self>>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSMenu").unwrap(), new] };
        if p.is_null() { None } else { Some(unsafe { AutoreleaseBox::from_id(p) }) }
    }
    pub fn add(&self, item: &NSMenuItem) -> &Self
    {
        unsafe { msg_send![&self.0, addItem: item as *const _] }; self
    }
}
pub struct NSMenuItem(Object); DeclareClassDerivative!(NSMenuItem : NSObject);
impl NSMenuItem
{
    pub fn new<Title: CocoaString + ?Sized>(title: &Title, action: Option<Sel>, key_equivalent: Option<&NSString>)
        -> Option<AutoreleaseBox<Self>>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSMenuItem").unwrap(), alloc] };
        if p.is_null() { return None; }
        let (title, action) = (title.to_nsstring(), action.unwrap_or(unsafe { zeroed() }));
        let k = key_equivalent.unwrap_or_else(|| NSString::empty().unwrap());
        let p: *mut Object = unsafe
        {
            msg_send![p, initWithTitle: title.id() action: action keyEquivalent: k as *const _]
        };
        if p.is_null() { None } else { Some(unsafe { AutoreleaseBox::from_id(p) }) }
    }
    pub fn separator() -> Option<AutoreleaseBox<Self>>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSMenuItem").unwrap(), separatorItem] };
        if p.is_null() { None } else { Some(unsafe { AutoreleaseBox::retain_id(p) }) }
    }

    pub fn set_submenu(&self, sub: &NSMenu) -> &Self
    {
        unsafe { msg_send![&self.0, setSubmenu: sub as *const _] }; self
    }
    /*pub fn set_target(&self, target: *mut Object) -> &Self
    {
        unsafe { msg_send![self.0, setTarget: target] }; self
    }*/
    pub fn set_key_equivalent_modifier_mask(&self, mods: NSEventModifierFlags) -> &Self
    {
        unsafe { msg_send![&self.0, setKeyEquivalentModifierMask: mods.bits] }; self
    }
    pub fn set_key_equivalent<Str: CocoaString + ?Sized>(&self, k: &Str) -> &Self
    {
        unsafe { msg_send![&self.0, setKeyEquivalent: k.to_nsstring().id()] }; self
    }
    pub fn set_accelerator<Str: CocoaString + ?Sized>(&self, mods: NSEventModifierFlags, key: &Str) -> &Self
    {
        self.set_key_equivalent(key).set_key_equivalent_modifier_mask(mods)
    }
    // pub fn set_action(&self, sel: Sel) -> &Self { unsafe { msg_send![self.0, setAction: sel] }; self }
}

pub struct NSView(Object); DeclareClassDerivative!(NSView : NSObject);
impl NSView
{
    // pub fn set_wants_layer(&self, flag: bool) { unsafe { msg_send![self.0, setWantsLayer: flag as BOOL] } }
    // pub fn set_layer(&self, layer: *mut Object) { unsafe { msg_send![self.0, setLayer: layer] } }
    pub fn layer_ptr(&self) -> *mut Object { unsafe { msg_send![&self.0, layer] } }
    pub fn layer(&self) -> Option<&'static CALayer>
    {
        let p: *mut Object = unsafe { msg_send![&self.0, layer] };
        unsafe { (p as *const CALayer).as_ref() }
    }
    pub fn set_wants_layer(&self, flag: bool) { unsafe { msg_send![&self.0, setWantsLayer: flag as BOOL] } }
    pub fn set_layer_contents_redraw_policy(&self, value: i32)
    {
        unsafe { msg_send![&self.0, setLayerContentsRedrawPolicy: value] }
    }
    pub fn set_needs_display(&self, flag: bool) { unsafe { msg_send![&self.0, setNeedsDisplay: flag as BOOL] } }
    pub fn set_frame(&self, f: &NSRect) { unsafe { msg_send![&self.0, setFrame: f.clone()] } }
    pub fn convert_size_to_backing(&self, size: &NSSize) -> NSSize
    {
        unsafe { msg_send![&self.0, convertSizeToBacking:size.clone()] }
    }
    pub fn set_opaque(&self, c: bool) { unsafe { msg_send![&self.0, setOpaque: if c { YES } else { NO }] } }
}
pub struct NSViewController(Object); DeclareClassDerivative!(NSViewController : NSObject);
impl NSViewController
{
    pub fn view(&self) -> Option<&NSView>
    {
        unsafe { let p: *mut Object = msg_send![&self.0, view]; (p as *const NSView).as_ref() }
    }
    /*pub fn title(&self) -> Option<&NSString>
    {
        unsafe { let p: *mut Object = msg_send![&self.0, title]; (p as *const NSString).as_ref() }
    }*/
    pub fn set_title<S: CocoaString + ?Sized>(&self, title: &S)
    {
        unsafe { msg_send![&self.0, setTitle: title.to_nsstring().id()] }
    }
}

pub struct CALayer(Object); DeclareClassDerivative!(CALayer : NSObject);
impl CALayer
{
    pub fn set_contents_scale(&self, scale: CGFloat) { unsafe { msg_send![&self.0, setContentsScale: scale]; } }
    pub fn set_needs_display_on_bounds_change(&self, v: bool)
    {
        unsafe { msg_send![&self.0, setNeedsDisplayOnBoundsChange: if v { YES } else { NO }] }
    }
    pub fn set_opaque(&self, c: bool) { unsafe { msg_send![&self.0, setOpaque: if c { YES } else { NO }] } }
}
#[cfg(feature = "with_ferrite")]
pub struct CAMetalLayer(Object);
#[cfg(feature = "with_ferrite")] DeclareClassDerivative!(CAMetalLayer : CALayer);
#[cfg(feature = "with_ferrite")]
impl NSRefCounted for CAMetalLayer { fn as_object(&self) -> &NSObject { unsafe { transmute(self) } } }
#[cfg(feature = "with_ferrite")]
impl CAMetalLayer
{
    pub fn layer() -> Option<AutoreleaseBox<Self>>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("CAMetalLayer").unwrap(), layer] };
        if p.is_null() { None } else { Some(unsafe { AutoreleaseBox::from_id(p) }) }
    }
}

#[cfg(all(feature = "with_ferrite", not(feature = "manual_rendering")))]
pub struct CVDisplayLink(CVDisplayLinkRef);
#[cfg(all(feature = "with_ferrite", not(feature = "manual_rendering")))]
impl CVDisplayLink
{
    pub fn with_active_display() -> Option<Self>
    {
        let mut p = unsafe { ::std::mem::uninitialized() };
        if unsafe { CVDisplayLinkCreateWithActiveCGDisplays(&mut p) } != 0 { return None; }
        Some(CVDisplayLink(p))
    }
    pub fn set_callback(&self, callback: CVDisplayLinkOutputCallback, ptr: *mut c_void)
    {
        unsafe { CVDisplayLinkSetOutputCallback(self.0, callback, ptr); }
    }
    pub fn start(&self) { unsafe { CVDisplayLinkStart(self.0); } }
    pub fn stop(&self) { unsafe { CVDisplayLinkStop(self.0); } }
}
#[cfg(all(feature = "with_ferrite", not(feature = "manual_rendering")))]
impl Drop for CVDisplayLink { fn drop(&mut self) { unsafe { CVDisplayLinkRelease(self.0) } } }
/*pub struct NSRunLoop(*mut Object);
impl NSRunLoop
{
    pub fn main() -> Option<NSRunLoop>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSRunLoop").expect("NSRunLoop"), mainRunLoop] };
        let p: *mut Object = unsafe { msg_send![p, retain] };
        if p.is_null() { None } else { Some(NSRunLoop(p)) }
    }
}
impl Drop for NSRunLoop { fn drop(&mut self) { unsafe { msg_send![self.0, release] } } }*/

pub struct NSBundle(Object);
impl NSBundle
{
    pub fn main() -> Option<&'static Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSBundle").unwrap(), mainBundle] };
        unsafe { (p as *const NSBundle).as_ref() }
    }
    pub fn object_for_info_dictionary_key<K: CocoaString + ?Sized, V>(&self, key: &K) -> Option<&V>
    {
        let k = key.to_nsstring();
        unsafe
        {
            let p: *mut Object = msg_send![&self.0, objectForInfoDictionaryKey: k.id()];
            (p as *const V).as_ref()
        }
    }
}
pub struct NSProcessInfo(Object);
impl NSProcessInfo
{
    pub fn current() -> Option<&'static Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSProcessInfo").unwrap(), processInfo] };
        unsafe { (p as *const NSProcessInfo).as_ref() }
    }
    pub fn name(&self) -> &NSString
    {
        unsafe { let p: *mut Object = msg_send![&self.0, processName]; &*(p as *const NSString) }
    }
}

pub struct NSString(Object); DeclareClassDerivative!(NSString : NSObject);
#[allow(dead_code)]
impl NSString
{
    pub fn new(s: &str) -> Option<AutoreleaseBox<Self>>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSString").unwrap(), alloc] };
        if p.is_null() { return None; }
        let bytes = s.as_bytes();
        let p: *mut Object = unsafe
        {
            msg_send![p, initWithBytes: bytes.as_ptr() length: bytes.len() as NSUInteger encoding: 4 as NSUInteger]
        };
        if p.is_null() { None } else { Some(unsafe { AutoreleaseBox::from_id(p) }) }
    }
    pub fn empty() -> Option<&'static Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSString").unwrap(), string] };
        unsafe { (p as *const Self).as_ref() }
    }
    pub fn to_str(&self) -> &str
    {
        let ps: *const c_char = unsafe { msg_send![&self.0, UTF8String] };
        unsafe { ::std::ffi::CStr::from_ptr(ps).to_str().unwrap() }
    }
}
/// Ref to NSString or Ref to str slice
pub trait CocoaString
{
    fn to_nsstring(&self) -> Cow<AutoreleaseBox<NSString>>;
}
impl CocoaString for AutoreleaseBox<NSString>
{
    fn to_nsstring(&self) -> Cow<AutoreleaseBox<NSString>> { Cow::Borrowed(self) }
}
impl CocoaString for str
{
    fn to_nsstring(&self) -> Cow<AutoreleaseBox<NSString>> { Cow::Owned(NSString::new(self).unwrap()) }
}
impl CocoaString for String
{
    fn to_nsstring(&self) -> Cow<AutoreleaseBox<NSString>> { Cow::Owned(NSString::new(self).unwrap()) }
}

pub struct NSColor(Object); DeclareClassDerivative!(NSColor : NSObject);
impl NSColor
{
    pub fn clear_color() -> Option<&'static Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSColor").unwrap(), clearColor] };
        unsafe { (p as *const Self).as_ref() }
    }
}
