//! Direct2D/DirectWrite surface used by the Windows overlay.
//!
//! The widget drawing code still obtains a compatible DC through the official
//! Direct2D GDI interop interface while it is being migrated. This keeps the
//! existing widget geometry stable while making the window surface and text
//! resources owned by Direct2D/DirectWrite.

use windows::core::{Interface, Result, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory, ID2D1GdiInteropRenderTarget, ID2D1HwndRenderTarget,
    D2D1_DC_INITIALIZE_MODE_COPY, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_FEATURE_LEVEL_DEFAULT, D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
    D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_MEASURING_MODE_NATURAL,
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

#[derive(Debug, Clone)]
pub struct TextCommand {
    pub x: i32,
    pub y: i32,
    pub color: u32,
    pub text: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ShapeCommand {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub fill: Option<u32>,
    pub stroke: Option<u32>,
    pub stroke_width: f32,
    pub radius: f32,
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

    #[allow(clippy::too_many_arguments)]
    pub unsafe fn end_gdi(
        &self,
        shapes: &[ShapeCommand],
        texts: &[TextCommand],
        font_family: &str,
        font_size: f32,
        font_weight: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.gdi.ReleaseDC(None)?;
        for command in shapes {
            let rect = D2D_RECT_F {
                left: command.left,
                top: command.top,
                right: command.right,
                bottom: command.bottom,
            };
            let rounded = D2D1_ROUNDED_RECT {
                rect,
                radiusX: command.radius,
                radiusY: command.radius,
            };
            if let Some(color) = command.fill {
                let brush = self.target.CreateSolidColorBrush(&color_f(color), None)?;
                if command.radius > 0.0 {
                    self.target.FillRoundedRectangle(&rounded, &brush);
                } else {
                    self.target.FillRectangle(&rect, &brush);
                }
            }
            if let Some(color) = command.stroke {
                let brush = self.target.CreateSolidColorBrush(&color_f(color), None)?;
                if command.radius > 0.0 {
                    self.target
                        .DrawRoundedRectangle(&rounded, &brush, command.stroke_width, None);
                } else {
                    self.target
                        .DrawRectangle(&rect, &brush, command.stroke_width, None);
                }
            }
        }
        if !texts.is_empty() {
            let family = wide_null(font_family);
            let format = self.text_factory.CreateTextFormat(
                PCWSTR(family.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT(font_weight.clamp(100, 900)),
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_size.max(1.0),
                PCWSTR::null(),
            )?;
            for command in texts {
                let color = color_f(command.color);
                let brush = self.target.CreateSolidColorBrush(&color, None)?;
                let text = wide_null(&command.text);
                let rect = D2D_RECT_F {
                    left: command.x as f32,
                    top: command.y as f32,
                    right: width as f32,
                    bottom: (command.y as f32 + font_size * 2.0).min(height as f32),
                };
                self.target.DrawText(
                    &text[..text.len().saturating_sub(1)],
                    &format,
                    &rect as *const _,
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
        }
        self.target.EndDraw(None, None)
    }

    pub unsafe fn resize(&self, width: u32, height: u32) -> Result<()> {
        let size = D2D_SIZE_U { width, height };
        self.target.Resize(&size)
    }

    #[allow(dead_code)]
    pub fn factory(&self) -> &ID2D1Factory {
        &self.factory
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn color_f(color: u32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: (color & 0xff) as f32 / 255.0,
        g: ((color >> 8) & 0xff) as f32 / 255.0,
        b: ((color >> 16) & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}
