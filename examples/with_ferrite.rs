// #![brature(link_args)]
// #![link_args = "-rpath /Users/pctgx8/Downloads/vulkansdk-macos-1.0.69.0/macOS/Frameworks"]

extern crate appframe;
extern crate bedrock;
extern crate libc;

use appframe::*;
use bedrock as br;
use br::traits::*;
use std::rc::{Rc, Weak};
use std::cell::{RefCell, Ref};
use std::borrow::Cow;

#[repr(C)] #[derive(Clone)] pub struct Vertex([f32; 4], [f32; 4]);

struct ShaderStore { v_pass: br::ShaderModule, f_phong: br::ShaderModule }
impl ShaderStore {
    fn load(device: &br::Device) -> Result<Self, Box<std::error::Error>> {
        Ok(ShaderStore
        {
            v_pass: br::ShaderModule::from_file(device, "shaders/pass.vso")?,
            f_phong: br::ShaderModule::from_file(device, "shaders/phong.fso")?
        })
    }
}
const VBIND: &[br::vk::VkVertexInputBindingDescription] = &[
    br::vk::VkVertexInputBindingDescription {
        binding: 0, stride: 16 * 2, inputRate: br::vk::VK_VERTEX_INPUT_RATE_VERTEX
    }
];
const VATTRS: &[br::vk::VkVertexInputAttributeDescription] = &[
    br::vk::VkVertexInputAttributeDescription {
        binding: 0, location: 0, offset: 0, format: br::vk::VK_FORMAT_R32G32B32A32_SFLOAT
    },
    br::vk::VkVertexInputAttributeDescription {
        binding: 0, location: 1, offset: 16, format: br::vk::VK_FORMAT_R32G32B32A32_SFLOAT
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
    gq: u32, _tq: u32, queue: br::Queue, tqueue: br::Queue, cmdpool: br::CommandPool, tcmdpool: br::CommandPool,
    semaphore_sync_next: br::Semaphore, semaphore_command_completion: br::Semaphore,
    brnce_command_completion: br::Fence,
    device: br::Device, adapter: br::PhysicalDevice, _d: br::DebugReportCallback, instance: br::Instance
}
pub struct Resources {
    buf: br::Buffer, _dmem: br::DeviceMemory, pl: br::PipelineLayout, shaders: ShaderStore
}
struct App { main_wnd: LazyInit<Rc<MainWindow>>, res: LazyInit<Rc<Resources>>, brrrite: LazyInit<Rc<Ferrite>> }
impl App {
    fn new() -> Self {
        App { main_wnd: LazyInit::new(), brrrite: LazyInit::new(), res: LazyInit::new() }
    }
}
impl EventDelegate for App {
    fn postinit(&self, server: &Rc<GUIApplication<Self>>) {
        extern "system" fn dbg_cb(_flags: br::vk::VkDebugReportFlagsEXT, _object_type: br::vk::VkDebugReportObjectTypeEXT,
            _object: u64, _location: libc::size_t, _message_code: i32, _layer_prefix: *const libc::c_char,
            message: *const libc::c_char, _user_data: *mut libc::c_void) -> br::vk::VkBool32
        {
            println!("dbg_cb: {}", unsafe { std::ffi::CStr::from_ptr(message).to_str().unwrap() });
            false as _
        }

        #[cfg(target_os = "macos")] const PLATFORM_SURFACE: &str = "VK_MVK_macos_surface";
        #[cfg(windows)] const PLATFORM_SURFACE: &str = "VK_KHR_win32_surface";
        #[cfg(brature = "with_xcb")] const PLATFORM_SURFACE: &str = "VK_KHR_xcb_surface";
        let instance = br::InstanceBuilder::new("appframe_integ", (0, 1, 0), "Ferrite", (0, 1, 0))
            .add_extensions(vec!["VK_KHR_surface", PLATFORM_SURFACE, "VK_EXT_debug_report"])
            .add_layer("VK_LAYER_LUNARG_standard_validation")
            .create().unwrap();
        let d = br::DebugReportCallbackBuilder::new(&instance, dbg_cb).report_error().report_warning()
            .report_performance_warning().create().unwrap();
        let adapter = instance.iter_physical_devices().unwrap().next().unwrap();
        println!("Vulkan AdapterName: {}", unsafe { std::ffi::CStr::from_ptr(adapter.properties().deviceName.as_ptr()).to_str().unwrap() });
        let memindices = adapter.memory_properties();
        let qfp = adapter.queue_family_properties();
        let gq = qfp.find_matching_index(br::QueueFlags::GRAPHICS).expect("Cannot find a graphics queue");
        let tq = qfp.find_another_matching_index(br::QueueFlags::TRANSFER, gq)
            .or_else(|| qfp.find_matching_index(br::QueueFlags::TRANSFER)).expect("No transferrable queue family found");
        let united_queue = gq == tq;
        let qs = if united_queue { vec![br::DeviceQueueCreateInfo(gq, vec![0.0; 2])] }
            else { vec![br::DeviceQueueCreateInfo(gq, vec![0.0]), br::DeviceQueueCreateInfo(tq, vec![0.0])] };
        let device = br::DeviceBuilder::new(&adapter)
            .add_extensions(vec!["VK_KHR_swapchain"]).add_queues(qs)
            .create().unwrap();
        let queue = device.queue(gq, 0);
        let cmdpool = br::CommandPool::new(&device, gq, false, false).unwrap();
        let device_memindex = memindices.find_device_local_index().unwrap();
        let upload_memindex = memindices.find_host_visible_index().unwrap();
        
        let bufsize = std::mem::size_of::<Vertex>() * 3;
        let buf = br::BufferDesc::new(bufsize, br::BufferUsage::VERTEX_BUFFER.transfer_dest()).create(&device).unwrap();
        let memreq = buf.requirements();
        let dmem = br::DeviceMemory::allocate(&device, memreq.size as _, device_memindex).unwrap();
        {
            let upload_buf = br::BufferDesc::new(bufsize, br::BufferUsage::VERTEX_BUFFER.transfer_src()).create(&device)
                .unwrap();
            let upload_memreq = upload_buf.requirements();
            let upload_mem = br::DeviceMemory::allocate(&device, upload_memreq.size as _, upload_memindex).unwrap();
            buf.bind(&dmem, 0).unwrap(); upload_buf.bind(&upload_mem, 0).unwrap();
            unsafe 
            {
                upload_mem.map(0 .. bufsize).unwrap().slice_mut(0, 3).clone_from_slice(&[
                    Vertex([0.0, -1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
                    Vertex([-1.0, 1.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]),
                    Vertex([1.0, 1.0, 0.0, 1.0], [0.5, 0.0, 1.0, 0.0])
                ]);
            }
            let fw = br::Fence::new(&device, false).unwrap();
            let init_commands = cmdpool.alloc(1, true).unwrap();
            init_commands[0].begin().unwrap()
                .pipeline_barrier(br::PipelineStageFlags::TOP_OF_PIPE, br::PipelineStageFlags::TRANSFER, true,
                    &[], &[
                        br::BufferMemoryBarrier::new(&buf, 0 .. bufsize, 0, br::AccessFlags::TRANSFER.write),
                        br::BufferMemoryBarrier::new(&upload_buf, 0 .. bufsize, 0, br::AccessFlags::TRANSFER.read)
                    ], &[])
                .copy_buffer(&upload_buf, &buf, &[br::vk::VkBufferCopy { srcOffset: 0, dstOffset: 0, size: bufsize as _ }])
                .pipeline_barrier(br::PipelineStageFlags::TRANSFER, br::PipelineStageFlags::VERTEX_INPUT, true,
                    &[], &[br::BufferMemoryBarrier::new(&buf, 0 .. bufsize, br::AccessFlags::TRANSFER.write,
                        br::AccessFlags::VERTEX_ATTRIBUTE_READ)], &[]);
            queue.submit(&[br::SubmissionBatch
            {
                command_buffers: Cow::Borrowed(&init_commands), .. Default::default()
            }], Some(&fw)).unwrap(); fw.wait().unwrap();
        }
        self.res.init(Rc::new(Resources {
            shaders: ShaderStore::load(&device).unwrap(),
            pl: br::PipelineLayout::new(&device, &[], &[]).unwrap(), buf, _dmem: dmem
        }));
        self.brrrite.init(Rc::new(Ferrite {
            brnce_command_completion: br::Fence::new(&device, false).unwrap(),
            semaphore_sync_next: br::Semaphore::new(&device).unwrap(),
            semaphore_command_completion: br::Semaphore::new(&device).unwrap(),
            tcmdpool: br::CommandPool::new(&device, tq, false, false).unwrap(),
            cmdpool, queue, tqueue: device.queue(tq, if united_queue { 1 } else { 0 }),
            device, adapter, instance, gq, _tq: tq, _d: d
        }));

        let w = MainWindow::new(server); w.window.get().show();
        self.main_wnd.init(w);
    }
}

pub struct DescribedSurface {
    object: br::Surface, format: br::vk::VkSurfaceFormatKHR, present_mode: br::PresentMode,
    composite_mode: br::CompositeAlpha, buffer_count: u32
}
impl DescribedSurface {
    pub fn from(adapter: &br::PhysicalDevice, s: br::Surface) -> br::Result<Self> {
        let caps = adapter.surface_capabilities(&s)?;
        let mut fmtq = br::FormatQueryPred::new();
        fmtq.bit(32).components(br::FormatComponents::RGBA).elements(br::ElementType::UNORM);
        let format = adapter.surface_formats(&s)?.into_iter().find(|f| fmtq.satisfy(f.format)).unwrap();
        let present_mode = adapter.surface_present_modes(&s)?.remove(0);
        let composite_mode = if (caps.supportedCompositeAlpha & br::CompositeAlpha::PostMultiplied as u32) != 0
        {
            br::CompositeAlpha::PostMultiplied
        }
        else { br::CompositeAlpha::Opaque };
        let buffer_count = caps.minImageCount.max(2);

        return Ok(DescribedSurface { object: s, format, present_mode, composite_mode, buffer_count })
    }
}

struct MainWindow {
    commands: Discardable<RenderCommands>, rts: Discardable<WindowRenderTargets>,
    render_res: LazyInit<RenderResources>, surface: LazyInit<DescribedSurface>,
    window: LazyInit<NativeWindow<MainWindow>>,
    brrrite: Rc<Ferrite>, comres: Rc<Resources>, app: Weak<GUIApplication<App>>
}
impl MainWindow {
    pub fn new(srv: &Rc<GUIApplication<App>>) -> Rc<Self> {
        let w = Rc::new(MainWindow {
            app: Rc::downgrade(srv), brrrite: srv.event_delegate().brrrite.get().clone(),
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

        if !server.presentation_support(&self.brrrite.adapter, self.brrrite.gq) {
            panic!("Vulkan Rendering is not supported by platform");
        }
        let surface = server.create_surface(view, &self.brrrite.instance).unwrap();
        if !self.brrrite.adapter.surface_support(self.brrrite.gq, &surface).unwrap() {
            panic!("Vulkan Rendering is not supported to this surface");
        }
        let s = DescribedSurface::from(&self.brrrite.adapter, surface).unwrap();
        let rr = RenderResources::new(&self.brrrite.device, &self.comres, &s).unwrap();
        let wrt = WindowRenderTargets::new(&self.brrrite, &rr, &s).unwrap().expect("Initially hidden surface");
        let rc = RenderCommands::populate(&self.brrrite, &self.comres, &wrt, &rr).unwrap();
        self.commands.set(rc); self.rts.set(wrt); self.render_res.init(rr); self.surface.init(s);
    }
    fn resize(&self, _w: u32, _h: u32, is_in_live_resize: bool) {
        if !is_in_live_resize {
            self.commands.discard(); self.rts.discard();
            let rts = WindowRenderTargets::new(&self.brrrite, &self.render_res.get(), &self.surface.get()).unwrap();
            if let Some(r) = rts { self.rts.set(r); } else { return; }
            self.commands.set(RenderCommands::populate(&self.brrrite, &self.comres, &self.rts.get(), &self.render_res.get()).unwrap());
            if self.window.is_presented() { self.window.get().mark_dirty(); }
        }
    }
    fn render(&self) {
        if self.rts.is_discarded() {
            let rts = WindowRenderTargets::new(&self.brrrite, &self.render_res.get(), &self.surface.get()).unwrap();
            if let Some(r) = rts { self.rts.set(r); } else { return; }
        }
        if self.commands.is_discarded() {
            self.commands.set(RenderCommands::populate(&self.brrrite, &self.comres, &self.rts.get(), &self.render_res.get()).unwrap());
        }

        match self.commit_frame() {
            Err(br::VkResultBox(br::vk::VK_ERROR_OUT_OF_DATE_KHR)) => {
                // Require to recreate resources(discarding resources)
                self.brrrite.brnce_command_completion.wait().unwrap();
                self.brrrite.brnce_command_completion.reset().unwrap();
                self.commands.discard(); self.rts.discard();

                // reissue rendering
                self.render();
            },
            r => r.expect("Committing a frame")
        }
    }
}
impl MainWindow {
    fn commit_frame(&self) -> br::Result<()>
    {
        let (wrt, commands) = (self.rts.get(), self.commands.get());

        let next = wrt.swapchain.acquire_next(None, br::CompletionHandler::Device(&self.brrrite.semaphore_sync_next))?
            as usize;
        self.brrrite.queue.submit(&[br::SubmissionBatch
        {
            command_buffers: Cow::Borrowed(&commands.0[next..next+1]),
            wait_semaphores: Cow::Borrowed(&[(&self.brrrite.semaphore_sync_next, br::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)]),
            signal_semaphores: Cow::Borrowed(&[&self.brrrite.semaphore_command_completion])
        }], Some(&self.brrrite.brnce_command_completion))?;
        self.brrrite.queue.present(&[(&wrt.swapchain, next as _)], &[&self.brrrite.semaphore_command_completion])?;
        // コマンドバッファの使用が終了したことを明示する
        self.brrrite.brnce_command_completion.wait()?; self.brrrite.brnce_command_completion.reset()?;
        return Ok(());
    }
}
struct WindowRenderTargets
{
    framebuffers: Vec<br::Framebuffer>, backbuffers: Vec<br::ImageView>,
    swapchain: br::Swapchain, size: br::Extent2D
}
impl WindowRenderTargets {
    fn new(f: &Ferrite, res: &RenderResources, surface: &DescribedSurface) -> br::Result<Option<Self>> {
        let surface_caps = f.adapter.surface_capabilities(&surface.object)?;
        let surface_size = match surface_caps.currentExtent {
            br::vk::VkExtent2D { width: 0xffff_ffff, height: 0xffff_ffff } => br::Extent2D(640, 360),
            br::vk::VkExtent2D { width, height } => br::Extent2D(width, height)
        };
        if surface_size.0 <= 0 || surface_size.1 <= 0 { return Ok(None); }

        let swapchain = br::SwapchainBuilder::new(&surface.object, surface.buffer_count, &surface.format,
            &surface_size, br::ImageUsage::COLOR_ATTACHMENT)
                .present_mode(surface.present_mode).pre_transform(br::SurfaceTransform::Identity)
                .composite_alpha(surface.composite_mode).create(&f.device)?;
        // acquire_nextより前にやらないと死ぬ(get_images)
        let backbuffers = swapchain.get_images()?;
        let isr = br::ImageSubresourceRange::color(0, 0);
        let (mut bb_views, mut framebuffers) = (Vec::with_capacity(backbuffers.len()), Vec::with_capacity(backbuffers.len()));
        for bb in &backbuffers {
            let view = bb.create_view(None, None, &br::ComponentMapping::default(), &isr)?;
            framebuffers.push(br::Framebuffer::new(&res.rp, &[&view], &surface_size, 1)?);
            bb_views.push(view);
        }

        let fw = br::Fence::new(&f.device, false).unwrap();
        let init_commands = f.cmdpool.alloc(1, true).unwrap();
        init_commands[0].begin().unwrap()
            .pipeline_barrier(br::PipelineStageFlags::TOP_OF_PIPE, br::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                true, &[], &[], &bb_views.iter().map(|iv| br::ImageMemoryBarrier::new(&br::ImageSubref::color(&iv, 0, 0),
                    br::ImageLayout::Undefined, br::ImageLayout::PresentSrc)).collect::<Vec<_>>());
        f.queue.submit(&[br::SubmissionBatch
        {
            command_buffers: Cow::Borrowed(&init_commands), .. Default::default()
        }], Some(&fw)).unwrap(); fw.wait().unwrap();
        
        Ok(Some(WindowRenderTargets
        {
            swapchain, backbuffers: bb_views, framebuffers, size: surface_size
        }))
    }
}
struct RenderResources { rp: br::RenderPass, gp: br::Pipeline }
impl RenderResources {
    fn new(dev: &br::Device, res: &Resources, surface: &DescribedSurface) -> br::Result<Self> {
        let rp = br::RenderPassBuilder::new()
            .add_attachment(br::AttachmentDescription::new(surface.format.format, br::ImageLayout::PresentSrc, br::ImageLayout::PresentSrc)
                .load_op(br::LoadOp::Clear).store_op(br::StoreOp::Store))
            .add_subpass(br::SubpassDescription::new().add_color_output(0, br::ImageLayout::ColorAttachmentOpt, None))
            .add_dependency(br::vk::VkSubpassDependency {
                srcSubpass: br::vk::VK_SUBPASS_EXTERNAL, dstSubpass: 0,
                srcAccessMask: 0, dstAccessMask: br::AccessFlags::COLOR_ATTACHMENT.write,
                srcStageMask: br::PipelineStageFlags::TOP_OF_PIPE.0, dstStageMask: br::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT.0,
                dependencyFlags: br::vk::VK_DEPENDENCY_BY_REGION_BIT
            }).create(dev)?;
        
        let gp;
        {
            let mut gpb = br::GraphicsPipelineBuilder::new(&res.pl, (&rp, 0));
            let mut vps = br::VertexProcessingStages::new(br::PipelineShader::new(&res.shaders.v_pass, "main", None),
                VBIND, VATTRS, br::vk::VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST);
            vps.fragment_shader(br::PipelineShader::new(&res.shaders.f_phong, "main", None));
            gp = gpb.vertex_processing(vps)
                .fixed_viewport_scissors(br::DynamicArrayState::Dynamic(1), br::DynamicArrayState::Dynamic(1))
                .add_attachment_blend(br::AttachmentColorBlendState::noblend()).create(dev, None)?;
        }
        
        return Ok(RenderResources { rp, gp })
    }
}
pub struct RenderCommands(Vec<br::CommandBuffer>);
impl RenderCommands {
    fn populate(f: &Ferrite, cr: &Resources, rt: &WindowRenderTargets, res: &RenderResources) -> br::Result<Self>
    {
        let vp = br::vk::VkViewport
        {
            x: 0.0, y: 0.0, width: rt.size.0 as _, height: rt.size.1 as _, minDepth: 0.0, maxDepth: 1.0
        };
        let scis = br::vk::VkRect2D
        {
            offset: br::vk::VkOffset2D { x: 0, y: 0 },
            extent: br::vk::VkExtent2D { width: vp.width as _, height: vp.height as _ }
        };

        let render_commands = f.cmdpool.alloc(rt.framebuffers.len() as _, true)?;
        for (c, fb) in render_commands.iter().zip(&rt.framebuffers)
        {
            c.begin()?
                .begin_render_pass(&res.rp, fb, br::vk::VkRect2D
                {
                    offset: br::vk::VkOffset2D { x: 0, y: 0 },
                    extent: br::vk::VkExtent2D { width: rt.size.0, height: rt.size.1 }
                }, &[br::ClearValue::Color([0.0, 0.0, 0.0, 0.5])], true)
                    .bind_graphics_pipeline_pair(&res.gp, &cr.pl)
                    .set_viewport(0, &[vp.clone()]).set_scissor(0, &[scis.clone()])
                    .bind_vertex_buffers(0, &[(&cr.buf, 0)]).draw(3, 1, 0, 0)
                .end_render_pass();
        }
        return Ok(RenderCommands(render_commands));
    }
}

fn main() { std::process::exit(GUIApplication::run(App::new())); }
