//! Direct2D/DirectWrite surface used by the Windows overlay.
//!
//! The widget drawing code still obtains a compatible DC through the official
//! Direct2D GDI interop interface while it is being migrated. This keeps the
//! existing widget geometry stable while making the window surface and text
//! resources owned by Direct2D/DirectWrite.

use windows::core::{Interface, Result};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{D2D1_PIXEL_FORMAT, D2D_SIZE_U};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory, ID2D1GdiInteropRenderTarget, ID2D1HwndRenderTarget,
    D2D1_DC_INITIALIZE_MODE_COPY, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

pub struct D2dBackend {
    #[allow(dead_code)]
    factory: ID2D1Factory,
    #[allow(dead_code)]
    text_factory: IDWriteFactory,
    target: ID2D1HwndRenderTarget,
    gdi: ID2D1GdiInteropRenderTarget,
}

impl D2dBackend {
    pub unsafe fn new(hwnd: HWND, width: u32, height: u32) -> Result<Self> {
        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let text_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode:
                    windows::Win32::Graphics::Direct2D::Common::D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let hwnd_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd,
            pixelSize: D2D_SIZE_U { width, height },
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };
        let target = factory.CreateHwndRenderTarget(&properties, &hwnd_properties)?;
        let gdi = target.cast::<ID2D1GdiInteropRenderTarget>()?;
        Ok(Self {
            factory,
            text_factory,
            target,
            gdi,
        })
    }

    #[allow(dead_code)]
    pub fn text_factory(&self) -> &IDWriteFactory {
        &self.text_factory
    }

    pub unsafe fn begin_gdi(&self) -> Result<windows_sys::Win32::Graphics::Gdi::HDC> {
        self.target.BeginDraw();
        Ok(self.gdi.GetDC(D2D1_DC_INITIALIZE_MODE_COPY)?.0)
    }

    pub unsafe fn end_gdi(&self) -> Result<()> {
        self.gdi.ReleaseDC(None)?;
        self.target.EndDraw(None, None)
    }

    #[allow(dead_code)]
    pub fn factory(&self) -> &ID2D1Factory {
        &self.factory
    }
}
