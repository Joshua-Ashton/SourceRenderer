use std::sync::{Arc, Mutex};
use crossbeam_channel::{Sender, unbounded};

use sourcerenderer_core::platform::{Platform, Window, WindowState};
use sourcerenderer_core::graphics::Backend;
use sourcerenderer_core::Matrix4;

use crate::asset::AssetManager;
use crate::renderer::Drawable;

use std::sync::atomic::{Ordering, AtomicUsize};

use crate::renderer::command::RendererCommand;
use legion::{World, Resources, Entity};
use legion::systems::Builder;

use crate::renderer::RendererInternal;
use crate::renderer::camera::PrimaryCamera;

pub struct Renderer<P: Platform> {
  sender: Sender<RendererCommand>,
  device: Arc<<P::GraphicsBackend as Backend>::Device>,
  window_state: Mutex<WindowState>,
  queued_frames_counter: AtomicUsize,
  primary_camera: Arc<PrimaryCamera<P::GraphicsBackend>>
}

impl<P: Platform> Renderer<P> {
  fn new(sender: Sender<RendererCommand>, device: &Arc<<P::GraphicsBackend as Backend>::Device>, window: &P::Window) -> Self {
    Self {
      sender,
      device: device.clone(),
      window_state: Mutex::new(window.state()),
      queued_frames_counter: AtomicUsize::new(0),
      primary_camera: Arc::new(PrimaryCamera::new(device.as_ref()))
    }
  }

  pub fn run(window: &P::Window,
             device: &Arc<<P::GraphicsBackend as Backend>::Device>,
             swapchain: &Arc<<P::GraphicsBackend as Backend>::Swapchain>,
             asset_manager: &Arc<AssetManager<P>>,
             simulation_tick_rate: u32) -> Arc<Renderer<P>> {
    let (sender, receiver) = unbounded::<RendererCommand>();
    let renderer = Arc::new(Renderer::new(sender.clone(), device, window));
    let mut internal = RendererInternal::new(&renderer, &device, &swapchain, asset_manager, sender, receiver, simulation_tick_rate, renderer.primary_camera());

    std::thread::spawn(move || {
      loop {
        internal.render();
      }
    });
    renderer
  }

  pub fn primary_camera(&self) -> &Arc<PrimaryCamera<P::GraphicsBackend>> {
    &self.primary_camera
  }

  pub fn set_window_state(&self, window_state: WindowState) {
    let mut guard = self.window_state.lock().unwrap();
    *guard = window_state
  }

  pub fn install(self: &Arc<Renderer<P>>, _world: &mut World, _resources: &mut Resources, systems: &mut Builder) {
    crate::renderer::ecs::install(systems, self);
  }

  pub fn register_static_renderable(&self, renderable: Drawable) {
    let result = self.sender.send(RendererCommand::Register(renderable));
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  pub fn unregister_static_renderable(&self, entity: Entity) {
    let result = self.sender.send(RendererCommand::UnregisterStatic(entity));
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  pub fn update_camera_transform(&self, camera_transform_mat: Matrix4, fov: f32) {
    let result = self.sender.send(RendererCommand::UpdateCameraTransform { camera_transform_mat, fov });
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  pub fn update_transform(&self, entity: Entity, transform: Matrix4) {
    let result = self.sender.send(RendererCommand::UpdateTransform { entity, transform_mat: transform });
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  pub fn end_frame(&self) {
    self.queued_frames_counter.fetch_add(1, Ordering::SeqCst);
    let result = self.sender.send(RendererCommand::EndFrame);
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  pub fn is_saturated(&self) -> bool {
    self.queued_frames_counter.load(Ordering::SeqCst) > 2
  }

  pub(super) fn window_state(&self) -> &Mutex<WindowState> {
    &self.window_state
  }

  pub(super) fn dec_queued_frames_counter(&self) -> usize {
    self.queued_frames_counter.fetch_sub(1, Ordering::SeqCst)
  }
}
