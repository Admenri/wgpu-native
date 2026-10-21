WILDCARD_SOURCE:=$(wildcard src/*.rs)

GIT_TAG=$(shell git describe --abbrev=0 --tags)
GIT_TAG_FULL=$(shell git describe --tags)
OS_NAME=

EXTRA_BUILD_ARGS=
TARGET_DIR=target
ifdef TARGET
	EXTRA_BUILD_ARGS=--target $(TARGET)
	TARGET_DIR=target/$(TARGET)
endif

ifndef ARCHIVE_NAME
	ARCHIVE_NAME=wgpu-$(TARGET)
endif

ifeq ($(OS),Windows_NT)
	OS_NAME=windows
else
	UNAME_S:=$(shell uname -s)
	ifeq ($(UNAME_S),Linux)
		OS_NAME=linux
	endif
	ifeq ($(UNAME_S),Darwin)
		OS_NAME=macos
	endif
endif

MKDIR_CMD:=mkdir -p

ifeq ($(origin MSYSTEM),undefined) # MSYSTEM env var is defined on msys2
	ifeq ($(OS),Windows_NT)
		SHELL:=cmd
		MKDIR_CMD=powershell -Command md -Force
	endif
endif

.PHONY: check test doc clear \
	lib-native lib-native-release lib-native-emscripten \
	example-emscripten run-example-emscripten \
	example-capture example-compute example-triangle \
	example-metal_interop \
	example-immediates example-immediates-release \
	example-metal_interop-release \
	run-example-immediates run-example-immediates-release \
	example-capture-release example-compute-release example-triangle-release \
	run-example-capture run-example-compute run-example-triangle run-example-metal_interop \
	run-example-capture-release run-example-compute-release run-example-triangle-release

package: lib-native lib-native-release
	mkdir -p dist
	echo "Zipping the binaries ..."
	echo "$(GIT_TAG_FULL)" > dist/wgpu-native-git-tag
	for RELEASE in debug release; do \
		ARCHIVEDIR=toarchive; \
		ARCHIVEFILE=$(ARCHIVE_NAME)-$$RELEASE.zip; \
		LIBDIR=$(TARGET_DIR)/$$RELEASE; \
		rm -r -f dist/$$ARCHIVEDIR; \
		rm -f dist/$$ARCHIVEFILE; \
		mkdir -p dist/$$ARCHIVEDIR/wgpu-native-meta; \
		mkdir -p dist/$$ARCHIVEDIR/include/webgpu; \
		mkdir -p dist/$$ARCHIVEDIR/lib; \
		cp ./dist/wgpu-native-git-tag                 dist/$$ARCHIVEDIR/wgpu-native-meta; \
		cp ./ffi/webgpu-headers/webgpu.yml            dist/$$ARCHIVEDIR/wgpu-native-meta; \
		cp ./ffi/webgpu-headers/webgpu.h              dist/$$ARCHIVEDIR/include/webgpu; \
		cp ./ffi/wgpu.h                               dist/$$ARCHIVEDIR/include/webgpu; \
		if [ $(OS_NAME) = linux ]; then \
			cp ./$$LIBDIR/libwgpu_native.so           dist/$$ARCHIVEDIR/lib; \
			cp ./$$LIBDIR/libwgpu_native.a            dist/$$ARCHIVEDIR/lib; \
		fi; \
		if [ $(OS_NAME) = macos ]; then \
			cp ./$$LIBDIR/libwgpu_native.dylib        dist/$$ARCHIVEDIR/lib; \
			cp ./$$LIBDIR/libwgpu_native.a            dist/$$ARCHIVEDIR/lib; \
		fi; \
		if [ $(OS_NAME) = windows ]; then \
			if [[ "$(TARGET)" == *"gnu"* ]]; then \
				cp ./$$LIBDIR/wgpu_native.dll         dist/$$ARCHIVEDIR/lib; \
				cp ./$$LIBDIR/libwgpu_native.a        dist/$$ARCHIVEDIR/lib; \
				cp ./$$LIBDIR/libwgpu_native.dll.a    dist/$$ARCHIVEDIR/lib; \
			else \
				cp ./$$LIBDIR/wgpu_native.dll         dist/$$ARCHIVEDIR/lib; \
				cp ./$$LIBDIR/wgpu_native.lib         dist/$$ARCHIVEDIR/lib; \
				cp ./$$LIBDIR/wgpu_native.dll.lib     dist/$$ARCHIVEDIR/lib; \
				cp ./$$LIBDIR/wgpu_native.pdb         dist/$$ARCHIVEDIR/lib; \
			fi;\
		fi; \
		cd dist/$$ARCHIVEDIR; \
		if [ $(OS_NAME) = windows ]; then \
			7z a -tzip ../$$ARCHIVEFILE *; \
		else \
			zip -r ../$$ARCHIVEFILE *; \
		fi; \
		cd ../..; \
		rm -r -f dist/$$ARCHIVEDIR; \
	done

clean:
	cargo clean
	rm -Rf examples/build

check:
	cargo check --all

test:
	cargo test --all

doc:
	cargo doc --all

clear:
	cargo clean

lib-native: Cargo.lock Cargo.toml Makefile $(WILDCARD_SOURCE)
	cargo build $(EXTRA_BUILD_ARGS)

lib-native-release: Cargo.lock Cargo.toml Makefile $(WILDCARD_SOURCE)
	cargo build --release $(EXTRA_BUILD_ARGS)

# Emscripten only has the GLES (WebGL 2) backend, so the desktop backends are
# disabled. Requires `rustup target add wasm32-unknown-emscripten` and an
# activated Emscripten SDK (see the README).
#
# Only the static library is produced: `emcc` refuses to link the `cdylib`
# variant, because an Emscripten module needs an entry point. Applications link
# `libwgpu_native.a` into their own module instead.
EMSCRIPTEN_TARGET=wasm32-unknown-emscripten
EMSCRIPTEN_LIB=target/$(EMSCRIPTEN_TARGET)/debug/libwgpu_native.a

lib-native-emscripten: Cargo.lock Cargo.toml Makefile $(WILDCARD_SOURCE)
	cargo rustc --lib --crate-type staticlib --target $(EMSCRIPTEN_TARGET) \
		--no-default-features --features wgsl,gles

# `-sGL_ENABLE_GET_PROC_ADDRESS` is required because wgpu-hal loads the GL
# entry points through `eglGetProcAddress`, and `-sSTACK_SIZE` has to be raised
# well above Emscripten's 64 KiB default for wgpu-core's validation code.
example-emscripten: lib-native-emscripten
	em++ -std=c++20 -O1 -g examples/emscripten/main.cpp $(EMSCRIPTEN_LIB) \
		-I ffi -I ffi/webgpu-headers \
		-sMIN_WEBGL_VERSION=2 -sMAX_WEBGL_VERSION=2 \
		-sGL_ENABLE_GET_PROC_ADDRESS -sSTACK_SIZE=2097152 \
		-sALLOW_MEMORY_GROWTH=1 -fwasm-exceptions \
		--shell-file examples/emscripten/shell.html \
		-o examples/emscripten/build/index.html

run-example-emscripten: example-emscripten
	cd examples/emscripten/build && python -m http.server 8000

examples-debug: lib-native
	cd examples && $(MKDIR_CMD) "build/Debug" && cd build/Debug && cmake -GNinja -DCMAKE_BUILD_TYPE=Debug -DCMAKE_EXPORT_COMPILE_COMMANDS=1 ../..

examples-release: lib-native-release
	cd examples && $(MKDIR_CMD) "build/RelWithDebInfo" && cd build/RelWithDebInfo && cmake -GNinja -DCMAKE_BUILD_TYPE=RelWithDebInfo -DCMAKE_EXPORT_COMPILE_COMMANDS=1 ../..

example-immediates: examples-debug
	cd examples/build/Debug && cmake --build . --target immediates

run-example-immediates: example-immediates
	cd examples/immediates && "../build/Debug/immediates/immediates"

example-immediates-release: examples-release
	cd examples/build/RelWithDebInfo && cmake --build . --target immediates

run-example-immediates-release: example-immediates-release
	cd examples/immediates && "../build/RelWithDebInfo/immediates/immediates"

example-capture: examples-debug
	cd examples/build/Debug && cmake --build . --target capture

run-example-capture: example-capture
	cd examples/capture && "../build/Debug/capture/capture"

example-capture-release: examples-release
	cd examples/build/RelWithDebInfo && cmake --build . --target capture

run-example-capture-release: example-capture-release
	cd examples/capture && "../build/RelWithDebInfo/capture/capture"

example-compute: examples-debug
	cd examples/build/Debug && cmake --build . --target compute

run-example-compute: example-compute
	cd examples/compute && "../build/Debug/compute/compute"

example-compute-release: examples-release
	cd examples/build/RelWithDebInfo && cmake --build . --target compute

run-example-compute-release: example-compute-release
	cd examples/compute && "../build/RelWithDebInfo/compute/compute"

example-enumerate_adapters: examples-debug
	cd examples/build/Debug && cmake --build . --target enumerate_adapters

run-example-enumerate_adapters: example-enumerate_adapters
	cd examples/enumerate_adapters && "../build/Debug/enumerate_adapters/enumerate_adapters"

example-enumerate_adapters-release: examples-release
	cd examples/build/RelWithDebInfo && cmake --build . --target enumerate_adapters

run-example-enumerate_adapters-release: example-enumerate_adapters-release
	cd examples/triangle && "../build/RelWithDebInfo/enumerate_adapters/enumerate_adapters"

example-texture_arrays: examples-debug
	cd examples/build/Debug && cmake --build . --target texture_arrays

run-example-texture_arrays: example-texture_arrays
	cd examples/texture_arrays && "../build/Debug/texture_arrays/texture_arrays"

example-metal_interop: examples-debug
	cd examples/build/Debug && cmake --build . --target metal_interop

run-example-metal_interop: example-metal_interop
	cd examples/metal_interop && "../build/Debug/metal_interop/metal_interop"

example-metal_interop-release: examples-release
	cd examples/build/RelWithDebInfo && cmake --build . --target metal_interop

run-example-metal_interop-release: example-metal_interop-release
	cd examples/metal_interop && "../build/RelWithDebInfo/metal_interop/metal_interop"

example-texture_arrays-release: examples-release
	cd examples/build/RelWithDebInfo && cmake --build . --target texture_arrays

run-example-texture_arrays-release: example-texture_arrays-release
	cd examples/texture_arrays && "../build/RelWithDebInfo/texture_arrays/texture_arrays"

example-triangle: examples-debug
	cd examples/build/Debug && cmake --build . --target triangle

run-example-triangle: example-triangle
	cd examples/triangle && "../build/Debug/triangle/triangle"

example-triangle-release: examples-release
	cd examples/build/RelWithDebInfo && cmake --build . --target triangle

run-example-triangle-release: example-triangle-release
	cd examples/triangle && "../build/RelWithDebInfo/triangle/triangle"
