// #![feature(link_args)]
// #![link_args = "-rpath /Users/pctgx8/Downloads/vulkansdk-macos-1.0.69.0/macOS/Frameworks"]

extern crate appframe;
extern crate ferrite;
extern crate libc;

use appframe::*;
use ferrite as fe;
use fe::traits::*;
use std::rc::{Rc, Weak};
use std::cell::{RefCell, Ref};
use std::borrow::Cow;

#[repr(C)] #[derive(Clone)] pub struct Vertex([f32; 4], [f32; 4]);

struct ShaderStore { v_pass: fe::ShaderModule, f_phong: fe::ShaderModule }
impl ShaderStore {
    fn load(device: &fe::Device) -> Result<Self, Box<std::error::Error>> {
        Ok(ShaderStore
        {
            v_pass: fe::ShaderModule::from_file(device, "shaders/pass.vso")?,
            f_phong: fe::ShaderModule::from_file(device, "shaders/phong.fso")?
        })
    }
}
const VBIND: &[fe::vk::VkVertexInputBindingDescription] = &[
    fe::vk::VkVertexInputBindingDescription {
        binding: 0, stride: 16 * 2, inputRate: fe::vk::VK_VERTEX_INPUT_RATE_VERTEX
    }
];
const VATTRS: &[fe::vk::VkVertexInputAttributeDescription] = &[
    fe::vk::VkVertexInputAttributeDescription {
        binding: 0, location: 0, offset: 0, format: fe::vk::VK_FORMAT_R32G32B32A32_SFLOAT
    },
    fe::vk::VkVertexInputAttributeDescription {
        binding: 0, location: 1, offset: 16, format: fe::vk::VK_FORMAT_R32G32B32A32_SFLOAT
    }
];

pub struct LazyInit<T>(RefCell<Option<T>>);
impl<T> LazyInit<T> {
    pub fn new() -> Self { LazyInit(None.into()) }
    pub fn init(&self, v: T) { *self.0.borrow_mut() = v.into(); }
    pub fn get(&self) -> Ref<T> { Ref::map(self.0.borrow(), |v| v.as_ref().unwrap()) }
    pub fn is_presented(&self) -> bool { self.0.borrow().is_some() }
    // pub fn get_mut(&self) -> RefMut<T> { RefMut::map(self.0.borrow_mut(), |v| v.as_mut().unwrap()) }
}
pub struct Discardable<T>(RefCell<Option<T>>);
impl<T> Discardable<T> {
    pub fn new() -> Self { Discardable(None.into()) }
    pub fn set(&self, v: T) { *self.0.borrow_mut() = v.into(); }
    pub fn get(&self) -> Ref<T> { Ref::map(self.0.borrow(), |v| v.as_ref().unwrap()) }

    pub fn discard(&self) { *self.0.borrow_mut() = None; }
    pub fn is_discarded(&self) -> bool { self.0.borrow().is_none() }
}

