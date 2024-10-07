#include "stereokit.hlsli"

//--time        = 1
//--color:color = 1, 0, 0, 1
//--tex_trans   = 0,0,1,1
//--diffuse     = white

float time;
float4 color;
float4 tex_trans;
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
    float3 world     : TEXCOORD1;
    float4 color     : COLOR0;
    uint view_id     : SV_RenderTargetArrayIndex;
};

psIn vs(vsIn input, uint id : SV_InstanceID) {
    psIn o;
    o.view_id = id % sk_view_count;
    id        = id / sk_view_count;

    o.world = mul(float4(input.pos.xyz, 1), sk_inst[id].world).xyz;
    o.pos   = mul(float4(o.world,  1), sk_viewproj[o.view_id]);
    
    o.uv        = (input.uv * tex_trans.zw) + tex_trans.xy;
    o.color     = input.col * color * sk_inst[id].color * abs(sin(sk_time * time % 100));
    return o;
}


float4 ps(psIn input) : SV_TARGET {
    float4 col     = diffuse.Sample(diffuse_s, input.uv);
    return col * input.color;
}
