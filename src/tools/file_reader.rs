use lopdf::Document;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum FileReaderError {
    Io(std::io::Error),
    UnsupportedExtension(String),
    PdfExtraction(lopdf::Error),
}

impl fmt::Display for FileReaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileReaderError::Io(err) => write!(f, "I/O error: {err}"),
            FileReaderError::UnsupportedExtension(ext) => {
                write!(f, "unsupported file extension: {ext}")
            }
            FileReaderError::PdfExtraction(err) => write!(f, "PDF extraction error: {err}"),
        }
    }
}

impl std::error::Error for FileReaderError {}

impl From<std::io::Error> for FileReaderError {
    fn from(error: std::io::Error) -> Self {
        FileReaderError::Io(error)
    }
}

impl From<lopdf::Error> for FileReaderError {
    fn from(error: lopdf::Error) -> Self {
        FileReaderError::PdfExtraction(error)
    }
}

pub struct FileReaderTool;

impl FileReaderTool {
    fn extract_pdf_text(path: &Path) -> Result<String, FileReaderError> {
        let document = Document::load(path)?;
        let mut text = String::new();

        let pages = document.get_pages();
        for page_number in 1..=pages.len() as u32 {
            if let Ok(page_text) = document.extract_text(&[page_number]) {
                if !page_text.is_empty() {
                    text.push_str(&page_text);
                    text.push('\n');
                }
            }
        }

        Ok(text)
    }

    fn read_text_file(path: &Path) -> Result<String, FileReaderError> {
        let content = fs::read_to_string(path)?;
        Ok(content)
    }
}

impl crate::tools::Tool for FileReaderTool {
    fn name(&self) -> &'static str {
        "file_reader"
    }

    fn execute(&self, params: &str) -> String {
        let file_path = PathBuf::from(params);

        let result = match file_path.extension().and_then(|ext| ext.to_str()) {
            Some("md") | Some("txt") => Self::read_text_file(&file_path),
            Some("pdf") => Self::extract_pdf_text(&file_path),
            Some(ext) => Err(FileReaderError::UnsupportedExtension(ext.to_string())),
            None => Err(FileReaderError::UnsupportedExtension("none".to_string())),
        };

        match result {
            Ok(text) => text,
            Err(err) => format!("Error reading file: {err}"),
        }
    }
}
