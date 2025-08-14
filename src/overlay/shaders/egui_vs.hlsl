cbuffer Globals : register(b0)
{
    float2 screen_size;
    float2 _pad;
}

struct VSIn {
    float2 pos : POSITION;
    float2 uv  : TEXCOORD0;
    float4 col : COLOR0;
};

struct VSOut {
    float4 pos : SV_Position;
    float2 uv  : TEXCOORD0;
    float4 col : COLOR0;
};

VSOut main(VSIn input) {
    VSOut o;
    float2 p = input.pos;
    float x = (p.x / screen_size.x) * 2.0 - 1.0;
    float y = 1.0 - (p.y / screen_size.y) * 2.0;
    o.pos = float4(x, y, 0.0, 1.0);
    o.uv = input.uv;
    o.col = input.col;
    return o;
}

