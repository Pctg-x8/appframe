//! Objective XCB Wrapper

#![allow(dead_code)]

extern crate univstring; use self::univstring::UnivString;
extern crate xcb;
use self::xcb::ffi::*;
use std::ptr::{null, null_mut};
use std::marker::PhantomData;
use std::io::{Error as IOError, ErrorKind};

#[repr(C)] pub enum WindowIOClass
{
	InputOnly = XCB_WINDOW_CLASS_INPUT_ONLY as _,
	InputOutput = XCB_WINDOW_CLASS_INPUT_OUTPUT as _,
	FromParent = XCB_WINDOW_CLASS_COPY_FROM_PARENT as _
}

pub struct Connection(*mut xcb_connection_t);
impl Connection
{
	pub fn new<S: UnivString + ?Sized>(display: Option<&S>) -> Option<Self>
	{
		let display_name = display.map(|s| s.to_cstr().unwrap());
		let p = unsafe
		{
			xcb_connect(display_name.as_ref().map(|p| p.as_ptr()).unwrap_or(null()), null_mut())
		};
		if p.is_null() { None } else { Some(Connection(p)) }
	}
	#[cfg(feature = "with_ferrite")]
	pub(crate) fn inner(&self) -> *mut xcb_connection_t { self.0 }
	pub fn setup(&self) -> &Setup { unsafe { &*(xcb_get_setup(self.0) as *mut _) } }
	pub fn new_id(&self) -> u32 { unsafe { xcb_generate_id(self.0) } }
	pub fn new_window_id(&self) -> Window { Window(self.new_id()) }

	/*pub fn try_intern(&self, name: &str) -> AtomCookie
	{
		AtomCookie(unsafe { xcb_intern_atom(self.0, 0, name.len() as _, name.as_ptr()) }, self)
	}*/
	pub fn intern(&self, name: &str) -> AtomCookie
	{
		AtomCookie(unsafe { xcb_intern_atom(self.0, 1, name.len() as _, name.as_ptr() as _) }, self)
	}
	pub fn flush(&self) { unsafe { xcb_flush(self.0); } }

	pub fn create_window(&self, depth: Option<u8>, id: &Window, parent: Option<xcb_window_t>,
		x: i16, y: i16, width: u16, height: u16, border_width: u16, class: WindowIOClass,
		visual: Option<VisualID>, valuelist: &WindowValueList) -> Result<(), GenericError>
	{
		let serialized = valuelist.serialize();
		unsafe
		{
			CheckedCookie(xcb_create_window_checked(self.0, depth.unwrap_or(XCB_COPY_FROM_PARENT as _), id.0,
				parent.unwrap_or_else(|| self.setup().iter_roots().next().unwrap().root()),
				x, y, width, height, border_width, class as _, visual.unwrap_or(XCB_COPY_FROM_PARENT as _),
				valuelist.0, serialized.0 as *const _), self).check()
		}
	}
	pub fn map_window(&self, w: &Window) -> Result<(), GenericError>
	{
		unsafe { CheckedCookie(xcb_map_window_checked(self.0, w.0), self).check() }
	}
	pub fn destroy_window(&self, w: &Window) -> Result<(), GenericError>
	{
		unsafe { CheckedCookie(xcb_destroy_window_checked(self.0, w.0), self).check() }
	}
}
impl Drop for Connection { fn drop(&mut self) { unsafe { xcb_disconnect(self.0) } } }

pub struct Setup(xcb_setup_t);
impl Setup
{
	pub fn iter_roots(&self) -> IterRootScreen { IterRootScreen(unsafe { xcb_setup_roots_iterator(&self.0) }) }
}
#[repr(C)] pub struct Screen(xcb_screen_t);
impl Screen
{
	pub fn root(&self) -> xcb_window_t { self.0.root }
	// pub fn default_colormap(&self) -> xcb_colormap_t { self.0.default_colormap }
}
pub struct IterRootScreen<'s>(xcb_screen_iterator_t<'s>);
impl<'s> Iterator for IterRootScreen<'s>
{
	type Item = &'s Screen;
	fn next(&mut self) -> Option<&'s Screen>
	{
		if self.0.rem <= 0 { None }
		else { let p = self.0.data as *mut _; unsafe { xcb_screen_next(&mut self.0); Some(&*p) } }
	}
}

