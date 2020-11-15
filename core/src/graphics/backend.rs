use crate::graphics::{Instance, TextureShaderResourceView, Fence};
use crate::graphics::Adapter;
use crate::graphics::Device;
use crate::graphics::Surface;
use crate::graphics::CommandBuffer;
use crate::graphics::Shader;
use crate::graphics::GraphicsPipelineInfo;
use crate::graphics::Texture;
use crate::graphics::Buffer;
use crate::graphics::Swapchain;
use crate::graphics::Resettable;
use crate::graphics::{RenderGraph, RenderGraphTemplate};
use std::hash::Hash;

// WANT https://github.com/rust-lang/rust/issues/44265
pub trait Backend: 'static + Sized {
  type Instance: Instance<Self> + Send + Sync;
  type Adapter: Adapter<Self> + Send + Sync;
  type Device: Device<Self> + Send + Sync;
  type Surface: Surface + Send + Sync;
  type Swapchain: Swapchain + Send + Sync;
  type CommandBuffer: CommandBuffer<Self>;
  type CommandBufferSubmission: Send + Sync;
  type Texture: Texture + Send + Sync;
  type TextureShaderResourceView: TextureShaderResourceView + Send + Sync;
  type Buffer: Buffer + Send + Sync;
  type Shader: Shader + Hash + Eq + PartialEq + Send + Sync;
  type GraphicsPipeline: Send + Sync;
  type ComputePipeline: Send + Sync;
  type RenderGraphTemplate: RenderGraphTemplate + Send + Sync;
  type RenderGraph: RenderGraph<Self> + Send + Sync;
  type Fence : Fence + Send + Sync;
}
