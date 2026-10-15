#[cfg(windows)]
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::{OcrBackend, OcrError, OcrOutput, WordBox};
use leptess::LepTess;

pub struct TesseractEngine {
    language: String,
    tessdata_dir: Option<PathBuf>,
}

impl TesseractEngine {
    pub fn new(language: Option<&str>, tessdata_dir: Option<PathBuf>) -> Self {
        Self {
            language: language.unwrap_or("eng").to_owned(),
            tessdata_dir,
        }
    }
}

impl OcrBackend for TesseractEngine {
    fn recognize(&mut self, image_path: &Path) -> Result<OcrOutput, OcrError> {
        let tessdata_dir = self
            .tessdata_dir
            .as_deref()
            .map(|path| path.to_string_lossy().into_owned());
        let mut tesseract = LepTess::new(tessdata_dir.as_deref(), &self.language)
            .map_err(|error| OcrError::Backend(format!("cannot initialize Tesseract: {error}")))?;
        #[cfg(not(windows))]
        tesseract.set_image(image_path).map_err(|error| {
            OcrError::Backend(format!(
                "Tesseract could not decode {}: {error}",
                image_path.display()
            ))
        })?;
        #[cfg(windows)]
        {
            let image = image::open(image_path).map_err(|error| {
                OcrError::Backend(format!(
                    "could not decode {}: {error}",
                    image_path.display()
                ))
            })?;
            let mut tiff = Cursor::new(Vec::new());
            image
                .write_to(&mut tiff, image::ImageFormat::Tiff)
                .map_err(|error| {
                    OcrError::Backend(format!("could not prepare image for Tesseract: {error}"))
                })?;
            tesseract
                .set_image_from_mem(tiff.get_ref())
                .map_err(|error| {
                    OcrError::Backend(format!(
                        "Tesseract could not load the decoded image: {error}"
                    ))
                })?;
        }

        let text = tesseract.get_utf8_text().map_err(|error| {
            OcrError::Backend(format!("Tesseract returned invalid UTF-8: {error}"))
        })?;
        let confidence = tesseract.mean_text_conf();
        let tsv = tesseract.get_tsv_text(0).map_err(|error| {
            OcrError::Backend(format!("Tesseract returned invalid TSV: {error}"))
        })?;

        Ok(OcrOutput {
            text,
            confidence_percent: (0..=100).contains(&confidence).then_some(confidence as f32),
            words: parse_word_boxes(&tsv),
        })
    }
}

fn parse_word_boxes(tsv: &str) -> Vec<WordBox> {
    tsv.lines()
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.splitn(12, '\t');
            if fields.next()? != "5" {
                return None;
            }
            fields.next()?;
            fields.next()?;
            fields.next()?;
            fields.next()?;
            fields.next()?;
            let left = fields.next()?.parse::<f32>().ok()?;
            let top = fields.next()?.parse::<f32>().ok()?;
            let width = fields.next()?.parse::<f32>().ok()?;
            let height = fields.next()?.parse::<f32>().ok()?;
            let confidence = fields.next()?.parse::<f32>().ok()?;
            let text = fields.next()?.to_owned();
            if text.is_empty() {
                return None;
            }

            Some(WordBox {
                text,
                left,
                top,
                width,
                height,
                confidence_percent: (0.0..=100.0).contains(&confidence).then_some(confidence),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_word_boxes;

    #[test]
    fn parses_word_rows_and_omits_non_word_rows() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n5\t1\t1\t1\t1\t1\t10\t20\t30\t12\t91.5\tRead\n4\t1\t1\t1\t1\t0\t10\t20\t30\t12\t-1\tRead this";

        let words = parse_word_boxes(tsv);

        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "Read");
        assert_eq!(words[0].left, 10.0);
        assert_eq!(words[0].confidence_percent, Some(91.5));
    }
}
