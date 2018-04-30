//! Windows Runner

#![allow(unused_imports)]

#[cfg(all(feature = "with_bedrock", not(feature = "manual_rendering")))]
extern crate comdrive;
#[cfg(all(feature = "with_bedrock", not(feature = "manual_rendering")))]
use self::comdrive::*;

use std::io::{Result as IOResult, Error as IOError};
use std::mem::{uninitialized, zeroed, size_of};
use std::ptr::{null_mut, null};
use std::ffi::{CString, /*CStr*/};
use std::cell::RefCell;
use winapi::ctypes::c_char;
use winapi::shared::basetsd::LONG_PTR;
use winapi::shared::windef::{HWND, RECT};
use winapi::shared::minwindef::*;
use winapi::shared::rpc::RPC_STATUS;
use winapi::shared::rpcdce::{UUID, RPC_CSTR};
use winapi::um::winuser::*;
use self::{
    GetMessageA as GetMessage, DispatchMessageA as DispatchMessage,
    DefWindowProcA as DefWindowProc, LoadCursorA as LoadCursor,
    RegisterClassExA as RegisterClassEx, CreateWindowExA as CreateWindowEx,
    WNDCLASSEXA as WNDCLASSEX, SetWindowLongPtrA as SetWindowLongPtr, GetWindowLongPtrA as GetWindowLongPtr
};
use winapi::um::libloaderapi::GetModuleHandleA as GetModuleHandle;
use winapi::um::combaseapi::{CoInitializeEx, CoUninitialize};
use winapi::um::objbase::COINIT_MULTITHREADED;
use std::rc::*;
use {EventDelegate, WindowEventDelegate, GUIApplicationRunner, Window, WindowBuilder};

#[cfg(feature = "with_bedrock")] use bedrock as fe;

pub struct GUIApplication<E: EventDelegate>(Option<E>);
impl<E: EventDelegate> GUIApplicationRunner<E> for GUIApplication<E>
{
    fn run(delegate: E) -> i32
    {
        unsafe { CoInitializeEx(null_mut(), COINIT_MULTITHREADED); }
        let app = Rc::new(GUIApplication(Some(delegate)));
        app.event_delegate().postinit(&app);

        let mut msg = unsafe { uninitialized() };
        while unsafe { GetMessage(&mut msg, null_mut(), 0, 0) > 0 }
        {
            unsafe { TranslateMessage(&mut msg); DispatchMessage(&mut msg); }
        }
        return msg.wParam as _;
    }
    fn event_delegate(&self) -> &E { self.0.as_ref().unwrap() }
}
impl<E: EventDelegate> Drop for GUIApplication<E>
{
    fn drop(&mut self)
    {
        self.0 = None; unsafe { CoUninitialize(); }
    }
}
#[cfg(feature = "with_bedrock")]
impl<E: EventDelegate> ::BedrockRenderingServer for GUIApplication<E>
{
    fn presentation_support(&self, adapter: &fe::PhysicalDevice, rendered_qf: u32) -> bool
    {
        adapter.win32_presentation_support(rendered_qf)
    }
    fn create_surface<WE: WindowEventDelegate>(&self, w: &NativeView<WE>, instance: &fe::Instance)
        -> fe::Result<fe::Surface>
    {
        fe::Surface::new_win32(&instance, unsafe { GetModuleHandle(null_mut()) }, w.handle)
    }
}

pub struct CallbackSet<WE: WindowEventDelegate> { w: Weak<WE> }
pub struct NativeWindow<WE: WindowEventDelegate> { handle: HWND, controller: NativeWindowController<WE> }
impl<WE: WindowEventDelegate> Window for NativeWindow<WE> {
    fn show(&self) { unsafe { ShowWindow(self.handle, SW_SHOWNORMAL); } }
    #[cfg(feature = "with_bedrock")]
    fn mark_dirty(&self) { unsafe { InvalidateRect(self.handle, null(), false as _); } }
}
pub type NativeView<WE> = NativeWindow<WE>;

