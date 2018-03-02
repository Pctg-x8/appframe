//! Windows Runner

use std::io::Error as IOError;
use std::mem::{uninitialized, zeroed, size_of};
use std::ptr::null_mut;
use std::ffi::{CString, /*CStr*/};
use winapi::ctypes::c_char;
use winapi::shared::windef::{HWND, RECT};
use winapi::shared::minwindef::{UINT, DWORD, WPARAM, LPARAM, LRESULT};
use winapi::shared::rpc::RPC_STATUS;
use winapi::shared::rpcdce::{UUID, RPC_CSTR};
use winapi::um::winuser::{
    TranslateMessage, ShowWindow, PostQuitMessage, AdjustWindowRectEx,
    WS_CAPTION, WS_BORDER, WS_SYSMENU, WS_THICKFRAME, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
    CS_OWNDC, CS_NOCLOSE, SW_SHOWNORMAL, CW_USEDEFAULT, IDC_ARROW,
    WM_DESTROY
};
use winapi::um::winuser::{
    GetMessageA as GetMessage, DispatchMessageA as DispatchMessage,
    DefWindowProcA as DefWindowProc, LoadCursorA as LoadCursor,
    RegisterClassExA as RegisterClassEx, CreateWindowExA as CreateWindowEx,
    WNDCLASSEXA as WNDCLASSEX
};
use winapi::um::libloaderapi::GetModuleHandleA as GetModuleHandle;
use std::rc::*;
use {EventDelegate, GUIApplicationRunner};
#[cfg(feature = "with_ferrite")]
use std::mem::transmute;

#[cfg(feature = "with_ferrite")] use ferrite as fe;

pub struct GUIApplication<E: EventDelegate>(Rc<E>);
impl<E: EventDelegate> GUIApplicationRunner<E> for GUIApplication<E>
{
    fn run(_appname: &str, delegate: E) -> i32
    {
        let app = Rc::new(GUIApplication(Rc::new(delegate)));
        #[cfg(feature = "with_ferrite")] app.0.postinit(&app);
        #[cfg(not(feature = "with_ferrite"))] app.0.postinit();

        let mut msg = unsafe { uninitialized() };
        while unsafe { GetMessage(&mut msg, null_mut(), 0, 0) > 0 }
        {
            unsafe { TranslateMessage(&mut msg); DispatchMessage(&mut msg); }
        }
        msg.wParam as _
    }
    fn event_delegate(&self) -> &Rc<E> { &self.0 }
}
#[cfg(feature = "with_ferrite")]
impl<E: EventDelegate> ::FerriteRenderingServer for GUIApplication<E>
{
    type SurfaceSource = NativeWindow;

    fn presentation_support(&self, adapter: &fe::PhysicalDevice, rendered_qf: u32) -> bool
    {
        adapter.win32_presentation_support(rendered_qf)
    }
    fn create_surface(&self, w: &NativeWindow, instance: &fe::Instance) -> fe::Result<fe::Surface>
    {
        fe::Surface::new_win32(&instance, unsafe { GetModuleHandle(null_mut()) }, w.0)
    }
}

pub struct NativeWindow(HWND);
impl ::Window for NativeWindow
{
    fn show(&self) { unsafe { ShowWindow(self.0, SW_SHOWNORMAL); } }
}

