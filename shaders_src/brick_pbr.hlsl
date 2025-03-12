#include <stereokit.hlsli>
#include <stereokit_pbr.hlsli>
//!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// This shader is for test only and shouldn't be used as regular material
// as it is insanely expensive and lack of MSAA
// adapted from https://www.shadertoy.com/view/wt3Sz4
//!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
//--name                  = the_name_of_brick_pbr
//--color:color           = 0.45,0.29,0.23,1
//--line_color:color      = 0.84,0.84,0.84
//--edge_pos              = 1.5
//--metallic              = 0
//--roughness             = 1
//--tex_trans             = 0,0,0.1,0.1
//--use_occlusion         = false
//--size_factors          = 300,-100,50,25
//--edge_limit            = 0.1,0.9
//!!shadertoy
//!!hlsl
float4  color;
float3  line_color;
float   edge_pos;
float   metallic;
float   roughness;
float4  tex_trans;
bool    use_occlusion;
int4    size_factors;
matrix  useless;
float2  edge_limit;

//--metal     = white
//--occlusion = white
Texture2D    metal       : register(t0);
SamplerState metal_s     : register(s0);
Texture2D    occlusion   : register(t1);
SamplerState occlusion_s : register(s1);

float mixcolor (float x, float y)
{
    return sin(   2.*cos(x/size_factors[1]) - 5.*sin(y/size_factors[1]) 
                + 7.*cos(x/size_factors[2]) - 11.*sin(y/size_factors[2])
                + 13.*cos(x/size_factors[3]) - 17.*sin(y/size_factors[3]))*0.1;
}

//brick_color function
float3 brick_color(float2 uv)
{
    //grid and coord inside each cell
    float2 coord = floor(uv);
    float2 gv = frac(uv);
    

    float moving_value = -mixcolor(coord.x , coord.y);

    float offset = floor(uv.y % 2.0)*(edge_pos);
    float vertical_edge = abs(cos(uv.x + offset));
    
    //color of the bricks
    float3 brick = color.rgb - moving_value;
    
    
    bool vrt_edge = step( 1. - 0.01, vertical_edge) == 1.;
    bool hrt_edge = gv.y > (edge_limit[1]) || gv.y < (edge_limit[0]);
    
    if(hrt_edge || vrt_edge)  
        return line_color;
    return brick;
}

//normal functions
float lum(float2 uv) {
    float3 rgb = brick_color(uv);
    return 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
}

float3 normal(float2 uv) {
    
    //edge size
    float r = 0.03;
    
    float x0 = lum(float2(uv.x + r, uv.y));
    float x1 = lum(float2(uv.x - r, uv.y));
    float y0 = lum(float2(uv.x, uv.y - r));
    float y1 = lum(float2(uv.x, uv.y + r));
    
    //Controls the "smoothness"
    float s = 1.0;
    float3 n = normalize(float3(x1 - x0, y1 - y0, s));

    float3 p = float3(uv * 2.0 - 1.0, 0.0);
    float3 v = float3(0.0, 0.0, 1.0);

    float3 l = v - p;
    float d_sqr = l.x * l.x + l.y * l.y + l.z * l.z;
    l *= (1.0 / sqrt(d_sqr));

    float3 h = normalize(l + v);

    float dot_nl = clamp(dot(n, l), 0.0, 1.0);
    float dot_nh = clamp(dot(n, h), 0.0, 1.0);

    float color = lum(uv) * pow(dot_nh, 14.0) * dot_nl * (1.0 / d_sqr);
    color = pow(abs(color), 1.0 / 2.2);

    return (n * 0.5 + 0.5);
 
}   

struct vsIn {
    float4 pos     : SV_Position;
    float3 norm    : NORMAL0;
    float2 uv      : TEXCOORD0;
    float4 color   : COLOR0;
};
struct psIn : sk_ps_input_t {
    float4 pos     : SV_POSITION;
    float3 normal  : NORMAL0;
    float2 uv      : TEXCOORD0;
    float3 irradiance: COLOR1;
    float3 world   : TEXCOORD2;
    float3 view_dir: TEXCOORD3;
};

psIn vs(vsIn input, sk_vs_input_t sk_in) {
	psIn o;
	uint view_id = sk_view_init(sk_in, o);
	uint id      = sk_inst_id  (sk_in);

    o.world = mul(float4(input.pos.xyz, 1), sk_inst[id].world).xyz;
    o.pos   = mul(float4(o.world,  1), sk_viewproj[view_id]);

    o.normal     = normalize(mul(float4(input.norm, 0), sk_inst[id].world).xyz);
    o.uv         = (input.uv * tex_trans.zw) + tex_trans.xy;
    o.irradiance = sk_lighting(o.normal);
    o.view_dir   = sk_camera_pos[view_id].xyz - o.world;
    return o;
}

float4 ps(psIn input) : SV_TARGET {
    
    float4 albedo = float4(brick_color(input.uv * size_factors[0]), 1.0);
    float3 normal_cal = normal(input.uv * size_factors[0]) * input.normal;
    float2 metal_rough = metal    .Sample(metal_s,    input.uv).gb; // rough is g, b is metallic
    float metallic_final = metal_rough.y * metallic;
    float rough_final    = metal_rough.x * roughness;
    float  ao  = 1.0;
    if (use_occlusion) {
        ao  = occlusion.Sample(occlusion_s,input.uv).r;  // occlusion is sometimes part of the metal tex, uses r channel
    }
    
    float4 color = sk_pbr_shade(albedo, input.irradiance, ao, metallic_final, rough_final, input.view_dir, normal_cal);

    return color;
}
    