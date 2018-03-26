// #![feature(link_args)]
// #![link_args = "-rpath /Users/pctgx8/Downloads/vulkansdk-macos-1.0.69.0/macOS/Frameworks"]

extern crate appframe;
extern crate ferrite;
extern crate libc;

use appframe::*;
use ferrite as fe;
use fe::traits::*;
use std::rc::Rc;
use std::cell::{RefCell, Ref};
use std::borrow::Cow;

#[repr(C)] #[derive(Clone)] pub struct Vertex([f32; 4], [f32; 4]);

struct ShaderStore
{
    v_pass: fe::ShaderModule, f_phong: fe::ShaderModule
}
impl ShaderStore
{
    fn load(device: &fe::Device) -> Result<Self, Box<std::error::Error>>
    {
        Ok(ShaderStore
        {
            v_pass: fe::ShaderModule::from_file(device, "shaders/pass.vso")?,
            f_phong: fe::ShaderModule::from_file(device, "shaders/phong.fso")?
        })
    }
}
const VBIND: &[fe::vk::VkVertexInputBindingDescription] = &[
    fe::vk::VkVertexInputBindingDescription
    {
        binding: 0, stride: 16 * 2, inputRate: fe::vk::VK_VERTEX_INPUT_RATE_VERTEX
    }
];
const VATTRS: &[fe::vk::VkVertexInputAttributeDescription] = &[
    fe::vk::VkVertexInputAttributeDescription
    {
        binding: 0, location: 0, offset: 0, format: fe::vk::VK_FORMAT_R32G32B32A32_SFLOAT
    },
    fe::vk::VkVertexInputAttributeDescription
    {
        binding: 0, location: 1, offset: 16, format: fe::vk::VK_FORMAT_R32G32B32A32_SFLOAT
    }
];

