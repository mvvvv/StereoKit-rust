#include "stereokit.hlsli"

//--color:color = 1, 1 , 1, 0.5
//--diffuse     = white
//--cutoff      = 0.35

float4       color;
Texture2D    diffuse   : register(t0);
float        cutoff;
SamplerState diffuse_s : register(s0);


struct vsIn {
    float4 pos    : SV_Position;
    float3 normal : NORMAL0;
    float2 uv     : TEXCOORD0;
    float4 col    : COLOR0;
};
struct psIn : sk_ps_input_t {
    float4 pos       : SV_Position;
    float2 uv        : TEXCOORD0;
    float4 color     : COLOR0;
};

psIn vs(vsIn input, sk_vs_input_t sk_in) {
	psIn o;
	uint view_id = sk_view_init(sk_in, o);
	uint id2     = sk_inst_id  (sk_in);

    float4x4 world_mat = sk_inst[id2].world;
        

    o.pos       = mul(float4(input.pos.xyz, 1), world_mat);
    o.uv        = input.uv;
    o.color     = input.col * color * sk_inst[id2].color;
    return o;
}


float4 ps(psIn input, out float out_depth : SV_DepthGreaterEqual) : SV_TARGET {
    float4 col     = diffuse.Sample(diffuse_s, input.uv);
    out_depth = 0;
    if (col.r < cutoff) discard;

    return input.color;
}
