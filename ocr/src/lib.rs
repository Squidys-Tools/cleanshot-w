use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

#[cfg(feature = "tesseract")]
mod tesseract;
pub mod windows_ocr;

#[cfg(feature = "tesseract")]
pub use tesseract::TesseractEngine;

#[derive(Debug, Clone, PartialEq)]
pub struct WordBox {
    pub text: String,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub confidence_percent: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OcrOutput {
    pub text: String,
    pub confidence_percent: Option<f32>,
    pub words: Vec<WordBox>,
}

#[derive(Debug)]
pub enum OcrError {
    ImagePathNotFile(PathBuf),
    EngineUnavailable(&'static str),
    Backend(String),
}

impl Display for OcrError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ImagePathNotFile(path) => {
                write!(f, "image path is not a file: {}", path.display())
            }
            Self::EngineUnavailable(engine) => {
                write!(f, "the {engine} OCR engine is unavailable on this platform")
            }
            Self::Backend(message) => f.write_str(message),
        }
    }
}

impl Error for OcrError {}

pub trait OcrBackend {
    fn recognize(&mut self, image_path: &Path) -> Result<OcrOutput, OcrError>;
}

pub fn recognize_image(
    backend: &mut dyn OcrBackend,
    image_path: &Path,
) -> Result<OcrOutput, OcrError> {
    if !image_path.is_file() {
        return Err(OcrError::ImagePathNotFile(image_path.to_owned()));
    }

    backend.recognize(image_path)
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::{OcrBackend, OcrError, OcrOutput, WordBox, recognize_image};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

    struct FakeBackend {
        received_path: Option<PathBuf>,
        output: OcrOutput,
    }

    impl OcrBackend for FakeBackend {
        fn recognize(&mut self, image_path: &Path) -> Result<OcrOutput, OcrError> {
            self.received_path = Some(image_path.to_owned());
            Ok(self.output.clone())
        }
    }

    #[test]
    fn passes_the_image_path_to_the_backend_and_preserves_optional_confidence() {
        let path = std::env::temp_dir().join(format!(
            "snipsy-ocr-test-{}-{}.png",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, b"fake image bytes").unwrap();
        let expected = OcrOutput {
            text: "Read this".to_owned(),
            confidence_percent: None,
            words: vec![WordBox {
                text: "Read".to_owned(),
                left: 10.0,
                top: 20.0,
                width: 30.0,
                height: 12.0,
                confidence_percent: None,
            }],
        };
        let mut backend = FakeBackend {
            received_path: None,
            output: expected.clone(),
        };

        let output = recognize_image(&mut backend, &path).unwrap();

        assert_eq!(backend.received_path.as_deref(), Some(path.as_path()));
        assert_eq!(output, expected);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_missing_files_without_calling_the_backend() {
        let path = std::env::temp_dir().join(format!(
            "snipsy-ocr-missing-{}-{}.png",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut backend = FakeBackend {
            received_path: None,
            output: OcrOutput {
                text: String::new(),
                confidence_percent: None,
                words: Vec::new(),
            },
        };

        assert!(matches!(
            recognize_image(&mut backend, &path),
            Err(OcrError::ImagePathNotFile(_))
        ));
        assert_eq!(backend.received_path, None);
    }
}
