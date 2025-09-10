// File: crates/chart-render-skia/src/lib.rs
// Summary: Skia renderer crate: surfaces and (future) GPU contexts. CPU path works everywhere.

use skia_safe as skia;

pub mod surfaces {
    use skia_safe as skia;

    /// Create a CPU raster surface (n32 premul) with given width/height.
    pub fn cpu_surface(width: i32, height: i32) -> Option<skia::Surface> {
        skia::surfaces::raster_n32_premul((width, height))
    }

    /// Placeholder for GPU-backed surface creation.
    /// Currently returns None; wiring depends on chosen backend (GL/Vulkan/Metal/D3D).
    /// Once enabled, provide functions to create a GPU DirectContext and render target.
    // Placeholder (intentionally not referencing GPU types to avoid feature requirements).
    // Once GPU backends are enabled on skia-safe, add constructors here.
    pub fn gpu_surface_placeholder() -> Option<skia::Surface> { None }
}

pub struct SkiaRenderer;

impl SkiaRenderer {
    pub fn new() -> Self { Self }

    pub fn cpu_surface(&self, width: i32, height: i32) -> Option<skia::Surface> {
        surfaces::cpu_surface(width, height)
    }
}

// Optional: OpenGL GPU scaffolding (behind feature). Not enabled by default.
// This avoids pulling GPU deps in normal builds and lets us wire GL/Vulkan later.
#[cfg(feature = "gpu-gl")]
pub mod gpu_gl {
    use skia_safe as skia;

    pub struct GpuGlContext {
        pub direct: skia::gpu::DirectContext,
    }

    impl GpuGlContext {
        /// Create a DirectContext from a GL interface. Caller provides a valid loader.
        pub fn from_interface(interface: skia::gpu::gl::Interface) -> Option<Self> {
            let ctx = skia::gpu::direct_contexts::make_gl(interface, None)?;
            Some(Self { direct: ctx })
        }

        /// Create a GPU surface for a backend render target (FBO/RT).
        pub fn surface_from_target(
            &mut self,
            target: &skia::gpu::BackendRenderTarget,
            origin: skia::gpu::SurfaceOrigin,
            color_type: skia::ColorType,
        ) -> Option<skia::Surface> {
            skia::gpu::surfaces::wrap_backend_render_target(&mut self.direct, target, origin, color_type, None, None)
        }
    }
}
