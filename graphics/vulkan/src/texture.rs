use std::sync::Arc;

use ash::vk;
use ash::version::DeviceV1_0;

use sourcerenderer_base::graphics::Texture;
use sourcerenderer_base::graphics::Format;
use sourcerenderer_base::graphics::RenderTargetView;

use crate::VkDevice;
use crate::format::format_to_vk;
use crate::VkBackend;

pub struct VkTexture {
  image: vk::Image,
  device: Arc<VkDevice>,
  format: Format,
  width: u32,
  height: u32,
  depth: u32,
  mip_levels: u32,
  array_length: u32,
  borrowed: bool
}

pub struct VkRenderTargetView {
  texture: Arc<VkTexture>,
  view: vk::ImageView
}

impl VkTexture {
  pub fn new(device: Arc<VkDevice>) -> Self {
    unimplemented!();
  }

  pub fn from_image(device: Arc<VkDevice>, image: vk::Image, format: Format, width: u32, height: u32, depth: u32, mip_levels: u32, array_length: u32) -> Self {
    return VkTexture {
      image: image,
      device: device,
      format: format,
      width: width,
      height: height,
      depth: depth,
      mip_levels: mip_levels,
      array_length: array_length,
      borrowed: true
    };
  }

  pub fn get_vk_image(&self) -> &vk::Image {
    return &self.image;
  }

  pub fn get_device(&self) -> &VkDevice {
    return &self.device;
  }
}

impl Drop for VkTexture {
  fn drop(&mut self) {
    if self.borrowed {
      return;
    }
    unsafe {
      let vk_device = self.device.get_ash_device();
      println!("DESTORY IMG");
      vk_device.destroy_image(self.image, None);
    }
  }
}

impl Texture<VkBackend> for VkTexture {

}

impl VkRenderTargetView {
  pub fn new(device: Arc<VkDevice>, texture: Arc<VkTexture>) -> Self {
    let vk_device = device.get_ash_device();
    let info = vk::ImageViewCreateInfo {
      image: *texture.get_vk_image(),
      view_type: if texture.depth > 1 { vk::ImageViewType::TYPE_3D } else { vk::ImageViewType::TYPE_2D },
      format: format_to_vk(texture.format),
      components: vk::ComponentMapping {
        r: vk::ComponentSwizzle::IDENTITY,
        g: vk::ComponentSwizzle::IDENTITY,
        b: vk::ComponentSwizzle::IDENTITY,
        a: vk::ComponentSwizzle::IDENTITY,
      },
      subresource_range: vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1
      },
      ..Default::default()
    };
    let view = unsafe { vk_device.create_image_view(&info, None).unwrap() };
    return VkRenderTargetView {
      texture: texture,
      view: view
    };
  }

  pub fn get_handle(&self) -> &vk::ImageView {
    return &self.view;
  }
}

impl Drop for VkRenderTargetView {
  fn drop(&mut self) {
    let vk_device = self.texture.get_device().get_ash_device();
    unsafe {
      vk_device.destroy_image_view(self.view, None);
    }
  }
}

impl RenderTargetView<VkBackend> for VkRenderTargetView {
  fn get_texture(&self) -> Arc<VkTexture> {
    return self.texture.clone();
  }
}