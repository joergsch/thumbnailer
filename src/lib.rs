//! # Thumbnailer
//!
//! This crate can be used to generate thumbnails for all kinds of files.
//!
//! Example:
//! ```
//! use thumbnailer::{create_thumbnails, Thumbnail, ThumbnailSize};
//! use std::fs::File;
//! use std::io::BufReader;
//! use std::io::Cursor;
//!
//! let file = File::open("tests/assets/test.png").unwrap();
//! let reader = BufReader::new(file);
//! let mut  thumbnails = create_thumbnails(reader, mime::IMAGE_PNG, [ThumbnailSize::Small, ThumbnailSize::Medium]).unwrap();
//!
//! let thumbnail = thumbnails.pop().unwrap();
//! let mut buf = Cursor::new(Vec::new());
//! thumbnail.write_png(&mut buf).unwrap();
//! ```

use crate::error::ThumbResult;
use image::{DynamicImage, GenericImageView, ImageFormat};
use mime::Mime;
use rayon::prelude::*;
use std::io::{BufRead, Seek, Write};

use crate::formats::get_base_image;
pub use size::ThumbnailSize;
use std::convert::From;

pub mod error;
mod formats;
mod size;
pub(crate) mod utils;

#[derive(Clone, Debug)]
pub struct Thumbnail {
    inner: DynamicImage,
}

#[derive(Clone, Debug)]
pub enum FilterType {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl FilterType {
    const fn translate_filter(&self) -> image::imageops::FilterType {
        match self {
            Self::Nearest => image::imageops::FilterType::Nearest,
            Self::Triangle => image::imageops::FilterType::Triangle,
            Self::CatmullRom => image::imageops::FilterType::CatmullRom,
            Self::Gaussian => image::imageops::FilterType::Gaussian,
            Self::Lanczos3 => image::imageops::FilterType::Lanczos3,
        }
    }
}

impl From<FilterType> for image::imageops::FilterType {
    fn from(filter_type: FilterType) -> Self {
        filter_type.translate_filter()
    }
}

impl Thumbnail {
    /// Writes the bytes of the image in a png format
    pub fn write_png<W: Write + Seek>(self, writer: &mut W) -> ThumbResult<()> {
        let image = DynamicImage::ImageRgba8(self.inner.into_rgba8());
        image.write_to(writer, ImageFormat::Png)?;

        Ok(())
    }

    /// Writes the bytes of the image in a jpeg format
    pub fn write_jpeg<W: Write + Seek>(self, writer: &mut W, quality: u8) -> ThumbResult<()> {
        let image = DynamicImage::ImageRgb8(self.inner.into_rgb8());
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, quality);
        encoder.encode_image(&image)?;

        Ok(())
    }

    /// Returns the size of the thumbnail as width,  height
    pub fn size(&self) -> (u32, u32) {
        self.inner.dimensions()
    }
}

/// Creates thumbnails of the requested sizes for the given reader providing the content as bytes and
/// the mime describing the contents type
pub fn create_thumbnails_samplefilter<R: BufRead + Seek, I: IntoIterator<Item = ThumbnailSize>>(
    reader: R,
    mime: Mime,
    sizes: I,
    filter: FilterType,
) -> ThumbResult<Vec<Thumbnail>> {
    let image = get_base_image(reader, mime)?;
    let sizes: Vec<ThumbnailSize> = sizes.into_iter().collect();
    let thumbnails = resize_images(image, &sizes, filter)
        .into_iter()
        .map(|image| Thumbnail { inner: image })
        .collect();

    Ok(thumbnails)
}

/// Creates thumbnails of the requested sizes for the given reader providing the content as bytes and
/// the mime describing the contents type
pub fn create_thumbnails<R: BufRead + Seek, I: IntoIterator<Item = ThumbnailSize>>(
    reader: R,
    mime: Mime,
    sizes: I,
) -> ThumbResult<Vec<Thumbnail>> {
    let image = get_base_image(reader, mime)?;
    let sizes: Vec<ThumbnailSize> = sizes.into_iter().collect();
    let thumbnails = resize_images(image, &sizes, FilterType::Lanczos3)
        .into_iter()
        .map(|image| Thumbnail { inner: image })
        .collect();

    Ok(thumbnails)
}

fn resize_images(
    image: DynamicImage,
    sizes: &[ThumbnailSize],
    filter_type: crate::FilterType,
) -> Vec<DynamicImage> {
    sizes
        .into_par_iter()
        .map(|size| {
            let (width, height) = size.dimensions();
            image.resize(
                width,
                height,
                image::imageops::FilterType::from(filter_type.clone()),
            )
        })
        .collect()
}
