#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in vec2 in_lightmap_uv;
layout(location = 4) in float in_alpha;

layout(location = 0) out vec3 out_normal;
layout(location = 1) out vec2 out_uv;
layout(location = 2) out vec2 out_lightmap_uv;

layout(set = 2, binding = 0) uniform LowFrequencyUbo {
  mat4 viewProjection;
};
layout(set = 2, binding = 1) uniform SwapchainTransformLowFrequencyUbo {
  mat4 swapchain_transform;
};

layout(push_constant) uniform VeryHighFrequencyUbo {
  mat4 model;
};

void main(void) {
  out_uv = in_uv;
  out_lightmap_uv = in_lightmap_uv;
  out_normal = in_normal;
  gl_Position = (swapchain_transform * (viewProjection * model)) * vec4(in_pos, 1);
  gl_Position.y = -gl_Position.y;
}
