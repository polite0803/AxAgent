use crate::error::{AxAgentError, Result};
use std::path::Path;

/// Extract plain text from a document file based on its MIME type.
pub fn extract_text(file_path: &Path, mime_type: &str) -> Result<String> {
    match mime_type {
        // Plain text files
        "text/plain" | "text/markdown" | "text/csv" | "text/html" | "text/xml"
        | "application/json" | "application/xml" => std::fs::read_to_string(file_path)
            .map_err(|e| AxAgentError::Provider(format!("Failed to read file: {e}"))),

        // PDF
        "application/pdf" => extract_pdf(file_path),

        // DOCX — basic XML extraction without external crate
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            extract_docx(file_path)
        }

        // XLSX — extract cell values from shared strings and sheet XML
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            extract_xlsx(file_path)
        }

        _ => {
            // Try reading as plain text as fallback
            std::fs::read_to_string(file_path).map_err(|e| {
                AxAgentError::Provider(format!(
                    "Unsupported MIME type '{}', fallback read failed: {e}",
                    mime_type
                ))
            })
        }
    }
}

/// Extract text from PDF using pdf-extract crate.
fn extract_pdf(file_path: &Path) -> Result<String> {
    let bytes = std::fs::read(file_path)
        .map_err(|e| AxAgentError::Provider(format!("Failed to read PDF file: {e}")))?;

    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| AxAgentError::Provider(format!("Failed to extract PDF text: {e}")))
}

/// Extract text from DOCX by reading the internal XML.
/// DOCX files are ZIP archives containing word/document.xml.
fn extract_docx(file_path: &Path) -> Result<String> {
    let file = std::fs::File::open(file_path)
        .map_err(|e| AxAgentError::Provider(format!("Failed to open DOCX file: {e}")))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AxAgentError::Provider(format!("Failed to read DOCX as ZIP: {e}")))?;

    let mut xml_content = String::new();
    if let Ok(mut entry) = archive.by_name("word/document.xml") {
        use std::io::Read;
        entry
            .read_to_string(&mut xml_content)
            .map_err(|e| AxAgentError::Provider(format!("Failed to read document.xml: {e}")))?;
    } else {
        return Err(AxAgentError::Provider(
            "DOCX: word/document.xml not found".into(),
        ));
    }

    // Simple XML text extraction: find all <w:t> tag contents
    Ok(extract_text_from_xml(&xml_content))
}

/// Simple XML text extraction — pulls text from <w:t> and <w:t xml:space="preserve"> tags.
fn extract_text_from_xml(xml: &str) -> String {
    let mut result = String::new();
    let mut in_paragraph = false;

    // Track <w:p> boundaries for paragraph breaks
    for part in xml.split("<w:p") {
        if in_paragraph && !result.is_empty() {
            result.push('\n');
        }
        in_paragraph = true;

        // Extract text from <w:t> tags within this paragraph
        for segment in part.split("<w:t") {
            if let Some(text_start) = segment.find('>') {
                let after_tag = &segment[text_start + 1..];
                if let Some(end) = after_tag.find("</w:t>") {
                    result.push_str(&after_tag[..end]);
                }
            }
        }
    }

    result
}

/// Determine the MIME type from a file extension.
pub fn mime_from_extension(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "txt" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "xml" => "text/xml",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "text/plain",
    }
}

/// Extract text from XLSX by reading shared strings and sheet XML.
/// XLSX files are ZIP archives containing xl/sharedStrings.xml and xl/worksheets/sheetN.xml.
fn extract_xlsx(file_path: &Path) -> Result<String> {
    let file = std::fs::File::open(file_path)
        .map_err(|e| AxAgentError::Provider(format!("Failed to open XLSX file: {e}")))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AxAgentError::Provider(format!("Failed to read XLSX as ZIP: {e}")))?;

    use std::io::Read;

    // Read shared strings table (xl/sharedStrings.xml)
    let mut shared_strings: Vec<String> = Vec::new();
    if let Ok(mut entry) = archive.by_name("xl/sharedStrings.xml") {
        let mut xml = String::new();
        entry
            .read_to_string(&mut xml)
            .map_err(|e| AxAgentError::Provider(format!("Failed to read sharedStrings.xml: {e}")))?;
        // Extract each <t> tag content as a shared string entry
        for segment in xml.split("<t") {
            if let Some(text_start) = segment.find('>') {
                let after_tag = &segment[text_start + 1..];
                if let Some(end) = after_tag.find("</t>") {
                    shared_strings.push(after_tag[..end].to_string());
                }
            }
        }
    }

    // Read all sheets and extract cell values
    let mut result = String::new();
    let mut sheet_index = 1;
    loop {
        let sheet_path = format!("xl/worksheets/sheet{}.xml", sheet_index);
        let mut xml = String::new();
        match archive.by_name(&sheet_path) {
            Ok(mut entry) => {
                entry
                    .read_to_string(&mut xml)
                    .map_err(|e| AxAgentError::Provider(format!("Failed to read {}: {e}", sheet_path)))?;
            }
            Err(_) => break, // No more sheets
        }

        if sheet_index > 1 {
            result.push_str("\n\n");
        }
        result.push_str(&format!("--- Sheet {} ---\n", sheet_index));

        // Parse rows: split by <row>, then extract <c> cells
        for row_part in xml.split("<row") {
            let mut row_values: Vec<String> = Vec::new();

            for cell_part in row_part.split("<c") {
                // Check if cell has a type attribute t="s" (shared string reference)
                let is_shared_string = cell_part.contains("t=\"s\"");

                // Extract the <v> value
                let value = if let Some(v_start) = cell_part.find("<v>") {
                    let after_v = &cell_part[v_start + 3..];
                    if let Some(v_end) = after_v.find("</v>") {
                        let v_content = &after_v[..v_end];
                        if is_shared_string {
                            // Value is an index into shared strings
                            if let Ok(idx) = v_content.parse::<usize>() {
                                if idx < shared_strings.len() {
                                    shared_strings[idx].clone()
                                } else {
                                    v_content.to_string()
                                }
                            } else {
                                v_content.to_string()
                            }
                        } else {
                            v_content.to_string()
                        }
                    } else {
                        continue;
                    }
                } else {
                    // Check for inline string <is><t>...</t></is>
                    let mut inline_str = String::new();
                    if let Some(is_start) = cell_part.find("<is>") {
                        let after_is = &cell_part[is_start..];
                        for seg in after_is.split("<t") {
                            if let Some(t_start) = seg.find('>') {
                                let after_t = &seg[t_start + 1..];
                                if let Some(t_end) = after_t.find("</t>") {
                                    inline_str = after_t[..t_end].to_string();
                                    break;
                                }
                            }
                        }
                    }
                    if inline_str.is_empty() {
                        continue;
                    }
                    inline_str
                };

                if !value.is_empty() {
                    row_values.push(value);
                }
            }

            if !row_values.is_empty() {
                result.push_str(&row_values.join("\t"));
                result.push('\n');
            }
        }

        sheet_index += 1;
    }

    Ok(result)
}
