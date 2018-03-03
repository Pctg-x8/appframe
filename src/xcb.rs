//! AppFrame XCB implementation

use rxcb; use rxcb::Event;
use std::rc::Rc;
use {GUIApplicationRunner, Window, WindowBuilder, EventDelegate};
#[cfg(feature = "with_ferrite")] use ferrite as fe;

pub const BITDEPTH: u32 = 24;

pub struct GUIApplication<E: EventDelegate>
{
	srv: Rc<rxcb::Connection>, dg: Rc<E>, root_id: rxcb::WindowID,
	wm_protocols: rxcb::Atom, wm_delete_window: rxcb::Atom,
	desired_visualid: rxcb::VisualID, colormap: rxcb::Colormap,
	action_atoms: ActionAtoms
}
impl<E: EventDelegate> GUIApplicationRunner<E> for GUIApplication<E>
{
    fn run(_appname: &str, delegate: E) -> i32
	{
		let srv = rxcb::Connection::new::<str>(None).expect("Failed to connect to X11 server");
		let (visualid, colormap, root_id);
		{
			let scrn = srv.setup().iter_roots().next().expect("There is no available screen");
			let depth = scrn.iter_allowed_depths().find(|x| x.depth() == BITDEPTH as _).expect("There is no 32bpp support");
			let visual = depth.iter_visuals().find(|x| x.is_truecolor()).expect("There is no TrueColor support");
			root_id = scrn.root();
			colormap = rxcb::Colormap::new(&srv, visual.id(), root_id);
			visualid = visual.id();
		}
		let app = Rc::new(GUIApplication
		{
			wm_protocols: srv.intern("WM_PROTOCOLS").reply(),
			wm_delete_window: srv.intern("WM_DELETE_WINDOW").reply(),
			desired_visualid: visualid, colormap, root_id,
			action_atoms: ActionAtoms::init(&srv),
			srv: Rc::new(srv), dg: Rc::new(delegate)
		});
        app.event_delegate().postinit(&app);

		app.srv.flush();
		while let Some(e) = app.srv.wait_event()
		{
			if e.response_type() == rxcb::ClientMessageEvent::RESPONSE_ENUM
			{
				let e = unsafe { rxcb::ClientMessageEvent::from_ref(&e) };
				if e.msg_type() == app.wm_protocols && e.data_as_u32() == app.wm_delete_window { break; }
			}
		}
		0
	}
    fn event_delegate(&self) -> &Rc<E> { &self.dg }
}
#[cfg(feature = "with_ferrite")]
impl<E: EventDelegate> ::FerriteRenderingServer for GUIApplication<E>
{
    type SurfaceSource = NativeWindow;

    fn presentation_support(&self, adapter: &fe::PhysicalDevice, rendered_qf: u32) -> bool
	{
		adapter.xcb_presentation_support(rendered_qf, self.srv.inner(), self.visual)
	}
    fn create_surface(&self, w: &NativeWindow, instance: &fe::Instance) -> fe::Result<fe::Surface>
	{
		fe::Surface::new_xcb(instance, self.srv.inner(), w.0.id())
	}
}

