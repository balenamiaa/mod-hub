use crate::errors::{Error, Result};
use crate::overlay::d3d::D3D;
use egui::ClippedPrimitive;
use std::collections::HashMap;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct3D::{
    D3D_SRV_DIMENSION_TEXTURE2D, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::core::PCSTR;

/// Egui mesh painter backed by Direct3D 11.
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
    fallback_srv: ID3D11ShaderResourceView,
}

struct Texture {
    srv: ID3D11ShaderResourceView,
    _size: [u32; 2],
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
    /// Creates shaders, pipeline state and persistent buffers for rendering.
    pub fn new(d3d: &D3D) -> Result<Self> {
        let device = d3d.device.clone();
        let context = d3d.context.clone();

        let vs_src = include_str!("shaders/egui_vs.hlsl");
        let ps_src = include_str!("shaders/egui_ps.hlsl");
        let vsb = compile_shader(vs_src, "vs_5_0", "main")?;
        let psb = compile_shader(ps_src, "ps_5_0", "main")?;
        let vs = unsafe {
            let mut vs = None;
            device
                .CreateVertexShader(vsb.as_slice(), None, Some(&mut vs))
                .map_err(Error::CreateVertexShader)?;
            vs.unwrap()
        };
        let ps = unsafe {
            let mut ps = None;
            device
                .CreatePixelShader(psb.as_slice(), None, Some(&mut ps))
                .map_err(Error::CreatePixelShader)?;
            ps.unwrap()
        };

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
        let layout = unsafe {
            let mut layout = None;
            device
                .CreateInputLayout(&elems, vsb.as_slice(), Some(&mut layout))
                .map_err(Error::CreateInputLayout)?;
            layout.unwrap()
        };

        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ..Default::default()
        };
        let sampler = unsafe {
            let mut sampler = None;
            device
                .CreateSamplerState(&sampler_desc, Some(&mut sampler))
                .map_err(Error::CreateSampler)?;
            sampler.unwrap()
        };

