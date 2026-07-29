#include "stereokit.hlsli"

//--name = stereo_array
// Stereo shader using a texture array: samples from a Texture2DArray where array slice 0 contains the left eye view, 
// and slice 1 contains the right eye view. The proper slice is selected per-view automatically by StereoKit.
float4 color     = {1, 1, 1, 1};
float  tex_scale = 1;

//--diffuse = white
Texture2DArray diffuse   : register(t0);
SamplerState   diffuse_s : register(s0);


struct vsIn {
    float4 pos    : SV_Position;
    float3 normal : NORMAL0;
    float2 uv     : TEXCOORD0;
    float4 col    : COLOR0;
};
struct psIn {
    float4      pos        : SV_Position;
    float2      uv         : TEXCOORD0;
    uint        array_idx  : TEXCOORD1;
    min16float4 color      : COLOR0;
};

psIn vs(vsIn input, sk_ids_t ids) {
    psIn o;

    float4 world = mul(float4(input.pos.xyz, 1), sk_inst    [ids.inst].world);
    o.pos        = mul(world,                    sk_viewproj[ids.view]);

    // Use the full UV coordinates (no horizontal splitting needed anymore)
    o.uv    = input.uv * tex_scale;
    
    // Select the correct array slice based on the eye:
    // Left eye (view 0) -> slice 0, Right eye (view 1) -> slice 1
    o.array_idx = ids.view + sk_eye_offset;
    
    o.color = (min16float4)(input.col * color * sk_inst[ids.inst].color);
    return o;
}


min16float4 ps(psIn input) : SV_TARGET {
    // Sample from the correct array slice (eye-specific view)
    float3 uvw = float3(input.uv, input.array_idx);
    min16float4 col = (min16float4)diffuse.Sample(diffuse_s, uvw);
    return col * input.color;
}
