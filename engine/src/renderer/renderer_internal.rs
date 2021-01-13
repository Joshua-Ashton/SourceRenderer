use std::sync::{Arc, Mutex};
use crate::renderer::Renderer;
use crossbeam_channel::{Sender, Receiver, TryRecvError};
use crate::renderer::command::RendererCommand;
use std::time::{SystemTime, Duration};
use crate::asset::{AssetManager, Asset};
use sourcerenderer_core::{Platform, Matrix4, Vec2, Vec3, Quaternion, Vec2UI, Vec2I};
use sourcerenderer_core::graphics::{Backend, ShaderType, SampleCount, RenderPassTextureExtent, Format, PassInfo, PassType, GraphicsSubpassInfo, SubpassOutput, LoadAction, StoreAction, Device, RenderGraphTemplateInfo, GraphicsPipelineInfo, Swapchain, VertexLayoutInfo, InputAssemblerElement, InputRate, ShaderInputElement, FillMode, CullMode, FrontFace, RasterizerInfo, DepthStencilInfo, CompareFunc, StencilInfo, BlendInfo, LogicOp, AttachmentBlendInfo, BufferUsage, CommandBuffer, PipelineBinding, Viewport, Scissor, BindingFrequency, RenderGraphInfo, RenderGraph, DepthStencilOutput, RenderPassCallbacks, PassInput, ComputeOutput, ExternalOutput, ExternalProducerType, ExternalResource};
use std::collections::{HashMap};
use crate::renderer::{DrawableType, View};
use sourcerenderer_core::platform::WindowState;
use nalgebra::{Matrix3};

use crate::renderer::camera::LateLatchCamera;
use crate::renderer::passes;
use crate::renderer::drawable::{RDrawable, RDrawableType};
use crate::renderer::renderer_assets::*;
use sourcerenderer_bsp::LumpType::TextureInfo;