pub type WindowID = xcb_window_t;
pub struct Window(WindowID);
impl Window
{
	pub(crate) fn id(&self) -> WindowID { self.0 }
	pub fn replace_property<T: PropertyType + ?Sized>(&self, con: &Connection, property: Atom, value: &T)
	{
		value.change_property_of(con, self, property, XCB_PROP_MODE_REPLACE)
	}
}
pub trait PropertyType
{
	const TYPE_ATOM: Atom; const DATA_STRIDE: u32;
	fn change_property_of(&self, connection: &Connection, window: &Window, property: Atom, mode: u32);
}
impl PropertyType for str
{
	const TYPE_ATOM: Atom = XCB_ATOM_STRING; const DATA_STRIDE: u32 = 8;
	fn change_property_of(&self, con: &Connection, window: &Window, props: Atom, mode: u32)
	{
		unsafe
		{
			xcb_change_property(con.0, mode as _, window.0, props, XCB_ATOM_STRING, 8,
				self.len() as _, self.as_ptr() as _);
		}
	}
}
impl PropertyType for Atom
{
	const TYPE_ATOM: Atom = XCB_ATOM_ATOM; const DATA_STRIDE: u32 = 32;
	fn change_property_of(&self, con: &Connection, window: &Window, props: Atom, mode: u32)
	{
		unsafe
		{
			xcb_change_property(con.0, mode as _, window.0, props, XCB_ATOM_ATOM, 32, 1,
				self as *const Atom as *const _);
		}
	}
}
impl<E: PropertyType> PropertyType for [E]
{
	const TYPE_ATOM: Atom = E::TYPE_ATOM; const DATA_STRIDE: u32 = E::DATA_STRIDE;
	fn change_property_of(&self, con: &Connection, window: &Window, props: Atom, mode: u32)
	{
		unsafe
		{
			xcb_change_property(con.0, mode as _, window.0, props, E::TYPE_ATOM, E::DATA_STRIDE as _,
				self.len() as _, self.as_ptr() as _);
		}
	}
}
pub use self::xcb::ffi::XCB_ATOM_WM_NAME;

pub struct CheckedCookie<'s>(xcb_void_cookie_t, &'s Connection);
impl<'s> CheckedCookie<'s>
{
	pub fn check(&self) -> Result<(), GenericError>
	{
		let r = unsafe { xcb_request_check(self.1 .0, self.0) };
		if r.is_null() { Ok(()) } else { Err(unsafe { GenericError::from_ptr(r) }) }
	}
}
pub struct AtomCookie<'s>(xcb_intern_atom_cookie_t, &'s Connection);
pub type Atom = xcb_atom_t;
impl<'s> AtomCookie<'s>
{
	pub fn reply(self) -> Result<Atom, GenericError>
	{
		let mut _eptr = null_mut();
		let r = unsafe { xcb_intern_atom_reply(self.1 .0, self.0, &mut _eptr) };
		if r.is_null() { Err(unsafe { GenericError::from_ptr(_eptr) }) } else { Ok(MallocBox(r).atom) }
	}
}

use std::mem::transmute;
pub struct GenericEvent(MallocBox<xcb_generic_event_t>);
impl Connection
{
	pub fn wait_event(&self) -> Option<GenericEvent>
	{
		let p = unsafe { xcb_wait_for_event(self.0) };
		if p.is_null() { None } else { Some(GenericEvent(MallocBox(p))) }
	}
	pub fn poll_event(&self) -> Option<GenericEvent>
	{
		let p = unsafe { xcb_poll_for_event(self.0) };
		if p.is_null() { None } else { Some(GenericEvent(MallocBox(p))) }
	}
}
impl GenericEvent
{
	pub fn response_type(&self) -> u8 { self.0.response_type & !0x80 }
}
pub struct ClientMessageEvent(MallocBox<xcb_client_message_event_t>);
impl ClientMessageEvent
{
	pub fn msg_type(&self) -> xcb_atom_t { self.0.type_ }
	pub fn data_as_u32(&self) -> u32 { unsafe { *(self.0.data.data.as_ptr() as *const u32) } }
}
pub struct GenericError(MallocBox<xcb_generic_error_t>);
impl GenericError
{
	unsafe fn from_ptr(p: *mut xcb_generic_error_t) -> Self { GenericError(MallocBox(p)) }
}
impl Debug for GenericError
{
	fn fmt(&self, fmt: &mut Formatter) -> FmtResult { write!(fmt, "GenericError(code={})", (*self.0).error_code) }
}
impl Display for GenericError
{
	fn fmt(&self, fmt: &mut Formatter) -> FmtResult { <Self as Debug>::fmt(self, fmt) }
}
impl From<GenericError> for IOError
{
	fn from(v: GenericError) -> IOError { IOError::new(ErrorKind::Other, Box::new(v)) }
}
impl ::std::error::Error for GenericError
{
	fn description(&self) -> &str { "XCB Generic Error" }
	fn cause(&self) -> Option<&::std::error::Error> { None }
}
unsafe impl Send for GenericError {}
unsafe impl Sync for GenericError {}
pub trait Event
{
	const RESPONSE_ENUM: u8;
	unsafe fn from_ref(g: &GenericEvent) -> &Self;
}
impl Event for ClientMessageEvent
{
	const RESPONSE_ENUM: u8 = XCB_CLIENT_MESSAGE;
	unsafe fn from_ref(g: &GenericEvent) -> &Self { transmute(g) }
}
impl Event for GenericError
{
	const RESPONSE_ENUM: u8 = 0; // unused
	unsafe fn from_ref(g: &GenericEvent) -> &Self { transmute(g) }
}

