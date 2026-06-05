//! Main render loop: swap-chain management, image upload, and per-frame drawing.

use bytemuck::cast_slice;
use wgpu::util::DeviceExt;

use crate::{
    context::{create_context_for_surface, RenderContext},
    error::RendererError,
    fit::{compute_uvs, FitMode},
    pipeline::{create_fullscreen_pipeline, RenderPipeline},
    texture::{load_texture_from_bytes, GpuTexture},
};

/// The main renderer: owns all GPU state required to display a wallpaper on a
/// wlr-layer-shell background surface.
pub struct Renderer {
    /// Wgpu adapter, device, and command queue.
    pub context: RenderContext,
    /// The wgpu surface backed by the Wayland layer-shell window.
    pub surface: wgpu::Surface<'static>,
    /// Swap-chain configuration; mutated by [`resize`](Self::resize).
    pub surface_config: wgpu::SurfaceConfiguration,
    /// Compiled fullscreen textured-quad pipeline.
    pub pipeline: RenderPipeline,
    /// Currently active wallpaper texture, or `None` before the first
    /// [`set_image`](Self::set_image) call.
    pub current_texture: Option<GpuTexture>,
    /// Surface width in pixels.
    pub width: u32,
    /// Surface height in pixels.
    pub height: u32,
    /// How the texture is scaled to fill the surface.
    fit_mode: FitMode,
}

impl Renderer {
    /// Initialises a renderer attached to a live Wayland surface.
    ///
    /// Creates the wgpu context via [`create_context_for_surface`], auto-selects
    /// an sRGB swap-chain format, configures the surface, and compiles the render
    /// pipeline.  The renderer produces black frames until
    /// [`set_image`](Self::set_image) is called.
    ///
    /// # Safety
    ///
    /// Both `raw.display_ptr` and `raw.surface_ptr` must be valid for the entire
    /// lifetime of the returned `Renderer`.  In practice the originating
    /// [`hypaper_wayland::surface::WaylandSurface`] must outlive this renderer.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError`] if the wgpu context cannot be created or the
    /// surface reports no compatible formats.
    pub async fn new(
        raw: &hypaper_wayland::raw_handle::RawWindowHandle,
        width: u32,
        height: u32,
    ) -> Result<Self, RendererError> {
        // SAFETY: Caller guarantees that both raw pointers are valid and will
        // remain valid for the lifetime of the returned Renderer.
        let (context, surface) = unsafe { create_context_for_surface(raw, width, height).await? };

        let caps = surface.get_capabilities(&context.adapter);

        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| RendererError::Surface("no supported surface format".into()))?;

        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&context.device, &surface_config);

        let pipeline = create_fullscreen_pipeline(&context.device, format);

        tracing::info!(width, height, ?format, "renderer initialised");

        Ok(Renderer {
            context,
            surface,
            surface_config,
            pipeline,
            current_texture: None,
            width,
            height,
            fit_mode: FitMode::Fill,
        })
    }

    /// Decodes `bytes` as an image and uploads it to the GPU as the active wallpaper.
    ///
    /// Supports any format recognised by the [`image`] crate (PNG, JPEG, WebP, …).
    /// The previous texture is replaced and its GPU memory is freed immediately.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::Texture`] if the bytes cannot be decoded.
    pub fn set_image(&mut self, bytes: &[u8]) -> Result<(), RendererError> {
        let tex = load_texture_from_bytes(
            &self.context.device,
            &self.context.queue,
            bytes,
            "wallpaper",
        )?;
        tracing::debug!(
            width = tex.width,
            height = tex.height,
            "wallpaper texture uploaded",
        );
        self.current_texture = Some(tex);
        Ok(())
    }

    /// Renders one frame to the swap chain.
    ///
    /// If no image has been set the frame is silently skipped.  UV coordinates
    /// are recomputed every frame from the current [`FitMode`] and texture
    /// dimensions, then uploaded via a per-frame uniform buffer.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::Render`] if the swap-chain texture cannot be
    /// acquired or command encoding fails.
    pub fn render(&mut self) -> Result<(), RendererError> {
        let tex = match &self.current_texture {
            Some(t) => t,
            None => return Ok(()),
        };

        let uvs = compute_uvs(
            self.fit_mode,
            tex.width,
            tex.height,
            self.width,
            self.height,
        );

        // Upload UV corners as two vec4s (32 bytes) into a per-frame uniform buffer.
        let uv_buffer = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uv-uniform"),
                contents: cast_slice(&uvs),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("wallpaper-bg"),
                layout: &self.pipeline.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&tex.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&tex.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uv_buffer.as_entire_binding(),
                    },
                ],
            });

        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| RendererError::Render(e.to_string()))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("frame-encoder"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fullscreen-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline.inner);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        self.context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Reconfigures the swap chain to the new surface dimensions.
    ///
    /// Must be called whenever the Wayland compositor sends a new configure event.
    /// Calls with zero width or height are ignored.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface
            .configure(&self.context.device, &self.surface_config);
        tracing::debug!(width, height, "renderer resized");
    }
}