pub struct NativeWindowBuilder<'c>
{
    style: DWORD, cstyle: DWORD, width: u16, height: u16, caption: &'c str
}
impl<'c> WindowBuilder<'c> for NativeWindowBuilder<'c>
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
    fn transparent(&mut self, _c: bool) -> &mut Self
    {
        println!("** Transparent Window support is incomplete for windows **");
        self
    }

    fn create<WE: WindowEventDelegate>(&self, _server: &Rc<GUIApplication<WE::ClientDelegate>>, event: &Rc<WE>)
        -> IOResult<NativeWindow<WE>>
    {
        let cname = UniqueString::generate();
        let wcap = CString::new(self.caption).unwrap();
        let wc = WNDCLASSEX
        {
            cbSize: size_of::<WNDCLASSEX>() as _, cbWndExtra: size_of::<usize>() as _,
            style: self.cstyle, lpszClassName: cname.as_ptr(), lpfnWndProc: Some(NativeWindowController::<WE>::wndproc),
            hInstance: unsafe { GetModuleHandle(null_mut()) },
            hCursor: unsafe { LoadCursor(null_mut(), IDC_ARROW as _) },
            .. unsafe { zeroed() }
        };
        let atom = unsafe { RegisterClassEx(&wc) };
        if atom == 0 { return Err(IOError::last_os_error()); }
        let r = self.adjusted_window_rect();
        let hw = unsafe
        {
            CreateWindowEx(0, wc.lpszClassName, wcap.as_ptr(), self.style,
                CW_USEDEFAULT, CW_USEDEFAULT, r.right - r.left, r.bottom - r.top,
                null_mut(), null_mut(), wc.hInstance, null_mut())
        };
        if hw.is_null() { return Err(IOError::last_os_error()); }

        let controller = NativeWindowController::new(event)?;
        unsafe { SetWindowLongPtr(hw, GWL_USERDATA, (&*controller.callbox) as *const _ as LONG_PTR); }
        return Ok(NativeWindow { handle: hw, controller });
    }
    #[cfg(feature = "with_bedrock")] #[allow(unused_mut)]
    fn create_renderable<WE: WindowEventDelegate>(&self, server: &Rc<GUIApplication<WE::ClientDelegate>>, event: &Rc<WE>)
        -> IOResult<NativeWindow<WE>> where WE::ClientDelegate: 'static
    {
        let mut w = self.create(server, event)?;
        w.controller.callbox.w.upgrade().unwrap().init_view(&w);
        return Ok(w);
    }
}
impl<'c> NativeWindowBuilder<'c>
{
    fn adjusted_window_rect(&self) -> RECT
    {
        let mut r = RECT { left: 0, top: 0, right: self.width as _, bottom: self.height as _ };
        unsafe { AdjustWindowRectEx(&mut r, self.style, false as _, 0) }; r
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

struct NativeWindowController<WE: WindowEventDelegate> {
    callbox: Box<CallbackSet<WE>>,
    #[cfg(all(feature = "with_bedrock", not(feature = "manual_rendering")))]
    _autotimer: (uianimation::Timer, UpdateTimerHandlerCell)
}
impl<WE: WindowEventDelegate> NativeWindowController<WE> {
    #[cfg(all(feature = "with_bedrock", not(feature = "manual_rendering")))]
    pub fn new(event: &Rc<WE>) -> IOResult<Self> {
        let mut timer = uianimation::Timer::new()?;
        let update_handler = UpdateTimerHandlerCell(UpdateTimerHandler::create(event));
        timer.set_update_handler(Some(&update_handler), uianimation::IdleBehavior::Disable)?;
        timer.enable()?;
        return Ok(NativeWindowController {
            callbox: Box::new(CallbackSet { w: Rc::downgrade(event) }),
            _autotimer: (timer, update_handler)
        });
    }
    #[cfg(any(not(feature = "with_bedrock"), feature = "manual_rendering"))]
    pub fn new(event: &Rc<WE>) -> IOResult<Self> {
        Ok(NativeWindowController { callbox: Box::new(CallbackSet { w: Rc::downgrade(event) }) })
    }

    unsafe fn extract_callset_ref<'a>(h: HWND) -> &'a CallbackSet<WE> {
        (GetWindowLongPtr(h, GWL_USERDATA) as *const CallbackSet<WE>).as_ref().unwrap()
    }
    extern "system" fn wndproc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
        match msg {
            WM_DESTROY => unsafe { PostQuitMessage(0); return 0; },
            #[cfg(all(feature = "with_bedrock", feature = "manual_rendering"))]
            WM_PAINT => {
                if let Some(cb) = unsafe { Self::extract_callset_ref(hwnd).w.upgrade() } {
                    unsafe {
                        let mut ps = uninitialized();
                        BeginPaint(hwnd, &mut ps);
                        cb.render();
                        EndPaint(hwnd, &ps);
                    }
                }
                return 0;
            },
            WM_SIZE => if let Some(cb) = unsafe { Self::extract_callset_ref(hwnd).w.upgrade() } {
                cb.resize(LOWORD(lp as _) as _, HIWORD(lp as _) as _, false);
            },
            _ => (/* nothing to do */)
        }
        return unsafe { DefWindowProc(hwnd, msg, wp, lp) };
    }
}

#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
use winapi::shared::winerror::*;
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
use winapi::ctypes::c_void;
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
use winapi::shared::guiddef::REFIID;
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
use winapi::um::unknwnbase::IUnknown;
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
use winapi::Interface;
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
#[repr(C)] pub struct UpdateTimerHandler<WE: WindowEventDelegate>
{
    vtbl: *const uianimation::IUIAnimationTimerUpdateHandlerVtbl, refcount: ULONG,
    client_handler: Option<TimerClientEventHandler>, callback: Weak<WE>
}
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
impl<WE: WindowEventDelegate> UpdateTimerHandler<WE>
{
    const UPDATE_TIMER_HANDLER_VTBL: &'static uianimation::IUIAnimationTimerUpdateHandlerVtbl =
        &uianimation::IUIAnimationTimerUpdateHandlerVtbl
        {
            QueryInterface: Self::query_interface, AddRef: Self::add_ref, Release: Self::release,
            OnUpdate: Self::on_update,
            SetTimerClientEventHandler: Self::set_timer_client_event_handler,
            ClearTimerClientEventHandler: Self::clear_timer_client_event_handler
        };
    
