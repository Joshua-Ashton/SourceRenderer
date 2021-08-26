#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec3 in_normal;

layout(location = 0) out vec4 out_position;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec4 out_oldPosition;

layout(set = 2, binding = 0) uniform CurrentLowFrequencyUbo {
    mat4 viewProjection;
};
layout(set = 2, binding = 1) uniform PreviousLowFrequencyUbo {
    mat4 oldViewProjection;
};
layout(set = 2, binding = 2) uniform PerFrameUbo {
    mat4 swapchainTransform;
    vec2 jitterPoint;
};

layout(push_constant) uniform VeryHighFrequencyUbo {
    mat4 model;
    mat4 oldModel;
};

void main(void) {
    vec4 pos = vec4(in_pos, 1);

    mat4 mvp = swapchainTransform * viewProjection * model;
    vec4 transformedPos = mvp * pos;
    transformedPos.y = -transformedPos.y;

    vec4 transformedOldPos = (swapchainTransform * (oldViewProjection * oldModel)) * pos;
    transformedOldPos.y = -transformedOldPos.y;

    // TODO remove scale from model matrix instead
    out_normal = normalize((model * vec4(in_normal, 0)).xyz);
    out_position = transformedPos;
    out_oldPosition = transformedOldPos;

    mat4 jitterMat;
    jitterMat[0] = vec4(1.0, 0.0, 0.0, 0.0);
    jitterMat[1] = vec4(0.0, 1.0, 0.0, 0.0);
    jitterMat[2] = vec4(0.0, 0.0, 1.0, 0.0);
    jitterMat[3] = vec4(jitterPoint.x, jitterPoint.y, 0.0, 1.0);
    vec4 jitteredPoint = (jitterMat * mvp) * pos;
    jitteredPoint.y = -jitteredPoint.y;
    gl_Position = jitteredPoint;
}
