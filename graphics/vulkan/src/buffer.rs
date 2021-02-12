use std::{fmt::Formatter, sync::{Arc, Mutex}};
use std::cmp::max;
use std::hash::{Hash, Hasher};
use std::collections::{VecDeque, BTreeMap};
use std::fmt::Debug;

use sourcerenderer_core::graphics::{Buffer, BufferUsage, MemoryUsage, MappedBuffer, MutMappedBuffer};

use ash::{version::InstanceV1_1, vk};

use crate::raw::*;
use crate::device::memory_usage_to_vma;
use smallvec::SmallVec;

pub struct VkBuffer {
  buffer: vk::Buffer,
  allocation: vk_mem::Allocation,
  allocation_info: vk_mem::AllocationInfo,
  device: Arc<RawVkDevice>,
  map_ptr: Option<*mut u8>,
  is_coherent: bool,
  memory_usage: MemoryUsage,
  buffer_usage: BufferUsage,
  slice_size: usize,
  slices: Mutex<VecDeque<VkBufferSlice>>
}

unsafe impl Send for VkBuffer {}
unsafe impl Sync for VkBuffer {}

impl VkBuffer {
  pub fn new(device: &Arc<RawVkDevice>, slice_size: usize, slices: usize, memory_usage: MemoryUsage, buffer_usage: BufferUsage, allocator: &vk_mem::Allocator) -> Arc<Self> {
    let mut queue_families = SmallVec::<[u32; 2]>::new();
    let mut sharing_mode = vk::SharingMode::EXCLUSIVE;
    if buffer_usage.contains(BufferUsage::COPY_SRC) {
      queue_families.push(device.graphics_queue_info.queue_family_index as u32);
      if let Some(info) = device.transfer_queue_info {
        sharing_mode = vk::SharingMode::CONCURRENT;
        queue_families.push(info.queue_family_index as u32);
      }
    }

    let buffer_info = vk::BufferCreateInfo {
      size: (slice_size * slices) as u64,
      usage: buffer_usage_to_vk(buffer_usage),
      sharing_mode,
      p_queue_family_indices: queue_families.as_ptr(),
      queue_family_index_count: queue_families.len() as u32,
      ..Default::default()
    };
    let allocation_info = vk_mem::AllocationCreateInfo {
      usage: memory_usage_to_vma(memory_usage),
      ..Default::default()
    };
    let (buffer, allocation, allocation_info) = allocator.create_buffer(&buffer_info, &allocation_info).expect("Failed to create buffer.");

    let map_ptr: Option<*mut u8> = if memory_usage != MemoryUsage::GpuOnly {
      Some(allocator.map_memory(&allocation).unwrap())
    } else {
      None
    };

    let is_coherent = if memory_usage != MemoryUsage::GpuOnly {
      let memory_type = allocation_info.get_memory_type();
      let memory_properties = allocator.get_memory_type_properties(memory_type).unwrap();
      memory_properties.intersects(vk::MemoryPropertyFlags::HOST_COHERENT)
    } else {
      false
    };

    let buffer = Arc::new(VkBuffer {
      buffer,
      allocation,
      allocation_info,
      device: device.clone(),
      map_ptr,
      is_coherent,
      memory_usage,
      buffer_usage,
      slice_size,
      slices: Mutex::new(VecDeque::with_capacity(slices))
    });

    {
      let mut slices_guard = buffer.slices.lock().unwrap();
      for i in 0..slices {
        slices_guard.push_back(VkBufferSlice {
          buffer: buffer.clone(),
          offset: i * slice_size,
          length: slice_size
        });
      }
    }

    buffer
  }

  pub fn get_handle(&self) -> &vk::Buffer {
    return &self.buffer;
  }

  fn return_slice(&self, slice: VkBufferSlice) {
    let mut guard = self.slices.lock().unwrap();
    guard.push_back(slice);
  }
}

impl Drop for VkBuffer {
  fn drop(&mut self) {
    self.device.allocator.destroy_buffer(self.buffer, &self.allocation).unwrap();
  }
}

impl Hash for VkBuffer {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.buffer.hash(state);
  }
}

impl PartialEq for VkBuffer {
  fn eq(&self, other: &Self) -> bool {
    self.buffer == other.buffer
  }
}

impl Eq for VkBuffer {}

