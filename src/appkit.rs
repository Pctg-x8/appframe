//! AppKit bindings

use libc::*;
use objc::runtime::*;
use std::mem::zeroed;
use std::borrow::Cow;
use std::mem::forget;
use objc::{Encode, Encoding};

#[cfg(feature = "with_ferrite")]
type NSRunLoopMode = *mut Object;
#[cfg(feature = "with_ferrite")]
pub enum __CVDisplayLink {}
#[cfg(feature = "with_ferrite")]
pub type CVDisplayLinkRef = *mut __CVDisplayLink;
#[cfg(feature = "with_ferrite")]
pub type CVReturn = i32;
#[cfg(feature = "with_ferrite")]
pub type CVDisplayLinkOutputCallback = Option<extern "system" fn(
    link: CVDisplayLinkRef, in_now: *const CVTimeStamp, in_output_time: *const CVTimeStamp,
    in_flags: CVOptionFlags, out_flags: *mut CVOptionFlags, context: *mut c_void) -> CVReturn>;
#[cfg(feature = "with_ferrite")] #[repr(C)]
pub struct CVTimeStamp
{
    pub version: u32, pub videoTimeScale: i32, pub videoTime: i64,
    pub hostTime: u64, pub rateScalar: c_double, pub videoRefreshPeriod: i64,
    pub smpteTime: CVSMPTETime, pub flags: u64, pub reserved: u64
}
#[cfg(feature = "with_ferrite")] #[repr(C)]
pub struct CVSMPTETime
{
    pub subframes: i16, pub subframeDivisor: i16, pub counter: u32,
    pub type_: u32, pub flags: u32, pub hours: i16, pub minutes: i16, pub seconds: i16, pub frames: i16
}
#[cfg(feature = "with_ferrite")] pub type CVOptionFlags = u64;
#[link(name = "AppKit", kind = "framework")] extern {}
#[cfg(feature = "with_ferrite")]
#[link(name = "QuartzCore", kind = "framework")] extern "system"
{
    fn CVDisplayLinkCreateWithActiveCGDisplays(displayLinkOut: *mut CVDisplayLinkRef) -> CVReturn;
    fn CVDisplayLinkSetOutputCallback(link: CVDisplayLinkRef, callback: CVDisplayLinkOutputCallback, userinfo: *mut c_void)
        -> CVReturn;
    fn CVDisplayLinkStart(link: CVDisplayLinkRef) -> CVReturn;
    fn CVDisplayLinkStop(link: CVDisplayLinkRef) -> CVReturn;
    fn CVDisplayLinkRelease(link: CVDisplayLinkRef);
}
#[cfg(feature = "with_ferrite")]
#[link(name = "Foundation", kind = "framework")] extern "system"
{
    pub static NSDefaultRunLoopMode: NSRunLoopMode;
}

#[cfg(target_pointer_width = "64")] pub type CGFloat = f64;
#[cfg(target_pointer_width = "64")] pub type NSInteger = i64;
#[cfg(target_pointer_width = "64")] pub type NSUInteger = u64;
#[cfg(not(target_pointer_width = "64"))] pub type CGFloat = f32;
#[cfg(not(target_pointer_width = "64"))] pub type NSInteger = i32;
#[cfg(not(target_pointer_width = "64"))] pub type NSUInteger = u32;
#[repr(C)] #[derive(Debug, Clone, PartialEq)] pub struct CGPoint { pub x: CGFloat, pub y: CGFloat }
#[repr(C)] #[derive(Debug, Clone, PartialEq)] pub struct CGSize  { pub width: CGFloat, pub height: CGFloat }
#[repr(C)] #[derive(Debug, Clone, PartialEq)] pub struct CGRect  { pub origin: CGPoint, pub size: CGSize }
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

