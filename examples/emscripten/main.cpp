// Minimal wgpu-native + Emscripten example.
//
// Creates a surface for the HTML canvas with id "canvas", configures it for
// WebGL 2 and draws a triangle from a render loop driven by
// emscripten_set_main_loop().
//
// Build with `make example-emscripten`, then serve the output directory over
// HTTP (browsers refuse to fetch .wasm from file:// URLs) and open index.html.

#include "webgpu-headers/webgpu.h"
#include "wgpu.h"

#include <cassert>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include <emscripten/emscripten.h>

namespace {

constexpr int kWidth = 640;
constexpr int kHeight = 480;

// wgpu-native declares its own structure types in `WGPUNativeSType`; C++
// requires an explicit conversion to the `WGPUSType` field type.
constexpr WGPUSType native_stype(WGPUNativeSType s_type) {
    return static_cast<WGPUSType>(s_type);
}

const char *kShader = R"(
struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    var positions = array<vec2f, 3>(
        vec2f(0.0, 0.6),
        vec2f(-0.6, -0.5),
        vec2f(0.6, -0.5),
    );
    var colors = array<vec4f, 3>(
        vec4f(1.0, 0.2, 0.2, 1.0),
        vec4f(0.2, 1.0, 0.2, 1.0),
        vec4f(0.2, 0.4, 1.0, 1.0),
    );

    var out: VertexOut;
    out.position = vec4f(positions[index], 0.0, 1.0);
    out.color = colors[index];
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    return in.color;
}
)";

struct Demo {
    WGPUInstance instance = nullptr;
    WGPUSurface surface = nullptr;
    WGPUAdapter adapter = nullptr;
    WGPUDevice device = nullptr;
    WGPUQueue queue = nullptr;
    WGPURenderPipeline pipeline = nullptr;
    WGPUSurfaceConfiguration config = {};
    unsigned frame = 0;
};

Demo demo;

void print_string_view(const char *what, WGPUStringView message) {
    const int length = message.length == WGPU_STRLEN
                           ? static_cast<int>(std::strlen(message.data))
                           : static_cast<int>(message.length);
    std::printf("[wgpu-emscripten] %s: %.*s\n", what, length, message.data);
}

void on_adapter(WGPURequestAdapterStatus status, WGPUAdapter adapter,
                WGPUStringView message, void *userdata1, void *) {
    if (status == WGPURequestAdapterStatus_Success) {
        static_cast<Demo *>(userdata1)->adapter = adapter;
    } else {
        print_string_view("requestAdapter failed", message);
    }
}

void on_device(WGPURequestDeviceStatus status, WGPUDevice device,
               WGPUStringView message, void *userdata1, void *) {
    if (status == WGPURequestDeviceStatus_Success) {
        static_cast<Demo *>(userdata1)->device = device;
    } else {
        print_string_view("requestDevice failed", message);
    }
}

void on_uncaptured_error(WGPUDevice const *, WGPUErrorType type,
                         WGPUStringView message, void *, void *) {
    std::printf("[wgpu-emscripten] uncaptured error (type %u)\n",
                static_cast<unsigned>(type));
    print_string_view("error", message);
}

WGPUShaderModule create_shader_module(WGPUDevice device) {
    WGPUShaderSourceWGSL wgsl = {};
    wgsl.chain.sType = WGPUSType_ShaderSourceWGSL;
    wgsl.code = WGPUStringView{kShader, WGPU_STRLEN};

    WGPUShaderModuleDescriptor descriptor = {};
    descriptor.label = WGPUStringView{"triangle shader", WGPU_STRLEN};
    descriptor.nextInChain = &wgsl.chain;

    WGPUShaderModule module = wgpuDeviceCreateShaderModule(device, &descriptor);
    assert(module);
    return module;
}

