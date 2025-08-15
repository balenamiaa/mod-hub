use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    // Windowing
    #[error("register window class failed")]
    RegisterClassFailed,
    #[error("create owner window failed")]
    CreateOwnerWindow(#[source] windows::core::Error),
    #[error("create overlay window failed")]
    CreateOverlayWindow(#[source] windows::core::Error),

    // D3D/DXGI
    #[error("D3D11 device creation failed")]
    D3dCreateDevice(#[source] windows::core::Error),
    #[error("DXGI factory creation failed")]
    DxgiCreateFactory(#[source] windows::core::Error),
    #[error("DXGI swap chain creation failed")]
    DxgiCreateSwapChain(#[source] windows::core::Error),
    #[error("DXGI resize buffers failed")]
    DxgiResizeBuffers(#[source] windows::core::Error),

    // DirectComposition
    #[error("DirectComposition device creation failed")]
    DcompCreateDevice(#[source] windows::core::Error),
    #[error("DirectComposition target creation failed")]
    DcompCreateTarget(#[source] windows::core::Error),
    #[error("DirectComposition visual creation failed")]
    DcompCreateVisual(#[source] windows::core::Error),
    #[error("DirectComposition set content failed")]
    DcompSetContent(#[source] windows::core::Error),
    #[error("DirectComposition set root failed")]
    DcompSetRoot(#[source] windows::core::Error),
    #[error("DirectComposition commit failed")]
    DcompCommit(#[source] windows::core::Error),

    // Shaders and pipeline
    #[error("shader compilation failed")]
    ShaderCompile(#[source] windows::core::Error),
    #[error("create vertex shader failed")]
    CreateVertexShader(#[source] windows::core::Error),
    #[error("create pixel shader failed")]
    CreatePixelShader(#[source] windows::core::Error),
    #[error("create input layout failed")]
    CreateInputLayout(#[source] windows::core::Error),
    #[error("create sampler state failed")]
    CreateSampler(#[source] windows::core::Error),
    #[error("create blend state failed")]
    CreateBlend(#[source] windows::core::Error),
    #[error("create rasterizer state failed")]
    CreateRaster(#[source] windows::core::Error),
    #[error("create buffer failed")]
    CreateBuffer(#[source] windows::core::Error),
    #[error("map buffer failed")]
    MapBuffer(#[source] windows::core::Error),
    #[error("create texture failed")]
    CreateTexture(#[source] windows::core::Error),
    #[error("create shader resource view failed")]
    CreateSrv(#[source] windows::core::Error),

    // Hooks
    #[error("hook install failed")]
    HookInstall(#[source] ilhook::HookError),

    // Generic fallbacks
    #[error("windows api error")]
    Windows(#[from] windows::core::Error),
    #[error("ffi nul error")]
    FfiNul(#[from] std::ffi::NulError),
    #[error("i/o error")]
    Io(#[from] std::io::Error),
}
