use sourcerenderer_core::Matrix4;

use legion::Entity;
use std::{sync::Arc, usize};
use std::f32;
use sourcerenderer_core::graphics::Backend;
use crate::renderer::renderer_assets::*;

pub struct RendererStaticDrawable<B: Backend> {
  pub entity: Entity,
  pub transform: Matrix4,
  pub old_transform: Matrix4,
  pub model: Arc<RendererModel<B>>,
  pub receive_shadows: bool,
  pub cast_shadows: bool,
  pub can_move: bool
}

#[derive(Clone)]
pub struct View {
  pub view_matrix: Matrix4,
  pub proj_matrix: Matrix4,
  pub old_camera_matrix: Matrix4,
  pub camera_transform: Matrix4,
  pub camera_fov: f32,
  pub near_plane: f32,
  pub far_plane: f32,
  pub drawable_parts: Vec<DrawablePart>
}

impl Default for View {
  fn default() -> Self {
    Self {
      camera_transform: Matrix4::identity(),
      old_camera_matrix: Matrix4::identity(),
      view_matrix: Matrix4::identity(),
      proj_matrix: Matrix4::identity(),
      camera_fov: f32::consts::PI / 2f32,
      near_plane: 0.1f32,
      far_plane: 100f32,
      drawable_parts: Vec::new()
    }
  }
}

#[derive(Clone)]
pub struct DrawablePart {
  pub drawable_index: usize,
  pub part_index: usize
}