WGPURenderPipeline create_pipeline(WGPUDevice device, WGPUShaderModule module,
                                   WGPUTextureFormat format) {
    WGPUPipelineLayoutDescriptor layout_descriptor = {};
    layout_descriptor.label = WGPUStringView{"pipeline layout", WGPU_STRLEN};
    WGPUPipelineLayout layout =
        wgpuDeviceCreatePipelineLayout(device, &layout_descriptor);
    assert(layout);

    WGPUColorTargetState color_target = {};
    color_target.format = format;
    color_target.writeMask = WGPUColorWriteMask_All;

    WGPUFragmentState fragment = {};
    fragment.module = module;
    fragment.entryPoint = WGPUStringView{"fs_main", WGPU_STRLEN};
    fragment.targetCount = 1;
    fragment.targets = &color_target;

    WGPUVertexState vertex = {};
    vertex.module = module;
    vertex.entryPoint = WGPUStringView{"vs_main", WGPU_STRLEN};

    WGPUPrimitiveState primitive = {};
    primitive.topology = WGPUPrimitiveTopology_TriangleList;

    WGPUMultisampleState multisample = {};
    multisample.count = 1;
    multisample.mask = 0xFFFFFFFF;

    WGPURenderPipelineDescriptor descriptor = {};
    descriptor.label = WGPUStringView{"triangle pipeline", WGPU_STRLEN};
    descriptor.layout = layout;
    descriptor.vertex = vertex;
    descriptor.primitive = primitive;
    descriptor.multisample = multisample;
    descriptor.fragment = &fragment;

    WGPURenderPipeline pipeline = wgpuDeviceCreateRenderPipeline(device, &descriptor);
    assert(pipeline);

    wgpuPipelineLayoutRelease(layout);
    return pipeline;
}

void on_log(WGPULogLevel level, WGPUStringView message, void *) {
    std::printf("[wgpu-emscripten] log(%u): %.*s\n", static_cast<unsigned>(level),
                message.length == WGPU_STRLEN ? static_cast<int>(std::strlen(message.data))
                                              : static_cast<int>(message.length),
                message.data);
}

void init() {
    wgpuSetLogCallback(on_log, nullptr);
    wgpuSetLogLevel(WGPULogLevel_Info);

    demo.instance = wgpuCreateInstance(nullptr);
    assert(demo.instance);

    // The surface is backed by the canvas selected through
    // WGPUEmscriptenSurfaceSourceCanvasHTMLSelector.
    WGPUEmscriptenSurfaceSourceCanvasHTMLSelector canvas = {};
    canvas.chain.sType = native_stype(WGPUSType_EmscriptenSurfaceSourceCanvasHTMLSelector);
    canvas.selector = WGPUStringView{"#canvas", WGPU_STRLEN};

    WGPUSurfaceDescriptor surface_descriptor = {};
    surface_descriptor.label = WGPUStringView{"canvas surface", WGPU_STRLEN};
    surface_descriptor.nextInChain = &canvas.chain;
    demo.surface = wgpuInstanceCreateSurface(demo.instance, &surface_descriptor);
    assert(demo.surface);

    WGPURequestAdapterOptions adapter_options = {};
    adapter_options.compatibleSurface = demo.surface;

    WGPURequestAdapterCallbackInfo adapter_callback = {};
    adapter_callback.callback = on_adapter;
    adapter_callback.userdata1 = &demo;
    wgpuInstanceRequestAdapter(demo.instance, &adapter_options, adapter_callback);
    assert(demo.adapter);

    WGPUUncapturedErrorCallbackInfo error_callback = {};
    error_callback.callback = on_uncaptured_error;

    WGPUDeviceDescriptor device_descriptor = {};
    device_descriptor.label = WGPUStringView{"device", WGPU_STRLEN};
    device_descriptor.uncapturedErrorCallbackInfo = error_callback;

    WGPURequestDeviceCallbackInfo device_callback = {};
    device_callback.callback = on_device;
    device_callback.userdata1 = &demo;
    wgpuAdapterRequestDevice(demo.adapter, &device_descriptor, device_callback);
    assert(demo.device);

    demo.queue = wgpuDeviceGetQueue(demo.device);
    assert(demo.queue);

    WGPUSurfaceCapabilities capabilities = {};
    wgpuSurfaceGetCapabilities(demo.surface, demo.adapter, &capabilities);
    assert(capabilities.formatCount > 0 && capabilities.alphaModeCount > 0);

    const WGPUTextureFormat format = capabilities.formats[0];
    const WGPUCompositeAlphaMode alpha_mode = capabilities.alphaModes[0];
    wgpuSurfaceCapabilitiesFreeMembers(capabilities);

    WGPUShaderModule module = create_shader_module(demo.device);
    demo.pipeline = create_pipeline(demo.device, module, format);
    wgpuShaderModuleRelease(module);

    demo.config = (const WGPUSurfaceConfiguration){
        .device = demo.device,
        .format = format,
        .usage = WGPUTextureUsage_RenderAttachment,
        .width = kWidth,
        .height = kHeight,
        .alphaMode = alpha_mode,
        .presentMode = WGPUPresentMode_Fifo,
    };
    wgpuSurfaceConfigure(demo.surface, &demo.config);

    std::printf("[wgpu-emscripten] surface configured: %dx%d, format %#x\n", kWidth,
                kHeight, static_cast<unsigned>(format));
}

