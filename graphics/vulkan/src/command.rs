use std::sync::{Arc, Mutex};

use ash::vk;
use ash::version::DeviceV1_0;

use sourcerenderer_core::graphics::{CommandPool, PipelineInfo, PipelineInfo2, Backend};
use sourcerenderer_core::graphics::CommandBuffer;
use sourcerenderer_core::graphics::CommandBufferType;
use sourcerenderer_core::graphics::RenderPass;
use sourcerenderer_core::graphics::RenderpassRecordingMode;
use sourcerenderer_core::graphics::Viewport;
use sourcerenderer_core::graphics::Scissor;
use sourcerenderer_core::graphics::Resettable;
use sourcerenderer_core::graphics::Submission;

use sourcerenderer_core::pool::Recyclable;
use std::sync::mpsc::{ Sender, Receiver, channel };

use crate::VkQueue;
use crate::VkDevice;
use crate::raw::RawVkDevice;
use crate::VkRenderPass;
use crate::VkBuffer;
use crate::VkPipeline;
use crate::VkBackend;

use crate::raw::*;
use pipeline::VkPipelineInfo;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use VkRenderPassLayout;
use context::{VkGraphicsContext, VkSharedCaches};
use std::cell::{RefCell, RefMut};

pub struct VkCommandPool {
  pool: vk::CommandPool,
  device: Arc<RawVkDevice>,
  buffers: Vec<VkCommandBuffer>,
  caches: Arc<VkSharedCaches>
}

pub struct VkCommandBuffer {
  buffer: vk::CommandBuffer,
  device: Arc<RawVkDevice>,
  caches: Arc<VkSharedCaches>,
  render_pass: Option<Arc<VkRenderPassLayout>>,
  sub_pass: u32,
  state: VkCommandBufferState
}

pub enum VkCommandBufferState {
  Ready,
  Recording,
  Executable
}

pub struct VkSubmission {
  buffer: vk::CommandBuffer
}

impl VkCommandPool {
  pub fn new(device: &Arc<RawVkDevice>, queue_family_index: u32, caches: &Arc<VkSharedCaches>) -> Self {
    let create_info = vk::CommandPoolCreateInfo {
      queue_family_index,
      flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
      ..Default::default()
    };

    return Self {
      pool: unsafe {
        device.create_command_pool(&create_info, None)
      }.unwrap(),
      device: device.clone(),
      buffers: Vec::new(),
      caches: caches.clone()
    };
  }

  pub fn test(&mut self) {}
}

impl Drop for VkCommandPool {
  fn drop(&mut self) {
    for cmd_buffer in &mut self.buffers.drain(..) {
      unsafe {
        self.device.device.free_command_buffers(self.pool, &[ cmd_buffer.buffer ])
      }
    }
    self.buffers.clear();
    unsafe {
      self.device.destroy_command_pool(self.pool, None);
    }
  }
}

impl CommandPool<VkBackend> for VkCommandPool {
  fn get_command_buffer(&mut self, command_buffer_type: CommandBufferType) -> &mut VkCommandBuffer {
    let ptr = &self.buffers as *const Vec<VkCommandBuffer>;
    // the borrow checker is not smart enough to realize that the reference only exists if we return here
    for cmd_buffer in unsafe { ptr.as_ref().unwrap() } {
      let cmd_buffer_ref = unsafe { ((cmd_buffer as *const VkCommandBuffer) as *mut VkCommandBuffer).as_mut().unwrap() };
      cmd_buffer_ref.begin();
      return cmd_buffer_ref;
    }

    let cmd_buffer = VkCommandBuffer::new(&self.device, &self.pool, command_buffer_type, &self.caches);
    self.buffers.push(cmd_buffer);
    let mut cmd_buffer_ref = unsafe { ((self.buffers.last().unwrap() as *const VkCommandBuffer) as *mut VkCommandBuffer).as_mut().unwrap() };
    cmd_buffer_ref.begin();
    return cmd_buffer_ref;
  }
}

impl Resettable for VkCommandPool {
  fn reset(&mut self) {
    unsafe {
      self.device.reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty()).unwrap();
    }
    for cmd_buffer_ref in &self.buffers {
      let mut cmd_buffer = unsafe { ((cmd_buffer_ref as *const VkCommandBuffer) as *mut VkCommandBuffer).as_mut().unwrap() };
      cmd_buffer.state = VkCommandBufferState::Ready
    }
  }
}

impl VkCommandBuffer {
  fn new(device: &Arc<RawVkDevice>, pool: &vk::CommandPool, command_buffer_type: CommandBufferType, caches: &Arc<VkSharedCaches>) -> Self {
    let buffers_create_info = vk::CommandBufferAllocateInfo {
      command_pool: *pool,
      level: if command_buffer_type == CommandBufferType::PRIMARY { vk::CommandBufferLevel::PRIMARY } else { vk::CommandBufferLevel::SECONDARY }, // TODO: support secondary command buffers / bundles
      command_buffer_count: 1, // TODO: figure out how many buffers per pool (maybe create a new pool once we've run out?)
      ..Default::default()
    };
    let mut buffers = unsafe { device.allocate_command_buffers(&buffers_create_info) }.unwrap();
    let buffer = buffers.pop().unwrap();

    return VkCommandBuffer {
      buffer,
      device: device.clone(),
      render_pass: None,
      sub_pass: 0u32,
      caches: caches.clone(),
      state: VkCommandBufferState::Recording
    };
  }

  pub fn get_handle(&self) -> &vk::CommandBuffer {
    return &self.buffer;
  }

