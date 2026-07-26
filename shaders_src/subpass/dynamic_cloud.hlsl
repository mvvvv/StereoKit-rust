//--name = app/postfx_dynamic_cloud_fog
//--base_density   = 0.05
//--noise_scale    = 0.5
//--fog_color      = 0.5, 0.55, 0.6
//--time           = 9.5
//--wind_direction = 1.0, 0.2, 0.5

#include <stereokit.hlsli>

float base_density;
float noise_scale;
float3 fog_color;
float time;
float3 wind_direction;

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

// Pseudo-3D noise function
float hash(float3 p)
{
    p = frac(p * 0.3183099 + float3(0.1, 0.1, 0.1));
    p *= 17.0;
    return frac(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float noise(float3 x)
{
    float3 i = floor(x);
    float3 f = frac(x);
    f = f * f * (3.0 - 2.0 * f);

    return lerp(
        lerp(lerp(hash(i + float3(0, 0, 0)), hash(i + float3(1, 0, 0)), f.x),
             lerp(hash(i + float3(0, 1, 0)), hash(i + float3(1, 1, 0)), f.x), f.y),
        lerp(lerp(hash(i + float3(0, 0, 1)), hash(i + float3(1, 0, 1)), f.x),
             lerp(hash(i + float3(0, 1, 1)), hash(i + float3(1, 1, 1)), f.x), f.y),
        f.z);
}

// Inverts a rigid body transformation view matrix (View -> World)
// Rotation is inverted by transposition; translation is adjusted accordingly.
matrix sk_matrix_view_inverse(matrix v)
{
    // Extract and transpose the 3x3 rotation component
    float3x3 inv_rot = transpose((float3x3)v);

    // Compute world space translation (-R^T * T)
    float3 inv_trans = -mul(inv_rot, v[3].xyz);

    return matrix(
        float4(inv_rot[0], 0.0),
        float4(inv_rot[1], 0.0),
        float4(inv_rot[2], 0.0),
        float4(inv_trans, 1.0));
}

float4 ps(psIn input, uint view_id : SV_ViewID) : SV_TARGET
{
    float4 c = color.SubpassLoad();
    float d = depth.SubpassLoad();

    // 1. Reconstruct view-space 3D position from NDC and linear depth
    float2 ndc = float2(input.uv.x * 2.0 - 1.0, 1.0 - input.uv.y * 2.0);
    float4 view = mul(float4(ndc, d, 1.0), sk_proj_inv[view_id]);
    float3 pos_view = view.xyz / view.w;
    float dist = length(pos_view);

    // 2. Transform position from View Space to World Space
    matrix view_inv = sk_matrix_view_inverse(sk_view[view_id]);
    float3 pos_world = mul(float4(pos_view, 1.0), view_inv).xyz;

    // 3. Compute wind displacement in World Space over time
    float3 wind = normalize(wind_direction) * (time * sk_time);

    // 4. Sample 3D noise using world space coordinates + wind offset
    float3 sample_pos = (pos_world + wind) * noise_scale;
    float n = noise(sample_pos);

    // 5. Combine base density with animated dynamic noise
    float dynamic_density = base_density * (0.4 + 0.6 * n);

    // 6. Calculate standard exponential fog attenuation
    float fog = saturate(exp(-dist * dynamic_density));

    return float4(lerp(fog_color, c.rgb, fog), c.a);
}