pub struct NativeWindow(rxcb::Window, Rc<rxcb::Connection>);
impl Window for NativeWindow
{
    fn show(&self) { self.1.map_window(&self.0) }
}
impl Drop for NativeWindow { fn drop(&mut self) { self.1.destroy_window(&self.0); } }
pub struct NativeWindowBuilder<'c>
{
	pos: (i16, i16), size: (u16, u16), caption: &'c str, closable_: bool, resizable_: bool
}
impl<'c> WindowBuilder<'c> for NativeWindowBuilder<'c>
{
    fn new(width: u16, height: u16, caption: &'c str) -> Self
	{
		NativeWindowBuilder
		{
			pos: (0, 0), size: (width, height), caption, closable_: true, resizable_: true
		}
	}
    /// Set window as closable(if true passed, default) or unclosable(if false passed)
    fn closable(&mut self, c: bool) -> &mut Self { self.closable_ = c; self }
    /// Set window as resizable(if true passed, default) or unresizable(if false passed)
    fn resizable(&mut self, c: bool) -> &mut Self { self.resizable_ = c; self }

    /// Create a window
    fn create<E: EventDelegate>(&self, server: &Rc<GUIApplication<E>>) -> Option<NativeWindow>
	{
		let mut vlist = rxcb::WindowValueList::new();
		vlist.back_pixel(0).border_pixel(0).colormap(&server.colormap);
		let mut allowed_actions = vec![
			server.action_atoms.move_,
			server.action_atoms.minimize,
			server.action_atoms.shade,
			server.action_atoms.stick,
			server.action_atoms.maximize_h,
			server.action_atoms.maximize_v,
			server.action_atoms.fullscreen,
			server.action_atoms.change_desktop,
			server.action_atoms.above,
			server.action_atoms.below
		];
		if self.closable_ { allowed_actions.push(server.action_atoms.close); }
		if self.resizable_ { allowed_actions.push(server.action_atoms.resize); }
		let w = server.srv.new_window_id();
		server.srv.create_window(Some(BITDEPTH as _), &w, Some(server.root_id), self.pos.0, self.pos.1,
			self.size.0, self.size.1, 0, rxcb::WindowIOClass::InputOutput, Some(server.desired_visualid), &vlist);
		w.replace_property(&server.srv, rxcb::XCB_ATOM_WM_NAME, self.caption);
		w.replace_property(&server.srv, server.action_atoms.allowed_actions, &allowed_actions[..]);
		Some(NativeWindow(w, server.srv.clone()))
	}
    #[cfg(feature = "with_ferrite")]
    /// Create a Renderable window
    fn create_renderable(&self, server: &Rc<GUIApplication<E>>) -> Option<NativeWindow>
	{

	}
}
pub struct ActionAtoms
{
	allowed_actions: rxcb::Atom,
	move_: rxcb::Atom, resize: rxcb::Atom, minimize: rxcb::Atom,
	shade: rxcb::Atom, stick: rxcb::Atom, maximize_h: rxcb::Atom, maximize_v: rxcb::Atom,
	fullscreen: rxcb::Atom, change_desktop: rxcb::Atom, close: rxcb::Atom, above: rxcb::Atom,
	below: rxcb::Atom
}
impl ActionAtoms
{
	pub fn init(con: &rxcb::Connection) -> Self
	{
		let aack = con.intern("_NET_WM_ALLOWED_ACTIONS");
		let (mvc, rszc) = (con.intern("_NET_WM_ACTION_MOVE"), con.intern("_NET_WM_ACTION_RESIZE"));
		let minc = con.intern("_NET_WM_ACTION_MINIMIZE");
		let (shdc, stkc) = (con.intern("_NET_WM_ACTION_SHADE"), con.intern("_NET_WM_ACTION_STICK"));
		let (mxhc, mxvc) = (con.intern("_NET_WM_ACTION_MAXIMIZE_HORZ"), con.intern("_NET_WM_ACTION_MAXIMIZE_VERT"));
		let (fsc, cdc) = (con.intern("_NET_WM_ACTION_FULLSCREEN"), con.intern("_NET_WM_ACTION_CHANGE_DESKTOP"));
		let (clc, abc, blc) = (con.intern("_NET_WM_ACTION_CLOSE"), con.intern("_NET_WM_ACTION_ABOVE"),
			con.intern("_NET_WM_ACTION_BELOW"));
		ActionAtoms
		{
			allowed_actions: aack.reply(),
			move_: mvc.reply(), resize: rszc.reply(), minimize: minc.reply(),
			shade: shdc.reply(), stick: stkc.reply(), maximize_h: mxhc.reply(), maximize_v: mxvc.reply(),
			fullscreen: fsc.reply(), change_desktop: cdc.reply(), close: clc.reply(),
			above: abc.reply(), below: blc.reply()
		}
	}
}
