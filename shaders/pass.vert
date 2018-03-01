#version 450

layout(location = 0) in vec4 pos_in;
layout(location = 1) in vec4 color_in;
out gl_PerVertex { vec4 gl_Position; };
layout(location = 0) out vec4 color_out;

void main()
{
    gl_Position = pos_in * 0.5f; gl_Position.w = 1.0;
    color_out = color_in;
}