pub(super) struct RendererInternal<P: Platform> {
  renderer: Arc<Renderer<P>>,
  device: Arc<<P::GraphicsBackend as Backend>::Device>,
  graph: <P::GraphicsBackend as Backend>::RenderGraph,
  swapchain: Arc<<P::GraphicsBackend as Backend>::Swapchain>,
  asset_manager: Arc<AssetManager<P>>,
  view: Arc<Mutex<View<P::GraphicsBackend>>>,
  sender: Sender<RendererCommand>,
  receiver: Receiver<RendererCommand>,
  simulation_tick_rate: u32,
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
    simulation_tick_rate: u32,
    primary_camera: &Arc<LateLatchCamera<P::GraphicsBackend>>) -> Self {

    let renderables = Arc::new(Mutex::new(View::default()));
    let graph = RendererInternal::<P>::build_graph(device, swapchain, &renderables, &primary_camera);

    Self {
      renderer: renderer.clone(),
      device: device.clone(),
      graph,
      swapchain: swapchain.clone(),
      asset_manager: asset_manager.clone(),
      view: renderables,
      sender,
      receiver,
      simulation_tick_rate,
      last_tick: SystemTime::now(),
      primary_camera: primary_camera.clone(),
      assets: RendererAssets::new(device.as_ref())
    }
  }

  fn build_graph(
    device: &Arc<<P::GraphicsBackend as Backend>::Device>,
    swapchain: &Arc<<P::GraphicsBackend as Backend>::Swapchain>,
    renderables: &Arc<Mutex<View<P::GraphicsBackend>>>,
    primary_camera: &Arc<LateLatchCamera<P::GraphicsBackend>>)
    -> <P::GraphicsBackend as Backend>::RenderGraph {

    let passes: Vec<PassInfo> = vec![
      passes::late_latching::build_pass_template::<P::GraphicsBackend>(),
      passes::desktop::geometry::build_pass_template::<P::GraphicsBackend>()
    ];

    let external_resources = vec![
      passes::late_latching::external_resource_template()
    ];

    let graph_template = device.create_render_graph_template(&RenderGraphTemplateInfo {
      external_resources,
      passes,
      swapchain_sample_count: swapchain.sample_count(),
      swapchain_format: swapchain.format()
    });

    let mut callbacks: HashMap<String, RenderPassCallbacks<P::GraphicsBackend>> = HashMap::new();
    let (geometry_pass_name, geometry_pass_callback) = passes::desktop::geometry::build_pass::<P>(device, &graph_template, &renderables);
    callbacks.insert(geometry_pass_name, geometry_pass_callback);

    let (late_latch_pass_name, late_latch_pass_callback) = passes::late_latching::build_pass::<P::GraphicsBackend>(device);
    callbacks.insert(late_latch_pass_name, late_latch_pass_callback);

    let mut external_resources = HashMap::<String, ExternalResource<P::GraphicsBackend>>::new();
    let (camera_resource_name, camera_resource) = passes::late_latching::external_resource(primary_camera);
    external_resources.insert(camera_resource_name, camera_resource);

    let graph = device.create_render_graph(&graph_template, &RenderGraphInfo {
      pass_callbacks: callbacks
    }, swapchain, Some(&external_resources));

    graph
  }

  fn receive_messages(&mut self) {
    let mut guard = self.view.lock().unwrap();

    let message_res = self.receiver.try_recv();
    if let Some(err) = message_res.as_ref().err() {
      if let TryRecvError::Disconnected = err {
        panic!("Rendering channel closed");
      }
    }
    let mut message_opt = message_res.ok();

    while message_opt.is_some() {
      let message = std::mem::replace(&mut message_opt, None).unwrap();
      match message {
        RendererCommand::EndFrame => {
          self.renderer.dec_queued_frames_counter();
          self.last_tick = SystemTime::now();

          for element in &mut guard.elements {
            if let RDrawableType::Static { can_move, .. } = &element.drawable_type {
              if *can_move {
                element.old_transform = element.transform;
              }
            }
          }
          guard.old_camera_transform = guard.camera_transform;
          guard.old_camera_fov = guard.camera_fov;
          break;
        }

        RendererCommand::UpdateCameraTransform { camera_transform_mat, fov } => {
          guard.camera_transform = camera_transform_mat;
          guard.camera_fov = fov;

          let position = camera_transform_mat.column(3).xyz();
          self.primary_camera.update_position(position);
          guard.camera_matrix = self.primary_camera.get_camera();
        }

        RendererCommand::UpdateTransform { entity, transform_mat } => {
          let element = guard.elements.iter_mut()
            .find(|r| r.entity == entity);
          // TODO optimize

          if let Some(element) = element {
            element.transform = transform_mat;
          }
        }

        RendererCommand::Register(drawable) => {
          guard.elements.push(RDrawable {
            drawable_type: match &drawable.drawable_type {
              DrawableType::Static {
                model_path, receive_shadows, cast_shadows, can_move
              } => {
                let model = self.assets.get_model(&self.asset_manager, model_path);
                RDrawableType::Static {
                  model: model,
                  receive_shadows: *receive_shadows,
                  cast_shadows: *cast_shadows,
                  can_move: *can_move
                }
              }
              _ => unimplemented!()
            },
            entity: drawable.entity,
            transform: drawable.transform,
            old_transform: drawable.transform
          });
        }

        RendererCommand::UnregisterStatic(entity) => {
          let index = guard.elements.iter()
            .position(|r| r.entity == entity);

          if let Some(index) = index {
            guard.elements.remove(index);
          }
        }
      }

      let message_res = self.receiver.try_recv();
      if let Some(err) = message_res.as_ref().err() {
        if let TryRecvError::Disconnected = err {
          panic!("Rendering channel closed");
        }
      }
      message_opt = message_res.ok();
    }
  }

  pub(super) fn render(&mut self) {
    let state = {
      let state_guard = self.renderer.window_state().lock().unwrap();
      state_guard.clone()
    };

    let mut swapchain_width = 0u32;
    let mut swapchain_height = 0u32;

    match state {
      WindowState::Minimized => {
        std::thread::sleep(Duration::new(1, 0));
        return;
      },
      WindowState::FullScreen {
        width, height
      } => {
        swapchain_width = width;
        swapchain_height = height;
      },
      WindowState::Visible {
        width, height, focussed: _focussed
      } => {
        swapchain_width = width;
        swapchain_height = height;
      },
      WindowState::Exited => {
        return;
      }
    }

    self.assets.receive_assets(&self.asset_manager);
    self.receive_messages();

    let result = self.graph.render();
    if result.is_err() {
      self.device.wait_for_idle();

      let new_swapchain_result = <P::GraphicsBackend as Backend>::Swapchain::recreate(&self.swapchain, swapchain_width, swapchain_height);
      if new_swapchain_result.is_err() {
        return;
      }
      let new_swapchain = new_swapchain_result.unwrap();
      if new_swapchain.format() != self.swapchain.format() || new_swapchain.sample_count() != self.swapchain.sample_count() {
        panic!("Swapchain format or sample count changed. Can not recreate render graph.");
      }

      let new_graph = <P::GraphicsBackend as Backend>::RenderGraph::recreate(&self.graph, &new_swapchain);
      self.swapchain = new_swapchain;
      self.graph = new_graph;
      let _ = self.graph.render();
    }
  }
}