struct App
{
    rcmds: RefCell<Option<RenderCommands>>,
    rtdres: RefCell<Option<RenderTargetDependentResources>>,
    rendertargets: RefCell<Option<WindowRenderTargets>>,
    surface: RefCell<Option<fe::Surface>>,
    res: RefCell<Option<Resources>>,
    ferrite: RefCell<Option<Ferrite>>,
    w: RefCell<Option<NativeWindow<App>>>
}
pub struct Ferrite
{
    gq: u32, _tq: u32, queue: fe::Queue, tqueue: fe::Queue, cmdpool: fe::CommandPool, tcmdpool: fe::CommandPool,
    semaphore_sync_next: fe::Semaphore, semaphore_command_completion: fe::Semaphore,
    fence_command_completion: fe::Fence,
    device: fe::Device, adapter: fe::PhysicalDevice, _d: fe::DebugReportCallback, instance: fe::Instance
}
pub struct Resources
{
    buf: fe::Buffer, _dmem: fe::DeviceMemory, pl: fe::PipelineLayout, shaders: ShaderStore
}
pub struct RenderCommands(Vec<fe::CommandBuffer>);
impl App
{
    fn new() -> Self
    {
        App
        {
            w: RefCell::new(None), ferrite: RefCell::new(None),
            rcmds: RefCell::new(None), surface: RefCell::new(None), rendertargets: RefCell::new(None),
            res: RefCell::new(None), rtdres: RefCell::new(None)
        }
    }
}
impl EventDelegate for App
{
    fn postinit(&self, server: &Rc<GUIApplication<Self>>)
    {
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
        *self.res.borrow_mut() = Some(Resources
        {
            shaders: ShaderStore::load(&device).unwrap(),
            pl: fe::PipelineLayout::new(&device, &[], &[]).unwrap(), buf, _dmem: dmem
        });

        *self.ferrite.borrow_mut() = Some(Ferrite
        {
            fence_command_completion: fe::Fence::new(&device, false).unwrap(),
            semaphore_sync_next: fe::Semaphore::new(&device).unwrap(),
            semaphore_command_completion: fe::Semaphore::new(&device).unwrap(),
            tcmdpool: fe::CommandPool::new(&device, tq, false, false).unwrap(),
            cmdpool, queue, tqueue: device.queue(tq, if united_queue { 1 } else { 0 }),
            device, adapter, instance, gq, _tq: tq, _d: d
        });

        let w = NativeWindowBuilder::new(640, 360, "Ferrite integration").transparent(true)
            .create_renderable(server).unwrap();
        *self.w.borrow_mut() = Some(w);
        self.w.borrow().as_ref().unwrap().show();
    }
    fn on_init_view(&self, server: &GUIApplication<Self>, surface_onto: &NativeView<Self>)
    {
        let fr = self.ferrite.borrow(); let f = fr.as_ref().unwrap();

        if !server.presentation_support(&f.adapter, f.gq) { panic!("Vulkan Rendering is not supported by platform"); }
        let surface = server.create_surface(surface_onto, &f.instance).unwrap();
        if !f.adapter.surface_support(f.gq, &surface).unwrap() { panic!("Vulkan Rendering is not supported to this surface"); }
        *self.surface.borrow_mut() = Some(surface);
        let rtvs = self.init_swapchains().unwrap().unwrap();
        *self.rtdres.borrow_mut() = Some(RenderTargetDependentResources::new(&f.device,
            self.res.borrow().as_ref().unwrap(), &rtvs).unwrap());
        *self.rendertargets.borrow_mut() = Some(rtvs);
        *self.rcmds.borrow_mut() = Some(self.populate_render_commands().unwrap());
    }
    fn on_render_period(&self)
    {
        if self.ensure_render_targets().unwrap()
        {
            if let Err(e) = self.render()
            {
                if e.0 == fe::vk::VK_ERROR_OUT_OF_DATE_KHR
                {
                    // Require to recreate resources(discarding resources)
                    let fr = self.ferrite.borrow(); let f = fr.as_ref().unwrap();

                    f.fence_command_completion.wait().unwrap(); f.fence_command_completion.reset().unwrap();
                    *self.rcmds.borrow_mut() = None;
                    *self.rtdres.borrow_mut() = None;
                    *self.rendertargets.borrow_mut() = None;

                    // reissue rendering
                    self.on_render_period();
                }
                else { let e: fe::Result<()> = Err(e); e.unwrap(); }
            }
        }
    }
}
impl App
{
    fn ensure_render_targets(&self) -> fe::Result<bool>
    {
        if self.rendertargets.borrow().is_none()
        {
            let rtv = self.init_swapchains()?;
            if rtv.is_none() { return Ok(false); }
            *self.rendertargets.borrow_mut() = rtv;
        }
        if self.rtdres.borrow().is_none()
        {
            let fr = self.ferrite.borrow(); let f = fr.as_ref().unwrap();
            let resr = self.res.borrow(); let res = resr.as_ref().unwrap();
            *self.rtdres.borrow_mut() = Some(RenderTargetDependentResources::new(&f.device, res,
                self.rendertargets.borrow().as_ref().unwrap())?);
        }
        if self.rcmds.borrow().is_none()
        {
            *self.rcmds.borrow_mut() = Some(self.populate_render_commands().unwrap());
        }
        Ok(true)
    }
    fn init_swapchains(&self) -> fe::Result<Option<WindowRenderTargets>>
    {
        let fr = self.ferrite.borrow(); let f = fr.as_ref().unwrap();
        let sr = self.surface.borrow(); let s = sr.as_ref().unwrap();

        let surface_caps = f.adapter.surface_capabilities(s)?;
        let surface_format = f.adapter.surface_formats(s)?.into_iter()
            .find(|f| fe::FormatQuery(f.format).eq_bit_width(32).is_component_of(fe::FormatComponents::RGBA).has_element_of(fe::ElementType::UNORM).passed()).unwrap();
        let surface_pm = f.adapter.surface_present_modes(s)?.remove(0);
        let surface_ca = if (surface_caps.supportedCompositeAlpha & fe::CompositeAlpha::PostMultiplied as u32) != 0
        {
            fe::CompositeAlpha::PostMultiplied
        }
        else { fe::CompositeAlpha::Opaque };
        let surface_size = match surface_caps.currentExtent
        {
            fe::vk::VkExtent2D { width: 0xffff_ffff, height: 0xffff_ffff } => fe::Extent2D(640, 360),
            fe::vk::VkExtent2D { width, height } => fe::Extent2D(width, height)
        };
        if surface_size.0 <= 0 || surface_size.1 <= 0 { return Ok(None); }
        let swapchain = fe::SwapchainBuilder::new(s, surface_caps.minImageCount.max(2),
            &surface_format, &surface_size, fe::ImageUsage::COLOR_ATTACHMENT)
                .present_mode(surface_pm).pre_transform(fe::SurfaceTransform::Identity)
                .composite_alpha(surface_ca).create(&f.device)?;
        // acquire_nextより前にやらないと死ぬ(get_images)
        let backbuffers = swapchain.get_images()?;
        let isr = fe::ImageSubresourceRange::color(0, 0);
        let bb_views = backbuffers.iter().map(|i| i.create_view(None, None, &fe::ComponentMapping::default(), &isr))
            .collect::<fe::Result<Vec<_>>>()?;

        let rp = fe::RenderPassBuilder::new()
            .add_attachment(fe::AttachmentDescription::new(surface_format.format, fe::ImageLayout::PresentSrc, fe::ImageLayout::PresentSrc)
                .load_op(fe::LoadOp::Clear).store_op(fe::StoreOp::Store))
            .add_subpass(fe::SubpassDescription::new().add_color_output(0, fe::ImageLayout::ColorAttachmentOpt, None))
            .add_dependency(fe::vk::VkSubpassDependency
            {
                srcSubpass: fe::vk::VK_SUBPASS_EXTERNAL, dstSubpass: 0,
                srcAccessMask: 0, dstAccessMask: fe::AccessFlags::COLOR_ATTACHMENT.write,
                srcStageMask: fe::PipelineStageFlags::TOP_OF_PIPE.0, dstStageMask: fe::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT.0,
                dependencyFlags: fe::vk::VK_DEPENDENCY_BY_REGION_BIT
            })
            .create(&f.device).unwrap();
        let framebuffers = bb_views.iter().map(|iv| fe::Framebuffer::new(&rp, &[iv], &surface_size, 1))
            .collect::<fe::Result<Vec<_>>>()?;

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
            swapchain, backbuffers: bb_views, framebuffers, renderpass: rp, size: surface_size
        }))
    }
    fn populate_render_commands(&self) -> fe::Result<RenderCommands>
    {
        let f = Ref::map(self.ferrite.borrow(), |f| f.as_ref().unwrap());
        let rtvs = Ref::map(self.rendertargets.borrow(), |r| r.as_ref().unwrap());
        let res = Ref::map(self.res.borrow(), |r| r.as_ref().unwrap());
        let rds = Ref::map(self.rtdres.borrow(), |r| r.as_ref().unwrap());

        let render_commands = f.cmdpool.alloc(rtvs.framebuffers.len() as _, true)?;
        for ((c, fb), iv) in render_commands.iter().zip(&rtvs.framebuffers).zip(&rtvs.backbuffers)
        {
            let subref = fe::ImageSubref::color(&iv, 0, 0);
            c.begin()?
                .begin_render_pass(&rtvs.renderpass, fb, fe::vk::VkRect2D
                {
                    offset: fe::vk::VkOffset2D { x: 0, y: 0 },
                    extent: fe::vk::VkExtent2D { width: rtvs.size.0, height: rtvs.size.1 }
                }, &[fe::ClearValue::Color([0.0, 0.0, 0.0, 0.5])], true)
                    .bind_graphics_pipeline_pair(&rds.gp, &res.pl)
                    .bind_vertex_buffers(0, &[(&res.buf, 0)]).draw(3, 1, 0, 0)
                .end_render_pass();
        }
        Ok(RenderCommands(render_commands))
    }
    fn render(&self) -> fe::Result<()>
    {
        let fr = self.ferrite.borrow(); let f = fr.as_ref().unwrap();
        let rtvr = self.rendertargets.borrow(); let rtvs = rtvr.as_ref().unwrap();
        let rcmdsr = self.rcmds.borrow(); let rcmds = rcmdsr.as_ref().unwrap();

        let next = rtvs.swapchain.acquire_next(None, fe::CompletionHandler::Device(&f.semaphore_sync_next))?
            as usize;
        f.queue.submit(&[fe::SubmissionBatch
        {
            command_buffers: Cow::Borrowed(&rcmds.0[next..next+1]),
            wait_semaphores: Cow::Borrowed(&[(&f.semaphore_sync_next, fe::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)]),
            signal_semaphores: Cow::Borrowed(&[&f.semaphore_command_completion])
        }], Some(&f.fence_command_completion))?;
        f.queue.present(&[(&rtvs.swapchain, next as _)], &[&f.semaphore_command_completion])?;
        // コマンドバッファの使用が終了したことを明示する
        f.fence_command_completion.wait()?; f.fence_command_completion.reset()?; Ok(())
    }
}
struct WindowRenderTargets
{
    framebuffers: Vec<fe::Framebuffer>, renderpass: fe::RenderPass, backbuffers: Vec<fe::ImageView>,
    swapchain: fe::Swapchain, size: fe::Extent2D
}
struct RenderTargetDependentResources
{
    gp: fe::Pipeline
}
impl RenderTargetDependentResources
{
    pub fn new(device: &fe::Device, res: &Resources, rtvs: &WindowRenderTargets) -> fe::Result<Self>
    {
        let vp = fe::vk::VkViewport
        {
            x: 0.0, y: 0.0, width: rtvs.size.0 as _, height: rtvs.size.1 as _, minDepth: 0.0, maxDepth: 1.0
        };
        let scis = fe::vk::VkRect2D
        {
            offset: fe::vk::VkOffset2D { x: 0, y: 0 },
            extent: fe::vk::VkExtent2D { width: vp.width as _, height: vp.height as _ }
        };
        let mut gpb = fe::GraphicsPipelineBuilder::new(&res.pl, (&rtvs.renderpass, 0));
        let mut vps = fe::VertexProcessingStages::new(fe::PipelineShader::new(&res.shaders.v_pass, "main", None),
            VBIND, VATTRS, fe::vk::VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST);
        vps.fragment_shader(fe::PipelineShader::new(&res.shaders.f_phong, "main", None));
        let gp = gpb.vertex_processing(vps)
            .fixed_viewport_scissors(fe::DynamicArrayState::Static(&[vp]), fe::DynamicArrayState::Static(&[scis]))
            .add_attachment_blend(fe::AttachmentColorBlendState::noblend()).create(device, None)?;
        
        Ok(RenderTargetDependentResources { gp })
    }
}

fn main() { std::process::exit(GUIApplication::run(App::new())); }