pub struct Ferrite {
    gq: u32, _tq: u32, queue: fe::Queue, tqueue: fe::Queue, cmdpool: fe::CommandPool, tcmdpool: fe::CommandPool,
    semaphore_sync_next: fe::Semaphore, semaphore_command_completion: fe::Semaphore,
    fence_command_completion: fe::Fence,
    device: fe::Device, adapter: fe::PhysicalDevice, _d: fe::DebugReportCallback, instance: fe::Instance
}
pub struct Resources {
    buf: fe::Buffer, _dmem: fe::DeviceMemory, pl: fe::PipelineLayout, shaders: ShaderStore
}
struct App { main_wnd: LazyInit<Rc<MainWindow>>, res: LazyInit<Rc<Resources>>, ferrite: LazyInit<Rc<Ferrite>> }
impl App {
    fn new() -> Self {
        App { main_wnd: LazyInit::new(), ferrite: LazyInit::new(), res: LazyInit::new() }
    }
}
impl EventDelegate for App {
    fn postinit(&self, server: &Rc<GUIApplication<Self>>) {
        extern "system" fn dbg_cb(_flags: fe::vk::VkDebugReportFlagsEXT, _object_type: fe::vk::VkDebugReportObjectTypeEXT,
            _object: u64, _location: libc::size_t, _message_code: i32, _layer_prefix: *const libc::c_char,
            message: *const libc::c_char, _user_data: *mut libc::c_void) -> fe::vk::VkBool32
        {
            println!("dbg_cb: {}", unsafe { std::ffi::CStr::from_ptr(message).to_str().unwrap() });
            false as _
        }

        #[cfg(target_os = "macos")] const PLATFORM_SURFACE: &str = "VK_MVK_macos_surface";
        #[cfg(windows)] const PLATFORM_SURFACE: &str = "VK_KHR_win32_surface";
        #[cfg(feature = "with_xcb")] const PLATFORM_SURFACE: &str = "VK_KHR_xcb_surface";
        let instance = fe::InstanceBuilder::new("appframe_integ", (0, 1, 0), "Ferrite", (0, 1, 0))
            .add_extensions(vec!["VK_KHR_surface", PLATFORM_SURFACE, "VK_EXT_debug_report"])
            .add_layer("VK_LAYER_LUNARG_standard_validation")
            .create().unwrap();
        let d = fe::DebugReportCallbackBuilder::new(&instance, dbg_cb).report_error().report_warning()
            .report_performance_warning().create().unwrap();
        let adapter = instance.iter_physical_devices().unwrap().next().unwrap();
        println!("Vulkan AdapterName: {}", unsafe { std::ffi::CStr::from_ptr(adapter.properties().deviceName.as_ptr()).to_str().unwrap() });
        let memindices = adapter.memory_properties();
        let qfp = adapter.queue_family_properties();
        let gq = qfp.find_matching_index(fe::QueueFlags::GRAPHICS).expect("Cannot find a graphics queue");
        let tq = qfp.find_another_matching_index(fe::QueueFlags::TRANSFER, gq)
            .or_else(|| qfp.find_matching_index(fe::QueueFlags::TRANSFER)).expect("No transferrable queue family found");
        let united_queue = gq == tq;
        let qs = if united_queue { vec![fe::DeviceQueueCreateInfo(gq, vec![0.0; 2])] }
            else { vec![fe::DeviceQueueCreateInfo(gq, vec![0.0]), fe::DeviceQueueCreateInfo(tq, vec![0.0])] };
        let device = fe::DeviceBuilder::new(&adapter)
            .add_extensions(vec!["VK_KHR_swapchain"]).add_queues(qs)
            .create().unwrap();
        let queue = device.queue(gq, 0);
        let cmdpool = fe::CommandPool::new(&device, gq, false, false).unwrap();
        let device_memindex = memindices.find_device_local_index().unwrap();
        let upload_memindex = memindices.find_host_visible_index().unwrap();
        
        let bufsize = std::mem::size_of::<Vertex>() * 3;
        let buf = fe::BufferDesc::new(bufsize, fe::BufferUsage::VERTEX_BUFFER.transfer_dest()).create(&device).unwrap();
        let memreq = buf.requirements();
        let dmem = fe::DeviceMemory::allocate(&device, memreq.size as _, device_memindex).unwrap();
        {
            let upload_buf = fe::BufferDesc::new(bufsize, fe::BufferUsage::VERTEX_BUFFER.transfer_src()).create(&device)
                .unwrap();
            let upload_memreq = upload_buf.requirements();
            let upload_mem = fe::DeviceMemory::allocate(&device, upload_memreq.size as _, upload_memindex).unwrap();
            buf.bind(&dmem, 0).unwrap(); upload_buf.bind(&upload_mem, 0).unwrap();
            unsafe 
            {
                upload_mem.map(0 .. bufsize).unwrap().slice_mut(0, 3).clone_from_slice(&[
                    Vertex([0.0, -1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
                    Vertex([-1.0, 1.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]),
                    Vertex([1.0, 1.0, 0.0, 1.0], [0.5, 0.0, 1.0, 0.0])
                ]);
            }
            let fw = fe::Fence::new(&device, false).unwrap();
            let init_commands = cmdpool.alloc(1, true).unwrap();
            init_commands[0].begin().unwrap()
                .pipeline_barrier(fe::PipelineStageFlags::TOP_OF_PIPE, fe::PipelineStageFlags::TRANSFER, true,
                    &[], &[
                        fe::BufferMemoryBarrier::new(&buf, 0 .. bufsize, 0, fe::AccessFlags::TRANSFER.write),
                        fe::BufferMemoryBarrier::new(&upload_buf, 0 .. bufsize, 0, fe::AccessFlags::TRANSFER.read)
                    ], &[])
                .copy_buffer(&upload_buf, &buf, &[fe::vk::VkBufferCopy { srcOffset: 0, dstOffset: 0, size: bufsize as _ }])
                .pipeline_barrier(fe::PipelineStageFlags::TRANSFER, fe::PipelineStageFlags::VERTEX_INPUT, true,
                    &[], &[fe::BufferMemoryBarrier::new(&buf, 0 .. bufsize, fe::AccessFlags::TRANSFER.write,
                        fe::AccessFlags::VERTEX_ATTRIBUTE_READ)], &[]);
            queue.submit(&[fe::SubmissionBatch
            {
                command_buffers: Cow::Borrowed(&init_commands), .. Default::default()
            }], Some(&fw)).unwrap(); fw.wait().unwrap();
        }
        self.res.init(Rc::new(Resources {
            shaders: ShaderStore::load(&device).unwrap(),
            pl: fe::PipelineLayout::new(&device, &[], &[]).unwrap(), buf, _dmem: dmem
        }));
        self.ferrite.init(Rc::new(Ferrite {
            fence_command_completion: fe::Fence::new(&device, false).unwrap(),
            semaphore_sync_next: fe::Semaphore::new(&device).unwrap(),
            semaphore_command_completion: fe::Semaphore::new(&device).unwrap(),
            tcmdpool: fe::CommandPool::new(&device, tq, false, false).unwrap(),
            cmdpool, queue, tqueue: device.queue(tq, if united_queue { 1 } else { 0 }),
            device, adapter, instance, gq, _tq: tq, _d: d
        }));

        let w = MainWindow::new(server); w.window.get().show();
        self.main_wnd.init(w);
    }
}

pub struct DescribedSurface {
    object: fe::Surface, format: fe::vk::VkSurfaceFormatKHR, present_mode: fe::PresentMode,
    composite_mode: fe::CompositeAlpha, buffer_count: u32
}
impl DescribedSurface {
    pub fn from(adapter: &fe::PhysicalDevice, s: fe::Surface) -> fe::Result<Self> {
        let caps = adapter.surface_capabilities(&s)?;
        let mut fmtq = fe::FormatQueryPred::new();
        fmtq.bit(32).components(fe::FormatComponents::RGBA).elements(fe::ElementType::UNORM);
        let format = adapter.surface_formats(&s)?.into_iter().find(|f| fmtq.satisfy(f.format)).unwrap();
        let present_mode = adapter.surface_present_modes(&s)?.remove(0);
        let composite_mode = if (caps.supportedCompositeAlpha & fe::CompositeAlpha::PostMultiplied as u32) != 0
        {
            fe::CompositeAlpha::PostMultiplied
        }
        else { fe::CompositeAlpha::Opaque };
        let buffer_count = caps.minImageCount.max(2);

        return Ok(DescribedSurface { object: s, format, present_mode, composite_mode, buffer_count })
    }
}

struct MainWindow {
    commands: Discardable<RenderCommands>, rts: Discardable<WindowRenderTargets>,
    render_res: LazyInit<RenderResources>, surface: LazyInit<DescribedSurface>,
    window: LazyInit<NativeWindow<MainWindow>>,
    ferrite: Rc<Ferrite>, comres: Rc<Resources>, app: Weak<GUIApplication<App>>
}
impl MainWindow {
    pub fn new(srv: &Rc<GUIApplication<App>>) -> Rc<Self> {
        let w = Rc::new(MainWindow {
            app: Rc::downgrade(srv), ferrite: srv.event_delegate().ferrite.get().clone(),
            comres: srv.event_delegate().res.get().clone(),
            window: LazyInit::new(), surface: LazyInit::new(), render_res: LazyInit::new(),
            rts: Discardable::new(), commands: Discardable::new()
        });
        let nw = NativeWindowBuilder::new(640, 360, "Ferrite integration").transparent(true)
            .create_renderable(srv, &w).unwrap();
        w.window.init(nw);
        return w;
    }
}
impl WindowEventDelegate for MainWindow {
    type ClientDelegate = App;

    fn init_view(&self, view: &NativeView<Self>) {
        let server = self.app.upgrade().unwrap();

        if !server.presentation_support(&self.ferrite.adapter, self.ferrite.gq) {
            panic!("Vulkan Rendering is not supported by platform");
        }
        let surface = server.create_surface(view, &self.ferrite.instance).unwrap();
        if !self.ferrite.adapter.surface_support(self.ferrite.gq, &surface).unwrap() {
            panic!("Vulkan Rendering is not supported to this surface");
        }
        let s = DescribedSurface::from(&self.ferrite.adapter, surface).unwrap();
        let rr = RenderResources::new(&self.ferrite.device, &self.comres, &s).unwrap();
        let wrt = WindowRenderTargets::new(&self.ferrite, &rr, &s).unwrap().expect("Initially hidden surface");
        let rc = RenderCommands::populate(&self.ferrite, &self.comres, &wrt, &rr).unwrap();
        self.commands.set(rc); self.rts.set(wrt); self.render_res.init(rr); self.surface.init(s);
    }
    fn resize(&self, _w: u32, _h: u32, is_in_live_resize: bool) {
        if !is_in_live_resize {
            self.commands.discard(); self.rts.discard();
            let rts = WindowRenderTargets::new(&self.ferrite, &self.render_res.get(), &self.surface.get()).unwrap();
            if let Some(r) = rts { self.rts.set(r); } else { return; }
            self.commands.set(RenderCommands::populate(&self.ferrite, &self.comres, &self.rts.get(), &self.render_res.get()).unwrap());
            if self.window.is_presented() { self.window.get().mark_dirty(); }
        }
    }
    fn render(&self) {
        if self.rts.is_discarded() {
            let rts = WindowRenderTargets::new(&self.ferrite, &self.render_res.get(), &self.surface.get()).unwrap();
            if let Some(r) = rts { self.rts.set(r); } else { return; }
        }
        if self.commands.is_discarded() {
            self.commands.set(RenderCommands::populate(&self.ferrite, &self.comres, &self.rts.get(), &self.render_res.get()).unwrap());
        }

        match self.commit_frame() {
            Err(fe::VkResultBox(fe::vk::VK_ERROR_OUT_OF_DATE_KHR)) => {
                // Require to recreate resources(discarding resources)
                self.ferrite.fence_command_completion.wait().unwrap();
                self.ferrite.fence_command_completion.reset().unwrap();
                self.commands.discard(); self.rts.discard();

                // reissue rendering
                self.render();
            },
            r => r.expect("Committing a frame")
        }
    }
}
impl MainWindow {
    fn commit_frame(&self) -> fe::Result<()>
    {
        let (wrt, commands) = (self.rts.get(), self.commands.get());

        let next = wrt.swapchain.acquire_next(None, fe::CompletionHandler::Device(&self.ferrite.semaphore_sync_next))?
            as usize;
        self.ferrite.queue.submit(&[fe::SubmissionBatch
        {
            command_buffers: Cow::Borrowed(&commands.0[next..next+1]),
            wait_semaphores: Cow::Borrowed(&[(&self.ferrite.semaphore_sync_next, fe::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)]),
            signal_semaphores: Cow::Borrowed(&[&self.ferrite.semaphore_command_completion])
        }], Some(&self.ferrite.fence_command_completion))?;
        self.ferrite.queue.present(&[(&wrt.swapchain, next as _)], &[&self.ferrite.semaphore_command_completion])?;
        // コマンドバッファの使用が終了したことを明示する
        self.ferrite.fence_command_completion.wait()?; self.ferrite.fence_command_completion.reset()?;
        return Ok(());
    }
}
struct WindowRenderTargets
{
    framebuffers: Vec<fe::Framebuffer>, backbuffers: Vec<fe::ImageView>,
    swapchain: fe::Swapchain, size: fe::Extent2D
}
impl WindowRenderTargets {
    fn new(f: &Ferrite, res: &RenderResources, surface: &DescribedSurface) -> fe::Result<Option<Self>> {
        let surface_caps = f.adapter.surface_capabilities(&surface.object)?;
        let surface_size = match surface_caps.currentExtent {
            fe::vk::VkExtent2D { width: 0xffff_ffff, height: 0xffff_ffff } => fe::Extent2D(640, 360),
            fe::vk::VkExtent2D { width, height } => fe::Extent2D(width, height)
        };
        if surface_size.0 <= 0 || surface_size.1 <= 0 { return Ok(None); }

        let swapchain = fe::SwapchainBuilder::new(&surface.object, surface.buffer_count, &surface.format,
            &surface_size, fe::ImageUsage::COLOR_ATTACHMENT)
                .present_mode(surface.present_mode).pre_transform(fe::SurfaceTransform::Identity)
                .composite_alpha(surface.composite_mode).create(&f.device)?;
        // acquire_nextより前にやらないと死ぬ(get_images)
        let backbuffers = swapchain.get_images()?;
        let isr = fe::ImageSubresourceRange::color(0, 0);
        let (mut bb_views, mut framebuffers) = (Vec::with_capacity(backbuffers.len()), Vec::with_capacity(backbuffers.len()));
        for bb in &backbuffers {
            let view = bb.create_view(None, None, &fe::ComponentMapping::default(), &isr)?;
            framebuffers.push(fe::Framebuffer::new(&res.rp, &[&view], &surface_size, 1)?);
            bb_views.push(view);
        }

        let fw = fe::Fence::new(&f.device, false).unwrap();
        let init_commands = f.cmdpool.alloc(1, true).unwrap();
        init_commands[0].begin().unwrap()
            .pipeline_barrier(fe::PipelineStageFlags::TOP_OF_PIPE, fe::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                true, &[], &[], &bb_views.iter().map(|iv| fe::ImageMemoryBarrier::new(&fe::ImageSubref::color(&iv, 0, 0),
                    fe::ImageLayout::Undefined, fe::ImageLayout::PresentSrc)).collect::<Vec<_>>());
        f.queue.submit(&[fe::SubmissionBatch
        {
            command_buffers: Cow::Borrowed(&init_commands), .. Default::default()
        }], Some(&fw)).unwrap(); fw.wait().unwrap();
        
        Ok(Some(WindowRenderTargets
        {
            swapchain, backbuffers: bb_views, framebuffers, size: surface_size
        }))
    }
}
struct RenderResources { rp: fe::RenderPass, gp: fe::Pipeline }
impl RenderResources {
    fn new(dev: &fe::Device, res: &Resources, surface: &DescribedSurface) -> fe::Result<Self> {
        let rp = fe::RenderPassBuilder::new()
            .add_attachment(fe::AttachmentDescription::new(surface.format.format, fe::ImageLayout::PresentSrc, fe::ImageLayout::PresentSrc)
                .load_op(fe::LoadOp::Clear).store_op(fe::StoreOp::Store))
            .add_subpass(fe::SubpassDescription::new().add_color_output(0, fe::ImageLayout::ColorAttachmentOpt, None))
            .add_dependency(fe::vk::VkSubpassDependency {
                srcSubpass: fe::vk::VK_SUBPASS_EXTERNAL, dstSubpass: 0,
                srcAccessMask: 0, dstAccessMask: fe::AccessFlags::COLOR_ATTACHMENT.write,
                srcStageMask: fe::PipelineStageFlags::TOP_OF_PIPE.0, dstStageMask: fe::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT.0,
                dependencyFlags: fe::vk::VK_DEPENDENCY_BY_REGION_BIT
            }).create(dev)?;
        
        let gp;
        {
            let mut gpb = fe::GraphicsPipelineBuilder::new(&res.pl, (&rp, 0));
            let mut vps = fe::VertexProcessingStages::new(fe::PipelineShader::new(&res.shaders.v_pass, "main", None),
                VBIND, VATTRS, fe::vk::VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST);
            vps.fragment_shader(fe::PipelineShader::new(&res.shaders.f_phong, "main", None));
            gp = gpb.vertex_processing(vps)
                .fixed_viewport_scissors(fe::DynamicArrayState::Dynamic(1), fe::DynamicArrayState::Dynamic(1))
                .add_attachment_blend(fe::AttachmentColorBlendState::noblend()).create(dev, None)?;
        }
        
        return Ok(RenderResources { rp, gp })
    }
}
pub struct RenderCommands(Vec<fe::CommandBuffer>);
impl RenderCommands {
    fn populate(f: &Ferrite, cr: &Resources, rt: &WindowRenderTargets, res: &RenderResources) -> fe::Result<Self>
    {
        let vp = fe::vk::VkViewport
        {
            x: 0.0, y: 0.0, width: rt.size.0 as _, height: rt.size.1 as _, minDepth: 0.0, maxDepth: 1.0
        };
        let scis = fe::vk::VkRect2D
        {
            offset: fe::vk::VkOffset2D { x: 0, y: 0 },
            extent: fe::vk::VkExtent2D { width: vp.width as _, height: vp.height as _ }
        };

        let render_commands = f.cmdpool.alloc(rt.framebuffers.len() as _, true)?;
        for (c, fb) in render_commands.iter().zip(&rt.framebuffers)
        {
            c.begin()?
                .begin_render_pass(&res.rp, fb, fe::vk::VkRect2D
                {
                    offset: fe::vk::VkOffset2D { x: 0, y: 0 },
                    extent: fe::vk::VkExtent2D { width: rt.size.0, height: rt.size.1 }
                }, &[fe::ClearValue::Color([0.0, 0.0, 0.0, 0.5])], true)
                    .bind_graphics_pipeline_pair(&res.gp, &cr.pl)
                    .set_viewport(0, &[vp.clone()]).set_scissor(0, &[scis.clone()])
                    .bind_vertex_buffers(0, &[(&cr.buf, 0)]).draw(3, 1, 0, 0)
                .end_render_pass();
        }
        return Ok(RenderCommands(render_commands));
    }
}

fn main() { std::process::exit(GUIApplication::run(App::new())); }