    pub fn create(callback: &Rc<WE>) -> *mut uianimation::IUIAnimationTimerUpdateHandler
    {
        Box::into_raw(Box::new(UpdateTimerHandler
        {
            vtbl: Self::UPDATE_TIMER_HANDLER_VTBL, refcount: 1, client_handler: None, callback: Rc::downgrade(callback)
        })) as _
    }
    unsafe fn refptr<'a>(ptr: *const uianimation::IUIAnimationTimerUpdateHandler) -> &'a Self { &*(ptr as *const Self) }
    unsafe fn refmut<'a>(ptr: *mut uianimation::IUIAnimationTimerUpdateHandler) -> &'a mut Self { &mut *(ptr as *mut Self) }
    extern "system" fn query_interface(this: *mut uianimation::IUIAnimationTimerUpdateHandler,
        riid: REFIID, obj: *mut *mut c_void) -> HRESULT
    {
        unsafe { *obj = null_mut(); }
        if riid == &uianimation::IUIAnimationTimerUpdateHandler::uuidof()
        {
            unsafe { (*this).AddRef(); *obj = this as _; S_OK }
        }
        else if riid == &IUnknown::uuidof()
        { 
            unsafe { (*this).AddRef(); *obj = this as *mut IUnknown as _; S_OK }
        }
        else { E_NOINTERFACE }
    }
    extern "system" fn add_ref(this: *mut uianimation::IUIAnimationTimerUpdateHandler) -> ULONG
    {
        unsafe { Self::refmut(this).refcount += 1; Self::refmut(this).refcount }
    }
    extern "system" fn release(this: *mut uianimation::IUIAnimationTimerUpdateHandler) -> ULONG
    {
        unsafe
        {
            Self::refmut(this).refcount -= 1; let v = Self::refmut(this).refcount;
            if v == 0 { drop(Box::from_raw(this as *mut Self)); }
            v
        }
    }

    extern "system" fn on_update(this: *mut uianimation::IUIAnimationTimerUpdateHandler,
        _time: uianimation::Seconds, result: *mut uianimation::UpdateResult) -> HRESULT
    {
        if let Some(e) = unsafe { Self::refptr(this).callback.upgrade() } { e.render(); }
        // println!("Update: {}", time);
        unsafe { *result = uianimation::UpdateResult::NoChange; }
        S_OK
    }
    extern "system" fn set_timer_client_event_handler(this: *mut uianimation::IUIAnimationTimerUpdateHandler,
        handler: *mut uianimation::IUIAnimationTimerClientEventHandler) -> HRESULT
    {
        unsafe
        {
            if Self::refptr(this).client_handler.is_some() { return UI_E_TIMER_CLIENT_ALREADY_CONNECTED; }
            (*handler).AddRef();
            Self::refmut(this).client_handler = Some(TimerClientEventHandler(handler)); S_OK
        }
    }
    extern "system" fn clear_timer_client_event_handler(this: *mut uianimation::IUIAnimationTimerUpdateHandler)
        -> HRESULT
    {
        unsafe { Self::refmut(this).client_handler = None; S_OK }
    }
}
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
pub struct UpdateTimerHandlerCell(*mut uianimation::IUIAnimationTimerUpdateHandler);
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
impl Drop for UpdateTimerHandlerCell { fn drop(&mut self) { unsafe { (*self.0).Release(); } } }
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
unsafe impl AsRawHandle<uianimation::IUIAnimationTimerUpdateHandler> for UpdateTimerHandlerCell
{
    fn as_raw_handle(&self) -> *mut uianimation::IUIAnimationTimerUpdateHandler { self.0 }
}
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
pub struct TimerClientEventHandler(*mut uianimation::IUIAnimationTimerClientEventHandler);
#[cfg(feature = "with_bedrock")] #[cfg(not(feature = "manual_rendering"))]
impl Drop for TimerClientEventHandler { fn drop(&mut self) { unsafe { (*self.0).Release(); } } }

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
