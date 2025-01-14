use std::{cmp::max, sync::{Arc, Mutex, MutexGuard, atomic::AtomicBool}};
use crossbeam_channel::{Sender, unbounded};

use sourcerenderer_core::{atomic_refcell::AtomicRefCell, graphics::SwapchainError, platform::{Platform, Window, WindowState}};
use sourcerenderer_core::graphics::{Backend, Swapchain};
use sourcerenderer_core::Matrix4;

use crate::{asset::AssetManager, transform::interpolation::InterpolatedTransform};

use std::sync::atomic::{Ordering, AtomicUsize};

use crate::renderer::command::RendererCommand;
use legion::{World, Resources, Entity};
use legion::systems::Builder;

use crate::renderer::RendererInternal;
use crate::renderer::camera::LateLatchCamera;

use super::{StaticRenderableComponent, drawable::View, ecs::{PointLightComponent, RendererInterface}, renderer_assets::RendererTexture, renderer_scene::RendererScene};

pub struct Renderer<P: Platform> {
  sender: Sender<RendererCommand>,
  instance: Arc<<P::GraphicsBackend as Backend>::Instance>,
  device: Arc<<P::GraphicsBackend as Backend>::Device>,
  window_state: Mutex<WindowState>,
  queued_frames_counter: AtomicUsize,
  primary_camera: Arc<LateLatchCamera<P::GraphicsBackend>>,
  surface: Mutex<Arc<<P::GraphicsBackend as Backend>::Surface>>,
  is_running: AtomicBool
}

impl<P: Platform> Renderer<P> {
  fn new(sender: Sender<RendererCommand>, instance: &Arc<<P::GraphicsBackend as Backend>::Instance>, device: &Arc<<P::GraphicsBackend as Backend>::Device>, window: &P::Window, surface: &Arc<<P::GraphicsBackend as Backend>::Surface>) -> Self {
    let window_state = window.state();
    let (width, height) = match &window_state {
        WindowState::Minimized => (0,0),
        WindowState::Visible { width, height, focussed: _focussed } => (*width, *height),
        WindowState::FullScreen { width, height } => (*width, *height),
        WindowState::Exited => (0,0)
    };

    Self {
      sender,
      instance: instance.clone(),
      device: device.clone(),
      window_state: Mutex::new(window.state()),
      queued_frames_counter: AtomicUsize::new(0),
      primary_camera: Arc::new(LateLatchCamera::new(device.as_ref(), (width as f32) / (max(1, height) as f32), std::f32::consts::FRAC_PI_2)),
      surface: Mutex::new(surface.clone()),
      is_running: AtomicBool::new(true)
    }
  }

  pub fn run(window: &P::Window,
             instance: &Arc<<P::GraphicsBackend as Backend>::Instance>,
             device: &Arc<<P::GraphicsBackend as Backend>::Device>,
             swapchain: &Arc<<P::GraphicsBackend as Backend>::Swapchain>,
             asset_manager: &Arc<AssetManager<P>>) -> Arc<Renderer<P>> {
    let (sender, receiver) = unbounded::<RendererCommand>();
    let renderer = Arc::new(Renderer::new(sender.clone(), instance, device, window, swapchain.surface()));

    let c_device = device.clone();
    let c_renderer = renderer.clone();
    let c_swapchain = swapchain.clone();
    let c_asset_manager = asset_manager.clone();

    std::thread::Builder::new()
      .name("RenderThread".to_string())
      .spawn(move || {
      let mut internal = RendererInternal::new(&c_renderer, &c_device, &c_swapchain, &c_asset_manager, sender, receiver, c_renderer.primary_camera());
      loop {
        if !c_renderer.is_running.load(Ordering::SeqCst) {
          break;
        }
        internal.render();
      }
    }).unwrap();
    renderer
  }

  pub fn primary_camera(&self) -> &Arc<LateLatchCamera<P::GraphicsBackend>> {
    &self.primary_camera
  }

  pub fn set_window_state(&self, window_state: WindowState) {
    let mut guard = self.window_state.lock().unwrap();
    *guard = window_state
  }

  pub fn install(self: &Arc<Renderer<P>>, _world: &mut World, _resources: &mut Resources, systems: &mut Builder) {
    crate::renderer::ecs::install::<P, Arc<Renderer<P>>>(systems, self.clone());
  }

  pub(super) fn window_state(&self) -> &Mutex<WindowState> {
    &self.window_state
  }

  pub(crate) fn change_surface(&self, surface: &Arc<<P::GraphicsBackend as Backend>::Surface>) {
    let mut surface_guard = self.surface.lock().unwrap();
    *surface_guard = surface.clone();
  }
  pub(super) fn surface(&self) -> MutexGuard<Arc<<P::GraphicsBackend as Backend>::Surface>> {
    self.surface.lock().unwrap()
  }

  pub(super) fn dec_queued_frames_counter(&self) -> usize {
    self.queued_frames_counter.fetch_sub(1, Ordering::SeqCst)
  }

  pub(crate) fn instance(&self) -> &Arc<<P::GraphicsBackend as Backend>::Instance> {
    &self.instance
  }

  pub fn stop(&self) {
    self.is_running.store(false, Ordering::SeqCst);
  }
}

impl<P: Platform> RendererInterface for Arc<Renderer<P>> {
  fn register_static_renderable(&self, entity: Entity, transform: &InterpolatedTransform, renderable: &StaticRenderableComponent) {
    let result = self.sender.send(RendererCommand::RegisterStatic {
      entity,
      transform: transform.0,
      model_path: renderable.model_path.to_string(),
      receive_shadows: renderable.receive_shadows,
      cast_shadows: renderable.cast_shadows,
      can_move: renderable.can_move
    });
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  fn unregister_static_renderable(&self, entity: Entity) {
    let result = self.sender.send(RendererCommand::UnregisterStatic(entity));
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  fn register_point_light(&self, entity: Entity, transform: &InterpolatedTransform, component: &PointLightComponent) {
    let result = self.sender.send(RendererCommand::RegisterPointLight {
      entity,
      transform: transform.0,
      intensity: component.intensity
    });
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  fn unregister_point_light(&self, entity: Entity) {
    let result = self.sender.send(RendererCommand::UnregisterPointLight(entity));
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  fn update_camera_transform(&self, camera_transform_mat: Matrix4, fov: f32) {
    let result = self.sender.send(RendererCommand::UpdateCameraTransform { camera_transform_mat, fov });
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  fn update_transform(&self, entity: Entity, transform: Matrix4) {
    let result = self.sender.send(RendererCommand::UpdateTransform { entity, transform_mat: transform });
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  fn end_frame(&self) {
    self.queued_frames_counter.fetch_add(1, Ordering::SeqCst);
    let result = self.sender.send(RendererCommand::EndFrame);
    if result.is_err() {
      panic!("Sending message to render thread failed");
    }
  }

  fn is_saturated(&self) -> bool {
    self.queued_frames_counter.load(Ordering::SeqCst) > 1
  }

  fn is_running(&self) -> bool {
    self.is_running.load(Ordering::SeqCst)
  }
}
