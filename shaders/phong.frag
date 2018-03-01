#version 450

layout(location = 0) in vec4 color_in;
layout(location = 0) out vec4 target;

void main() { target = color_in; }
