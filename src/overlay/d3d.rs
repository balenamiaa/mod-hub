use crate::errors::{Error, Result};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::core::Interface;

/// Direct3D 11 device, context and DXGI swap chain configured for composition.
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
    /// Creates a BGRA‑capable device and a premultiplied swap chain sized to the given dimensions.
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let mut device = None;
        let mut context = None;
        let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE(std::ptr::null_mut()),
                flags,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .map_err(Error::D3dCreateDevice)?;
        }
        let device = device.unwrap();
        let context = context.unwrap();
        let dxgi_device: IDXGIDevice = device
            .cast()
            .map_err(Error::Windows)?;

        let factory: IDXGIFactory2 = unsafe {
            CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)).map_err(Error::DxgiCreateFactory)?
        };

        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            BufferCount: 2,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Width: width,
            Height: height,
            ..Default::default()
        };

        let swap_chain = unsafe {
            factory
                .CreateSwapChainForComposition(&dxgi_device, &desc, None)
                .map_err(Error::DxgiCreateSwapChain)?
        };
        log::debug!("D3D11 device + swapchain created: {}x{}", width, height);

        Ok(Self {
            device,
            context,
            dxgi_device,
            swap_chain,
            width,
            height,
            rtv: None,
        })
    }

    /// Resizes the swap chain and invalidates any cached render targets.
    pub fn resize(&mut self, w: u32, h: u32) -> Result<()> {
        if w == 0 || h == 0 {
            return Ok(());
        }
        unsafe {
            self.swap_chain
                .ResizeBuffers(0, w, h, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0))
                .map_err(Error::DxgiResizeBuffers)?;
        }
        self.width = w;
        self.height = h;
        self.rtv = None;
        log::debug!("swapchain resized: {}x{}", w, h);
        Ok(())
    }

    /// Binds the backbuffer to the pipeline and clears it to transparent.
    pub fn begin_frame(&self) {
        unsafe {
            if let Ok(tex) = self.swap_chain.GetBuffer::<ID3D11Texture2D>(0) {
                let mut rtv = None;
                if self
                    .device
                    .CreateRenderTargetView(&tex, None, Some(&mut rtv))
                    .is_ok()
                {
                    if let Some(rtv) = rtv {
                        self.context
                            .OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
                        let vp = D3D11_VIEWPORT {
                            TopLeftX: 0.0,
                            TopLeftY: 0.0,
                            Width: self.width as f32,
                            Height: self.height as f32,
                            MinDepth: 0.0,
                            MaxDepth: 1.0,
                        };
                        self.context.RSSetViewports(Some(&[vp]));
                        let clear = [0.0f32, 0.0, 0.0, 0.0];
                        self.context.ClearRenderTargetView(&rtv, &clear);
                    }
                }
            }
        }
    }

    /// Presents the current backbuffer.
    pub fn present(&self) {
        unsafe {
            let _ = self.swap_chain.Present(0, DXGI_PRESENT(0));
        }
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("frame presented");
        }
    }
}
