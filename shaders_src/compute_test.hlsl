//--name = app/compute_test
// Minimal compute shader exposing every cbuffer parameter type supported
// by the StereoKit Rust bindings.  Used for documentation round-trip tests.

// ── cbuffer parameters ──────────────────────────────────────────────────────
float     ring_freq;
int       arm_count;
uint      tex_size;
bool      center_glow;
float2    uv_offset;
float3    spiral_twist;
float4    highlight;
float4    base_color;
float4x4  brightness;
float     rotation;

// ── output ──────────────────────────────────────────────────────────────────
RWTexture2D<float4> out_tex : register(u0);

[numthreads(8, 8, 1)]
void cs(uint3 id : SV_DispatchThreadID) {
    // Normalised UV in [-1, 1], optionally offset by uv_offset
    float2 uv    = (float2(id.xy) / float(tex_size) - 0.5 + uv_offset) * 2.0;
    float  cos_r = cos(rotation);
    float  sin_r = sin(rotation);
    uv = float2(uv.x * cos_r - uv.y * sin_r, uv.x * sin_r + uv.y * cos_r);
    float  dist  = length(uv);
    float  angle = atan2(uv.y, uv.x) * (1.0 / 6.28318);

    // Concentric rings — frequency driven by ring_freq
    float rings  = sin(dist * 20.0 * ring_freq) * 0.5 + 0.5;

    // arm_count spiral arms, twist amount from spiral_twist.x
    float spiral = sin((angle + dist * spiral_twist.x) * float(arm_count) * 6.28318) * 0.5 + 0.5;

    // Soft vignette — attenuates towards the edges
    float vignette = 1.0 - smoothstep(0.7, 1.2, dist);

    // center_glow: optional centre glow
    float glow = center_glow ? exp(-dist * dist * 8.0) : 0.0;

    // highlight tint; brightness scaled by brightness[0][0]
    float  t   = saturate(rings * spiral * vignette + glow);
    float4 col = lerp(base_color, highlight, t) * brightness[0][0];
    out_tex[id.xy] = saturate(col);
}
