struct Camera {
    viewport: mat4x4<f32>,
    transform: mat4x4<f32>,
}

struct VertexOutput {
    @location(0)
    color: vec3<f32>,
    @builtin(position)
    pos: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(
    @location(0) pos: vec3<f32>,
    @location(1) color: vec3<f32>,
) -> VertexOutput {
    let pos = camera.transform * vec4<f32>(pos, 1.0);
    let pos = vec4<f32>(pos.xy / (1.0 + pos.z), 0.0, 1.0);
    var output: VertexOutput;
    output.pos = camera.viewport * pos;
    output.color = color;
    return output;
}

@fragment
fn fs_main(@location(0) color: vec3<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(color, 1.0);
}
