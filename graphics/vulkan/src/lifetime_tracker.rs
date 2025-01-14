use crate::{VkFence, VkSemaphore, texture::VkSampler};
use std::sync::Arc;
use crate::buffer::VkBufferSlice;
use crate::{VkPipeline, VkRenderPass, VkTexture};
use crate::texture::VkTextureView;
use crate::VkFrameBuffer;

pub struct VkLifetimeTrackers {
  semaphores: Vec<Arc<VkSemaphore>>,
  fences: Vec<Arc<VkFence>>,
  buffers: Vec<Arc<VkBufferSlice>>,
  textures: Vec<Arc<VkTexture>>,
  texture_views: Vec<Arc<VkTextureView>>,
  render_passes: Vec<Arc<VkRenderPass>>,
  frame_buffers: Vec<Arc<VkFrameBuffer>>,
  samplers: Vec<Arc<VkSampler>>,
  pipelines: Vec<Arc<VkPipeline>>
}

impl VkLifetimeTrackers {
  pub(crate) fn new() -> Self {
    Self {
      semaphores: Vec::new(),
      fences: Vec::new(),
      buffers: Vec::new(),
      textures: Vec::new(),
      texture_views: Vec::new(),
      render_passes: Vec::new(),
      frame_buffers: Vec::new(),
      samplers: Vec::new(),
      pipelines: Vec::new()
    }
  }

  pub(crate) fn reset(&mut self) {
    self.semaphores.clear();
    self.fences.clear();
    self.buffers.clear();
    self.textures.clear();
    self.texture_views.clear();
    self.render_passes.clear();
    self.frame_buffers.clear();
    self.samplers.clear();
    self.pipelines.clear();
  }

  pub(crate) fn track_semaphore(&mut self, semaphore: &Arc<VkSemaphore>) {
    self.semaphores.push(semaphore.clone());
  }

  pub(crate) fn track_fence(&mut self, fence: &Arc<VkFence>) {
    self.fences.push(fence.clone());
  }

  pub(crate) fn track_buffer(&mut self, buffer: &Arc<VkBufferSlice>) {
    self.buffers.push(buffer.clone());
  }

  pub(crate) fn track_texture(&mut self, texture: &Arc<VkTexture>) {
    self.textures.push(texture.clone());
  }

  pub(crate) fn track_render_pass(&mut self, render_pass: &Arc<VkRenderPass>) {
    self.render_passes.push(render_pass.clone());
  }

  pub(crate) fn track_frame_buffer(&mut self, frame_buffer: &Arc<VkFrameBuffer>) {
    self.frame_buffers.push(frame_buffer.clone());
  }

  pub(crate) fn track_texture_view(&mut self, texture_view: &Arc<VkTextureView>) {
    self.texture_views.push(texture_view.clone());
  }

  pub(crate) fn track_sampler(&mut self, sampler: &Arc<VkSampler>) {
    self.samplers.push(sampler.clone());
  }

  pub(crate) fn track_pipeline(&mut self, pipeline: &Arc<VkPipeline>) {
    self.pipelines.push(pipeline.clone());
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.texture_views.is_empty()
    && self.semaphores.is_empty()
    && self.fences.is_empty()
    && self.buffers.is_empty()
    && self.textures.is_empty()
    && self.render_passes.is_empty()
    && self.frame_buffers.is_empty()
    && self.samplers.is_empty()
    && self.pipelines.is_empty()
  }
}
