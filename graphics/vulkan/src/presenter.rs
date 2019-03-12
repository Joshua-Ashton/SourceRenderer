use ash::vk;
use ash::extensions::khr;

pub struct Presenter {
  surface: vk::SurfaceKHR,
  swapchain: vk::SwapchainKHR
}

pub const SWAPCHAIN_EXT_NAME: &str = "VK_KHR_swapchain";

impl Presenter {
  pub unsafe fn new(physical_device: &vk::PhysicalDevice, device: &ash::Device, surface_ext: khr::Surface, surface: vk::SurfaceKHR, swapchain_ext: khr::Swapchain) -> Presenter {
    let present_modes = surface_ext.get_physical_device_surface_present_modes(*physical_device, surface).unwrap();
    let present_mode = Presenter::pick_present_mode(present_modes);

    let formats = surface_ext.get_physical_device_surface_formats(*physical_device, surface).unwrap();
    let format = Presenter::pick_format(formats);

    let capabilities = surface_ext.get_physical_device_surface_capabilities(*physical_device, surface).unwrap();
    let extent = Presenter::pick_swap_extent(&capabilities);

    let image_count = if capabilities.max_image_count > 0 {
      capabilities.max_image_count
    } else {
      capabilities.min_image_count + 1
    };

    let swapchain_create_info = vk::SwapchainCreateInfoKHR {
      surface: surface,
      min_image_count: image_count,
      image_format: format.format,
      image_color_space: format.color_space,
      image_extent: extent,
      image_array_layers: 1,
      image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
      present_mode: present_mode,
      image_sharing_mode: vk::SharingMode::EXCLUSIVE,
      pre_transform: capabilities.current_transform,
      composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
      clipped: vk::TRUE,
      old_swapchain: vk::SwapchainKHR::null(),
      ..Default::default()
    };

    let swapchain = swapchain_ext.create_swapchain(&swapchain_create_info, None).unwrap();

    return Presenter {
      surface: surface,
      swapchain: swapchain
    };
  }

  unsafe fn pick_present_mode(present_modes: Vec<vk::PresentModeKHR>) -> vk::PresentModeKHR {
    return *present_modes
      .iter()
      .filter(|&&mode| mode == vk::PresentModeKHR::FIFO)
      .nth(0).expect("No compatible present mode found");
  }

  unsafe fn pick_format(formats: Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {
    return if formats.len() == 1 && formats[0].format == vk::Format::UNDEFINED {
      vk::SurfaceFormatKHR {
        format: vk::Format::B8G8R8A8_UNORM,
        color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR
      }
    } else {
      *formats
        .iter()
        .filter(|&format| format.format == vk::Format::B8G8R8A8_UNORM && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
        .nth(0).expect("No compatible format found")
    }
  }

  unsafe fn pick_swap_extent(capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
    return if capabilities.current_extent.width != u32::max_value() {
      capabilities.current_extent
    } else {
      panic!("No current extent")
    }
  }

}
