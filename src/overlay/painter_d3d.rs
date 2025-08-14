use crate::overlay::d3d::D3D;
use egui::ClippedPrimitive;
use std::collections::HashMap;
use windows::Win32::Graphics::Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::core::PCSTR;

pub struct PainterD3D {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    vs: ID3D11VertexShader,
    ps: ID3D11PixelShader,
    layout: ID3D11InputLayout,
    sampler: ID3D11SamplerState,
    blend: ID3D11BlendState,
    raster: ID3D11RasterizerState,
    cbuf: ID3D11Buffer,
    vb: ID3D11Buffer,
    ib: ID3D11Buffer,
    textures: HashMap<egui::TextureId, Texture>,
}

struct Texture {
    srv: ID3D11ShaderResourceView,
    size: [u32; 2],
}

#[repr(C)]
struct CbData {
    screen_size: [f32; 2],
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

impl PainterD3D {
    pub fn new(d3d: &D3D) -> Result<Self, String> {
        let device = d3d.device.clone();
        let context = d3d.context.clone();

        let vs_src = include_str!("shaders/egui_vs.hlsl");
        let ps_src = include_str!("shaders/egui_ps.hlsl");
        let vsb = compile_shader(vs_src, "vs_5_0", "main")?;
        let psb = compile_shader(ps_src, "ps_5_0", "main")?;
        let vs = unsafe { device.CreateVertexShader(vsb.as_slice(), None) }
            .map_err(|e| format!("CreateVertexShader: {e}"))?;
        let ps = unsafe { device.CreatePixelShader(psb.as_slice(), None) }
            .map_err(|e| format!("CreatePixelShader: {e}"))?;

        let elems = [
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR::from_raw(b"POSITION\0".as_ptr()),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 0,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR::from_raw(b"TEXCOORD\0".as_ptr()),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 8,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR::from_raw(b"COLOR\0".as_ptr()),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 16,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];
        let layout = unsafe { device.CreateInputLayout(&elems, vsb.as_slice()) }
            .map_err(|e| format!("CreateInputLayout: {e}"))?;

        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ..Default::default()
        };
        let sampler = unsafe { device.CreateSamplerState(&sampler_desc) }
            .map_err(|e| format!("CreateSamplerState: {e}"))?;

        let blend_desc = D3D11_BLEND_DESC {
            RenderTarget: [
                D3D11_RENDER_TARGET_BLEND_DESC {
                    BlendEnable: 1,
                    SrcBlend: D3D11_BLEND_ONE,
                    DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                    BlendOp: D3D11_BLEND_OP_ADD,
                    SrcBlendAlpha: D3D11_BLEND_ONE,
                    DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                    BlendOpAlpha: D3D11_BLEND_OP_ADD,
                    RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
                },
                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                D3D11_RENDER_TARGET_BLEND_DESC::default(),
            ],
            ..Default::default()
        };
        let blend = unsafe { device.CreateBlendState(&blend_desc) }
            .map_err(|e| format!("CreateBlendState: {e}"))?;

        let rast_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            ScissorEnable: windows::Win32::Foundation::BOOL::from(true),
            ..Default::default()
        };
        let raster = unsafe { device.CreateRasterizerState(&rast_desc) }
            .map_err(|e| format!("CreateRasterizerState: {e}"))?;

        let cbuf_desc = D3D11_BUFFER_DESC {
            ByteWidth: std::mem::size_of::<CbData>() as u32,
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_CONSTANT_BUFFER.0,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..Default::default()
        };
        let cbuf = unsafe { device.CreateBuffer(&cbuf_desc, None) }
            .map_err(|e| format!("CreateBuffer cbuf: {e}"))?;

        let vb_desc = D3D11_BUFFER_DESC {
            ByteWidth: 1024 * 1024,
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..Default::default()
        };
        let vb = unsafe { device.CreateBuffer(&vb_desc, None) }
            .map_err(|e| format!("CreateBuffer vb: {e}"))?;
        let ib_desc = D3D11_BUFFER_DESC {
            ByteWidth: 1024 * 1024,
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_INDEX_BUFFER.0,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..Default::default()
        };
        let ib = unsafe { device.CreateBuffer(&ib_desc, None) }
            .map_err(|e| format!("CreateBuffer ib: {e}"))?;

