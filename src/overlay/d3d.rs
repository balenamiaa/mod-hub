use windows::core::Interface;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;

pub struct D3D {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    pub dxgi_device: IDXGIDevice,
    pub swap_chain: IDXGISwapChain1,
    pub width: u32,
    pub height: u32,
    rtv: Option<ID3D11RenderTargetView>,
}

impl D3D {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let mut device = None;
        let mut context = None;
        let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                flags,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .map_err(|e| format!("D3D11CreateDevice: {e}"))?;
        }
        let device = device.unwrap();
        let context = context.unwrap();
        let dxgi_device: IDXGIDevice = device.cast().map_err(|e| format!("IDXGIDevice: {e}"))?;

        let factory: IDXGIFactory2 = unsafe {
            let mut f: Option<IDXGIFactory2> = None;
            CreateDXGIFactory2(0, &IDXGIFactory2::IID, f.set_abi()).ok().map_err(|e| format!("CreateDXGIFactory2: {e}"))?;
            f.unwrap()
        };

        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            BufferCount: 2,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Width: width,
            Height: height,
            ..Default::default()
        };

        let swap_chain = unsafe {
            let mut sc: Option<IDXGISwapChain1> = None;
            factory
                .CreateSwapChainForComposition(&dxgi_device, &desc, None, sc.set_abi())
                .ok()
                .map_err(|e| format!("CreateSwapChainForComposition: {e}"))?;
            sc.unwrap()
        };

        Ok(Self { device, context, dxgi_device, swap_chain, width, height, rtv: None })
    }

    pub fn resize(&mut self, w: u32, h: u32) -> Result<(), String> {
        if w == 0 || h == 0 { return Ok(()); }
        unsafe { self.swap_chain.ResizeBuffers(0, w, h, DXGI_FORMAT_UNKNOWN, 0).ok().map_err(|e| format!("ResizeBuffers: {e}"))?; }
        self.width = w; self.height = h;
        self.rtv = None;
        Ok(())
    }

    pub fn begin_frame(&self) {
        unsafe {
            let mut buf: Option<ID3D11Texture2D> = None;
            self.swap_chain.GetBuffer(0, &ID3D11Texture2D::IID, buf.set_abi()).ok().ok();
            if let Some(tex) = buf {
                let rtv = self.device.CreateRenderTargetView(&tex, None).ok();
                if let Ok(rtv) = rtv {
                    self.context.OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
                    let vp = D3D11_VIEWPORT { TopLeftX: 0.0, TopLeftY: 0.0, Width: self.width as f32, Height: self.height as f32, MinDepth: 0.0, MaxDepth: 1.0 };
                    self.context.RSSetViewports(Some(&[vp]));
                    let clear = [0.0f32, 0.0, 0.0, 0.0];
                    self.context.ClearRenderTargetView(&rtv, &clear);
                }
            }
        }
    }

    pub fn present(&self) {
        unsafe { let _ = self.swap_chain.Present(0, 0); }
    }
}