void frame() {
    WGPUSurfaceTexture surface_texture = {};
    wgpuSurfaceGetCurrentTexture(demo.surface, &surface_texture);
    if (surface_texture.status != WGPUSurfaceGetCurrentTextureStatus_SuccessOptimal &&
        surface_texture.status != WGPUSurfaceGetCurrentTextureStatus_SuccessSuboptimal) {
        std::printf("[wgpu-emscripten] getCurrentTexture status=%#.8x\n",
                    static_cast<unsigned>(surface_texture.status));
        if (surface_texture.texture) {
            wgpuTextureRelease(surface_texture.texture);
        }
        return;
    }
    assert(surface_texture.texture);

    WGPUTextureView view = wgpuTextureCreateView(surface_texture.texture, nullptr);
    assert(view);

    WGPUCommandEncoder encoder = wgpuDeviceCreateCommandEncoder(demo.device, nullptr);
    assert(encoder);

    WGPURenderPassColorAttachment color_attachment = {};
    color_attachment.view = view;
    color_attachment.depthSlice = WGPU_DEPTH_SLICE_UNDEFINED;
    color_attachment.loadOp = WGPULoadOp_Clear;
    color_attachment.storeOp = WGPUStoreOp_Store;
    color_attachment.clearValue = WGPUColor{0.05f, 0.05f, 0.10f, 1.0f};

    WGPURenderPassDescriptor render_pass_descriptor = {};
    render_pass_descriptor.colorAttachmentCount = 1;
    render_pass_descriptor.colorAttachments = &color_attachment;

    WGPURenderPassEncoder pass =
        wgpuCommandEncoderBeginRenderPass(encoder, &render_pass_descriptor);
    assert(pass);

    wgpuRenderPassEncoderSetPipeline(pass, demo.pipeline);
    wgpuRenderPassEncoderDraw(pass, 3, 1, 0, 0);
    wgpuRenderPassEncoderEnd(pass);
    wgpuRenderPassEncoderRelease(pass);

    WGPUCommandBuffer commands = wgpuCommandEncoderFinish(encoder, nullptr);
    assert(commands);

    wgpuQueueSubmit(demo.queue, 1, &commands);
    wgpuSurfacePresent(demo.surface);

    if (demo.frame == 0) {
        std::printf("[wgpu-emscripten] first frame presented\n");
    }

    wgpuCommandBufferRelease(commands);
    wgpuCommandEncoderRelease(encoder);
    wgpuTextureViewRelease(view);
    wgpuTextureRelease(surface_texture.texture);

    // Let the page (and headless test harnesses) know that frames are being
    // produced.
    demo.frame += 1;
    EM_ASM({ window.wgpuNativeFrames = $0; }, demo.frame);
}

} // namespace

int main() {
    init();
    emscripten_set_main_loop(frame, 0, 1);
    return 0;
}
