# wgpu-native
[![Matrix](https://img.shields.io/badge/Matrix-%23wgpu%3Amatrix.org-blueviolet.svg)](https://matrix.to/#/#wgpu:matrix.org)
[![Build Status](https://github.com/gfx-rs/wgpu-native/workflows/CI/badge.svg)](https://github.com/gfx-rs/wgpu-native/actions)

This is a native WebGPU implementation in Rust, based on [wgpu-core](https://github.com/gfx-rs/wgpu).
The bindings are based on the WebGPU-native header found at `ffi/webgpu-headers/webgpu.h` and wgpu-native specific items in `ffi/wgpu.h`

# Bindings

> [!NOTE]
> *This list is also available on the [wgpu website](https://wgpu.rs/#bindings).*

- [gfx-rs/wgpu-rs](https://github.com/gfx-rs/wgpu/tree/master/wgpu) - idiomatic Rust wrapper with [a few more examples](https://github.com/gfx-rs/wgpu/tree/master/wgpu/examples) to get a feel of the API
- [pygfx/wgpu-py](https://github.com/pygfx/wgpu-py) - Python wrapper
- [trivaxy/wgpu.NET](https://github.com/trivaxy/WGPU.NET) - Raw .NET bindings with optional wrappers
- [dotnet/Silk.NET](https://github.com/dotnet/Silk.NET) - Raw .NET bindings
- [Alimer.Bindings.WebGPU](https://github.com/amerkoleci/Alimer.Bindings.WebGPU) - Cross platform .NET bindings for WebGPU
- [wgpu.cr](https://github.com/chances/wgpu-crystal) - Crystal wrapper
- [bindc-wgpu](https://github.com/gecko0307/bindbc-wgpu) - D wrapper ([package](https://code.dlang.org/packages/bindbc-wgpu))
- [porky11/wgpu](https://gitlab.com/scopes-libraries/wgpu) - experimental [Scopes](http://scopes.rocks) wrapper
- [cshenton/WebGPU.jl](https://github.com/cshenton/WebGPU.jl) - experimental Julia wrapper
- [dvijaha/WGPUNative.jl](https://github.com/dvijaha/WGPUNative.jl) - stable Julia wrapper
- [kgpu/wgpuj](https://github.com/kgpu/kgpu/tree/master/wgpuj) - Java/Kotlin wrapper
- [club-doki7/vulkan4j](https://github.com/club-doki7/vulkan4j) - Java wrapper (FFM-based)
- [xpenatan/jWebGPU](https://github.com/xpenatan/jWebGPU) / [gdx-webgpu](https://github.com/MonstrousSoftware/gdx-webgpu) - Java/TeaVM wrapper for libGDX
- [wgpu4k/wgpu4k](https://github.com/wgpu4k/wgpu4k) / [wgpu4k/wgpu4k-native](https://github.com/wgpu4k/wgpu4k-native) - Kotlin/Multiplatform wrappers
- [karmakrafts/Multiplatform wgpu](https://git.karmakrafts.dev/kk/multiplatform-wgpu) - Kotlin/Native wrapper
- [go-webgpu/webgpu](https://github.com/go-webgpu/webgpu) - Go wrapper (Zero-CGO)
- [WebGPU-C++](https://github.com/eliemichel/WebGPU-Cpp) - Auto-generated C++ wrapper (developed for the [Learn WebGPU native](https://eliemichel.github.io/LearnWebGPU) course)
- [jai_wgpu_native](https://github.com/SogoCZE/jai_wgpu_native) - Raw Jai bindings
- [wgpu-mojo](https://github.com/Hundo1018/wgpu-mojo) - Mojo bindings for wgpu-native
- [WebGPU::Direct](https://github.com/atrodo/WebGPU-Direct) - Perl wrapper ([package](https://metacpan.org/pod/WebGPU::Direct))
- [bronter/wgpu_native_zig](https://github.com/bronter/wgpu_native_zig) - Zig wrapper

## Pre-built binaries

Automated 32 and 64-bit builds for MacOS, Windows and Linux are available as Github [releases](https://github.com/gfx-rs/wgpu-native/releases). Details can be found in the  [Binary Releases](https://github.com/gfx-rs/wgpu-native/wiki/Binary-Releases) page in the wiki.

## Usage

This repository contains C-language examples that link to the native library targets and perform basic rendering and computation. Please refer to our [Getting Started](https://github.com/gfx-rs/wgpu-native/wiki/Getting-Started) page at the wiki for more information.

There's also a (small) [contributor guide](https://github.com/gfx-rs/wgpu-native/wiki/Contributing).

## Emscripten

wgpu-native builds for `wasm32-unknown-emscripten`, where it renders through WebGL 2 (wgpu's
GLES backend). That is the only backend available there, so the desktop backends are disabled:

```sh
rustup target add wasm32-unknown-emscripten
source /path/to/emsdk/emsdk_env.sh   # exports EMSDK, which bindgen needs for the wasm32 headers

make lib-native-emscripten
# equivalent to:
cargo rustc --lib --crate-type staticlib --target wasm32-unknown-emscripten \
    --no-default-features --features wgsl,gles
```

Only the static library is produced: `emcc` refuses to link the `cdylib` variant of the crate
(an Emscripten module needs an entry point), so applications link
`target/wasm32-unknown-emscripten/debug/libwgpu_native.a` into their own module:

```sh
em++ main.cpp libwgpu_native.a -I ffi -I ffi/webgpu-headers -std=c++20 \
    -sMIN_WEBGL_VERSION=2 -sMAX_WEBGL_VERSION=2 \
    -sGL_ENABLE_GET_PROC_ADDRESS -sSTACK_SIZE=2097152 \
    -sALLOW_MEMORY_GROWTH=1 -fwasm-exceptions \
    -o index.html
```

* `-sGL_ENABLE_GET_PROC_ADDRESS` — wgpu-hal loads every GL entry point through
  `eglGetProcAddress`, which Emscripten only links in with this setting.
* `-sSTACK_SIZE` — Emscripten defaults to a 64 KiB stack, which is far too small for
  wgpu-core/naga; 2 MiB is a safe starting point.
* `-fwasm-exceptions` — Rust panics are compiled with WebAssembly exceptions.

`Browser.getCanvas()` is just `Module['canvas']` in current Emscripten versions, and its EGL
implementation always renders into that canvas, so it has to be assigned before the module
starts:

```html
<canvas id="canvas" width="640" height="480"></canvas>
<script>
  var Module = { canvas: document.getElementById('canvas') };
</script>
<script src="index.js"></script>
```

Surfaces are then created from an HTML canvas with
`WGPUEmscriptenSurfaceSourceCanvasHTMLSelector`:

```c
WGPUEmscriptenSurfaceSourceCanvasHTMLSelector canvas = {0};
canvas.chain.sType = WGPUSType_EmscriptenSurfaceSourceCanvasHTMLSelector;
canvas.selector = (WGPUStringView){ .data = "#canvas", .length = WGPU_STRLEN };

WGPUSurfaceDescriptor descriptor = {0};
descriptor.nextInChain = &canvas.chain;
WGPUSurface surface = wgpuInstanceCreateSurface(instance, &descriptor);
```

> [!NOTE]
> Emscripten's EGL implementation, which the backend renders through, always binds to the
> Emscripten module canvas, so `selector` has to resolve to that element. Leaving the structure
> out of the chain is equivalent to selecting the module canvas.

A complete C++ example lives in [examples/emscripten](examples/emscripten); build it with
`make example-emscripten` and serve `examples/emscripten/build` over HTTP
(`make run-example-emscripten`), since browsers refuse to fetch `.wasm` files from `file://`
URLs.

## Minimum Supported Rust Version

The minimum supported Rust version (MSRV) for wgpu-native is **1.87**. MSRV bumps are considered breaking changes.
