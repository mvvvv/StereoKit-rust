#include "stereokit.hlsli"

//--name = app/vert_custom

// For meshes with a custom position + color vertex format, see
// shaders2.rs. Mesh data is matched to these inputs by semantic, so the
// field order in the vertex struct doesn't need to match the order here.
struct vsIn {
	float4 pos : SV_Position;
	float4 col : COLOR0;
};
struct psIn {
	float4 pos : SV_POSITION;
	float4 col : COLOR0;
	float3 world: TEXCOORD0;
};

psIn vs(vsIn input, sk_ids_t ids) {
	psIn o;

	float4 world  = mul(input.pos, sk_inst[ids.inst].world);
	float3x3 rot  = (float3x3)sk_inst[ids.inst].world;
	o.world       = world.xyz;
	o.pos         = mul(world, sk_viewproj[ids.view]);
	o.col         = input.col * sk_inst[ids.inst].color;
	return o;
}

float4 ps(psIn input) : SV_TARGET {
	// Fake a little directional lighting using the world-space position
	// so a flat-shaded custom format still has depth cues.
	float3 normal = normalize(input.world);
	float3 light  = normalize(float3(0.5, 1.0, -0.3));
	float  diff   = max(dot(normal, light), 0.2);
	return input.col * diff;
}