use crate::overlay::d3d::D3D;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::DirectComposition::*;

pub struct Composition {
    device: IDCompositionDevice,
    target: IDCompositionTarget,
    visual: IDCompositionVisual,
}

impl Composition {
    pub fn new(hwnd: HWND, d3d: &crate::overlay::d3d::D3D) -> Result<Self, String> {
        let dxgi_device = &d3d.dxgi_device;
        let device: IDCompositionDevice = unsafe { DCompositionCreateDevice(Some(dxgi_device)) }
            .map_err(|e| format!("DCompositionCreateDevice: {e}"))?;
        let target = unsafe { device.CreateTargetForHwnd(hwnd, true) }
            .map_err(|e| format!("CreateTargetForHwnd: {e}"))?;
        let visual = unsafe { device.CreateVisual() }.map_err(|e| format!("CreateVisual: {e}"))?;
        Ok(Self {
            device,
            target,
            visual,
        })
    }

    pub fn bind_swap_chain(&mut self, d3d: &D3D) -> Result<(), String> {
        unsafe {
            self.visual
                .SetContent(&d3d.swap_chain)
                .map_err(|e| format!("Visual::SetContent: {e}"))?;
        }
        unsafe {
            self.target
                .SetRoot(&self.visual)
                .map_err(|e| format!("Target::SetRoot: {e}"))?;
        }
        unsafe {
            self.device
                .Commit()
                .map_err(|e| format!("DComp Commit: {e}"))?;
        }
        Ok(())
    }
}
