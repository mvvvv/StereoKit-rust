//--name = app/postfx_scanwave
//--scan_distance = 2.5
//--scan_width    = 0.15
//--scan_color    = 0.0, 0.8, 1.0

#include <stereokit.hlsli>

float scan_distance;
float scan_width;
float3 scan_color;

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

    // Reconstruct 3D position in view space
    float2 ndc = float2(input.uv.x * 2 - 1, 1 - input.uv.y * 2);
    float4 view = mul(float4(ndc, d, 1), sk_proj_inv[view_id]);
    float dist = length(view.xyz / view.w);

    // Calculate scanwave ring intensity
    float scan = 1.0 - saturate(abs(dist - scan_distance) / scan_width);
    scan = pow(scan, 2.0); // Sharpen the ring edges

    // Add scan color to original pixel
    float3 finalColor = c.rgb + (scan_color * scan);
    return float4(finalColor, c.a);
}