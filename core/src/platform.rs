use std::error::Error;
use std::sync::Arc;

use crate::graphics::SwapchainInfo;

use crate::graphics;

#[derive(PartialEq)]
pub enum PlatformEvent {
  Continue,
  Quit
}

#[derive(PartialEq)]
#[derive(Copy)]
#[derive(Clone)]
pub enum GraphicsApi {
  OpenGLES,
  Vulkan
}

pub trait Platform: 'static + Sized {
  type GraphicsBackend: graphics::Backend;
  type Window: Window<Self>;

  fn window(&mut self) -> &Self::Window;
  fn handle_events(&mut self) -> PlatformEvent;
  fn create_graphics(&self, debug_layers: bool) -> Result<Arc<<Self::GraphicsBackend as graphics::Backend>::Instance>, Box<dyn Error>>;
}

pub trait Window<P: Platform> {
  fn create_surface(&self, graphics_instance: Arc<<P::GraphicsBackend as graphics::Backend>::Instance>) -> Arc<<P::GraphicsBackend as graphics::Backend>::Surface>;
  fn create_swapchain(&self, info: SwapchainInfo, device: Arc<<P::GraphicsBackend as graphics::Backend>::Device>, surface: Arc<<P::GraphicsBackend as graphics::Backend>::Surface>) -> Arc<<P::GraphicsBackend as graphics::Backend>::Swapchain>;
}