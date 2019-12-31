use std::sync::Arc;
use std::sync::Mutex;
use std::rc::Rc;

use ash::vk;
use ash::extensions::khr;
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};

use sourcerenderer_core::graphics::Adapter;
use sourcerenderer_core::graphics::Device;
use sourcerenderer_core::graphics::Queue;
use sourcerenderer_core::graphics::QueueType;
use sourcerenderer_core::graphics::CommandPool;
use crate::device::VkDevice;
use crate::command::VkCommandPool;
use crate::command::VkCommandBuffer;
use crate::swapchain::VkSwapchain;
use crate::VkBackend;
use sourcerenderer_core::graphics::Backend;

#[derive(Clone, Debug, Copy)]
pub struct VkQueueInfo {
  pub queue_family_index: usize,
  pub queue_index: usize,
  pub queue_type: QueueType,
  pub supports_presentation: bool
}

pub struct VkQueue {
  info: VkQueueInfo,
  queue: Mutex<vk::Queue>,
  device: Arc<VkDevice>
}

impl VkQueue {
  pub fn new(info: VkQueueInfo, queue: vk::Queue, device: Arc<VkDevice>) -> Self {
    return VkQueue {
      info: info,
      queue: Mutex::new(queue),
      device: device
    };
  }

  pub fn get_queue_family_index(&self) -> u32 {
    return self.info.queue_family_index as u32;
  }
}

// Vulkan queues are implicitly freed with the logical device

impl Queue<VkBackend> for VkQueue {
  fn create_command_pool(self: Arc<Self>) -> VkCommandPool {
    return VkCommandPool::new(self.device.clone(), self.clone());
  }

  fn get_queue_type(&self) -> QueueType {
    return self.info.queue_type;
  }

  fn supports_presentation(&self) -> bool {
    return self.info.supports_presentation;
  }

  fn submit(&self, command_buffer: &VkCommandBuffer) {
    let info = vk::SubmitInfo {
      p_command_buffers: command_buffer.get_handle() as *const vk::CommandBuffer,
      command_buffer_count: 1,
      ..Default::default()
    };
    let vk_queue = self.queue.lock().unwrap();
    unsafe {
      self.device.get_ash_device().queue_submit(*vk_queue, &[info], vk::Fence::null());
    }
  }

  fn present(&self, swapchain: &VkSwapchain, image_index: u32) {
    let present_info = vk::PresentInfoKHR {
      p_swapchains: swapchain.get_handle() as *const vk::SwapchainKHR,
      swapchain_count: 1,
      p_image_indices: &image_index as *const u32,
      ..Default::default()
    };
    let vk_queue = self.queue.lock().unwrap();
    unsafe {
      swapchain.get_loader().queue_present(*vk_queue, &present_info);
    }
  }
}
