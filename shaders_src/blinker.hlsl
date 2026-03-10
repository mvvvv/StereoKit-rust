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
    half4  color     : COLOR0;
SK_LAYER_OUTPUT
};

psIn vs(vsIn input, sk_input_t sys) {
    psIn o;
    sk_ids_t ids = sk_resolve_ids(sys);

    float3x3 world3x3 = (float3x3)sk_inst[ids.inst].world;
    o.world = mul(input.pos.xyz, world3x3) + sk_inst[ids.inst].world[3].xyz;
    o.pos   = mul(float4(o.world, 1), sk_viewproj[ids.view]);
    
    o.uv        = (input.uv * tex_trans.zw) + tex_trans.xy;
    o.color     = input.col * color * sk_inst[ids.inst].color * abs(sin(sk_time * time % 100));
    SK_SET_LAYER(o, ids.view);
    return o;
}


float4 ps(psIn input) : SV_TARGET {
    half4 col = (half4)diffuse.Sample(diffuse_s, input.uv);
    return (float4)(col * input.color);
}
