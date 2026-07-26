//--name = app/postfx_depth_vignette
//--vignette_power  = 2.0
//--vignette_smooth = 0.5

#include <stereokit.hlsli>

float vignette_power;
float vignette_smooth;

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
    float d = depth.SubpassLoad();

    // Distance to screen center (0 at center, 1 at corners)
    float2 uvCentered = input.uv * 2.0 - 1.0;
    float distToCenter = length(uvCentered);

    // Smooth radial vignette
    float vig = smoothstep(1.0, 1.0 - vignette_smooth, distToCenter);
    vig = pow(vig, vignette_power);

    // Fade out vignette for far objects (e.g. the skybox)
    float mask = lerp(vig, 1.0, step(0.999, d));

    return float4(c.rgb * mask, c.a);
}