use std::sync::{Arc, Mutex};
use crate::renderer::{Renderer, RendererStaticDrawable};
use crossbeam_channel::{Receiver, Sender};
use crate::renderer::command::RendererCommand;
use std::time::{SystemTime, Duration};
use crate::asset::AssetManager;
use sourcerenderer_core::{Platform, Vec4};
use sourcerenderer_core::graphics::{SwapchainError, Backend,Swapchain, Device};
use crate::renderer::View;
use sourcerenderer_core::platform::WindowState;
use smallvec::SmallVec;
use crate::renderer::camera::LateLatchCamera;
use crate::renderer::drawable::DrawablePart;
use crate::renderer::renderer_assets::*;
use sourcerenderer_core::atomic_refcell::AtomicRefCell;
use rayon::prelude::*;
use crate::math::Frustum;

use super::PointLight;
use super::passes::desktop::desktop_renderer::DesktopRenderer;
use super::render_path::RenderPath;
use super::renderer_scene::RendererScene;

pub(super) struct RendererInternal<P: Platform> {
  renderer: Arc<Renderer<P>>,
  device: Arc<<P::GraphicsBackend as Backend>::Device>,
  swapchain: Arc<<P::GraphicsBackend as Backend>::Swapchain>,
  render_path: Box<dyn RenderPath<P::GraphicsBackend>>,
  asset_manager: Arc<AssetManager<P>>,
  lightmap: Arc<RendererTexture<P::GraphicsBackend>>,
  scene: Arc<AtomicRefCell<RendererScene<P::GraphicsBackend>>>,
  view: Arc<AtomicRefCell<View>>,
  sender: Sender<RendererCommand>,
  receiver: Receiver<RendererCommand>,
  last_tick: SystemTime,
  primary_camera: Arc<LateLatchCamera<P::GraphicsBackend>>,
  assets: RendererAssets<P>
}

impl<P: Platform> RendererInternal<P> {
  pub(super) fn new(
    renderer: &Arc<Renderer<P>>,
    device: &Arc<<P::GraphicsBackend as Backend>::Device>,
    swapchain: &Arc<<P::GraphicsBackend as Backend>::Swapchain>,
    asset_manager: &Arc<AssetManager<P>>,
    sender: Sender<RendererCommand>,
    receiver: Receiver<RendererCommand>,
    primary_camera: &Arc<LateLatchCamera<P::GraphicsBackend>>) -> Self {

    let mut assets = RendererAssets::new(device);
    let lightmap = assets.insert_placeholder_texture("lightmap");

    let scene = Arc::new(AtomicRefCell::new(RendererScene::new()));
    let view = Arc::new(AtomicRefCell::new(View::default()));

    let path = Box::new(DesktopRenderer::new::<P>(device, swapchain));

    Self {
      renderer: renderer.clone(),
      device: device.clone(),
      render_path: path,
      swapchain: swapchain.clone(),
      scene,
      asset_manager: asset_manager.clone(),
      view,
      sender,
      receiver,
      last_tick: SystemTime::now(),
      primary_camera: primary_camera.clone(),
      assets,
      lightmap
    }
  }

  fn receive_messages(&mut self) {
    let mut scene = self.scene.borrow_mut();
    let mut view = self.view.borrow_mut();

    let message_res = self.receiver.recv();
    if message_res.is_err() {
      panic!("Rendering channel closed");
    }
    let mut message_opt = message_res.ok();

    while message_opt.is_some() {
      let message = message_opt.take().unwrap();
      match message {
        RendererCommand::EndFrame => {
          self.last_tick = SystemTime::now();
          break;
        }

        RendererCommand::UpdateCameraTransform { camera_transform_mat, fov } => {
          view.camera_transform = camera_transform_mat;
          view.camera_fov = fov;

          view.old_camera_matrix = view.proj_matrix * view.view_matrix;
          let position = camera_transform_mat.column(3).xyz();
          self.primary_camera.update_position(position);
          view.view_matrix = self.primary_camera.view();
          view.proj_matrix = self.primary_camera.proj();
        }

        RendererCommand::UpdateTransform { entity, transform_mat } => {
          scene.update_transform(&entity, transform_mat);
        }

        RendererCommand::RegisterStatic {
          model_path, entity, transform, receive_shadows, cast_shadows, can_move
         } => {
          let model = self.assets.get_model(&model_path);
          scene.add_static_drawable(entity, RendererStaticDrawable::<P::GraphicsBackend> {
            entity,
            transform,
            old_transform: transform,
            model,
            receive_shadows,
            cast_shadows,
            can_move
          });
        }

        RendererCommand::UnregisterStatic(entity) => {
          scene.remove_static_drawable(&entity);
        }
        RendererCommand::RegisterPointLight {
          entity,
          transform,
          intensity
        } => {
          scene.add_point_light(entity, PointLight {
            position: (transform * Vec4::new(0f32, 0f32, 0f32, 1f32)).xyz(),
            intensity,
          });
        },
        RendererCommand::UnregisterPointLight(entity) => {
          scene.remove_point_light(&entity);
        },
      }

      let message_res = self.receiver.recv();
      if message_res.is_err() {
        panic!("Rendering channel closed");
      }
      message_opt = message_res.ok();
    }
  }

