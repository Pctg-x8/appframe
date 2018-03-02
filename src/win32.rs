//! Windows Runner

#[cfg(feature = "with_ferrite")]
extern crate comdrive;
#[cfg(feature = "with_ferrite")]
use self::comdrive::*;

use std::io::Error as IOError;
use std::mem::{uninitialized, zeroed, size_of};
use std::ptr::null_mut;
use std::ffi::{CString, /*CStr*/};
use winapi::ctypes::c_char;
use winapi::shared::windef::{HWND, RECT};
use winapi::shared::minwindef::*;
use winapi::shared::rpc::RPC_STATUS;
use winapi::shared::rpcdce::{UUID, RPC_CSTR};
use winapi::um::winuser::*;
use self::{
    GetMessageA as GetMessage, DispatchMessageA as DispatchMessage,
    DefWindowProcA as DefWindowProc, LoadCursorA as LoadCursor,
    RegisterClassExA as RegisterClassEx, CreateWindowExA as CreateWindowEx,
    WNDCLASSEXA as WNDCLASSEX
};
use winapi::um::libloaderapi::GetModuleHandleA as GetModuleHandle;
use winapi::um::combaseapi::{CoInitializeEx, CoUninitialize};
use winapi::um::objbase::COINIT_MULTITHREADED;
use std::rc::*;
use {EventDelegate, GUIApplicationRunner};
#[cfg(feature = "with_ferrite")] use std::io::Result as IOResult;
use std::marker::PhantomData;

#[cfg(feature = "with_ferrite")] use ferrite as fe;

pub struct GUIApplication<E: EventDelegate>(Option<Rc<E>>);
impl<E: EventDelegate> GUIApplicationRunner<E> for GUIApplication<E>
{
    fn run(_appname: &str, delegate: E) -> i32
    {
        unsafe { CoInitializeEx(null_mut(), COINIT_MULTITHREADED); }
        let app = Rc::new(GUIApplication(Some(Rc::new(delegate))));
        #[cfg(feature = "with_ferrite")] app.event_delegate().postinit(&app);
        #[cfg(not(feature = "with_ferrite"))] app.event_delegate().postinit();

        let mut msg = unsafe { uninitialized() };
        while unsafe { GetMessage(&mut msg, null_mut(), 0, 0) > 0 }
        {
            unsafe { TranslateMessage(&mut msg); DispatchMessage(&mut msg); }
        }
        msg.wParam as _
    }
    fn event_delegate(&self) -> &Rc<E> { self.0.as_ref().unwrap() }
}
impl<E: EventDelegate> Drop for GUIApplication<E>
{
    fn drop(&mut self)
    {
        drop(self.0.take()); unsafe { CoUninitialize(); }
    }
}
#[cfg(feature = "with_ferrite")]
impl<E: EventDelegate> ::FerriteRenderingServer for GUIApplication<E>
{
    type SurfaceSource = NativeWindow<E>;

    fn presentation_support(&self, adapter: &fe::PhysicalDevice, rendered_qf: u32) -> bool
    {
        adapter.win32_presentation_support(rendered_qf)
    }
    fn create_surface(&self, w: &NativeWindow<E>, instance: &fe::Instance) -> fe::Result<fe::Surface>
    {
        fe::Surface::new_win32(&instance, unsafe { GetModuleHandle(null_mut()) }, w.0)
    }
}

#[cfg(feature = "with_ferrite")]
pub struct NativeWindow<E: EventDelegate>(HWND, Option<NativeWindowController<E>>);
#[cfg(not(feature = "with_ferrite"))]
pub struct NativeWindow<E: EventDelegate>(HWND, PhantomData<Weak<GUIApplication<E>>>);
impl<E: EventDelegate> ::Window for NativeWindow<E>
{
    fn show(&self) { unsafe { ShowWindow(self.0, SW_SHOWNORMAL); } }
}


