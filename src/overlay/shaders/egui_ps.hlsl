Texture2D tex0 : register(t0);
SamplerState samp0 : register(s0);

struct PSIn {
    float4 pos : SV_Position;
    float2 uv  : TEXCOORD0;
    float4 col : COLOR0;
};

float4 main(PSIn input) : SV_Target {
    float4 t = tex0.Sample(samp0, input.uv);
    return t * input.col;
}

