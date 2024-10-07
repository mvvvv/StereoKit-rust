#include "stereokit.hlsli"

//--color:color = 1, 1 , 1, 1
//--diffuse     = white

float4       color;
Texture2D    diffuse   : register(t0);
SamplerState diffuse_s : register(s0);


struct vsIn {
    float4 pos    : SV_Position;
    float3 normal : NORMAL0;
    float2 uv     : TEXCOORD0;
    float4 col    : COLOR0;
};
struct psIn {
    float4 pos       : SV_Position;
    float2 uv        : TEXCOORD0;
    float4 color     : COLOR0;
    uint view_id : SV_RenderTargetArrayIndex;
};

psIn vs(vsIn input, uint id : SV_InstanceID) {
    psIn o;
    o.view_id = id % sk_view_count;
    id        = id / sk_view_count;

    float4x4 world_mat = sk_inst[id].world;
        

    o.pos       = mul(float4(input.pos.xyz, 1), world_mat);
    o.uv        = input.uv;
    o.color     = input.col * color * sk_inst[id].color;
    return o;
}


float4 ps(psIn input, out float out_depth : SV_DepthGreaterEqual) : SV_TARGET {
    float4 col     = diffuse.Sample(diffuse_s, input.uv);
    out_depth = 0;

    return col * input.color;
}
