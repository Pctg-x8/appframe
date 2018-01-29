//! AppKit bindings

use libc::*;
use objc::runtime::*;
use std::mem::zeroed;

#[link(name = "AppKit", kind = "framework")] extern {}

#[cfg(target_pointer_width = "64")] pub type CGFloat = f64;
#[cfg(target_pointer_width = "64")] pub type NSInteger = i64;
#[cfg(target_pointer_width = "64")] pub type NSUInteger = u64;
#[cfg(not(target_pointer_width = "64"))] pub type CGFloat = f32;
#[cfg(not(target_pointer_width = "64"))] pub type NSInteger = i32;
#[cfg(not(target_pointer_width = "64"))] pub type NSUInteger = u32;
#[repr(C)] pub struct CGPoint { pub x: CGFloat, pub y: CGFloat }
#[repr(C)] pub struct CGSize  { pub width: CGFloat, pub y: CGFloat }
#[repr(C)] pub struct CGRect  { pub origin: CGPoint, pub size: CGSize }
pub type NSRect = CGRect;

#[repr(C)]
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

pub struct NSApplication(*mut Object);
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

    pub fn center(&self) { unsafe { msg_send![self.0, center] } }
    pub fn make_key_and_order_front(&self, sender: *mut Object)
    {
        unsafe { msg_send![self.0, makeKeyAndOrderFront: sender] }
    }
    pub fn set_title(&self, title: &NSString)
    {
        unsafe { msg_send![self.0, setTitle: title.0] }
    }
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
}
impl Drop for NSMenu { fn drop(&mut self) { unsafe { msg_send![self.0, release] } } }
pub struct NSMenuItem(*mut Object);
impl NSMenuItem
{
    pub fn new(title: &NSString, action: Option<Sel>, key_equivalent: Option<&NSString>) -> Option<Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSMenuItem").unwrap(), alloc] };
        if p.is_null() { return None; }
        let k = if let Some(k) = key_equivalent { k.clone() } else { NSString::empty().unwrap() };
        let p: *mut Object = unsafe { msg_send![p, initWithTitle: title.0 action: action.unwrap_or(zeroed()) keyEquivalent: k.0] };
        if p.is_null() { None } else { Some(NSMenuItem(p)) }
    }
    pub fn separator() -> Option<Self>
    {
        let p: *mut Object = unsafe { msg_send![Class::get("NSMenuItem").unwrap(), separatorItem] };
        if p.is_null() { None } else { Some(NSMenuItem(p)) }
    }

    pub fn set_submenu(&self, sub: &NSMenu) -> &Self
    {
        unsafe { msg_send![self.0, setSubmenu: sub.0] }; self
    }
    pub fn set_target(&self, target: *mut Object) -> &Self
    {
        unsafe { msg_send![self.0, setTarget: target] }; self
    }
    pub fn set_key_equivalent_modifier_mask(&self, mods: NSEventModifierFlags) -> &Self
    {
        unsafe { msg_send![self.0, setKeyEquivalentModifierMask: mods.bits] }; self
    }
    pub fn set_key_equivalent(&self, k: &NSString) -> &Self
    {
        unsafe { msg_send![self.0, setKeyEquivalent: k.0] }; self
    }
    pub fn set_accelerator(&self, mods: NSEventModifierFlags, key: &NSString) -> &Self
    {
        self.set_key_equivalent(key).set_key_equivalent_modifier_mask(mods)
    }
    pub fn set_action(&self, sel: Sel) -> &Self
    {
        unsafe { msg_send![self.0, setAction: sel] }
    }
}
impl Drop for NSMenuItem { fn drop(&mut self) { unsafe { msg_send![self.0, release] } } }

pub struct NSString(*mut Object);
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

macro_rules! FunTypeEncoding
{
    (fn ($($arg: ty),*) -> $ret: ty) =>
    {
        format!("{r}{f}{args}",
            r = <$ret as ObjcEncode>::encode_type(),
            f = FunTypeEncoding!(#FrameSize(0) $($arg),*),
            args = FunTypeEncoding!(#ArgEncode(0) $($arg),*))
    };
    (fn ($($arg: ty),*)) =>
    {
        format!("v{f}{args}",
            f = FunTypeEncoding!(#FrameSize(0) $($arg),*),
            args = FunTypeEncoding!(#ArgEncode(0) $($arg),*))
    };
    (#FrameSize($n: expr) $ty1: ty, $($type: ty),*) => { FunTypeEncoding!(#FrameSize($n + ::std::mem::size_of::<$ty1>()) $($type),*) };
    (#FrameSize($n: expr) $ty1: ty) => { $n + ::std::mem::size_of::<$ty1>() };
    (#ArgEncode($offs: expr) $ty1: ty, $($type: ty),*) =>
    {
        format!("{t}{o}{r}",
            t = <$ty1 as ObjcEncode>::encode_type(), o = $offs,
            r = FunTypeEncoding!(#ArgEncode($offs + ::std::mem::size_of::<$ty1>()) $($type),*))
    };
    (#ArgEncode($offs: expr) $ty1: ty) => { format!("{t}{o}", t = <$ty1 as ObjcEncode>::encode_type(), o = $offs) }
}
macro_rules! DeclareObjcClass
{
    (class $t: ident : $b: ident { $($content: tt)* }) =>
    {{
        let mut cc = ClassConstructor::begin(stringify!($t), Some(Class::require(stringify!($b)))).unwrap();
        DeclareObjcClass!(#Construct(cc) $($content)*);
        cc.register()
    }};
    (#Construct($cc: expr) protocol $t: ident; $($rest: tt)*) =>
    {
        $cc = $cc.add_protocol(Protocol::get(stringify!($t)).expect(concat!("Protocol `", stringify!($t), "` could not be found.")));
        DeclareObjcClass!(#Construct($cc) $($rest)*);
    };
    (#Construct($cc: expr) - $selname: ident : (fn($($arg: ty),*)) = $rustfunc: expr; $($rest: tt)*) =>
    {
        $cc = $cc.add_method_typed(concat!(stringify!($selname), ":"), transmute($rustfunc as extern fn(Object, Selector $(, $arg)*)), &FunTypeEncoding!(fn(Object, Selector $(, $arg)*)));
        DeclareObjcClass!(#Construct($cc) $($rest)*);
    };
    (#Construct($cc: expr) - $selname: ident : (fn($($arg: ty),*) -> $ret: ty) = $rustfunc: expr; $($rest: tt)*) =>
    {
        $cc = $cc.add_method_typed(concat!(stringify!($selname), ":"), transmute($rustfunc as extern fn(Object, Selector $(, $arg)*) -> $ret), &FunTypeEncoding!(fn(Object, Selector $(, $arg)*) -> $ret));
        DeclareObjcClass!(#Construct($cc) $($rest)*);
    };
    (#Construct($_cc: expr)) => {}
}
