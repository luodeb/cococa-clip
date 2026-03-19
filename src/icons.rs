#[cfg(target_os = "macos")]
use png::{BitDepth, ColorType, Encoder};
#[cfg(target_os = "macos")]
use resvg::tiny_skia::{Pixmap, Transform};
#[cfg(target_os = "macos")]
use resvg::usvg::{Options, Tree};

#[cfg(target_os = "macos")]
pub fn render_svg_rgba(svg: &str, canvas_size: u32) -> Result<Vec<u8>, String> {
    let options = Options::default();
    let tree = Tree::from_str(svg, &options)
        .map_err(|error| format!("Failed to parse SVG icon: {error}"))?;

    let svg_size = tree.size();
    let width = svg_size.width();
    let height = svg_size.height();
    let scale = f32::min(canvas_size as f32 / width, canvas_size as f32 / height);
    let translate_x = (canvas_size as f32 - width * scale) * 0.5;
    let translate_y = (canvas_size as f32 - height * scale) * 0.5;

    let mut pixmap = Pixmap::new(canvas_size, canvas_size)
        .ok_or_else(|| "Failed to allocate SVG pixmap".to_owned())?;
    let transform = Transform::from_scale(scale, scale).post_translate(translate_x, translate_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Ok(pixmap.take())
}

#[cfg(target_os = "macos")]
pub fn render_svg_png(svg: &str, canvas_size: u32) -> Result<Vec<u8>, String> {
    let rgba = render_svg_rgba(svg, canvas_size)?;
    let mut png_bytes = Vec::new();

    {
        let mut encoder = Encoder::new(&mut png_bytes, canvas_size, canvas_size);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);

        let mut writer = encoder
            .write_header()
            .map_err(|error| format!("Failed to create PNG header: {error}"))?;
        writer
            .write_image_data(&rgba)
            .map_err(|error| format!("Failed to encode PNG bytes: {error}"))?;
    }

    Ok(png_bytes)
}

#[cfg(not(target_os = "macos"))]
pub fn render_svg_rgba(_: &str, _: u32) -> Result<Vec<u8>, String> {
    Err("SVG rendering is only supported on macOS in this build".to_owned())
}

#[cfg(not(target_os = "macos"))]
pub fn render_svg_png(_: &str, _: u32) -> Result<Vec<u8>, String> {
    Err("SVG rendering is only supported on macOS in this build".to_owned())
}
