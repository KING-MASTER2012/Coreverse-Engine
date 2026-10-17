#version 450

// Built-in "colored mesh" vertex shader: position + per-vertex color, transformed by one
// matrix from the push constant block (Mat4 from cv-math, column-major - the same layout as
// GLSL's mat4). Compiled into src/BuiltinShaders.cpp by shaders/generate_builtin_shaders.sh.

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(push_constant) uniform PushConstants
{
    mat4 mvp;
} pc;

layout(location = 0) out vec3 vColor;

void main()
{
    vColor = inColor;
    gl_Position = pc.mvp * vec4(inPosition, 1.0);
}
