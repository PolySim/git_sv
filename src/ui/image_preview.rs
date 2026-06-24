//! Décodage et état de rendu des prévisualisations d'images.

use image::{DynamicImage, RgbaImage};
use ratatui::{layout::Rect, Frame};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, Resize, StatefulImage};
use std::io::Cursor;

use crate::git::diff::{ImageFormat, ImagePreview};

const MAX_RASTER_DIMENSION: u32 = 8_192;
const MAX_RASTER_ALLOCATION: u64 = 128 * 1_048_576;
const MAX_SVG_DIMENSION: f32 = 2_048.0;

/// Cache le protocole encodé pour éviter de décoder l'image à chaque frame.
#[derive(Default)]
pub struct ImagePreviewState {
    picker: Option<Picker>,
    current_key: Option<(usize, usize)>,
    protocol: Option<StatefulProtocol>,
    error: Option<String>,
}

impl ImagePreviewState {
    /// Détecte le meilleur protocole pris en charge par le terminal.
    pub fn initialize(&mut self) {
        match Picker::from_query_stdio() {
            Ok(picker) => self.picker = Some(picker),
            Err(_) => self.picker = Some(Picker::from_fontsize((10, 20))),
        }
    }

    /// Rend l'image dans la zone fournie ou retourne la raison du repli texte.
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        preview: &ImagePreview,
    ) -> Result<(), &str> {
        self.prepare(preview);

        if let Some(protocol) = self.protocol.as_mut() {
            frame.render_stateful_widget(
                StatefulImage::new().resize(Resize::Fit(None)),
                area,
                protocol,
            );
            return Ok(());
        }

        Err(self
            .error
            .as_deref()
            .unwrap_or("protocole d'image indisponible"))
    }

    fn prepare(&mut self, preview: &ImagePreview) {
        let key = (preview.bytes.as_ptr() as usize, preview.bytes.len());
        if self.current_key == Some(key) {
            return;
        }

        self.current_key = Some(key);
        self.protocol = None;
        self.error = None;

        let Some(picker) = self.picker.as_ref() else {
            self.error = Some("protocole d'image indisponible".to_string());
            return;
        };

        match decode(preview) {
            Ok(image) => self.protocol = Some(picker.new_resize_protocol(image)),
            Err(error) => self.error = Some(error),
        }
    }
}

fn decode(preview: &ImagePreview) -> Result<DynamicImage, String> {
    match preview.format {
        ImageFormat::Raster => decode_raster(&preview.bytes),
        ImageFormat::Svg => decode_svg(&preview.bytes),
    }
}

fn decode_raster(bytes: &[u8]) -> Result<DynamicImage, String> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_RASTER_DIMENSION);
    limits.max_image_height = Some(MAX_RASTER_DIMENSION);
    limits.max_alloc = Some(MAX_RASTER_ALLOCATION);

    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| format!("format d'image inconnu: {error}"))?;
    reader.limits(limits);
    reader
        .decode()
        .map_err(|error| format!("image invalide: {error}"))
}

fn decode_svg(bytes: &[u8]) -> Result<DynamicImage, String> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(bytes, &options)
        .map_err(|error| format!("SVG invalide: {error}"))?;
    let source_size = tree.size();
    let scale = (MAX_SVG_DIMENSION / source_size.width())
        .min(MAX_SVG_DIMENSION / source_size.height())
        .min(1.0);
    let width = (source_size.width() * scale).ceil().max(1.0) as u32;
    let height = (source_size.height() * scale).ceil().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "dimensions SVG invalides".to_string())?;

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        rgba.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    let image = RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| "rasterisation SVG impossible".to_string())?;
    Ok(DynamicImage::ImageRgba8(image))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageFormat as RasterFormat;
    use std::sync::Arc;

    #[test]
    fn test_decode_png() {
        let source = DynamicImage::new_rgba8(3, 2);
        let mut bytes = Cursor::new(Vec::new());
        source.write_to(&mut bytes, RasterFormat::Png).unwrap();
        let preview = ImagePreview {
            bytes: Arc::from(bytes.into_inner()),
            format: ImageFormat::Raster,
        };

        let image = decode(&preview).unwrap();

        assert_eq!(image.width(), 3);
        assert_eq!(image.height(), 2);
    }

    #[test]
    fn test_decode_svg() {
        let preview = ImagePreview {
            bytes: Arc::from(
                br#"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"><rect width="12" height="8" fill="red"/></svg>"#
                    .as_slice(),
            ),
            format: ImageFormat::Svg,
        };

        let image = decode(&preview).unwrap();

        assert_eq!(image.width(), 12);
        assert_eq!(image.height(), 8);
    }
}