pub struct NativeWindowBuilder<'c>
{
    style: DWORD, cstyle: DWORD, width: u16, height: u16, caption: &'c str
}
impl<'c> ::WindowBuilder<'c> for NativeWindowBuilder<'c>
{
    fn new(width: u16, height: u16, caption: &'c str) -> Self
    {
        NativeWindowBuilder
        {
            style: WS_CAPTION | WS_BORDER | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_THICKFRAME,
            cstyle: CS_OWNDC, width, height, caption
        }
    }
    fn closable(&mut self, c: bool) -> &mut Self
    {
        if c { self.cstyle &= !CS_NOCLOSE; } else { self.cstyle |= CS_NOCLOSE; } self
    }
    fn resizable(&mut self, c: bool) -> &mut Self
    {
        let bits = WS_THICKFRAME | WS_MAXIMIZEBOX;
        if c { self.style |= bits; } else { self.style &= !bits; } self
    }

    type WindowTy = NativeWindow;
    fn create(&self) -> Option<NativeWindow>
    {
        let cname = UniqueString::generate();
        let wcap = CString::new(self.caption).unwrap();
        let wc = WNDCLASSEX
        {
            cbSize: size_of::<WNDCLASSEX>() as _,
            style: self.cstyle, lpszClassName: cname.as_ptr(), lpfnWndProc: Some(wndproc),
            hInstance: unsafe { GetModuleHandle(null_mut()) },
            hCursor: unsafe { LoadCursor(null_mut(), IDC_ARROW as _) },
            .. unsafe { zeroed() }
        };
        let atom = unsafe { RegisterClassEx(&wc) };
        if atom == 0 { panic!("Failed to allocate WindowClass: {:?}", IOError::last_os_error()); }
        let r = self.adjusted_window_rect();
        let hw = unsafe
        {
            CreateWindowEx(0, wc.lpszClassName, wcap.as_ptr(), self.style,
                CW_USEDEFAULT, CW_USEDEFAULT, r.right - r.left, r.bottom - r.top,
                null_mut(), null_mut(), wc.hInstance, null_mut())
        };
        if hw.is_null() { panic!("Failed to create window: {:?}", IOError::last_os_error()); }
        Some(NativeWindow(hw))
    }
    #[cfg(feature = "with_ferrite")]
    fn create_renderable<E, S>(&self, _server: &Rc<S>) -> Option<Self::WindowTy> where
        E: EventDelegate, S: ::FerriteRenderingServer + GUIApplicationRunner<E>
    {
        let w = if let Some(v) = self.create() { v } else { return None; };
        _server.event_delegate().on_init_view::<S>(&_server, unsafe { transmute(&w) }); Some(w)
    }
}
impl<'c> NativeWindowBuilder<'c>
{
    fn adjusted_window_rect(&self) -> RECT
    {
        let mut r = RECT
        {
            left: 0, top: 0, right: self.width as _, bottom: self.height as _
        };
        unsafe { AdjustWindowRectEx(&mut r, self.style, false as _, 0) }; r
    }
}
extern "system" fn wndproc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT
{
    match msg
    {
        WM_DESTROY => unsafe { PostQuitMessage(0); 0 },
        _ => unsafe { DefWindowProc(hwnd, msg, wp, lp) }
    }
}

/// Extern APIs
#[link(name = "rpcrt4")]
extern
{
    fn UuidCreate(uuid: *mut UUID) -> RPC_STATUS;
    fn UuidToStringA(uuid: *const UUID, string_uuid: *mut RPC_CSTR) -> RPC_STATUS;
    fn RpcStringFreeA(string: *mut RPC_CSTR) -> RPC_STATUS;
}

// use std::str::Utf8Error;
struct UniqueString(RPC_CSTR);
impl UniqueString
{
    fn generate() -> Self
    {
        let mut uuid = unsafe { uninitialized() };
        let r = unsafe { UuidCreate(&mut uuid) };
        // 0 = RPC_S_OK
        if r != 0 { panic!("Unable to create UUID for Window Class"); }
        let mut sptr = unsafe { uninitialized() };
        let r = unsafe { UuidToStringA(&uuid, &mut sptr) };
        if r != 0 { panic!("Unable to allocate memory for UniqueString"); }
        UniqueString(sptr)
    }
    /*fn to_str(&self) -> Result<&str, Utf8Error>
    {
        unsafe { CStr::from_ptr(self.0 as *const _).to_str() }
    }*/
    fn as_ptr(&self) -> *const c_char { self.0 as *const _ }
}
impl Drop for UniqueString
{
    fn drop(&mut self)
    {
        let r = unsafe { RpcStringFreeA(&mut self.0) };
        if r != 0 { panic!("Failed releasing RPCString"); }
    }
}
