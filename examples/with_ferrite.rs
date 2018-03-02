// #![feature(link_args)]
// #![link_args = "-rpath /Users/pctgx8/Downloads/vulkansdk-macos-1.0.69.0/macOS/Frameworks"]

extern crate appframe;
extern crate ferrite;

use appframe::*;
use ferrite as fe;
use fe::traits::*;
use std::rc::Rc;
use std::cell::{RefCell, Cell};
use std::borrow::Cow;

#[repr(C)] #[derive(Clone)] pub struct Vertex([f32; 4], [f32; 4]);

struct App
{
    renderlayer: RefCell<Option<RenderLayer>>, ferrite: RefCell<Option<Ferrite>>, w: RefCell<Option<NativeWindow<App>>>,
    dirty: Cell<bool>
}
pub struct Ferrite
{
    gq: u32, _tq: u32, queue: fe::Queue, tqueue: fe::Queue, cmdpool: fe::CommandPool, tcmdpool: fe::CommandPool,
    semaphore_sync_next: fe::Semaphore, semaphore_command_completion: fe::Semaphore,
    fence_command_completion: fe::Fence,
    device: fe::Device, adapter: fe::PhysicalDevice, instance: fe::Instance,
    device_memindex: u32, upload_memindex: u32,
}
pub struct RenderLayer
{
    _dmem: fe::DeviceMemory, _buf: fe::Buffer, _gp: fe::Pipeline, _pl: fe::PipelineLayout,
    render_commands: Vec<fe::CommandBuffer>,
    _framebuffers: Vec<fe::Framebuffer>, _renderpass: fe::RenderPass, _bb_views: Vec<fe::ImageView>,
    swapchain: fe::Swapchain, _surface: fe::Surface
}
impl App
{
    fn new() -> Self
    {
        App
        {
            w: RefCell::new(None), ferrite: RefCell::new(None), renderlayer: RefCell::new(None), dirty: Cell::new(false)
        }
    }
}
impl EventDelegate for App
{
    fn postinit(&self, server: &Rc<GUIApplication<Self>>)
    {
        #[cfg(target_os = "macos")] const PLATFORM_SURFACE: &str = "VK_MVK_macos_surface";
        #[cfg(windows)] const PLATFORM_SURFACE: &str = "VK_KHR_win32_surface";
        let instance = fe::InstanceBuilder::new("appframe_integ", (0, 1, 0), "Ferrite", (0, 1, 0))
            .add_extensions(vec!["VK_KHR_surface", PLATFORM_SURFACE, "VK_EXT_debug_report"])
            .add_layer("VK_LAYER_LUNARG_standard_validation")
            .create().unwrap();
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
        *self.ferrite.borrow_mut() = Some(Ferrite
        {
            device_memindex: memindices.find_device_local_index().unwrap(),
            upload_memindex: memindices.find_host_visible_index().unwrap(),
            fence_command_completion: fe::Fence::new(&device, false).unwrap(),
            semaphore_sync_next: fe::Semaphore::new(&device).unwrap(),
            semaphore_command_completion: fe::Semaphore::new(&device).unwrap(),
            tcmdpool: fe::CommandPool::new(&device, tq, false, false).unwrap(),
            cmdpool: fe::CommandPool::new(&device, gq, false, false).unwrap(),
            queue: device.queue(gq, 0), tqueue: device.queue(tq, if united_queue { 1 } else { 0 }),
            device, adapter, instance, gq, _tq: tq
        });

        let w = NativeWindowBuilder::new(640, 360, "Ferrite integration").create_renderable(server).unwrap();
        *self.w.borrow_mut() = Some(w);
        self.w.borrow().as_ref().unwrap().show();
    }
    fn on_init_view(&self, server: &GUIApplication<Self>, surface_onto: &<GUIApplication<Self> as FerriteRenderingServer>::SurfaceSource)
    {
        let fr = self.ferrite.borrow(); let f = fr.as_ref().unwrap();

        if !server.presentation_support(&f.adapter, f.gq) { panic!("Vulkan Rendering is not supported by platform"); }
        let surface = server.create_surface(surface_onto, &f.instance).unwrap();
        if !f.adapter.surface_support(f.gq, &surface).unwrap() { panic!("Vulkan Rendering is not supported to this surface"); }
        let surface_caps = f.adapter.surface_capabilities(&surface).unwrap();
        let surface_format = f.adapter.surface_formats(&surface).unwrap().into_iter()
            .find(|f| fe::FormatQuery(f.format).eq_bit_width(32).has_components(fe::FormatComponents::RGBA).has_element_of(fe::ElementType::UNORM).passed()).unwrap();
        let surface_pm = f.adapter.surface_present_modes(&surface).unwrap().remove(0);
        let surface_size = match surface_caps.currentExtent
        {
            fe::vk::VkExtent2D { width: 0xffff_ffff, height: 0xffff_ffff } => fe::Extent2D(640, 360),
            fe::vk::VkExtent2D { width, height } => fe::Extent2D(width, height)
        };
        let swapchain = fe::SwapchainBuilder::new(&surface, surface_caps.minImageCount.max(2),
            surface_format.clone(), surface_size.clone(), fe::ImageUsage::COLOR_ATTACHMENT)
                .present_mode(surface_pm).pre_transform(fe::SurfaceTransform::Identity)
                .composite_alpha(fe::CompositeAlpha::Opaque).create(&f.device).unwrap();
        // acquire_nextより前にやらないと死ぬ(get_images)
        let backbuffers = swapchain.get_images().unwrap();
        let isr = fe::ImageSubresourceRange
        {
            aspect_mask: fe::AspectMask::COLOR, mip_levels: 0 .. 1, array_layers: 0 .. 1
        };
        let bb_views = backbuffers.iter()
            .map(|i| i.create_view(None, None, &fe::ComponentMapping::default(), &isr))
            .collect::<fe::Result<Vec<_>>>().unwrap();

        let rp = fe::RenderPassBuilder::new()
            .add_attachment(fe::vk::VkAttachmentDescription
            {
                format: surface_format.format, samples: 1,
                initialLayout: fe::ImageLayout::ColorAttachmentOpt as _, finalLayout: fe::ImageLayout::PresentSrc as _,
                loadOp: fe::vk::VK_ATTACHMENT_LOAD_OP_CLEAR, storeOp: fe::vk::VK_ATTACHMENT_STORE_OP_STORE,
                stencilLoadOp: fe::vk::VK_ATTACHMENT_LOAD_OP_DONT_CARE,
                stencilStoreOp: fe::vk::VK_ATTACHMENT_STORE_OP_DONT_CARE, flags: 0
            })
            .add_subpass(fe::SubpassDescription::new().add_color_output(0, fe::ImageLayout::ColorAttachmentOpt, None))
            .create(&f.device).unwrap();
        let framebuffers = bb_views.iter().map(|iv| fe::Framebuffer::new(&rp, &[iv], &surface_size, 1))
            .collect::<fe::Result<Vec<_>>>().unwrap();
        
        let vsh = fe::ShaderModule::from_file(&f.device, "shaders/pass.vso").unwrap();
        let fsh = fe::ShaderModule::from_file(&f.device, "shaders/phong.fso").unwrap();
        let vbind = vec![
            fe::vk::VkVertexInputBindingDescription
            {
                binding: 0, stride: 16 * 2, inputRate: fe::vk::VK_VERTEX_INPUT_RATE_VERTEX
            }
        ];
        let vattrs = vec![
            fe::vk::VkVertexInputAttributeDescription
            {
                binding: 0, location: 0, offset: 0, format: fe::vk::VK_FORMAT_R32G32B32A32_SFLOAT
            },
            fe::vk::VkVertexInputAttributeDescription
            {
                binding: 0, location: 1, offset: 16, format: fe::vk::VK_FORMAT_R32G32B32A32_SFLOAT
            }
        ];
        let pl = fe::PipelineLayout::new(&f.device, &[], &[]).unwrap();
        let vp = fe::vk::VkViewport
        {
            x: 0.0, y: 0.0, width: surface_size.0 as _, height: surface_size.1 as _,
            minDepth: 0.0, maxDepth: 1.0
        };
        let scis = fe::vk::VkRect2D
        {
            offset: fe::vk::VkOffset2D { x: 0, y: 0 },
            extent: fe::vk::VkExtent2D { width: vp.width as _, height: vp.height as _ }
        };
        let gp = fe::GraphicsPipelineBuilder::new(&pl, (&rp, 0))
            .vertex_processing(fe::PipelineShader::new(&vsh, "main", None), vbind, vattrs)
            .fragment_shader(fe::PipelineShader::new(&fsh, "main", None))
            .primitive_topology(fe::vk::VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST, false)
            .fixed_viewport_scissors(fe::DynamicArrayState::Static(vec![vp]), fe::DynamicArrayState::Static(vec![scis]))
            .rasterization_samples(1, vec![])
            .add_attachment_blend(fe::vk::VkPipelineColorBlendAttachmentState
            {
                colorWriteMask: fe::vk::VK_COLOR_COMPONENT_A_BIT | fe::vk::VK_COLOR_COMPONENT_B_BIT |
                    fe::vk::VK_COLOR_COMPONENT_G_BIT | fe::vk::VK_COLOR_COMPONENT_R_BIT, .. Default::default()
            }).create(&f.device, None).unwrap();
        
        let bufsize = std::mem::size_of::<Vertex>() * 3;
        let buf = fe::BufferDesc::new(bufsize, fe::BufferUsage::VERTEX_BUFFER.transfer_dest())
            .create(&f.device).unwrap();
        let upload_buf = fe::BufferDesc::new(bufsize, fe::BufferUsage::VERTEX_BUFFER.transfer_src())
            .create(&f.device).unwrap();
        let (memreq, upload_memreq) = (buf.requirements(), upload_buf.requirements());
        let dmem = fe::DeviceMemory::allocate(&f.device, memreq.size as _, f.device_memindex).unwrap();
        let upload_mem = fe::DeviceMemory::allocate(&f.device, upload_memreq.size as _, f.upload_memindex).unwrap();
        buf.bind(&dmem, 0).unwrap(); upload_buf.bind(&upload_mem, 0).unwrap();
        unsafe 
        {
            upload_mem.map(0 .. bufsize).unwrap().slice_mut(0, 3).clone_from_slice(&[
                Vertex([0.0, -1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0]),
                Vertex([-1.0, 1.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]),
                Vertex([1.0, 1.0, 0.0, 1.0], [0.5, 0.0, 1.0, 0.0])
            ]);
        }
        
        let render_commands = f.cmdpool.alloc(framebuffers.len() as _, true).unwrap();
        for ((c, fb), iv) in render_commands.iter().zip(&framebuffers).zip(&bb_views)
        {
            c.begin().unwrap()
                .pipeline_barrier(fe::PipelineStageFlags::TOP_OF_PIPE, fe::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    true, &[], &[], &[fe::vk::VkImageMemoryBarrier
                    {
                        image: fe::VkHandle::native_ptr(&**iv), subresourceRange: fe::vk::VkImageSubresourceRange
                        {
                            aspectMask: fe::vk::VK_IMAGE_ASPECT_COLOR_BIT, .. Default::default()
                        },
                        oldLayout: fe::ImageLayout::PresentSrc as _, newLayout: fe::ImageLayout::ColorAttachmentOpt as _,
                        srcAccessMask: fe::vk::VK_ACCESS_MEMORY_READ_BIT, dstAccessMask: fe::vk::VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                        .. Default::default()
                    }])
                .begin_render_pass(&rp, fb, fe::vk::VkRect2D
                {
                    offset: fe::vk::VkOffset2D { x: 0, y: 0 },
                    extent: fe::vk::VkExtent2D { width: surface_size.0, height: surface_size.1 }
                }, &[fe::ClearValue::Color([0.0, 0.0, 0.0, 1.0])], true)
                    .bind_graphics_pipeline(&gp, &pl)
                    .bind_vertex_buffers(0, &[(&buf, 0)]).draw(3, 1, 0, 0)
                .end_render_pass();
        }
        let fw = fe::Fence::new(&f.device, false).unwrap();
        let init_commands = f.cmdpool.alloc(1, true).unwrap();
        init_commands[0].begin().unwrap()
            .pipeline_barrier(fe::PipelineStageFlags::TOP_OF_PIPE, fe::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                true, &[], &[], &bb_views.iter().map(|iv| fe::vk::VkImageMemoryBarrier
                {
                    image: fe::VkHandle::native_ptr(&**iv), subresourceRange: fe::vk::VkImageSubresourceRange
                    {
                        aspectMask: fe::vk::VK_IMAGE_ASPECT_COLOR_BIT, .. Default::default()
                    },
                    oldLayout: fe::ImageLayout::Undefined as _, newLayout: fe::ImageLayout::PresentSrc as _,
                    dstAccessMask: fe::vk::VK_ACCESS_MEMORY_READ_BIT, .. Default::default()
                }).collect::<Vec<_>>())
            .pipeline_barrier(fe::PipelineStageFlags::TOP_OF_PIPE, fe::PipelineStageFlags::TRANSFER, true,
                &[], &[fe::vk::VkBufferMemoryBarrier
                {
                    buffer: buf.native_ptr(), offset: 0, size: bufsize as _,
                    srcAccessMask: 0, dstAccessMask: fe::vk::VK_ACCESS_TRANSFER_WRITE_BIT,
                    .. Default::default()
                }, fe::vk::VkBufferMemoryBarrier
                {
                    buffer: upload_buf.native_ptr(), offset: 0, size: bufsize as _,
                    srcAccessMask: 0, dstAccessMask: fe::vk::VK_ACCESS_TRANSFER_READ_BIT,
                    .. Default::default()
                }], &[])
            .copy_buffer(&upload_buf, &buf, &[fe::vk::VkBufferCopy { srcOffset: 0, dstOffset: 0, size: bufsize as _ }])
            .pipeline_barrier(fe::PipelineStageFlags::TRANSFER, fe::PipelineStageFlags::VERTEX_INPUT, true,
                &[], &[fe::vk::VkBufferMemoryBarrier
                {
                    buffer: buf.native_ptr(), offset: 0, size: bufsize as _,
                    srcAccessMask: fe::vk::VK_ACCESS_TRANSFER_WRITE_BIT,
                    dstAccessMask: fe::vk::VK_ACCESS_VERTEX_ATTRIBUTE_READ_BIT,
                    .. Default::default()
                }], &[]);
        f.queue.submit(&[fe::SubmissionBatch
        {
            command_buffers: Cow::Borrowed(&init_commands), .. Default::default()
        }], Some(&fw)).unwrap(); fw.wait().unwrap();
        
        *self.renderlayer.borrow_mut() = Some(RenderLayer
        {
            _dmem: dmem, _buf: buf, _gp: gp, _pl: pl,
            render_commands, _framebuffers: framebuffers, _renderpass: rp, _bb_views: bb_views,
            swapchain, _surface: surface
        });
        self.dirty.set(true);
    }
    fn on_render_period(&self)
    {
        if self.dirty.get()
        {
            let fr = self.ferrite.borrow(); let f = fr.as_ref().unwrap();
            let rlr = self.renderlayer.borrow(); let rl = rlr.as_ref().unwrap();

            let next_drawable = rl.swapchain.acquire_next(None, fe::CompletionHandler::Device(&f.semaphore_sync_next))
                .unwrap() as usize;
            f.queue.submit(&[fe::SubmissionBatch
            {
                command_buffers: Cow::Borrowed(&rl.render_commands[next_drawable..next_drawable+1]),
                wait_semaphores: Cow::Borrowed(&[(&f.semaphore_sync_next, fe::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)]),
                signal_semaphores: Cow::Borrowed(&[&f.semaphore_command_completion])
            }], Some(&f.fence_command_completion)).unwrap();
            f.queue.present(&[(&rl.swapchain, next_drawable as _)], &[&f.semaphore_command_completion]).unwrap();
            // コマンドバッファの使用が終了したことを明示する
            f.fence_command_completion.wait().unwrap(); f.fence_command_completion.reset().unwrap();

            self.dirty.set(false);
        }
    }
}

fn main() { std::process::exit(GUIApplication::run("Ferrite integration demo", App::new())); }
