use graphics::Instance;
use graphics::Adapter;
use graphics::Device;
use graphics::Surface;
use graphics::CommandPool;
use graphics::CommandBuffer;
use graphics::Queue;
use graphics::Shader;
use graphics::PipelineInfo;
use graphics::Pipeline;
use graphics::Texture;
use graphics::Buffer;
use graphics::RenderPassLayout;
use graphics::RenderPass;
use graphics::RenderTargetView;
use graphics::Swapchain;
use graphics::Resettable;
use graphics::Fence;
use graphics::Semaphore;

pub trait Backend: 'static + Sized {
  type Instance: Instance<Self>;
  type Adapter: Adapter<Self>;
  type Device: Device<Self>;
  type Surface: Surface<Self>;
  type Swapchain: Swapchain<Self>;
  type CommandPool: CommandPool<Self> + Resettable;
  type CommandBuffer: CommandBuffer<Self>;
  type Queue: Queue<Self>;
  type Texture: Texture;
  type Buffer: Buffer;
  type Shader: Shader;
  type Pipeline: Pipeline<Self>;
  type RenderPassLayout: RenderPassLayout<Self>;
  type RenderPass: RenderPass<Self>;
  type RenderTargetView: RenderTargetView<Self>;
  type Semaphore: Semaphore + Resettable;
  type Fence: Fence + Resettable;
}