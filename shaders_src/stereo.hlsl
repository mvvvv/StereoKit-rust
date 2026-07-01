#include "stereokit.hlsli"

//--name = stereo
// Stereo side-by-side unlit shader: samples a single texture that contains
// the left eye view in its left half, and the right eye view in its right
// half. The proper half is selected per-view in the vertex shader.
float4 color     = {1, 1, 1, 1};
float  tex_scale = 1;

//--diffuse = white
Texture2D    diffuse   : register(t0);
SamplerState diffuse_s : register(s0);


struct vsIn {
    float4 pos    : SV_Position;
    float3 normal : NORMAL0;
    float2 uv     : TEXCOORD0;
    float4 col    : COLOR0;
};
struct psIn {
    float4      pos   : SV_Position;
    float2      uv    : TEXCOORD0;
    min16float4 color : COLOR0;
};

psIn vs(vsIn input, sk_ids_t ids) {
    psIn o;

    float4 world = mul(float4(input.pos.xyz, 1), sk_inst    [ids.inst].world);
    o.pos        = mul(world,                    sk_viewproj[ids.view]);

    // Select the proper horizontal half of the side-by-side stereo texture:
    //   - scale u to the [0, 0.5] range
    //   - shift it to the left  half for view 0 (left  eye)
    //   - shift it to the right half for view 1 (right eye)
    // sk_eye_offset is accounted for to match StereoKit's eye indexing.
    o.uv    = input.uv * tex_scale * float2(0.5, 1)
            + float2((ids.view + sk_eye_offset) * 0.5, 0);
    o.color = (min16float4)(input.col * color * sk_inst[ids.inst].color);
    return o;
}


min16float4 ps(psIn input) : SV_TARGET {
    min16float4 col = (min16float4)diffuse.Sample(diffuse_s, input.uv);
    return col * input.color;
}