pub struct NSApplication(pub(crate) *mut Object);
impl NSApplication
{
    pub fn shared() -> Option<Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSApplication").unwrap(), sharedApplication] };
        if p.is_null() { None } else { Some(NSApplication(p)) }
    }

    pub fn set_activation_policy(&self, policy: NSApplicationActivationPolicy) -> bool
    {
        let b: BOOL = unsafe { msg_send![self.0, setActivationPolicy: policy as NSInteger] };
        b == YES
    }
    pub fn run(&self) { unsafe { msg_send![self.0, run] } }
    pub fn activate_ignoring_other_apps(&self)
    {
        unsafe { msg_send![self.0, activateIgnoringOtherApps: YES] }
    }
    pub fn set_delegate(&self, delegate: *mut Object)
    {
        unsafe { msg_send![self.0, setDelegate: delegate] }
    }
    pub fn set_main_menu(&self, menu: &NSMenu)
    {
        unsafe { msg_send![self.0, setMainMenu: menu.0] }
    }
}
pub struct NSWindow(*mut Object);
impl NSWindow
{
    pub fn new(content_rect: NSRect, style_mask: NSWindowStyleMask) -> Option<Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSWindow").unwrap(), alloc] };
        if p.is_null() { return None; }
        let p: *mut Object = unsafe { msg_send![p, initWithContentRect: content_rect styleMask: style_mask backing: 2 defer: YES] };
        if p.is_null() { None } else { Some(NSWindow(p)) }
    }
    pub unsafe fn with_view_controller_ptr(vc: *mut Object) -> Option<Self>
    {
        let p: *mut Object = msg_send![Class::get("NSWindow").unwrap(), windowWithContentViewController: vc];
        if p.is_null() { None } else { Some(NSWindow(p)) }
    }

    pub fn center(&self) { unsafe { msg_send![self.0, center] } }
    pub fn make_key_and_order_front(&self, sender: *mut Object)
    {
        unsafe { msg_send![self.0, makeKeyAndOrderFront: sender] }
    }
    pub fn set_title<Title: CocoaString + ?Sized>(&self, title: &Title)
    {
        unsafe { msg_send![self.0, setTitle: title.to_nsstring().0] }
    }

    #[cfg(feature = "with_ferrite")]
    pub fn view_ptr(&self) -> *mut Object { unsafe { msg_send![self.0, contentView] } }
    pub fn view(&self) -> NSView
    {
        let v: *mut Object = unsafe { msg_send![self.0, contentView] };
        NSView(unsafe { msg_send![v, retain] })
    }
    pub fn set_content_view(&self, view: &NSView) { unsafe { msg_send![self.0, setContentView: view.0] } }
    pub unsafe fn set_content_view_raw(&self, view_ptr: *mut Object) { msg_send![self.0, setContentView: view_ptr] }
    pub fn content_view_controller_ptr(&self) -> *mut Object { unsafe { msg_send![self.0, contentViewController] } }
}
impl Drop for NSWindow { fn drop(&mut self) { unsafe { msg_send![self.0, release] } } }
pub struct NSMenu(*mut Object);
impl NSMenu
{
    pub fn new() -> Option<Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSMenu").unwrap(), new] };
        if p.is_null() { None } else { Some(NSMenu(p)) }
    }
    pub fn add_item(&self, item: &NSMenuItem) -> &Self
    {
        unsafe { msg_send![self.0, addItem: item.0] }; self
    }
    pub fn add_separator(&self) -> &Self
    {
        let sep: *mut Object = unsafe { msg_send![Class::get("NSMenuItem").unwrap(), separatorItem] };
        if sep.is_null() { panic!("Null Separator"); }
        unsafe { msg_send![self.0, addItem: sep] }; self
    }
}
impl Drop for NSMenu { fn drop(&mut self) { unsafe { msg_send![self.0, release] } } }
pub struct NSMenuItem(*mut Object);
impl NSMenuItem
{
    pub fn new<Title: CocoaString + ?Sized>(title: &Title, action: Option<Sel>, key_equivalent: Option<&NSString>) -> Option<Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSMenuItem").unwrap(), alloc] };
        if p.is_null() { return None; }
        let k = if let Some(k) = key_equivalent { k.clone() } else { NSString::empty().unwrap() };
        let p: *mut Object = unsafe { msg_send![p, initWithTitle: title.to_nsstring().0 action: action.unwrap_or(zeroed()) keyEquivalent: k.0] };
        if p.is_null() { None } else { Some(NSMenuItem(p)) }
    }
    /*pub fn separator() -> Option<Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSMenuItem").unwrap(), separatorItem] };
        if p.is_null() { None } else { Some(NSMenuItem(unsafe { msg_send![p, retain] })) }
    }*/

    pub fn set_submenu(&self, sub: &NSMenu) -> &Self
    {
        unsafe { msg_send![self.0, setSubmenu: sub.0] }; self
    }
    /*pub fn set_target(&self, target: *mut Object) -> &Self
    {
        unsafe { msg_send![self.0, setTarget: target] }; self
    }*/
    pub fn set_key_equivalent_modifier_mask(&self, mods: NSEventModifierFlags) -> &Self
    {
        unsafe { msg_send![self.0, setKeyEquivalentModifierMask: mods.bits] }; self
    }
    pub fn set_key_equivalent<Str: CocoaString + ?Sized>(&self, k: &Str) -> &Self
    {
        unsafe { msg_send![self.0, setKeyEquivalent: k.to_nsstring().0] }; self
    }
    pub fn set_accelerator<Str: CocoaString + ?Sized>(&self, mods: NSEventModifierFlags, key: &Str) -> &Self
    {
        self.set_key_equivalent(key).set_key_equivalent_modifier_mask(mods)
    }
    // pub fn set_action(&self, sel: Sel) -> &Self { unsafe { msg_send![self.0, setAction: sel] }; self }
}
impl Drop for NSMenuItem { fn drop(&mut self) { unsafe { msg_send![self.0, release] } } }

