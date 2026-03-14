#include "stereokit.hlsli"
#include <stereokit_pbr.hlsli>

//--name = water_pbr
float4 color           = {0, 0, 1, 0.9};
float4 emission_factor = {0.01, 0.07, 0, 01};
float4 tex_trans       = {0, 0, 10.1, 10.1};
float  time            = 5;
float  metallic        = 0.6;
float  roughness       = 1.4;


//--diffuse   = white
Texture2D    diffuse        : register(t0);
SamplerState diffuse_s   : register(s0);
//--emission  = white
Texture2D    emission       : register(t1);
SamplerState emission_s  : register(s1);
//--metal     = white
Texture2D    metal          : register(t2);
SamplerState metal_s     : register(s2);
//--occlusion = white
Texture2D    occlusion      : register(t3);
SamplerState occlusion_s : register(s3);
struct vsIn {
    float4 pos    : SV_Position;
    float3 normal : NORMAL0;
    float2 uv     : TEXCOORD0;
    float4 col    : COLOR0;
};
struct psIn {
    float4 pos       : SV_Position;
    float2 uv        : TEXCOORD0;
    half3  normal    : NORMAL0;
    half4  color     : COLOR0;
    half3  irradiance: COLOR1;
    float3 world     : TEXCOORD1;    
    float3 view_dir  : TEXCOORD2;
};

psIn vs(vsIn input, sk_ids_t ids) {
        psIn o;

        float3x3 world3x3 = (float3x3)sk_inst[ids.inst].world;
        o.world      = mul(input.pos.xyz, world3x3) + sk_inst[ids.inst].world[3].xyz;
        o.pos        = mul(float4(o.world, 1), sk_viewproj[ids.view]);

        o.normal     = normalize(mul(input.normal, world3x3));
        o.uv         = (input.uv * tex_trans.zw) + tex_trans.xy;
        o.color      = input.col * color * sk_inst[ids.inst].color;
        o.irradiance = sk_lighting(o.normal);
        o.view_dir   = sk_camera_pos[ids.view].xyz - o.world;
	return o;
}


float4 ps(psIn input) : SV_TARGET {
    float2 uv = input.uv;
    float offset = time* sk_time/100;
    uv.x += sin (sk_time * time+ (uv.x + uv.y) * 25) * 0.01;
    uv.y += cos (sk_time * time+ (uv.x - uv.y) * 25) * 0.01;
    half4 albedo       = (half4)diffuse.  Sample(diffuse_s,  uv) * input.color;
    
    uv.x += offset;
    uv.y += offset;
    half3 emissive     = (half3)emission .Sample(emission_s, uv).rgb * (half3)emission_factor.rgb;
    half2 metal_rough  = (half2)metal    .Sample(metal_s,    uv * 0.2 ).gb; // rough is g, b is metallic
    half  ao           = (half )occlusion.Sample(occlusion_s,uv * 0.6 ).r;  // occlusion is sometimes part of the metal tex, uses r channel

    half metallic_final = metal_rough.y * (half)metallic;
    half rough_final    = metal_rough.x * (half)roughness;

    float4 color = sk_pbr_shade(albedo, input.irradiance, ao, metallic_final, rough_final, input.view_dir, input.normal);
    color.rgb += (float3)emissive;
    return color;
}
