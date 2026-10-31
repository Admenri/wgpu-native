use std::env;
use std::path::PathBuf;
use std::process::Command;

fn sdk_path(sdk_name: &str) -> String {
    let output = Command::new("xcrun")
        .args(["--sdk", sdk_name, "--show-sdk-path"])
        .output()
        .expect("xcrun failed")
        .stdout;
    std::str::from_utf8(&output)
        .expect("invalid output from `xcrun`")
        .trim()
        .to_owned()
}

/// Emscripten sysroot, used to lay out the bindings for
/// `wasm32-unknown-emscripten` when the Emscripten SDK is available.
fn emscripten_sysroot() -> Option<PathBuf> {
    let sysroot = PathBuf::from(env::var("EMSDK").ok()?)
        .join("upstream")
        .join("emscripten")
        .join("cache")
        .join("sysroot");
    sysroot.is_dir().then_some(sysroot)
}

fn main() {
    #[cfg(windows)]
    thunk::thunk();

    println!("cargo:rerun-if-changed=ffi/webgpu-headers/webgpu.h");
    println!("cargo:rerun-if-changed=ffi/wgpu.h");
    println!("cargo:rerun-if-env-changed=TARGET");
    println!("cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS");
    println!("cargo:rerun-if-env-changed=EMSDK");

    #[rustfmt::skip]
    let types_to_rename = vec![
        ("WGPUAdapter", "WGPUAdapterImpl"),
        ("WGPUBindGroup", "WGPUBindGroupImpl"),
        ("WGPUBindGroupLayout", "WGPUBindGroupLayoutImpl"),
        ("WGPUBuffer", "WGPUBufferImpl"),
        ("WGPUCommandBuffer", "WGPUCommandBufferImpl"),
        ("WGPUCommandEncoder", "WGPUCommandEncoderImpl"),
        ("WGPUComputePassEncoder", "WGPUComputePassEncoderImpl"),
        ("WGPUComputePipeline", "WGPUComputePipelineImpl"),
        ("WGPUDevice", "WGPUDeviceImpl"),
        ("WGPUInstance", "WGPUInstanceImpl"),
        ("WGPUPipelineLayout", "WGPUPipelineLayoutImpl"),
        ("WGPUQuerySet", "WGPUQuerySetImpl"),
        ("WGPUQueue", "WGPUQueueImpl"),
        ("WGPURenderBundle", "WGPURenderBundleImpl"),
        ("WGPURenderBundleEncoder", "WGPURenderBundleEncoderImpl"),
        ("WGPURenderPassEncoder", "WGPURenderPassEncoderImpl"),
        ("WGPURenderPipeline", "WGPURenderPipelineImpl"),
        ("WGPUSampler", "WGPUSamplerImpl"),
        ("WGPUShaderModule", "WGPUShaderModuleImpl"),
        ("WGPUSurface", "WGPUSurfaceImpl"),
        ("WGPUTexture", "WGPUTextureImpl"),
        ("WGPUTextureView", "WGPUTextureViewImpl"),
    ];
    let mut builder = bindgen::Builder::default()
        .header("ffi/wgpu.h")
        .clang_arg("-Iffi/webgpu-headers")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_item("WGPU.*")
        .allowlist_item("wgpu.*")
        .blocklist_function("wgpuGetProcAddress")
        .prepend_enum_name(false)
        .size_t_is_usize(true)
        .ignore_functions()
        .layout_tests(true)
        .clang_macro_fallback();

    for (old_name, new_name) in types_to_rename {
        let line = format!("pub type {old_name} = *const crate::{new_name};");
        builder = builder
            .blocklist_type(old_name)
            .blocklist_type(format!("{old_name}Impl"))
            .raw_line(line);
    }

    if let Ok(target) = env::var("TARGET") {
        match target.as_str() {
            "aarch64-apple-ios" => {
                builder = builder
                    .clang_arg("-isysroot")
                    .clang_arg(sdk_path("iphoneos"))
                    .clang_arg("--target=arm64-apple-ios");
            }
            "aarch64-apple-ios-sim" => {
                builder = builder
                    .clang_arg("-isysroot")
                    .clang_arg(sdk_path("iphonesimulator"))
                    .clang_arg("--target=arm64-apple-ios-simulator");
            }
            "x86_64-apple-ios" => {
                builder = builder
                    .clang_arg("-isysroot")
                    .clang_arg(sdk_path("iphonesimulator"))
                    .clang_arg("--target=x86_64-apple-ios-simulator");
            }
            "aarch64-apple-darwin" | "x86_64-apple-darwin" => {
                builder = builder.clang_arg("-isysroot").clang_arg(sdk_path("macosx"));
            }
            // Lay the bindings out for wasm32 (32-bit pointers) instead of the
            // host, otherwise every structure containing a pointer or a
            // `size_t` would have the wrong size and offset on Emscripten.
            "wasm32-unknown-emscripten" => {
                builder = builder.clang_arg("--target=wasm32-unknown-emscripten");
                match emscripten_sysroot() {
                    Some(sysroot) => {
                        // Clang's Emscripten toolchain does not search
                        // `<sysroot>/include` on its own (emcc passes those
                        // directories explicitly), so add them by hand.
                        for dir in [sysroot.join("include"), sysroot.join("include/compat")] {
                            if dir.is_dir() {
                                builder = builder.clang_arg(format!(
                                    "-I{}",
                                    dir.display().to_string().replace('\\', "/")
                                ));
                            }
                        }
                    }
                    // Without the Emscripten SDK there is no libc for that
                    // target, so fall back to Clang's freestanding headers
                    // (they are still laid out for wasm32).
                    None => {
                        builder = builder.clang_arg("-ffreestanding");
                    }
                }
            }
            _ => {}
        }
    }

    let bindings = builder.generate().unwrap_or_else(|error| {
        if env::var("TARGET").as_deref() == Ok("wasm32-unknown-emscripten")
            && emscripten_sysroot().is_none()
        {
            panic!(
                "Unable to generate bindings: {error}\n\n\
                 hint: building for Emscripten needs the Emscripten SDK so that bindgen can find \
                 the wasm32 libc headers. Activate the SDK (`source ./emsdk_env.sh`, which exports \
                 `EMSDK`), or point `BINDGEN_EXTRA_CLANG_ARGS` at the headers."
            );
        }
        panic!("Unable to generate bindings: {error}");
    });
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