#[repr(C)] pub struct Depth(xcb_depth_t);
impl Depth
{
	pub fn depth(&self) -> u8 { self.0.depth }
}
pub struct IterDepths<'c>(xcb_depth_iterator_t<'c>);
impl<'c> Iterator for IterDepths<'c>
{
	type Item = &'c Depth;
	fn next(&mut self) -> Option<&'c Depth>
	{
		if self.0.rem <= 0 { None }
		else { let p = self.0.data as *mut _; unsafe { xcb_depth_next(&mut self.0); Some(&*p) } }
	}
	fn size_hint(&self) -> (usize, Option<usize>) { (self.0.rem as _, Some(self.0.rem as _)) }
}
impl Screen
{
	pub fn iter_allowed_depths(&self) -> IterDepths { IterDepths(unsafe { xcb_screen_allowed_depths_iterator(&self.0) }) }
}
pub type VisualID = xcb_visualid_t;
#[repr(C)] pub struct VisualType(xcb_visualtype_t);
impl VisualType
{
	pub fn id(&self) -> VisualID { self.0.visual_id }
	pub fn is_truecolor(&self) -> bool { self.0.class == XCB_VISUAL_CLASS_TRUE_COLOR as _ }
}
pub struct IterVisualTypes<'c>(xcb_visualtype_iterator_t, PhantomData<&'c Connection>);
impl<'c> Iterator for IterVisualTypes<'c>
{
	type Item = &'c VisualType;
	fn next(&mut self) -> Option<&'c VisualType>
	{
		if self.0.rem <= 0 { None }
		else { let p = self.0.data as *mut _; unsafe { xcb_visualtype_next(&mut self.0); Some(&*p) } }
	}
}
impl Depth
{
	pub fn iter_visuals(&self) -> IterVisualTypes
	{
		IterVisualTypes(unsafe { xcb_depth_visuals_iterator(&self.0) }, PhantomData)
	}
}

#[allow(non_camel_case_types)]
pub type xcb_bool32_t = u32;
#[repr(C)] #[allow(non_camel_case_types)]
pub struct xcb_create_window_value_list_t
{
	pub background_pixmap: xcb_pixmap_t, pub background_pixel: u32,
	pub border_pixmap: xcb_pixmap_t, pub border_pixel: u32,
	pub bit_gravity: u32, pub win_gravity: u32, pub backing_store: u32, pub backing_planes: u32, pub backing_pixel: u32,
	pub override_redirect: xcb_bool32_t, pub save_under: xcb_bool32_t, pub event_mask: u32,
	pub do_not_propagate_mask: u32, pub colormap: xcb_colormap_t, pub cursor: xcb_cursor_t
}
extern "C"
{
	fn xcb_create_window_value_list_serialize(buffer: *mut *mut ::libc::c_void, value_mask: u32,
		aux: *const xcb_create_window_value_list_t) -> ::libc::c_int;
}
#[repr(C)]
pub struct WindowValueList(u32, xcb_create_window_value_list_t);
impl WindowValueList
{
	pub fn new() -> Self { WindowValueList(0, unsafe { ::std::mem::zeroed() }) }
	pub fn border_pixel(&mut self, p: u32) -> &mut Self
	{
		self.0 |= XCB_CW_BORDER_PIXEL; self.1.border_pixel = p; self
	}
	pub fn back_pixel(&mut self, p: u32) -> &mut Self
	{
		self.0 |= XCB_CW_BACK_PIXEL; self.1.background_pixel = p; self
	}
	pub fn colormap(&mut self, c: &Colormap) -> &mut Self
	{
		self.0 |= XCB_CW_COLORMAP; self.1.colormap = c.id(); self
	}
	
	pub fn serialize(&self) -> MallocBox<::libc::c_void>
	{
		let mut p = null_mut();
		unsafe { xcb_create_window_value_list_serialize(&mut p, self.0, &self.1) };
		MallocBox(p)
	}
}
pub struct Colormap(xcb_colormap_t);
impl Colormap
{
	pub fn new(con: &Connection, visual: VisualID, window: xcb_window_t) -> Self
	{
		let id = con.new_id();
		unsafe { xcb_create_colormap(con.0, XCB_COLORMAP_ALLOC_NONE as _, id, window, visual) }; Colormap(id)
	}
	pub fn id(&self) -> xcb_colormap_t { self.0 }
}

use std::ops::{Deref, DerefMut};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
/// Owned malloc-ed pointer box
pub struct MallocBox<T: ?Sized>(pub *mut T);
impl<T: ?Sized> Deref for MallocBox<T> { type Target = T; fn deref(&self) -> &T { unsafe { &*self.0 } } }
impl<T: ?Sized> DerefMut for MallocBox<T> { fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.0 } } }
impl<T: ?Sized> Drop for MallocBox<T>
{
	fn drop(&mut self) { unsafe { ::libc::free(self.0 as *mut _) } }
}
impl<T: ?Sized> Debug for MallocBox<T> where T: Debug
{
	fn fmt(&self, fmt: &mut Formatter) -> FmtResult { <T as Debug>::fmt(&self, fmt) }
}
