use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

#[cfg(not(feature = "tesseract"))]
use cleanshot_ocr_eval::OcrError;
#[cfg(feature = "tesseract")]
use cleanshot_ocr_eval::TesseractEngine;
use cleanshot_ocr_eval::windows_ocr::WindowsMediaOcr;
use cleanshot_ocr_eval::{OcrBackend, recognize_image};

struct Arguments {
    engine: String,
    language: Option<String>,
    #[cfg_attr(not(feature = "tesseract"), allow(dead_code))]
    tessdata_dir: Option<PathBuf>,
    image_path: PathBuf,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cleanshot-ocr: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = parse_arguments()?;
    let mut backend: Box<dyn OcrBackend> = match arguments.engine.as_str() {
        #[cfg(feature = "tesseract")]
        "tesseract" => Box::new(TesseractEngine::new(
            arguments.language.as_deref(),
            arguments.tessdata_dir,
        )),
        #[cfg(not(feature = "tesseract"))]
        "tesseract" => {
            return Err(OcrError::EngineUnavailable(
                "Tesseract (build with the `tesseract` feature)",
            )
            .into());
        }
        "winrt" => Box::new(WindowsMediaOcr::new(arguments.language)),
        engine => return Err(format!("unknown OCR engine: {engine}\n{}", usage()).into()),
    };

    let started = Instant::now();
    let result = recognize_image(&mut *backend, &arguments.image_path)?;
    let elapsed = started.elapsed();

    println!("{}", result.text.trim_end());
    println!(
        "\nConfidence: {}",
        confidence_label(result.confidence_percent)
    );
    println!("Elapsed: {:.3} ms", elapsed.as_secs_f64() * 1000.0);
    println!("Word boxes: {}", result.words.len());
    for word in result.words {
        println!(
            "  {:?} [{:.0}, {:.0}, {:.0}, {:.0}]{}",
            word.text,
            word.left,
            word.top,
            word.width,
            word.height,
            word.confidence_percent
                .map(|confidence| format!(" ({confidence:.1}%)"))
                .unwrap_or_default()
        );
    }

    Ok(())
}

fn parse_arguments() -> Result<Arguments, Box<dyn Error>> {
    let mut engine = "tesseract".to_owned();
    let mut language = None;
    let mut tessdata_dir = None;
    let mut image_path = None;
    let mut args = std::env::args_os().skip(1);

    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--help" | "-h") => {
                println!("{}", usage());
                std::process::exit(0);
            }
            Some("--engine") => {
                engine = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or("--engine needs `winrt` or `tesseract`")?;
            }
            Some("--language") => {
                language = Some(
                    args.next()
                        .and_then(|value| value.into_string().ok())
                        .ok_or("--language needs a language identifier")?,
                );
            }
            Some("--tessdata-dir") => {
                tessdata_dir = Some(PathBuf::from(
                    args.next().ok_or("--tessdata-dir needs a path")?,
                ));
            }
            Some(option) if option.starts_with('-') => {
                return Err(format!("unknown option: {option}\n{}", usage()).into());
            }
            _ => {
                if image_path.is_some() {
                    return Err(format!("expected one image path\n{}", usage()).into());
                }
                image_path = Some(PathBuf::from(argument));
            }
        }
    }

    Ok(Arguments {
        engine,
        language,
        tessdata_dir,
        image_path: image_path.ok_or_else(|| format!("missing image path\n{}", usage()))?,
    })
}

fn confidence_label(confidence: Option<f32>) -> String {
    confidence
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| {
            "not provided by Windows.Media.Ocr (the API exposes no confidence score)".to_owned()
        })
}

fn usage() -> &'static str {
    "Usage: cleanshot-ocr [--engine winrt|tesseract] [--language CODE] [--tessdata-dir DIR] IMAGE\n\
     Default engine: Tesseract. On Windows, use --engine winrt for comparison.\n\
     Tesseract language uses traineddata codes (for example, eng); WinRT uses Windows language tags (for example, en-US)."
}

#[cfg(test)]
mod tests {
    use super::confidence_label;

    #[test]
    fn explains_when_an_engine_does_not_supply_confidence() {
        assert!(confidence_label(None).contains("API exposes no confidence score"));
        assert_eq!(confidence_label(Some(98.5)), "98.5%");
    }

    #[test]
    fn tesseract_mean_confidence_is_displayed_as_percent() {
        assert_eq!(confidence_label(Some(81.0)), "81.0%");
    }
}
