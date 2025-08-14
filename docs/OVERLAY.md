Overview
- Pure Win32 + D3D11 + DirectComposition overlay with per‑pixel transparency
- No wgpu/winit; integrates egui via a D3D11 painter
- Hidden owner window hides overlay from Alt‑Tab and taskbar while preserving Aero Peek

Windowing
- Registers two classes and creates a hidden tool‑window owner and an owned WS_POPUP overlay
- Extended styles: `WS_EX_NOREDIRECTIONBITMAP | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST`
- Click‑through toggling adds/removes `WS_EX_TRANSPARENT | WS_EX_NOACTIVATE`
- Transparency: `SetLayeredWindowAttributes(…, alpha=255, LWA_ALPHA)` and `DwmExtendFrameIntoClientArea(margins=-1)`
- Shown with `SW_SHOWNOACTIVATE` and pinned `HWND_TOPMOST`

Rendering
- D3D11 device/context (BGRA) + `IDXGISwapChain1` created via `CreateSwapChainForComposition` with premultiplied alpha
- DirectComposition device/target/visual set to the swap chain and committed
- Each frame sets an RTV from the backbuffer, clears to transparent, draws egui meshes, and presents
- Egui painter compiles tiny HLSL shaders at runtime via `D3DCompile`, uses premultiplied alpha blend

Egui Integration
- Win32 message pump translates input events to `egui::Event`
- On each frame: `begin_frame(raw)`, `app.ui(ctx)`, `end_frame()`, tessellate, update textures, paint, present
- `TexturesDelta` is applied by uploading immutable BGRA textures and binding SRVs per mesh texture

Behavior Goals
- Transparent and topmost from startup, click‑through enabled by default
- Overlay does not appear in Alt‑Tab or taskbar; remains visible during Aero Peek
- Maintains transparency after Alt‑Tab and after toggling input passthrough

Usage
- Construct `OverlayBuilder`, then call `.run(AppUi)`
- Toggle click‑through via `VK_INSERT` by default

Notes
- Textures are stored as immutable BGRA8 premultiplied; font atlas alpha is expanded to white premultiplied
- Clip rectangles are implemented via D3D11 scissor rectangles
