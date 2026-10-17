use std::path::Path;

use crate::{OcrBackend, OcrError, OcrOutput};

#[cfg(windows)]
mod winrt {
    use super::*;
    use futures::executor::block_on;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::BitmapDecoder;
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::{FileAccessMode, StorageFile};
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};
    use windows::core::HSTRING;

    pub struct WindowsMediaOcr {
        language: Option<String>,
    }

    impl WindowsMediaOcr {
        pub fn new(language: Option<String>) -> Self {
            Self { language }
        }
    }

    impl OcrBackend for WindowsMediaOcr {
        fn recognize(&mut self, image_path: &Path) -> Result<OcrOutput, OcrError> {
            unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|error| {
                OcrError::Backend(format!("could not initialize WinRT: {error}"))
            })?;
            let _apartment = Apartment;
            block_on(async {
                let path = HSTRING::from(image_path.to_string_lossy().as_ref());
                let file = StorageFile::GetFileFromPathAsync(&path)
                    .map_err(backend_error)?
                    .await
                    .map_err(backend_error)?;
                let stream = file
                    .OpenAsync(FileAccessMode::Read)
                    .map_err(backend_error)?
                    .await
                    .map_err(backend_error)?;
                let decoder = BitmapDecoder::CreateAsync(&stream)
                    .map_err(backend_error)?
                    .await
                    .map_err(backend_error)?;
                let bitmap = decoder
                    .GetSoftwareBitmapAsync()
                    .map_err(backend_error)?
                    .await
                    .map_err(backend_error)?;

                let engine = match self.language.as_deref() {
                    Some(language_tag) => {
                        let language = Language::CreateLanguage(&HSTRING::from(language_tag))
                            .map_err(backend_error)?;
                        if !OcrEngine::IsLanguageSupported(&language).map_err(backend_error)? {
                            return Err(OcrError::Backend(format!(
                                "Windows OCR language is not installed or supported: {language_tag}"
                            )));
                        }
                        OcrEngine::TryCreateFromLanguage(&language).map_err(backend_error)?
                    }
                    None => {
                        OcrEngine::TryCreateFromUserProfileLanguages().map_err(backend_error)?
                    }
                };
                let result = engine
                    .RecognizeAsync(&bitmap)
                    .map_err(backend_error)?
                    .await
                    .map_err(backend_error)?;
                let lines = result.Lines().map_err(backend_error)?;
                let mut words = Vec::new();
                for line_index in 0..lines.Size().map_err(backend_error)? {
                    let line = lines.GetAt(line_index).map_err(backend_error)?;
                    let line_words = line.Words().map_err(backend_error)?;
                    for word_index in 0..line_words.Size().map_err(backend_error)? {
                        let word = line_words.GetAt(word_index).map_err(backend_error)?;
                        let bounds = word.BoundingRect().map_err(backend_error)?;
                        words.push(crate::WordBox {
                            text: word.Text().map_err(backend_error)?.to_string(),
                            left: bounds.X,
                            top: bounds.Y,
                            width: bounds.Width,
                            height: bounds.Height,
                            confidence_percent: None,
                        });
                    }
                }

                Ok(OcrOutput {
                    text: result.Text().map_err(backend_error)?.to_string(),
                    confidence_percent: None,
                    words,
                })
            })
        }
    }

    struct Apartment;

    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { RoUninitialize() };
        }
    }

    fn backend_error(error: windows::core::Error) -> OcrError {
        OcrError::Backend(format!("Windows.Media.Ocr failed: {error}"))
    }
}

#[cfg(not(windows))]
mod winrt {
    use super::*;

    pub struct WindowsMediaOcr;

    impl WindowsMediaOcr {
        pub fn new(_language: Option<String>) -> Self {
            Self
        }
    }

    impl OcrBackend for WindowsMediaOcr {
        fn recognize(&mut self, _image_path: &Path) -> Result<OcrOutput, OcrError> {
            Err(OcrError::EngineUnavailable("Windows.Media.Ocr"))
        }
    }
}

pub use winrt::WindowsMediaOcr;
