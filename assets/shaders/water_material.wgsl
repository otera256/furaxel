#import bevy_pbr::{
    mesh_view_bindings::globals,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    prepass_utils::prepass_depth,
    view_transformations::{
        depth_ndc_to_view_z,
        position_ndc_to_world,
    },
}

fn hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453123);
}

fn noise(x: vec2<f32>) -> f32 {
    let p = floor(x);
    let f = fract(x);
    let f2 = f * f * (3.0 - 2.0 * f);
    let n = p.x + p.y * 57.0;
    return mix(mix(hash(n + 0.0), hash(n + 1.0), f2.x),
               mix(hash(n + 57.0), hash(n + 58.0), f2.x), f2.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var f = 0.0;
    var p2 = p;
    f += 0.5000 * noise(p2); p2 *= 2.02;
    f += 0.2500 * noise(p2); p2 *= 2.03;
    f += 0.1250 * noise(p2); p2 *= 2.01;
    f += 0.0625 * noise(p2);
    return f;
}

fn get_wave_height(p: vec2<f32>, delta: f32) -> f32 {
    // 3方向からの波
    let d1 = vec2<f32>(1.0, 0.0);
    let d2 = vec2<f32>(-0.5, 0.866);
    let d3 = vec2<f32>(-0.5, -0.866);

    let v1 = fbm(p + d1 * delta);
    let v2 = fbm(p + d2 * delta);
    let v3 = fbm(p + d3 * delta);

    return (v1 + v2 + v3) / 3.0;
}

struct WaterExtension {
    deep_color: vec4<f32>,
    shallow_color: vec4<f32>,
    depth_scale: f32,
    _padding: vec3<f32>,
}

const WaveScale: f32 = 1.8;
const WaveSpeed: f32 = 0.2;
const WaveStrength: f32 = 0.2;

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> water_ext: WaterExtension;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // 波の法線計算
    let t = globals.time;
    var custom_in = in;
    
    let pos_xz = custom_in.world_position.xz / WaveScale;
    
    // 数値微分で波の高さから法線を計算
    let epsilon = 0.01;
    let delta = t * WaveSpeed;
    let h = get_wave_height(pos_xz, delta);
    let h_x = get_wave_height(pos_xz + vec2(epsilon, 0.0), delta);
    let h_z = get_wave_height(pos_xz + vec2(0.0, epsilon), delta);
    
    let dx = (h - h_x) / epsilon;
    let dz = (h - h_z) / epsilon;
    
    custom_in.world_normal = normalize(vec3<f32>(dx * WaveStrength, 1.0, dz * WaveStrength));
    var pbr_input = pbr_input_from_standard_material(custom_in, is_front);

    var out: FragmentOutput;


    // PBRライティング計算
    out.color = apply_pbr_lighting(pbr_input);

    // 現在のピクセル（水面）の深度情報
    let surface_raw_depth = in.position.z;
    let surface_view_z = depth_ndc_to_view_z(surface_raw_depth);

    // 背景（地形）の深度情報
    let background_raw_depth = prepass_depth(in.position, 0u);
    let background_view_z = depth_ndc_to_view_z(background_raw_depth);

    // 水深の計算
    // BevyのView Zはカメラ前方に向かって負の値なので引き算の順番に注意
    let water_depth = surface_view_z - background_view_z;
    // 念のため
    let safe_depth = max(0.0, water_depth);


    // 水の色設定
    let deep_color = vec4<f32>(0.0, 0.03, 0.2, 0.99);
    let shallow_color = vec4<f32>(0.2, 0.4, 1.0, 0.5);
    let depth_scale = 0.5;

    // let deep_color = water_ext.deep_color;
    // let shallow_color = water_ext.shallow_color;
    // let depth_scale = water_ext.depth_scale;

    // Beer's Lawを適用
    let absorption = exp(-safe_depth * depth_scale);
    let water_color = mix(deep_color, shallow_color, absorption);

    // 半透明合成
    out.color = mix(out.color, water_color, water_color.a);


    return out;
}