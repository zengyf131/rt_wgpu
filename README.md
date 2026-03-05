# WebGPU Ray Tracing Renderer

This is a Ray Tracing Renderer written in Rust and WGSL (WebGPU Shading Language). It serves as a course project of Advance Computer Graphics (COMP 5411) of HKUST. 

It is derived from and heavily inspired by the [Ray Tracing in One Weekend](https://github.com/RayTracing/raytracing.github.io) series, which is a great tutorial for building an offline CPU-based Path Tracing renderer from scratch. This renderer reimplements it with WebGPU and extends it with more functionalities such as Wavefront Path Tracing and multiple importance sampling. 

An online demo is available [here](https://yunfan.zone/rt_wgpu). A technical report is included [here](report.pdf). 

## Interaction

The renderer requires a web browser that supports WebGPU (e.g., latest Chrome). It starts from a config menu, where you can choose:

- Scene
- Rendering algorithm: (Mega-kernel) Path Tracing / Wavefront Path Tracing
- Sampling strategy: material / light / multiple importance sampling
- Sample per pixel: when set to 0, defaults to scene-specific setting


Click __render__ to start rendering the scene.

During rendering, the user can also control the camera using a mouse similar to that in Blender: 
- Holding right click while dragging: orbits the camera
- Scrolling the wheel: zoom in or out
- Holding middle click while dragging: moves the focus position

The user can also hide or unhide the GUI by pressing H key. To switch to another scene, click __config__ and go back to config menu.

## Build

The renderer is written in Rust and targets WebAssembly (WASM). It is currently built with `rustc 1.92.0-nightly` and `wasm-pack 0.13.1`. They are required for building this project.

To build the project, run the following command:

```
wasm-pack build --target web
```

Then the package for release should be available in `./pkg`. It can be deployed on a web server together with `./index.html`. 

To test it locally, [sfz](https://github.com/weihanglo/sfz) is recommended. Run the following command to start a local server:

```
sfz -r --coi
```