pub struct NSView(*mut Object);
impl NSView
{
    pub fn set_wants_layer(&self, flag: bool) { unsafe { msg_send![self.0, setWantsLayer: flag as BOOL] } }
    pub fn set_layer(&self, layer: *mut Object) { unsafe { msg_send![self.0, setLayer: layer] } }
    pub fn layer_ptr(&self) -> *mut Object { unsafe { msg_send![self.0, layer] } }
}
pub struct CAMetalLayer(pub(crate) *mut Object);
impl CAMetalLayer
{
    pub fn layer() -> Option<Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("CAMetalLayer").unwrap(), layer] };
        if p.is_null() { None } else { Some(CAMetalLayer(p)) }
    }
    pub fn set_contents_scale(&self, scale: CGFloat) { unsafe { msg_send![self.0, setContentsScale: scale]; } }

    pub fn leave_id(self) -> *mut Object { let p = self.0; forget(self); p }
}
impl Drop for CAMetalLayer { fn drop(&mut self) { unsafe { msg_send![self.0, release] } } }

#[cfg(feature = "with_ferrite")]
pub struct CVDisplayLink(CVDisplayLinkRef);
#[cfg(feature = "with_ferrite")]
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
#[cfg(feature = "with_ferrite")]
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

pub struct NSString(pub(crate) *mut Object);
impl NSString
{
    pub fn new(s: &str) -> Option<Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSString").unwrap(), alloc] };
        if p.is_null() { return None; }
        let bytes = s.as_bytes();
        let p: *mut Object = unsafe { msg_send![p, initWithBytes: bytes.as_ptr() length: bytes.len() as NSUInteger encoding: 4 as NSUInteger] };
        if p.is_null() { None } else { Some(NSString(p)) }
    }
    pub fn empty() -> Option<Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSString").unwrap(), string] };
        if p.is_null() { None } else { Some(NSString(p)) }
    }
    pub fn to_str(&self) -> &str
    {
        let ps: *const c_char = unsafe { msg_send![self.0, UTF8String] };
        unsafe { ::std::ffi::CStr::from_ptr(ps).to_str().unwrap() }
    }

    pub(crate) fn raw(&self) -> *mut Object { self.0 }
    pub(crate) fn leave_id(self) -> *mut Object { let p = self.0; forget(self); p }
    pub(crate) unsafe fn retain_id(id: *mut Object) -> Self { NSString(msg_send![id, retain]) }
}
impl Drop for NSString { fn drop(&mut self) { unsafe { msg_send![self.0, release] } } }
impl Clone for NSString
{
    fn clone(&self) -> Self
    {
        let p: *mut Object = unsafe { msg_send![self.0, retain] };
        if p.is_null() { panic!("Failed retaining"); }
        NSString(p)
    }
}
/// Ref to NSString or Ref to str slice
pub trait CocoaString
{
    fn to_nsstring(&self) -> Cow<NSString>;
}
impl CocoaString for NSString
{
    fn to_nsstring(&self) -> Cow<NSString> { Cow::Borrowed(self) }
}
impl CocoaString for str
{
    fn to_nsstring(&self) -> Cow<NSString> { Cow::Owned(NSString::new(self).unwrap()) }
}
impl CocoaString for String
{
    fn to_nsstring(&self) -> Cow<NSString> { Cow::Owned(NSString::new(self).unwrap()) }
}