        Ok(Self {
            device,
            context,
            vs,
            ps,
            layout,
            sampler,
            blend,
            raster,
            cbuf,
            vb,
            ib,
            textures: HashMap::new(),
        })
    }

    pub fn on_resize(&mut self, _d3d: &D3D) -> Result<(), String> {
        Ok(())
    }

    pub fn update_textures(&mut self, delta: &egui::TexturesDelta) -> Result<(), String> {
        for (id, img_delta) in &delta.set {
            let mut full = None;
            match &img_delta.image {
                egui::ImageData::Color(color) => {
                    let (w, h) = (color.size[0] as u32, color.size[1] as u32);
                    let mut buf = vec![0u8; (w * h * 4) as usize];
                    for y in 0..h {
                        for x in 0..w {
                            let c = color.pixels[(y * w + x) as usize];
                            let a = c.a() as f32 / 255.0;
                            let r = c.r() as f32 / 255.0 * a;
                            let g = c.g() as f32 / 255.0 * a;
                            let b = c.b() as f32 / 255.0 * a;
                            let i = ((y * w + x) * 4) as usize;
                            buf[i] = (b * 255.0) as u8;
                            buf[i + 1] = (g * 255.0) as u8;
                            buf[i + 2] = (r * 255.0) as u8;
                            buf[i + 3] = (a * 255.0) as u8;
                        }
                    }
                    full = Some((w, h, buf));
                }
                egui::ImageData::Font(font) => {
                    let (w, h) = (font.size[0] as u32, font.size[1] as u32);
                    let mut buf = vec![0u8; (w * h * 4) as usize];
                    for y in 0..h {
                        for x in 0..w {
                            let a = font.pixels[(y * w + x) as usize] as f32 / 255.0;
                            let i = ((y * w + x) * 4) as usize;
                            buf[i] = (1.0 * a * 255.0) as u8;
                            buf[i + 1] = (1.0 * a * 255.0) as u8;
                            buf[i + 2] = (1.0 * a * 255.0) as u8;
                            buf[i + 3] = (a * 255.0) as u8;
                        }
                    }
                    full = Some((w, h, buf));
                }
            }
            if let Some((w, h, buf)) = full {
                self.create_or_update_texture(*id, w, h, &buf)?;
            }
        }
        for id in &delta.free {
            self.textures.remove(id);
        }
        Ok(())
    }

    fn create_or_update_texture(
        &mut self,
        id: egui::TextureId,
        w: u32,
        h: u32,
        data: &[u8],
    ) -> Result<(), String> {
        let tex_desc = D3D11_TEXTURE2D_DESC {
            Width: w,
            Height: h,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_IMMUTABLE,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            ..Default::default()
        };
        let init = D3D11_SUBRESOURCE_DATA {
            pSysMem: data.as_ptr() as *const _,
            SysMemPitch: (w * 4) as u32,
            ..Default::default()
        };
        unsafe {
            let tex = self
                .device
                .CreateTexture2D(&tex_desc, Some(&init))
                .map_err(|e| format!("CreateTexture2D: {e}"))?;
            let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: tex_desc.Format,
                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: 1,
                    },
                },
            };
            let srv = self
                .device
                .CreateShaderResourceView(&tex, Some(&srv_desc))
                .map_err(|e| format!("CreateShaderResourceView: {e}"))?;
            self.textures.insert(id, Texture { srv, size: [w, h] });
        }
        Ok(())
    }

    pub fn paint(
        &mut self,
        width: u32,
        height: u32,
        clipped: &[ClippedPrimitive],
    ) -> Result<(), String> {
        unsafe {
            let ctx = &self.context;

            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            let hr = ctx.Map(&self.cbuf, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped));
            if hr.is_ok() {
                let data = mapped.pData as *mut CbData;
                (*data).screen_size = [width as f32, height as f32];
                (*data)._pad = [0.0, 0.0];
                ctx.Unmap(&self.cbuf, 0);
            }

            let blend_factors = [0.0f32; 4];
            ctx.OMSetBlendState(&self.blend, Some(&blend_factors), 0xffffffff);
            ctx.RSSetState(&self.raster);
            ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            ctx.VSSetShader(&self.vs, None);
            ctx.PSSetShader(&self.ps, None);
            ctx.PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
            ctx.VSSetConstantBuffers(0, Some(&[Some(self.cbuf.clone())]));

            for cp in clipped {
                let sc = rect_to_scissor(&cp.clip_rect, width, height);
                ctx.RSSetScissorRects(&[sc]);
                match &cp.primitive {
                    egui::epaint::Primitive::Mesh(mesh) => {
                        let tex_id = mesh.texture_id;
                        let tex = self.textures.get(&tex_id);
                        if let Some(tex) = tex {
                            upload_mesh(&self.context, &self.vb, &self.ib, mesh)?;
                            let stride = std::mem::size_of::<Vertex>() as u32;
                            let offset = 0u32;
                            ctx.IASetInputLayout(&self.layout);
                            ctx.IASetVertexBuffers(
                                0,
                                Some(&[Some(self.vb.clone())]),
                                Some(&[stride]),
                                Some(&[offset]),
                            );
                            ctx.IASetIndexBuffer(&self.ib, DXGI_FORMAT_R32_UINT, 0);
                            ctx.PSSetShaderResources(0, Some(&[Some(tex.srv.clone())]));
                            ctx.DrawIndexed(mesh.indices.len() as u32, 0, 0);
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

fn compile_shader(src: &str, profile: &str, entry: &str) -> Result<Vec<u8>, String> {
    use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
    let mut code = None;
    unsafe {
        D3DCompile(
            src.as_ptr() as *const _,
            src.len(),
            None,
            None,
            None,
            entry,
            profile,
            0,
            0,
            Some(&mut code),
            None,
        )
        .ok()
        .map_err(|e| format!("D3DCompile: {e}"))?;
    }
    let blob = code.unwrap();
    let mut v = Vec::new();
    unsafe {
        std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize())
    }
    .clone_into(&mut v);
    Ok(v)
}

fn rect_to_scissor(r: &egui::Rect, w: u32, h: u32) -> windows::Win32::Foundation::RECT {
    let mut x0 = r.min.x.max(0.0).floor() as i32;
    let mut y0 = r.min.y.max(0.0).floor() as i32;
    let mut x1 = r.max.x.min(w as f32).ceil() as i32;
    let mut y1 = r.max.y.min(h as f32).ceil() as i32;
    if x0 > x1 {
        std::mem::swap(&mut x0, &mut x1);
    }
    if y0 > y1 {
        std::mem::swap(&mut y0, &mut y1);
    }
    windows::Win32::Foundation::RECT {
        left: x0,
        top: y0,
        right: x1,
        bottom: y1,
    }
}

fn upload_mesh(
    ctx: &ID3D11DeviceContext,
    vb: &ID3D11Buffer,
    ib: &ID3D11Buffer,
    mesh: &egui::epaint::Mesh,
) -> Result<(), String> {
    let mut vertices: Vec<Vertex> = Vec::with_capacity(mesh.vertices.len());
    for v in &mesh.vertices {
        let c = v.color.to_array();
        let a = c[3] / 255.0;
        let premul = [
            c[0] as f32 / 255.0 * a,
            c[1] as f32 / 255.0 * a,
            c[2] as f32 / 255.0 * a,
            a,
        ];
        vertices.push(Vertex {
            pos: [v.pos.x, v.pos.y],
            uv: [v.uv.x, v.uv.y],
            color: premul,
        });
    }
    let indices: Vec<u32> = mesh.indices.iter().map(|&i| i as u32).collect();
    unsafe {
        let mut m = D3D11_MAPPED_SUBRESOURCE::default();
        ctx.Map(vb, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut m))
            .ok()
            .map_err(|e| format!("Map vb: {e}"))?;
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr() as *const u8,
            m.pData as *mut u8,
            vertices.len() * std::mem::size_of::<Vertex>(),
        );
        ctx.Unmap(vb, 0);
        let mut mi = D3D11_MAPPED_SUBRESOURCE::default();
        ctx.Map(ib, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mi))
            .ok()
            .map_err(|e| format!("Map ib: {e}"))?;
        std::ptr::copy_nonoverlapping(
            indices.as_ptr() as *const u8,
            mi.pData as *mut u8,
            indices.len() * std::mem::size_of::<u32>(),
        );
        ctx.Unmap(ib, 0);
    }
    Ok(())
}

#[allow(dead_code)]
fn srgb_to_linear(x: u8) -> f32 {
    let x = x as f32 / 255.0;
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