impl Buffer for VkBufferSlice {
  fn map_mut<T>(&self) -> Option<MutMappedBuffer<Self, T>>
    where T: 'static + Send + Sync + Sized + Clone {
    MutMappedBuffer::new(self, true)
  }

  fn map<T>(&self) -> Option<MappedBuffer<Self, T>>
    where T: 'static + Send + Sync + Sized + Clone {
    MappedBuffer::new(self, true)
  }

  unsafe fn map_unsafe(&self, invalidate: bool) -> Option<*mut u8> {
    if !self.buffer.is_coherent &&
      (self.buffer.memory_usage == MemoryUsage::CpuToGpu
        || self.buffer.memory_usage == MemoryUsage::CpuOnly
        || self.buffer.memory_usage == MemoryUsage::GpuToCpu) {
      let allocator = &self.buffer.device.allocator;
      if invalidate {
        allocator.invalidate_allocation(&self.buffer.allocation, self.buffer.allocation_info.get_offset() + self.offset, self.length).unwrap();
      }
    }
    return self.buffer.map_ptr.map(|ptr| ptr.add(self.offset));
  }

  unsafe fn unmap_unsafe(&self, flush: bool) {
    if !self.buffer.is_coherent &&
      (self.buffer.memory_usage == MemoryUsage::CpuToGpu
        || self.buffer.memory_usage == MemoryUsage::CpuOnly
        || self.buffer.memory_usage == MemoryUsage::GpuToCpu) {
      let allocator = &self.buffer.device.allocator;
      if flush {
        allocator.flush_allocation(&self.buffer.allocation, self.buffer.allocation_info.get_offset() + self.offset, self.length).unwrap();
      }
    }
  }

  fn get_length(&self) -> usize {
    self.length
  }
}

pub fn buffer_usage_to_vk(usage: BufferUsage) -> vk::BufferUsageFlags {
  use self::vk::BufferUsageFlags as VkUsage;
  let usage_bits = usage.bits();
  let mut flags = 0u32;
  flags |= usage_bits.rotate_left(VkUsage::VERTEX_BUFFER.as_raw().trailing_zeros() - BufferUsage::VERTEX.bits().trailing_zeros()) & VkUsage::VERTEX_BUFFER.as_raw();
  flags |= usage_bits.rotate_left(VkUsage::INDEX_BUFFER.as_raw().trailing_zeros() - BufferUsage::INDEX.bits().trailing_zeros()) & VkUsage::INDEX_BUFFER.as_raw();
  flags |= usage_bits.rotate_left(VkUsage::UNIFORM_BUFFER.as_raw().trailing_zeros() - BufferUsage::CONSTANT.bits().trailing_zeros()) & VkUsage::UNIFORM_BUFFER.as_raw();
  flags |= usage_bits.rotate_left(VkUsage::INDIRECT_BUFFER.as_raw().trailing_zeros() - BufferUsage::INDIRECT.bits().trailing_zeros()) & VkUsage::INDIRECT_BUFFER.as_raw();
  flags |= usage_bits.rotate_left(VkUsage::STORAGE_BUFFER.as_raw().trailing_zeros() - BufferUsage::STORAGE.bits().trailing_zeros()) & VkUsage::STORAGE_BUFFER.as_raw();
  flags |= usage_bits.rotate_right(BufferUsage::STORAGE_TEXEL.bits().trailing_zeros() - VkUsage::STORAGE_TEXEL_BUFFER.as_raw().trailing_zeros()) & VkUsage::STORAGE_TEXEL_BUFFER.as_raw();
  flags |= usage_bits.rotate_right(BufferUsage::UNIFORM_TEXEL.bits().trailing_zeros() - VkUsage::UNIFORM_TEXEL_BUFFER.as_raw().trailing_zeros()) & VkUsage::UNIFORM_TEXEL_BUFFER.as_raw();
  flags |= usage_bits.rotate_right(BufferUsage::COPY_SRC.bits().trailing_zeros() - VkUsage::TRANSFER_SRC.as_raw().trailing_zeros()) & VkUsage::TRANSFER_SRC.as_raw();
  flags |= usage_bits.rotate_right(BufferUsage::COPY_DST.bits().trailing_zeros() - VkUsage::TRANSFER_DST.as_raw().trailing_zeros()) & VkUsage::TRANSFER_DST.as_raw();
  return vk::BufferUsageFlags::from_raw(flags);
}

fn align_up(value: usize, alignment: usize) -> usize {
  if alignment == 0 {
    return value
  }
  if value == 0 {
    return 0
  }
  (value + alignment - 1) & !(alignment - 1)
}

fn align_down(value: usize, alignment: usize) -> usize {
  if alignment == 0 {
    return value
  }
  (value / alignment) * alignment
}

fn align_up_32(value: u32, alignment: u32) -> u32 {
  if alignment == 0 {
    return value
  }
  if value == 0 {
    return 0
  }
  (value + alignment - 1) & (alignment - 1)
}

fn align_down_32(value: u32, alignment: u32) -> u32 {
  if alignment == 0 {
    return value
  }
  (value / alignment) * alignment
}

fn align_up_64(value: u64, alignment: u64) -> u64 {
  if alignment == 0 {
    return value
  }
  (value + alignment - 1) & (alignment - 1)
}

fn align_down_64(value: u64, alignment: u64) -> u64 {
  if alignment == 0 {
    return value
  }
  (value / alignment) * alignment
}

