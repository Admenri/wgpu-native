//! Emscripten (`wasm32-unknown-emscripten`) platform support.
//!
//! On Emscripten there is no native windowing system: wgpu-native renders
//! through wgpu-hal's GLES backend, which Emscripten implements on top of
//! WebGL 2. A surface is therefore backed by an HTML `<canvas>` element, which
//! [`WGPUEmscriptenSurfaceSourceCanvasHTMLSelector`] selects by CSS selector.
//!
//! [`WGPUEmscriptenSurfaceSourceCanvasHTMLSelector`]: crate::native::WGPUEmscriptenSurfaceSourceCanvasHTMLSelector

use core::ffi::{c_char, c_void};
use std::ffi::CString;

use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WebDisplayHandle, WebWindowHandle};

/// Mirror of `EmscriptenWebGLContextAttributes` from
/// `<emscripten/html5_webgl.h>`.
///
/// Emscripten fills this structure in through
/// `emscripten_webgl_init_context_attributes()`, so the field order has to match
/// the C definition (Emscripten 4.0 and newer, where the boolean fields use
/// `bool` rather than `EM_BOOL`).
///
/// The trailing `_reserved` array keeps our allocation at least as large as the
/// C structure, so that a newer Emscripten appending fields can never make
/// `emscripten_webgl_init_context_attributes()` write out of bounds.
#[repr(C)]
struct WebGlContextAttributes {
    alpha: bool,
    depth: bool,
    stencil: bool,
    antialias: bool,
    premultiplied_alpha: bool,
    preserve_drawing_buffer: bool,
    /// `EM_WEBGL_POWER_PREFERENCE`
    power_preference: i32,
    fail_if_major_performance_caveat: bool,
    major_version: i32,
    minor_version: i32,
    enable_extensions_by_default: bool,
    explicit_swap_control: bool,
    /// `EMSCRIPTEN_WEBGL_CONTEXT_PROXY_MODE`
    proxy_context_to_main_thread: i32,
    render_via_offscreen_back_buffer: bool,
    desynchronized: bool,
    _reserved: [u32; 8],
}

// `EM_WEBGL_POWER_PREFERENCE_HIGH_PERFORMANCE`
const WEBGL_POWER_PREFERENCE_HIGH_PERFORMANCE: i32 = 2;
// `EMSCRIPTEN_WEBGL_CONTEXT_PROXY_DISALLOW`
const WEBGL_CONTEXT_PROXY_DISALLOW: i32 = 0;

extern "C" {
    fn emscripten_webgl_init_context_attributes(attributes: *mut WebGlContextAttributes);
    /// Returns `0` (an invalid `EMSCRIPTEN_WEBGL_CONTEXT_HANDLE`) on failure.
    fn emscripten_webgl_create_context(
        target: *const c_char,
        attributes: *const WebGlContextAttributes,
    ) -> usize;
}

/// Builds the raw window handles describing an Emscripten surface.
///
/// `selector` is the CSS selector of the canvas to render into. When it is
/// absent or empty the Emscripten module canvas (`Module['canvas']`) is used.
///
/// wgpu-hal's EGL backend hands the id of [`RawWindowHandle::Web`] to
/// `eglCreateWindowSurface()` as the native window, which is how the WebGL
/// context created for the canvas reaches the backend.
pub fn surface_handles(selector: Option<&str>) -> (RawDisplayHandle, RawWindowHandle) {
    let context = match selector.filter(|selector| !selector.is_empty()) {
        Some(selector) => create_webgl2_context(selector).unwrap_or_else(|| {
            log::warn!(
                "unable to create a WebGL 2 context for canvas selector {selector:?}, falling \
                 back to the Emscripten module canvas"
            );
            0
        }),
        None => 0,
    };

    (
        RawDisplayHandle::Web(WebDisplayHandle::new()),
        RawWindowHandle::Web(WebWindowHandle::new(context)),
    )
}

/// Creates (or reuses) a WebGL 2 context on the canvas matched by `selector`.
///
/// Returns the `EMSCRIPTEN_WEBGL_CONTEXT_HANDLE` narrowed to a `u32`, which is
/// the width of [`WebWindowHandle::id`], or `None` if the selector does not
/// resolve to a canvas that can host a WebGL 2 context.
fn create_webgl2_context(selector: &str) -> Option<u32> {
    let target = CString::new(selector).ok()?;

    // SAFETY: `WebGlContextAttributes` is `#[repr(C)]` plain data that mirrors
    // `EmscriptenWebGLContextAttributes`, so a zeroed value is a valid starting
    // point for `emscripten_webgl_init_context_attributes()`.
    let mut attributes: WebGlContextAttributes = unsafe { std::mem::zeroed() };
    unsafe { emscripten_webgl_init_context_attributes(&mut attributes) };

    // Ask for WebGL 2, which is what wgpu-hal's GLES backend targets on
    // Emscripten. The remaining values mirror what wgpu-hal's EGL path requests
    // for a surface it owns.
    attributes.major_version = 2;
    attributes.minor_version = 0;
    attributes.alpha = false;
    attributes.depth = true;
    attributes.stencil = true;
    attributes.antialias = false;
    attributes.premultiplied_alpha = true;
    attributes.preserve_drawing_buffer = false;
    attributes.power_preference = WEBGL_POWER_PREFERENCE_HIGH_PERFORMANCE;
    attributes.fail_if_major_performance_caveat = false;
    attributes.enable_extensions_by_default = true;
    attributes.explicit_swap_control = false;
    attributes.proxy_context_to_main_thread = WEBGL_CONTEXT_PROXY_DISALLOW;
    attributes.render_via_offscreen_back_buffer = false;
    attributes.desynchronized = false;

    // SAFETY: `target` is a NUL-terminated string and `attributes` outlives the
    // call. `emscripten_webgl_create_context` only reads both.
    let handle = unsafe { emscripten_webgl_create_context(target.as_ptr(), &attributes) };
    if handle == 0 {
        return None;
    }

    log::debug!("created WebGL 2 context {handle} for canvas selector {selector:?}");

    u32::try_from(handle).ok()
}

// wgpu-hal's EGL backend calls the `EGL_EXT_platform_base` entry points when the
// EGL implementation advertises a real windowing system. Emscripten's EGL has
// none (it reports no extensions), so those code paths are unreachable, but the
// static library still has to resolve the symbols when an application links it.
// Returning NULL makes a hypothetical call fail gracefully instead of trapping.
#[no_mangle]
unsafe extern "C" fn eglGetPlatformDisplay(
    _platform: u32,
    _native_display: *mut c_void,
    _attrib_list: *const usize,
) -> *mut c_void {
    log::error!("eglGetPlatformDisplay is not implemented by Emscripten's EGL");
    std::ptr::null_mut()
}

#[no_mangle]
unsafe extern "C" fn eglCreatePlatformWindowSurface(
    _display: *mut c_void,
    _config: *mut c_void,
    _native_window: *mut c_void,
    _attrib_list: *const usize,
) -> *mut c_void {
    log::error!("eglCreatePlatformWindowSurface is not implemented by Emscripten's EGL");
    std::ptr::null_mut()
}
