use lopdf::Document;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum PdfBookLoaderError {
    Io(std::io::Error),
    PdfExtraction(lopdf::Error),
    UnsupportedExtension(String),
}

impl fmt::Display for PdfBookLoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfBookLoaderError::Io(err) => write!(f, "I/O error: {err}"),
            PdfBookLoaderError::UnsupportedExtension(ext) => {
                write!(f, "unsupported file extension: {ext}")
            }
            PdfBookLoaderError::PdfExtraction(err) => write!(f, "PDF extraction error: {err}"),
        }
    }
}

impl std::error::Error for PdfBookLoaderError {}

impl From<std::io::Error> for PdfBookLoaderError {
    fn from(error: std::io::Error) -> Self {
        PdfBookLoaderError::Io(error)
    }
}

impl From<lopdf::Error> for PdfBookLoaderError {
    fn from(error: lopdf::Error) -> Self {
        PdfBookLoaderError::PdfExtraction(error)
    }
}

pub struct PdfBookLoaderTool;

impl PdfBookLoaderTool {
    fn extract_pdf_text(path: &Path) -> Result<String, PdfBookLoaderError> {
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

    fn read_text_file(path: &Path) -> Result<String, PdfBookLoaderError> {
        let content = fs::read_to_string(path)?;
        Ok(content)
    }

    fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), PdfBookLoaderError> {
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    Self::collect_files(&path, files)?;
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                        let ext = ext.to_lowercase();
                        if ext == "md" || ext == "txt" || ext == "pdf" {
                            files.push(path);
                        }
                    }
                }
            }
        } else if path.is_file() {
            files.push(path.to_path_buf());
        }
        Ok(())
    }

    fn read_path(&self, params: &str) -> Result<String, PdfBookLoaderError> {
        let path = PathBuf::from(params);
        if !path.exists() {
            return Err(PdfBookLoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("path not found: {params}"),
            )));
        }

        let mut files = Vec::new();
        Self::collect_files(&path, &mut files)?;
        files.sort();

        if files.is_empty() {
            return Ok("No supported files found in path.".to_string());
        }

        let mut output = String::new();
        for file in files {
            let file_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            let ext = file
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase());

            let text = match ext.as_deref() {
                Some("md") | Some("txt") => Self::read_text_file(&file),
                Some("pdf") => Self::extract_pdf_text(&file),
                Some(ext) => Err(PdfBookLoaderError::UnsupportedExtension(ext.to_string())),
                None => Err(PdfBookLoaderError::UnsupportedExtension("none".to_string())),
            };

            match text {
                Ok(content) => {
                    output.push_str(&format!("=== {file_name} ===\n{content}\n\n"));
                }
                Err(err) => {
                    output.push_str(&format!("=== {file_name} ===\nError: {err}\n\n"));
                }
            }
        }

        Ok(output)
    }
}

impl crate::tools::Tool for PdfBookLoaderTool {
    fn name(&self) -> &'static str {
        "pdf_book_loader"
    }

    fn execute(&self, params: &str) -> String {
        match self.read_path(params) {
            Ok(text) => text,
            Err(err) => format!("Error loading books: {err}"),
        }
    }
}