pub struct VkBufferSlice {
  buffer: Arc<VkBuffer>,
  offset: usize,
  length: usize
}

impl Drop for VkBufferSlice {
  fn drop(&mut self) {
    self.buffer.return_slice(VkBufferSlice {
      buffer: self.buffer.clone(),
      offset: self.offset,
      length: self.length
    });
  }
}

impl Debug for VkBufferSlice {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "(Buffer Slice: {}-{} (length: {}))", self.offset, self.offset + self.length, self.length)
  }
}

impl Hash for VkBufferSlice {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.buffer.hash(state);
    self.offset.hash(state);
    self.length.hash(state);
  }
}

impl PartialEq for VkBufferSlice {
  fn eq(&self, other: &Self) -> bool {
    self.buffer == other.buffer
      && self.length == other.length
      && self.offset == other.offset
  }
}

impl Eq for VkBufferSlice {}

impl VkBufferSlice {
  pub fn get_buffer(&self) -> &Arc<VkBuffer> {
    &self.buffer
  }

  pub fn get_offset_and_length(&self) -> (usize, usize) {
    (self.offset, self.length)
  }

  pub fn get_offset(&self) -> usize {
    self.offset
  }

  pub fn get_length(&self) -> usize {
    self.length
  }
}

const SLICED_BUFFER_SIZE: usize = 16384;
const BIG_BUFFER_SLAB_SIZE: usize = 4096;
const BUFFER_SLAB_SIZE: usize = 1024;
const SMALL_BUFFER_SLAB_SIZE: usize = 512;
const TINY_BUFFER_SLAB_SIZE: usize = 256;

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct BufferKey {
  memory_usage: MemoryUsage,
  buffer_usage: BufferUsage
}

pub struct BufferAllocator {
  device: Arc<RawVkDevice>,
  buffers: Mutex<BTreeMap<BufferKey, Vec<Arc<VkBuffer>>>>,
  device_limits: vk::PhysicalDeviceLimits
}

impl BufferAllocator {
  pub fn new(device: &Arc<RawVkDevice>) -> Self {
    let buffers: BTreeMap<BufferKey, Vec<Arc<VkBuffer>>> = BTreeMap::new();
    let mut limits2 = vk::PhysicalDeviceProperties2 {
      ..Default::default()
    };

    unsafe {
      device.instance.get_physical_device_properties2(device.physical_device, &mut limits2)
    }

    BufferAllocator {
      device: device.clone(),
      buffers: Mutex::new(buffers),
      device_limits: limits2.properties.limits.clone()
    }
  }

  pub fn get_slice(&self, memory_usage: MemoryUsage, buffer_usage: BufferUsage, length: usize) -> VkBufferSlice {
    if length > BIG_BUFFER_SLAB_SIZE {
      let buffer = VkBuffer::new(&self.device, length, 1, memory_usage, buffer_usage, &self.device.allocator);
      let mut guard = buffer.slices.lock().unwrap();
      let slice = guard.pop_front().unwrap();
      return slice;
    }

    let mut alignment: usize = 4;
    if (buffer_usage & BufferUsage::CONSTANT) == BufferUsage::CONSTANT {
      // TODO max doesnt guarantee both alignments
      alignment = max(alignment, self.device_limits.min_uniform_buffer_offset_alignment as usize);
    }
    if (buffer_usage & BufferUsage::STORAGE) == BufferUsage::STORAGE {
      // TODO max doesnt guarantee both alignments
      alignment = max(alignment, self.device_limits.min_storage_buffer_offset_alignment as usize);
    }

    let key = BufferKey { memory_usage, buffer_usage };
    let mut guard = self.buffers.lock().unwrap();
    let matching_buffers = guard.entry(key).or_default();
    for buffer in matching_buffers.iter() {
      if buffer.slice_size % alignment == 0 && buffer.slice_size > length {
        let mut slices = buffer.slices.lock().unwrap();
        if let Some(slice) = slices.pop_front() {
          return slice;
        }
      }
    }

    let mut slice_size = max(length, alignment);
    slice_size = if slice_size <= TINY_BUFFER_SLAB_SIZE {
      TINY_BUFFER_SLAB_SIZE
    } else if length <= SMALL_BUFFER_SLAB_SIZE {
      SMALL_BUFFER_SLAB_SIZE
    } else if length <= BUFFER_SLAB_SIZE {
      BUFFER_SLAB_SIZE
    } else {
      BIG_BUFFER_SLAB_SIZE
    };

    let buffer = VkBuffer::new(&self.device, slice_size, SLICED_BUFFER_SIZE / slice_size, memory_usage, buffer_usage, &self.device.allocator);
    let slice = {
      let mut buffer_guard = buffer.slices.lock().unwrap();
      buffer_guard.pop_front().unwrap()
    };
    matching_buffers.push(buffer);
    slice
  }
}
