struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec4<f32>,
    @location(4) rotation: f32,
    @location(5) z: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var quad = array<vec2<f32>, 6>(
        vec2(-0.5, -0.5),
        vec2(0.5, -0.5),
        vec2(0.5, 0.5),
        vec2(-0.5, -0.5),
        vec2(0.5, 0.5),
        vec2(-0.5, 0.5),
    );

    let local_position = quad[vertex_index] * input.size;
    let cosine = cos(input.rotation);
    let sine = sin(input.rotation);
    let rotated_position = vec2(
        local_position.x * cosine - local_position.y * sine,
        local_position.x * sine + local_position.y * cosine,
    ) + input.position;

    var output: VertexOutput;
    output.position = vec4(rotated_position, input.z, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
