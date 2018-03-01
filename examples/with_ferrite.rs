// #![feature(link_args)]
// #![link_args = "-rpath /Users/pctgx8/Downloads/vulkansdk-macos-1.0.69.0/macOS/Frameworks"]

extern crate appframe;
extern crate ferrite;

use appframe::*;
use ferrite as fe;
use fe::traits::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::Cow;

struct App
{
    w: RefCell<Option<NativeWindow>>, ferrite: RefCell<Option<Ferrite>>, renderlayer: RefCell<Option<RenderLayer>>
}
pub struct Ferrite
{
    device: fe::Device, adapter: fe::PhysicalDevice, instance: fe::Instance,
    gq: u32, queue: fe::Queue, cmdpool: fe::CommandPool,
    semaphore_sync_next: fe::Semaphore, semaphore_command_completion: fe::Semaphore,
    fence_command_completion: fe::Fence
}
pub struct RenderLayer
{
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
            w: RefCell::new(None), ferrite: RefCell::new(None), renderlayer: RefCell::new(None)
        }
    }
}
impl EventDelegate for App
{
    fn postinit<S: FerriteRenderingServer + GUIApplicationRunner<Self>>(&self, server: &Rc<S>)
    {
        let instance = fe::InstanceBuilder::new("appframe_integ", (0, 1, 0), "Ferrite", (0, 1, 0))
            .add_extensions(vec!["VK_KHR_surface", "VK_MVK_macos_surface", "VK_EXT_debug_report"])
            .add_layer("VK_LAYER_LUNARG_standard_validation")
            .create().unwrap();
        let adapter = instance.enumerate_physical_devices().unwrap().remove(0);
        println!("Vulkan AdapterName: {}", unsafe { std::ffi::CStr::from_ptr(adapter.properties().deviceName.as_ptr()).to_str().unwrap() });
        let gq = adapter.queue_family_properties().find_matching_index(fe::QueueFlags::GRAPHICS).expect("Cannot find a graphics queue");
        let device = fe::DeviceBuilder::new(&adapter)
            .add_extensions(vec!["VK_KHR_swapchain"])
            .add_queue(fe::DeviceQueueCreateInfo(gq, vec![0.0]))
            .create().unwrap();
        *self.ferrite.borrow_mut() = Some(Ferrite
        {
            fence_command_completion: fe::Fence::new(&device, false).unwrap(),
            semaphore_sync_next: fe::Semaphore::new(&device).unwrap(),
            semaphore_command_completion: fe::Semaphore::new(&device).unwrap(),
            cmdpool: fe::CommandPool::new(&device, gq, false, false).unwrap(),
            queue: device.queue(gq, 0),
            device, adapter, instance, gq
        });

        let w = NativeWindowBuilder::new(640, 360, "Ferrite integration").create_renderable(server).unwrap();
        *self.w.borrow_mut() = Some(w);
        self.w.borrow().as_ref().unwrap().show();
    }
    fn on_init_view<S: FerriteRenderingServer>(&self, server: &S, surface_onto: &<S as FerriteRenderingServer>::SurfaceSource)
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
                }, &[fe::ClearValue::Color([0.0, 0.0, 0.0, 1.0])], true).end_render_pass();
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
                }).collect::<Vec<_>>());
        f.queue.submit(&[fe::SubmissionBatch
        {
            command_buffers: Cow::Borrowed(&init_commands), .. Default::default()
        }], Some(&fw)).unwrap(); fw.wait().unwrap();
        
        *self.renderlayer.borrow_mut() = Some(RenderLayer
        {
            render_commands, _framebuffers: framebuffers, _renderpass: rp, _bb_views: bb_views, swapchain, _surface: surface
        });
    }
    fn on_render_period(&self)
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
    }
}

fn main() { std::process::exit(GUIApplication::run("Ferrite integration demo", App::new())); }