  pub fn begin(&mut self) {
    self.state = VkCommandBufferState::Recording;
    unsafe {
      let begin_info = vk::CommandBufferBeginInfo {
        flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        ..Default::default()
      };
      self.device.begin_command_buffer(self.buffer, &begin_info);
    }
  }

  pub fn end(&mut self) {
    unsafe {
      self.device.end_command_buffer(self.buffer);
    }
    self.state = VkCommandBufferState::Executable;
  }
}

impl CommandBuffer<VkBackend> for VkCommandBuffer {

  fn set_pipeline2(&mut self, pipeline: &PipelineInfo2<VkBackend>) {
    if self.render_pass.is_none() {
      panic!("Cant set pipeline outside of render pass");
    }

    let render_pass = self.render_pass.clone().unwrap();

    let info = VkPipelineInfo {
      info: pipeline,
      render_pass: &render_pass,
      sub_pass: self.sub_pass
    };

    let mut hasher = DefaultHasher::new();
    info.hash(&mut hasher);
    let hash = hasher.finish();

    {
      let lock = self.caches.get_pipelines().read().unwrap();
      let cached_pipeline = lock.get(&hash);
      if let Some(pipeline) = cached_pipeline {
        let vk_pipeline = *pipeline.get_handle();
        unsafe {
          self.device.cmd_bind_pipeline(self.buffer, vk::PipelineBindPoint::GRAPHICS, vk_pipeline);
        }
        return;
      }
    }
    let pipeline = VkPipeline::new2(&self.device, &info);
    let mut lock = self.caches.get_pipelines().write().unwrap();
    lock.insert(hash, pipeline);
    let stored_pipeline = lock.get(&hash).unwrap();
    let vk_pipeline = *stored_pipeline.get_handle();
    unsafe {
      self.device.cmd_bind_pipeline(self.buffer, vk::PipelineBindPoint::GRAPHICS, vk_pipeline);
    }
  }

  fn begin_render_pass(&mut self, renderpass: &VkRenderPass, recording_mode: RenderpassRecordingMode) {
    unsafe {
      let begin_info = vk::RenderPassBeginInfo {
        framebuffer: *renderpass.get_framebuffer(),
        render_pass: *renderpass.get_layout().get_handle(),
        render_area: vk::Rect2D {
          offset: vk::Offset2D { x: 0i32, y: 0i32 },
          extent: vk::Extent2D { width: renderpass.get_info().width, height: renderpass.get_info().height }
        },
        clear_value_count: 1,
        p_clear_values: &[
          vk::ClearValue {
            color: vk::ClearColorValue {
              float32: [0.0f32, 0.0f32, 0.0f32, 1.0f32]
            }
         },
         vk::ClearValue {
           depth_stencil: vk::ClearDepthStencilValue {
            depth: 0.0f32,
            stencil: 0u32
          }
         }
        ] as *const vk::ClearValue,
        ..Default::default()
      };
      self.device.cmd_begin_render_pass(self.buffer, &begin_info, if recording_mode == RenderpassRecordingMode::Commands { vk::SubpassContents::INLINE } else { vk::SubpassContents::SECONDARY_COMMAND_BUFFERS });
    }
    self.render_pass = Some(renderpass.get_layout().clone());
    self.sub_pass = 0;
  }

  fn end_render_pass(&mut self) {
    unsafe {
      self.device.cmd_end_render_pass(self.buffer);
    }
    self.render_pass = None;
  }

  fn set_pipeline(&mut self, pipeline: Arc<VkPipeline>) {
    unsafe {
      self.device.cmd_bind_pipeline(self.buffer, vk::PipelineBindPoint::GRAPHICS, *pipeline.get_handle());
    }
  }

  fn set_vertex_buffer(&mut self, vertex_buffer: Arc<VkBuffer>) {
    unsafe {
      self.device.cmd_bind_vertex_buffers(self.buffer, 0, &[*(*vertex_buffer).get_handle()], &[0]);
    }
  }

  fn set_viewports(&mut self, viewports: &[ Viewport ]) {
    unsafe {
      for i in 0..viewports.len() {
        self.device.cmd_set_viewport(self.buffer, i as u32, &[vk::Viewport {
          x: viewports[i].position.x,
          y: viewports[i].position.y,
          width: viewports[i].extent.x,
          height: viewports[i].extent.y,
          min_depth: viewports[i].min_depth,
          max_depth: viewports[i].max_depth
        }]);
      }
    }
  }

  fn set_scissors(&mut self, scissors: &[ Scissor ])  {
    unsafe {
      let vk_scissors: Vec<vk::Rect2D> = scissors.iter().map(|scissor| vk::Rect2D {
        offset: vk::Offset2D {
          x: scissor.position.x,
          y: scissor.position.y
        },
        extent: vk::Extent2D {
          width: scissor.extent.x,
          height: scissor.extent.y
        }
      }).collect();
      self.device.cmd_set_scissor(self.buffer, 0, &vk_scissors);
    }
  }

  fn draw(&mut self, vertices: u32, offset: u32) {
    unsafe {
      self.device.cmd_draw(self.buffer, vertices, 1, offset, 0);
    }
  }

  fn finish(&mut self) -> <VkBackend as Backend>::Submission {
    self.end();
    VkSubmission::new(self.buffer)
  }
}

impl VkSubmission {
  pub fn new(command_buffer: vk::CommandBuffer) -> Self {
    Self {
      buffer: command_buffer
    }
  }

  pub fn get_cmd_buffer(&self) -> &vk::CommandBuffer {
    &self.buffer
  }
}

impl Submission for VkSubmission {

}
