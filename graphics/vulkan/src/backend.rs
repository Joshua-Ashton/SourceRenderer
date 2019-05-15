use crate::VkDevice;
use crate::*;
use crate::pipeline::VkShader;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum VkBackend {}

impl sourcerenderer_base::graphics::Backend for VkBackend {
  type Device = VkDevice;
  type CommandPool = VkCommandPool;
  type Instance = VkInstance;
  type CommandBuffer = VkCommandBuffer;
  type Adapter = VkAdapter;
  type Surface = VkSurface;
  type Queue = VkQueue;
  type Texture = VkTexture;
  type Buffer = VkBuffer;
  type Shader = VkShader;
  type Pipeline = VkPipeline;
  type RenderTargetView = VkRenderTargetView;
  type RenderPass = VkRenderPass;
  type RenderPassLayout = VkRenderPassLayout;
  type Swapchain = VkSwapchain;
}