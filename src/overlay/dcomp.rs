use crate::errors::{Error, Result};
use crate::overlay::d3d::D3D;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::DirectComposition::*;

/// DirectComposition device and visual tree targeting the overlay window.
pub struct Composition {
    device: IDCompositionDevice,
    target: IDCompositionTarget,
    visual: IDCompositionVisual,
}

impl Composition {
    /// Creates a DirectComposition device and a target bound to `hwnd`.
    pub fn new(hwnd: HWND, d3d: &crate::overlay::d3d::D3D) -> Result<Self> {
        let dxgi_device = &d3d.dxgi_device;
        let device: IDCompositionDevice =
            unsafe { DCompositionCreateDevice(Some(dxgi_device)) }
                .map_err(Error::DcompCreateDevice)?;
        let target = unsafe { device.CreateTargetForHwnd(hwnd, true) }
            .map_err(Error::DcompCreateTarget)?;
        let visual = unsafe { device.CreateVisual() }.map_err(Error::DcompCreateVisual)?;
        log::debug!("dcomp device/target/visual created");
        Ok(Self {
            device,
            target,
            visual,
        })
    }

    /// Sets the swap chain as the root visual content and commits the scene.
    pub fn bind_swap_chain(&mut self, d3d: &D3D) -> Result<()> {
        unsafe {
            self.visual
                .SetContent(&d3d.swap_chain)
                .map_err(Error::DcompSetContent)?;
        }
        unsafe {
            self.target
                .SetRoot(&self.visual)
                .map_err(Error::DcompSetRoot)?;
        }
        unsafe {
            self.device.Commit().map_err(Error::DcompCommit)?;
        }
        log::debug!("dcomp bound swapchain and committed");
        Ok(())
    }
}
