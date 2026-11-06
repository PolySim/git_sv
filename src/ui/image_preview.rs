//! Décodage et état de rendu des prévisualisations d'images.

#[cfg(feature = "image-preview")]
use image::{DynamicImage, RgbaImage};
use ratatui::{layout::Rect, Frame};
#[cfg(feature = "image-preview")]
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, Resize, StatefulImage};
#[cfg(feature = "image-preview")]
use std::io::Cursor;
#[cfg(feature = "image-preview")]
use std::sync::Arc;

#[cfg(feature = "image-preview")]
use crate::git::diff::ImageFormat;
use crate::git::diff::ImagePreview;

#[cfg(feature = "image-preview")]
const MAX_RASTER_DIMENSION: u32 = 8_192;
#[cfg(feature = "image-preview")]
const MAX_RASTER_ALLOCATION: u64 = 128 * 1_048_576;
#[cfg(feature = "image-preview")]
const MAX_SVG_DIMENSION: f32 = 2_048.0;
#[cfg(feature = "image-preview")]
const MAX_SVG_SOURCE_BYTES: usize = 20 * 1_048_576;

/// Cache le protocole encodé pour éviter de décoder l'image à chaque frame.
#[derive(Default)]
pub struct ImagePreviewState {
    #[cfg(feature = "image-preview")]
    picker: Option<Picker>,
    #[cfg(feature = "image-preview")]
    current_preview: Option<ImagePreview>,
    #[cfg(feature = "image-preview")]
    protocol: Option<StatefulProtocol>,
    #[cfg(feature = "image-preview")]
    error: Option<String>,
}

impl ImagePreviewState {
    /// Détecte le meilleur protocole pris en charge par le terminal.
    pub fn initialize(&mut self) {
        #[cfg(feature = "image-preview")]
        match Picker::from_query_stdio() {
            Ok(picker) => self.picker = Some(picker),
            Err(_) => self.picker = Some(Picker::from_fontsize((10, 20))),
        }
    }

    /// Libère le protocole et l'image décodée lorsqu'aucune image n'est visible.
    pub fn clear(&mut self) {
        #[cfg(feature = "image-preview")]
        {
            self.current_preview = None;
            self.protocol = None;
            self.error = None;
        }
    }

    /// Rend l'image dans la zone fournie ou retourne la raison du repli texte.
    #[cfg(feature = "image-preview")]
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

    /// Indique que le support a été retiré de cette build minimale.
    #[cfg(not(feature = "image-preview"))]
    pub fn render(
        &mut self,
        _frame: &mut Frame,
        _area: Rect,
        _preview: &ImagePreview,
    ) -> Result<(), &str> {
        Err("prévisualisation d'image désactivée à la compilation")
    }

    #[cfg(feature = "image-preview")]
    fn prepare(&mut self, preview: &ImagePreview) {
        if self.current_preview.as_ref().is_some_and(|current| {
            current.format == preview.format && Arc::ptr_eq(&current.bytes, &preview.bytes)
        }) {
            return;
        }

        // Garder l'allocation vivante évite qu'une nouvelle image réutilise la même adresse.
        self.current_preview = Some(preview.clone());
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

#[cfg(feature = "image-preview")]
fn decode(preview: &ImagePreview) -> Result<DynamicImage, String> {
    match preview.format {
        ImageFormat::Raster => decode_raster(&preview.bytes),
        ImageFormat::Svg => decode_svg(&preview.bytes),
    }
}

#[cfg(feature = "image-preview")]
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

#[cfg(feature = "image-preview")]
fn decode_svg(bytes: &[u8]) -> Result<DynamicImage, String> {
    if bytes.len() > MAX_SVG_SOURCE_BYTES {
        return Err("SVG trop volumineux".to_string());
    }
    let source = std::str::from_utf8(bytes).map_err(|_| "SVG non UTF-8".to_string())?;
    let data_resolver = resvg::usvg::ImageHrefResolver::default_data_resolver();
    let options = resvg::usvg::Options {
        image_href_resolver: resvg::usvg::ImageHrefResolver {
            // Un aperçu de commit ne doit pas charger des fichiers du système courant.
            resolve_string: Box::new(|_, _| None),
            resolve_data: Box::new(move |mime, data, options| {
                // Éviter la décompression non bornée des SVGZ embarqués.
                if data.len() > MAX_SVG_SOURCE_BYTES || data.starts_with(&[0x1f, 0x8b]) {
                    return None;
                }
                data_resolver(mime, data, options)
            }),
        },
        ..Default::default()
    };
    // from_str évite la décompression SVGZ implicite de from_data.
    let tree = resvg::usvg::Tree::from_str(source, &options)
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

#[cfg(all(test, feature = "image-preview"))]
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

    #[test]
    fn audit_preview_cache_distinguishes_format() {
        let bytes: Arc<[u8]> = Arc::from(
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"/>"#.as_slice(),
        );
        let mut state = ImagePreviewState {
            picker: Some(Picker::from_fontsize((10, 20))),
            ..Default::default()
        };
        state.prepare(&ImagePreview {
            bytes: bytes.clone(),
            format: ImageFormat::Raster,
        });
        assert!(state.error.is_some());

        state.prepare(&ImagePreview {
            bytes,
            format: ImageFormat::Svg,
        });

        assert!(state.error.is_none());
        assert!(state.protocol.is_some());
    }

    #[test]
    fn audit_preview_cache_releases_source_when_cleared() {
        let preview = ImagePreview {
            bytes: Arc::from(
                br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>"#.as_slice(),
            ),
            format: ImageFormat::Svg,
        };
        let source = Arc::downgrade(&preview.bytes);
        let mut state = ImagePreviewState {
            picker: Some(Picker::from_fontsize((10, 20))),
            ..Default::default()
        };
        state.prepare(&preview);
        drop(preview);
        assert!(source.upgrade().is_some());

        state.clear();

        assert!(source.upgrade().is_none());
        assert!(state.protocol.is_none());
    }

    #[test]
    fn audit_svg_preview_does_not_read_external_images() {
        let directory = tempfile::TempDir::new().unwrap();
        let external = directory.path().join("external.svg");
        std::fs::write(&external, r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="red"/></svg>"#).unwrap();
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><image href="{}" width="1" height="1"/></svg>"#,
            external.display()
        );

        let image = decode_svg(source.as_bytes()).unwrap().to_rgba8();

        assert_eq!(image.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn audit_svg_preview_does_not_decompress_embedded_svgz() {
        let source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><image href="data:image/svg+xml;base64,H4sIAAAAAAAC/22MSQ6AIAwAv9L0ARb1ZoDPCFISXAKN9fnK3dskMxnb7gTPXo7mkEWuhUhVB52HsyaajDH0FQiag7DDEYFjTiwdva1xlV8FWy7FYY0Bydu+8C9JJfVYagAAAA==" width="1" height="1"/></svg>"#;

        let image = decode_svg(source.as_bytes()).unwrap().to_rgba8();

        assert_eq!(image.get_pixel(0, 0)[3], 0);
    }
}
