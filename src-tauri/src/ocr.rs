use std::fmt;

pub trait OcrBackend {
    fn name(&self) -> &str;
    fn extract_text(&self, image_bytes: &[u8]) -> Result<Option<String>, OcrError>;
}

#[derive(Debug)]
pub struct OcrError {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for OcrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for OcrError {}

#[cfg(windows)]
mod windows_impl {
    use super::{OcrBackend, OcrError};
    use windows::{
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        Graphics::Imaging::{
            BitmapDecoder, BitmapPixelFormat, SoftwareBitmap,
        },
        Media::Ocr::OcrEngine,
        Storage::Streams::{DataWriter, InMemoryRandomAccessStream},
    };

    pub struct WindowsMediaOcr {
        runtime: tokio::runtime::Runtime,
    }

    impl WindowsMediaOcr {
        pub fn new() -> Self {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create tokio runtime for OCR");
            Self { runtime }
        }

        pub fn name(&self) -> &'static str {
            "windows-media-ocr"
        }

        fn extract_text_inner(&self, image_bytes: &[u8]) -> std::result::Result<Option<String>, OcrError> {
            unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
                .map_err(|e| OcrError { code: "runtime-init", message: e.to_string() })?;

            let result = self.runtime.block_on(async {
                extract_ocr_text(image_bytes).await
            });

            unsafe { RoUninitialize() };
            result
        }
    }

    impl Default for WindowsMediaOcr {
        fn default() -> Self {
            Self::new()
        }
    }

    impl OcrBackend for WindowsMediaOcr {
        fn name(&self) -> &str {
            Self::name(self)
        }

        fn extract_text(&self, image_bytes: &[u8]) -> std::result::Result<Option<String>, OcrError> {
            self.extract_text_inner(image_bytes)
        }
    }

    async fn extract_ocr_text(image_bytes: &[u8]) -> std::result::Result<Option<String>, OcrError> {
        let stream = InMemoryRandomAccessStream::new()
            .map_err(|e| OcrError { code: "stream-create", message: e.to_string() })?;

        {
            let writer = DataWriter::CreateDataWriter(&stream)
                .map_err(|e| OcrError { code: "writer-create", message: e.to_string() })?;
            writer
                .WriteBytes(image_bytes)
                .map_err(|e| OcrError { code: "stream-write", message: e.to_string() })?;
            let _ = writer
                .StoreAsync()
                .map_err(|e| OcrError { code: "store-create", message: e.to_string() })?
                .await
                .map_err(|e| OcrError { code: "store-async", message: e.to_string() })?;
            let _ = writer
                .FlushAsync()
                .map_err(|e| OcrError { code: "flush-create", message: e.to_string() })?
                .await
                .map_err(|e| OcrError { code: "flush-async", message: e.to_string() })?;
        }

        stream
            .Seek(0u64)
            .map_err(|e| OcrError { code: "stream-seek", message: e.to_string() })?;

        let size = stream
            .Size()
            .map_err(|e| OcrError { code: "stream-size", message: e.to_string() })?;
        if size == 0 {
            return Ok(None);
        }

        let decoder = BitmapDecoder::CreateAsync(&stream)
            .map_err(|e| OcrError { code: "bitmap-create", message: e.to_string() })?
            .await
            .map_err(|e| OcrError { code: "bitmap-await", message: e.to_string() })?;

        let bitmap = decoder
            .GetSoftwareBitmapAsync()
            .map_err(|e| OcrError { code: "software-bitmap-create", message: e.to_string() })?
            .await
            .map_err(|e| OcrError { code: "software-bitmap", message: e.to_string() })?;

        let bitmap = ensure_bitmap_format(bitmap)?;

        let engine = OcrEngine::TryCreateFromUserProfileLanguages()
            .map_err(|e| OcrError { code: "engine-create", message: e.to_string() })?;

        let ocr_result = engine
            .RecognizeAsync(&bitmap)
            .map_err(|e| OcrError { code: "recognize-failed", message: e.to_string() })?
            .await
            .map_err(|e| OcrError { code: "recognize-error", message: e.to_string() })?;

        let text = ocr_result
            .Text()
            .map_err(|e| OcrError { code: "result-text", message: e.to_string() })?
            .to_string_lossy();

        let trimmed = text.trim();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed.to_string()))
        }
    }

    fn ensure_bitmap_format(bitmap: SoftwareBitmap) -> std::result::Result<SoftwareBitmap, OcrError> {
        let current_format = bitmap
            .BitmapPixelFormat()
            .map_err(|e| OcrError { code: "pixel-format", message: e.to_string() })?;

        if current_format == BitmapPixelFormat::Bgra8 {
            return Ok(bitmap);
        }

        let converted = SoftwareBitmap::Convert(
            &bitmap,
            BitmapPixelFormat::Bgra8,
        )
        .map_err(|e| OcrError { code: "convert-format", message: e.to_string() })?;

        Ok(converted)
    }
}

#[cfg(not(windows))]
mod fallback {
    use super::{OcrBackend, OcrError};

    pub struct NoOpOcr;

    impl OcrBackend for NoOpOcr {
        fn name(&self) -> &str {
            "noop"
        }

        fn extract_text(&self, _image_bytes: &[u8]) -> Result<Option<String>, OcrError> {
            Ok(None)
        }
    }
}

#[cfg(windows)]
pub use windows_impl::WindowsMediaOcr;

#[cfg(not(windows))]
pub use fallback::NoOpOcr;

pub fn create_ocr_backend() -> Box<dyn OcrBackend + Send> {
    #[cfg(windows)]
    {
        Box::new(WindowsMediaOcr::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(NoOpOcr)
    }
}
