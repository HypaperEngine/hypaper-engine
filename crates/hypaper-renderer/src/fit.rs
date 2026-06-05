//! UV coordinate computation for texture-to-screen fit modes.

/// How a texture is scaled to fill the output surface.
///
/// Mirrors [`hypaper_types::layer::FitMode`] so the renderer does not need to
/// depend on that crate's full feature set on the hot rendering path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FitMode {
    /// Scale uniformly to cover the entire surface (may crop edges).
    #[default]
    Fill,
    /// Scale uniformly to fit inside the surface (may letterbox).
    ///
    /// The UV coordinates returned for this mode may extend outside `[0, 1]`.
    /// Callers that want true black letterboxing must bind a sampler with
    /// `AddressMode::ClampToBorder` and a black border colour.
    Fit,
    /// Stretch non-uniformly to fill exactly (no cropping, no letterboxing).
    Stretch,
}

/// Computes UV coordinates for the four corners of a fullscreen quad.
///
/// Returns `[u_BL, v_BL, u_BR, v_BR, u_TL, v_TL, u_TR, v_TR]` where BL/BR/TL/TR
/// are the bottom-left, bottom-right, top-left, and top-right screen corners.
///
/// In wgpu texture space `v = 0.0` is the top edge and `v = 1.0` the bottom edge.
///
/// If any dimension is zero the function returns the [`FitMode::Stretch`] UVs as a
/// safe fallback to avoid division by zero.
pub fn compute_uvs(
    fit: FitMode,
    texture_w: u32,
    texture_h: u32,
    screen_w: u32,
    screen_h: u32,
) -> [f32; 8] {
    // Safe fallback: full texture maps to full screen (Stretch).
    const STRETCH: [f32; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];

    if texture_w == 0 || texture_h == 0 || screen_w == 0 || screen_h == 0 {
        return STRETCH;
    }

    match fit {
        FitMode::Stretch => STRETCH,

        FitMode::Fill => {
            let tex_ar = texture_w as f32 / texture_h as f32;
            let scr_ar = screen_w as f32 / screen_h as f32;

            if tex_ar > scr_ar {
                // Texture is wider than the screen — crop left/right edges.
                let u_range = scr_ar / tex_ar;
                let u_min = (1.0 - u_range) * 0.5;
                let u_max = u_min + u_range;
                [u_min, 1.0, u_max, 1.0, u_min, 0.0, u_max, 0.0]
            } else {
                // Texture is taller than the screen — crop top/bottom edges.
                let v_range = tex_ar / scr_ar;
                let v_min = (1.0 - v_range) * 0.5;
                let v_max = v_min + v_range;
                [0.0, v_max, 1.0, v_max, 0.0, v_min, 1.0, v_min]
            }
        }

        FitMode::Fit => {
            let tex_ar = texture_w as f32 / texture_h as f32;
            let scr_ar = screen_w as f32 / screen_h as f32;

            if tex_ar > scr_ar {
                // Texture fits by width — letterbox top and bottom.
                let v_excess = (tex_ar / scr_ar - 1.0) * 0.5;
                let v_min = -v_excess;
                let v_max = 1.0 + v_excess;
                [0.0, v_max, 1.0, v_max, 0.0, v_min, 1.0, v_min]
            } else {
                // Texture fits by height — letterbox left and right.
                let u_excess = (scr_ar / tex_ar - 1.0) * 0.5;
                let u_min = -u_excess;
                let u_max = 1.0 + u_excess;
                [u_min, 1.0, u_max, 1.0, u_min, 0.0, u_max, 0.0]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stretch_returns_full_uvs() {
        let uvs = compute_uvs(FitMode::Stretch, 800, 600, 1920, 1080);
        assert_eq!(uvs, [0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_fill_same_aspect_ratio_returns_full_uvs() {
        // 16:9 texture on 16:9 screen — no crop needed.
        let uvs = compute_uvs(FitMode::Fill, 1920, 1080, 1920, 1080);
        assert_eq!(uvs, [0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_fill_wide_texture_crops_sides() {
        // 16:9 texture (1920×1080) on 4:3 screen (1280×960) — crop left/right.
        let uvs = compute_uvs(FitMode::Fill, 1920, 1080, 1280, 960);
        let tex_ar = 1920.0f32 / 1080.0;
        let scr_ar = 1280.0f32 / 960.0;
        let u_range = scr_ar / tex_ar;
        let u_min = (1.0 - u_range) * 0.5;
        let u_max = u_min + u_range;
        assert!((uvs[0] - u_min).abs() < 1e-5, "u_BL mismatch");
        assert!((uvs[2] - u_max).abs() < 1e-5, "u_BR mismatch");
        assert!((uvs[1] - 1.0).abs() < 1e-5, "v_BL should be 1.0");
        assert!((uvs[5] - 0.0).abs() < 1e-5, "v_TL should be 0.0");
    }

    #[test]
    fn test_fill_tall_texture_crops_top_bottom() {
        // 4:3 texture on 16:9 screen — crop top/bottom.
        let uvs = compute_uvs(FitMode::Fill, 800, 600, 1920, 1080);
        let tex_ar = 800.0f32 / 600.0;
        let scr_ar = 1920.0f32 / 1080.0;
        let v_range = tex_ar / scr_ar;
        let v_min = (1.0 - v_range) * 0.5;
        let v_max = v_min + v_range;
        assert!((uvs[1] - v_max).abs() < 1e-5, "v_BL mismatch");
        assert!((uvs[5] - v_min).abs() < 1e-5, "v_TL mismatch");
        assert!((uvs[0] - 0.0).abs() < 1e-5, "u_BL should be 0.0");
        assert!((uvs[2] - 1.0).abs() < 1e-5, "u_BR should be 1.0");
    }

    #[test]
    fn test_fit_wide_texture_letterboxes_vertically() {
        // 16:9 texture on 4:3 screen — bars on top/bottom (v exceeds [0,1]).
        let uvs = compute_uvs(FitMode::Fit, 1920, 1080, 1280, 960);
        // v_min < 0 and v_max > 1
        assert!(uvs[5] < 0.0, "v_TL should be negative for letterbox");
        assert!(uvs[1] > 1.0, "v_BL should exceed 1.0 for letterbox");
        assert!((uvs[0] - 0.0).abs() < 1e-5);
        assert!((uvs[2] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_fit_tall_texture_letterboxes_horizontally() {
        // 4:3 texture on 16:9 screen — bars on left/right (u exceeds [0,1]).
        let uvs = compute_uvs(FitMode::Fit, 800, 600, 1920, 1080);
        assert!(uvs[0] < 0.0, "u_BL should be negative for letterbox");
        assert!(uvs[2] > 1.0, "u_BR should exceed 1.0 for letterbox");
        assert!((uvs[1] - 1.0).abs() < 1e-5);
        assert!((uvs[5] - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_zero_dimension_returns_stretch_fallback() {
        let uvs = compute_uvs(FitMode::Fill, 0, 600, 1920, 1080);
        assert_eq!(uvs, [0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0]);
    }
}