pub struct NativeWindowBuilder<'c, E: EventDelegate + 'c>
{
    style: DWORD, cstyle: DWORD, width: u16, height: u16, caption: &'c str, ph: PhantomData<&'c E>
}
impl<'c, E: EventDelegate> ::WindowBuilder<'c, E> for NativeWindowBuilder<'c, E>
{
    fn new(width: u16, height: u16, caption: &'c str) -> Self
    {
        NativeWindowBuilder
        {
            style: WS_CAPTION | WS_BORDER | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_THICKFRAME,
            cstyle: CS_OWNDC, width, height, caption, ph: PhantomData
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

    type WindowTy = NativeWindow<E>;
    fn create(&self) -> Option<NativeWindow<E>>
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
        #[cfg(feature = "with_ferrite")] { Some(NativeWindow(hw, None)) }
        #[cfg(not(feature = "with_ferrite"))] { Some(NativeWindow(hw, PhantomData)) }
    }
    #[cfg(feature = "with_ferrite")]
    fn create_renderable(&self, server: &Rc<GUIApplication<E>>) -> Option<Self::WindowTy>
    {
        let mut w = if let Some(v) = self.create() { v } else { return None; };
        w.1 = match NativeWindowController::new(server)
        {
            Ok(v) => Some(v),
            Err(e) => { println!("Error while initialising NativeWindowController: {}", e); return None; }
        };
        server.event_delegate().on_init_view(&server, &w); Some(w)
    }
}
impl<'c, E: EventDelegate> NativeWindowBuilder<'c, E>
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

#[cfg(feature = "with_ferrite")]
struct NativeWindowController<E: EventDelegate>(uianimation::Timer, Weak<GUIApplication<E>>, UpdateTimerHandlerCell);
#[cfg(feature = "with_ferrite")]
impl<E: EventDelegate> NativeWindowController<E>
{
    pub fn new(srv: &Rc<GUIApplication<E>>) -> IOResult<Self>
    {
        let mut timer = uianimation::Timer::new()?;
        let update_handler = UpdateTimerHandlerCell(UpdateTimerHandler::create(srv.event_delegate()));
        timer.set_update_handler(Some(&update_handler), uianimation::IdleBehavior::Disable)?;
        timer.enable()?;
        Ok(NativeWindowController(timer, Rc::downgrade(srv), update_handler))
    }
}

#[cfg(feature = "with_ferrite")]
use winapi::shared::winerror::*;
#[cfg(feature = "with_ferrite")]
use winapi::ctypes::c_void;
#[cfg(feature = "with_ferrite")]
use winapi::shared::guiddef::REFIID;
#[cfg(feature = "with_ferrite")]
use winapi::um::unknwnbase::IUnknown;
#[cfg(feature = "with_ferrite")]
use winapi::Interface;
#[cfg(feature = "with_ferrite")]
#[repr(C)] pub struct UpdateTimerHandler<E: EventDelegate>
{
    vtbl: *const uianimation::IUIAnimationTimerUpdateHandlerVtbl, refcount: ULONG,
    client_handler: Option<TimerClientEventHandler>, callback: Weak<E>
}
#[cfg(feature = "with_ferrite")]
impl<E: EventDelegate> UpdateTimerHandler<E>
{
    const UPDATE_TIMER_HANDLER_VTBL: &'static uianimation::IUIAnimationTimerUpdateHandlerVtbl =
        &uianimation::IUIAnimationTimerUpdateHandlerVtbl
        {
            QueryInterface: Self::query_interface, AddRef: Self::add_ref, Release: Self::release,
            OnUpdate: Self::on_update,
            SetTimerClientEventHandler: Self::set_timer_client_event_handler,
            ClearTimerClientEventHandler: Self::clear_timer_client_event_handler
        };
    
    pub fn create(callback: &Rc<E>) -> *mut uianimation::IUIAnimationTimerUpdateHandler
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
        time: uianimation::Seconds, result: *mut uianimation::UpdateResult) -> HRESULT
    {
        if let Some(e) = unsafe { Self::refptr(this).callback.upgrade() } { e.on_render_period(); }
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
#[cfg(feature = "with_ferrite")]
pub struct UpdateTimerHandlerCell(*mut uianimation::IUIAnimationTimerUpdateHandler);
#[cfg(feature = "with_ferrite")]
impl Drop for UpdateTimerHandlerCell { fn drop(&mut self) { unsafe { (*self.0).Release(); } } }
#[cfg(feature = "with_ferrite")]
impl AsRawHandle<uianimation::IUIAnimationTimerUpdateHandler> for UpdateTimerHandlerCell
{
    fn as_raw_handle(&self) -> *mut uianimation::IUIAnimationTimerUpdateHandler { self.0 }
}
#[cfg(feature = "with_ferrite")]
pub struct TimerClientEventHandler(*mut uianimation::IUIAnimationTimerClientEventHandler);
#[cfg(feature = "with_ferrite")]
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
