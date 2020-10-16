use std::io::{Read, Result as IOResult};
use lump_data::{LumpData, LumpType};
use nalgebra::Vector3;
use read_f32;

#[derive(Clone, Debug)]
pub struct Vertex {
  pub position: Vector3<f32>
}

impl LumpData for Vertex {
  fn lump_type() -> LumpType {
    LumpType::Vertices
  }

  fn element_size(_version: i32) -> usize {
    12
  }

  fn read(reader: &mut dyn Read, _version: i32) -> IOResult<Self> {
    let vec3 = Vector3::<f32>::new(read_f32(reader)?, read_f32(reader)?, read_f32(reader)?);
    return Ok(Self {
      position: vec3
    });
  }
}
