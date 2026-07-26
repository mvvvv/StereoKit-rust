//--name = app/postfx_night_vision
//--nv_color = 0.1, 1.0, 0.2
//--grain_size = 4.0

#include <stereokit.hlsli>

float3 nv_color;
float grain_size; // Grain block size in pixels (e.g. 2.0 = 2x2 pixel blocks)

[[vk::input_attachment_index(0)]] SubpassInput<float4> color;
[[vk::input_attachment_index(1)]] SubpassInput<float> depth;

struct psIn
{
    float4 pos : SV_POSITION;
    float2 uv : TEXCOORD0;
};

psIn vs(uint id : SV_VertexID)
{
    psIn o;
    float2 uv = float2(id & 2, (id << 1) & 2);
    o.pos = float4(uv * float2(2, -2) + float2(-1, 1), 0, 1);
    o.uv = uv;
    return o;
}

float4 ps(psIn input, uint view_id : SV_ViewID) : SV_TARGET
{
    float4 c = color.SubpassLoad();

    // Calculate scene luminance
    float lum = dot(c.rgb, float3(0.299, 0.587, 0.114));

    // Calculate pixel-aligned blocky UVs
    // sk_lighting_sh_sk_screen_size gives screen dimensions (width, height)
    // Grain size controls the width/height of the grain blocks in pixels
    float2 screen_res = sk_screen_size.xy;
    float2 pixel_coord = input.uv * screen_res;
    float2 block_uv = floor(pixel_coord / max(grain_size, 1.0)) / screen_res;

    // Dynamic noise based on quantized block UVs
    float time_seed = frac(sk_time.x);
    float noise = frac(sin(dot(block_uv + time_seed, float2(12.9898, 78.233))) * 43758.5453);

    // Apply green tinting, amplification, and noise overlay
    float3 vision = (lum * 1.5 + noise * 0.1) * nv_color;

    return float4(vision, c.a);
}