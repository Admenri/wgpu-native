
project('wgpu-native', 'c', version : '0.0.0', license : 'MIT OR Apache-2.0')

inc_dirs = [ ]

subdir('ffi')

cargo = find_program('cargo')
cargo_args = ['build', '--release']
#determine which dir contain platform library
if host_machine.system() == 'emscripten'
    # Emscripten only has the GLES (WebGL 2) backend.
    cargo_args += ['--target', 'wasm32-unknown-emscripten', '--no-default-features', '--features', 'wgsl,gles']
    lib_path = 'target/wasm32-unknown-emscripten/release/libwgpu_native.a'
elif ['linux','android','darwin','cygwin','freebsd','netbsd','openbsd'].contains(host_machine.system())
    lib_path='target/release/libwgpu_native.a'
else
    lib_path='target/release/wgpu_native.lib'
endif

#build with cargo
run_command(cargo, cargo_args, check: true)

#mark cargo artifact for installation
install_data(lib_path, install_dir : get_option('libdir'))

pkg = import('pkgconfig')
pkg.generate(name : 'wgpu-native',
             description : 'Headers for using wgpu-native',
             libraries : ['-L${libdir}', '-lwgpu_native'],
             version : meson.project_version(),
             )