        let blend_desc = D3D11_BLEND_DESC {
            RenderTarget: [
                D3D11_RENDER_TARGET_BLEND_DESC {
                    BlendEnable: true.into(),
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
        let blend = unsafe {
            let mut blend = None;
            device
                .CreateBlendState(&blend_desc, Some(&mut blend))
                .map_err(Error::CreateBlend)?;
            blend.unwrap()
        };

        let rast_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            ScissorEnable: true.into(),
            ..Default::default()
        };
        let raster = unsafe {
            let mut raster = None;
            device
                .CreateRasterizerState(&rast_desc, Some(&mut raster))
                .map_err(Error::CreateRaster)?;
            raster.unwrap()
        };

        let cbuf_desc = D3D11_BUFFER_DESC {
            ByteWidth: std::mem::size_of::<CbData>() as u32,
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..Default::default()
        };
        let cbuf = unsafe {
            let mut cbuf = None;
            device
                .CreateBuffer(&cbuf_desc, None, Some(&mut cbuf))
                .map_err(Error::CreateBuffer)?;
            cbuf.unwrap()
        };

        let vb_desc = D3D11_BUFFER_DESC {
            ByteWidth: 1024 * 1024,
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..Default::default()
        };
        let vb = unsafe {
            let mut vb = None;
            device
                .CreateBuffer(&vb_desc, None, Some(&mut vb))
                .map_err(Error::CreateBuffer)?;
            vb.unwrap()
        };
        let ib_desc = D3D11_BUFFER_DESC {
            ByteWidth: 1024 * 1024,
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_INDEX_BUFFER.0 as u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..Default::default()
        };
        let ib = unsafe {
            let mut ib = None;
            device
                .CreateBuffer(&ib_desc, None, Some(&mut ib))
                .map_err(Error::CreateBuffer)?;
            ib.unwrap()
        };

        let fallback_srv = create_fallback_white(&device)?;
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
            fallback_srv,
        })
    }

    /// Notifies the painter that the backbuffer size changed.
    pub fn on_resize(&mut self, _d3d: &D3D) -> Result<()> {
        Ok(())
    }

    /// Applies texture uploads and frees according to egui's `TexturesDelta`.
    pub fn update_textures(&mut self, delta: &egui::TexturesDelta) -> Result<()> {
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
    ) -> Result<()> {
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
            let mut tex = None;
            self.device
                .CreateTexture2D(&tex_desc, Some(&init), Some(&mut tex))
                .map_err(Error::CreateTexture)?;
            let tex = tex.unwrap();
            let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: tex_desc.Format,
                ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: 1,
                    },
                },
            };
            let mut srv = None;
            self.device
                .CreateShaderResourceView(&tex, Some(&srv_desc), Some(&mut srv))
                .map_err(Error::CreateSrv)?;
            let srv = srv.unwrap();
            self.textures.insert(id, Texture { srv, _size: [w, h] });
        }
        Ok(())
    }

    /// Renders a list of clipped egui primitives into the current backbuffer.
    pub fn paint(
        &mut self,
        width: u32,
        height: u32,
        clipped: &[ClippedPrimitive],
    ) -> Result<()> {
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
                ctx.RSSetScissorRects(Some(&[sc]));
                match &cp.primitive {
                    egui::epaint::Primitive::Mesh(mesh) => {
                        let tex_id = mesh.texture_id;
                        let srv = self
                            .textures
                            .get(&tex_id)
                            .map(|t| t.srv.clone())
                            .unwrap_or_else(|| self.fallback_srv.clone());
                        upload_mesh(&self.context, &self.vb, &self.ib, mesh)?;
                        let stride = std::mem::size_of::<Vertex>() as u32;
                        let offset = 0u32;
                        ctx.IASetInputLayout(&self.layout);
                        let strides = [stride];
                        let offsets = [offset];
                        let bufs = [Some(self.vb.clone())];
                        ctx.IASetVertexBuffers(
                            0,
                            1,
                            Some(bufs.as_ptr()),
                            Some(strides.as_ptr()),
                            Some(offsets.as_ptr()),
                        );
                        ctx.IASetIndexBuffer(&self.ib, DXGI_FORMAT_R32_UINT, 0);
                        ctx.PSSetShaderResources(0, Some(&[Some(srv)]));
                        ctx.DrawIndexed(mesh.indices.len() as u32, 0, 0);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

fn compile_shader(src: &str, profile: &str, entry: &str) -> Result<Vec<u8>> {
    use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
    let mut code = None;
    unsafe {
        let entry_c = std::ffi::CString::new(entry)?;
        let profile_c = std::ffi::CString::new(profile)?;
        D3DCompile(
            src.as_ptr() as *const _,
            src.len(),
            None,
            None,
            None,
            PCSTR(entry_c.as_ptr() as _),
            PCSTR(profile_c.as_ptr() as _),
            0,
            0,
            &mut code,
            Some(std::ptr::null_mut()),
        )
        .map_err(Error::ShaderCompile)?;
    }
    let blob = code.unwrap();
    let mut v = Vec::new();
    unsafe {
        std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize())
    }
    .clone_into(&mut v);
    Ok(v)
}

fn rect_to_scissor(r: &egui::Rect, w: u32, h: u32) -> RECT {
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
    RECT {
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
) -> Result<()> {
    let mut vertices: Vec<Vertex> = Vec::with_capacity(mesh.vertices.len());
    for v in &mesh.vertices {
        let c = v.color.to_array();
        let a = c[3] as f32 / 255.0;
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
            .map_err(Error::MapBuffer)?;
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr() as *const u8,
            m.pData as *mut u8,
            vertices.len() * std::mem::size_of::<Vertex>(),
        );
        ctx.Unmap(vb, 0);
        let mut mi = D3D11_MAPPED_SUBRESOURCE::default();
        ctx.Map(ib, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mi))
            .map_err(Error::MapBuffer)?;
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

fn create_fallback_white(device: &ID3D11Device) -> Result<ID3D11ShaderResourceView> {
    let tex_desc = D3D11_TEXTURE2D_DESC {
        Width: 1,
        Height: 1,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_IMMUTABLE,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        ..Default::default()
    };
    let data = [255u8, 255, 255, 255];
    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: data.as_ptr() as *const _,
        SysMemPitch: 4,
        ..Default::default()
    };
    unsafe {
        let mut tex = None;
        device
            .CreateTexture2D(&tex_desc, Some(&init), Some(&mut tex))
            .map_err(Error::CreateTexture)?;
        let tex = tex.unwrap();
        let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: tex_desc.Format,
            ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_SRV {
                    MostDetailedMip: 0,
                    MipLevels: 1,
                },
            },
        };
        let mut srv = None;
        device
            .CreateShaderResourceView(&tex, Some(&srv_desc), Some(&mut srv))
            .map_err(Error::CreateSrv)?;
        Ok(srv.unwrap())
    }
}
