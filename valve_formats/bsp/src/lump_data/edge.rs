use std::io::{Read, Result as IOResult};
use lump_data::{LumpData, LumpType};
use read_u16;

#[derive(Copy, Clone, Debug, Default)]
pub struct Edge {
  pub vertex_index: [u16; 2]
}

impl LumpData for Edge {
  fn lump_type() -> LumpType {
    LumpType::Edges
  }

  fn element_size(_version: i32) -> usize {
    4
  }

  fn read(reader: &mut dyn Read, _version: i32) -> IOResult<Self> {
    let vertex_index = [
      read_u16(reader)?,
      read_u16(reader)?
    ];
    return Ok(Self {
      vertex_index
    });
  }
}
