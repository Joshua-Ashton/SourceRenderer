use sourcerenderer_core::platform::{Platform, InputCommands};
use std::sync::Arc;
use std::sync::MutexGuard;

use sourcerenderer_core::ThreadPoolBuilder;
use sourcerenderer_core::graphics::*;
use sourcerenderer_core::platform::Window;

use crate::{asset::AssetManager, renderer::RendererInterface};
use crate::renderer::Renderer;
use crate::game::Game;

const TICK_RATE: u32 = 5;

pub struct Engine<P: Platform> {
  renderer: Arc<Renderer<P>>,
  game: Arc<Game<P>>,
  platform: Box<P>
}

impl<P: Platform> Engine<P> {
  #[cfg(not(feature = "web"))]
  pub fn initialize_global() {
    let cores = num_cpus::get();
    ThreadPoolBuilder::new().num_threads(cores - 2).build_global().unwrap();
  }

  #[cfg(feature = "web")]
  pub fn initialize_global() {}

  pub fn run(platform: Box<P>) -> Self {
    let instance = platform.create_graphics(true).expect("Failed to initialize graphics");
    let surface = platform.window().create_surface(instance.clone());

    let mut adapters = instance.clone().list_adapters();
    let device = Arc::new(adapters.remove(0).create_device(&surface));
    let swapchain = Arc::new(platform.window().create_swapchain(false, &device, &surface));
    let asset_manager = AssetManager::<P>::new(&platform, &device);
    let renderer = Renderer::<P>::run(&platform, platform.window(), &instance, &device, &swapchain, &asset_manager);
    let game = Game::<P>::run(&platform, &renderer, &asset_manager, TICK_RATE);
    Self {
      renderer,
      game,
      platform
    }
  }

  pub fn receive_input_commands(&self) -> InputCommands {
    self.game.receive_input_commands()
  }

  pub fn platform(&mut self) -> &mut P {
    &mut self.platform
  }

  pub fn notify_window_changed(&self) {
    let surface = self.platform.window().create_surface(self.renderer.instance().clone());
    self.renderer.change_surface(&surface);
  }

  pub fn poll_platform(&self) {
    self.game.update_input_state(self.platform.input_state());
    self.renderer.set_window_state(self.platform.window().state());
    if !self.game.is_running() || !self.renderer.is_running() {
      self.stop(); // if just one system dies, kill the others too
    }
  }

  pub fn stop(&self) {
    self.game.stop();
    self.renderer.stop();
  }

  pub fn is_running(&self) -> bool {
    if !self.game.is_running() || !self.renderer.is_running() {
      self.stop(); // if just one system dies, kill the others too
      return false;
    }
    return true;
  }

  pub fn device(&self) -> &Arc<<P::GraphicsBackend as Backend>::Device> {
    self.renderer.device()
  }

  pub fn surface(&self) -> MutexGuard<Arc<<P::GraphicsBackend as Backend>::Surface>> {
    self.renderer.surface()
  }
}