  pub(super) fn render(&mut self) {
    let state = {
      let state_guard = self.renderer.window_state().lock().unwrap();
      state_guard.clone()
    };

    let (swapchain_width, swapchain_height) = match state {
      WindowState::Minimized => {
        std::thread::sleep(Duration::new(1, 0));
        return;
      },
      WindowState::FullScreen {
        width, height
      } => {
        (width, height)
      },
      WindowState::Visible {
        width, height, focussed: _focussed
      } => {
        (width, height)
      },
      WindowState::Exited => {
        self.renderer.stop();
        return;
      }
    };

    self.assets.receive_assets(&self.asset_manager);
    self.receive_messages();
    self.update_visibility();
    self.reorder();

    let render_result = self.render_path.render(&self.scene, &self.view, &self.lightmap, &self.primary_camera);
    if let Err(swapchain_error) = render_result {
      self.device.wait_for_idle();

      let new_swapchain = if swapchain_error == SwapchainError::SurfaceLost {
        // No point in trying to recreate with the old surface
        let renderer_surface = self.renderer.surface();
        if &*renderer_surface != self.swapchain.surface() {
          println!("Recreating swapchain on a different surface");
          let new_swapchain_result = <P::GraphicsBackend as Backend>::Swapchain::recreate_on_surface(&self.swapchain, &*renderer_surface, swapchain_width, swapchain_height);
          if new_swapchain_result.is_err() {
            println!("Swapchain recreation failed: {:?}", new_swapchain_result.err().unwrap());
            return;
          }
          new_swapchain_result.unwrap()
        } else {
          return;
        }
      } else {
        println!("Recreating swapchain");
        let new_swapchain_result = <P::GraphicsBackend as Backend>::Swapchain::recreate(&self.swapchain, swapchain_width, swapchain_height);
        if new_swapchain_result.is_err() {
          println!("Swapchain recreation failed: {:?}", new_swapchain_result.err().unwrap());
          return;
        }
        new_swapchain_result.unwrap()
      };
      self.render_path.on_swapchain_changed(&new_swapchain);
      self.render_path.render(&self.scene, &self.view, &self.lightmap, &self.primary_camera).expect("Rendering still fails after recreating swapchain.");
      self.swapchain = new_swapchain;
    }
    self.renderer.dec_queued_frames_counter();
  }

  fn update_visibility(&mut self) {
    let scene = self.scene.borrow();
    let static_meshes = scene.static_drawables();

    let mut view_mut = self.view.borrow_mut();

    let mut existing_parts = std::mem::replace(&mut view_mut.drawable_parts, Vec::new());
    // take out vector, creating a new one doesn't allocate until we push an element to it.
    existing_parts.clear();
    let visible_parts = Mutex::new(existing_parts);

    let frustum = Frustum::new(self.primary_camera.z_near(), self.primary_camera.z_far(), self.primary_camera.fov(), self.primary_camera.aspect_ratio());
    let camera_matrix = self.primary_camera.view();
    const CHUNK_SIZE: usize = 64;
    static_meshes.par_chunks(CHUNK_SIZE).enumerate().for_each(|(chunk_index, chunk)| {
      let mut chunk_visible_parts = SmallVec::<[DrawablePart; 64]>::new();
      for (index, static_mesh) in chunk.iter().enumerate() {
        let model_view_matrix = camera_matrix * static_mesh.transform;
        let model = &static_mesh.model;
        let bounding_box = &model.mesh.bounding_box;
        if let Some(bounding_box) = bounding_box {
          let is_visible = frustum.intersects(bounding_box, &model_view_matrix);
          if !is_visible {
            continue;
          }
          let drawable_index = chunk_index * CHUNK_SIZE + index;
          for part_index in 0..model.mesh.parts.len() {
            if chunk_visible_parts.len() == chunk_visible_parts.capacity() {
              let mut global_parts = visible_parts.lock().unwrap();
              global_parts.extend_from_slice(&chunk_visible_parts[..]);
              chunk_visible_parts.clear();
            }

            chunk_visible_parts.push(DrawablePart {
              drawable_index,
              part_index
            });
          }
        }
      }

      let mut global_parts = visible_parts.lock().unwrap();
      global_parts.extend_from_slice(&chunk_visible_parts[..]);
      chunk_visible_parts.clear();
    });

    view_mut.drawable_parts = visible_parts.into_inner().unwrap();
  }

  fn reorder(&mut self) {
    let scene = self.scene.borrow();
    let static_meshes = scene.static_drawables();

    let mut view_mut = self.view.borrow_mut();
    view_mut.drawable_parts.sort_by(|a, b| {
      // if the drawable index is greater than the amount of static meshes, it is a skinned mesh
      let b_is_skinned = a.drawable_index > static_meshes.len();
      let a_is_skinned = a.drawable_index > static_meshes.len();
      return if b_is_skinned && a_is_skinned {
        unimplemented!()
      } else if b_is_skinned {
        std::cmp::Ordering::Less
      } else if a_is_skinned {
        std::cmp::Ordering::Greater
      } else {
        let static_mesh_a = &static_meshes[a.drawable_index];
        let static_mesh_b = &static_meshes[b.drawable_index];
        let material_a = &static_mesh_a.model.materials[a.part_index];
        let material_b = &static_mesh_b.model.materials[b.part_index];
        material_a.cmp(material_b)
      }
    });
  }
}

impl<P: Platform> Drop for RendererInternal<P> {
  fn drop(&mut self) {
    self.renderer.stop();
  }
}
