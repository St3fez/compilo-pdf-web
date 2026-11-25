//! Word files (`.docx`, `.doc`) to PDF, so a Word file can be opened, filled in and signed like any
//! other document. WordCraft reads the file and lays it out, and writes the PDF, all in Rust; PdfCraft
//! does not parse Word itself.
//!
//! Detection ([`is_word`]) is pure and works on every target. Conversion ([`word_to_pdf`]) runs only
//! in the browser build (`wasm32`), and in this crate's tests. Off the web, WordCraft's font database
//! also reads the machine's own fonts (Calibri on Windows, for one) and writes them into the PDF, which
//! AGENTS.md §1.4 forbids; in the browser it has only its bundled OFL fonts. So a desktop build says
//! plainly that it does not convert Word files, instead of quietly breaking that rule.

use crate::CreateError;

/// File extensions of the Word files [`word_to_pdf`] converts. The file's first bytes decide which
/// reader it gets, so a `.doc` that is really a `.docx` (it happens) still opens.
pub const WORD_EXTENSIONS: [&str; 3] = ["docx", "docm", "doc"];

/// The biggest Word file converted, in bytes. A package can unpack into much more than it weighs
/// (WordCraft caps that too); this keeps the conversion, which runs on the interface thread in a
/// browser, to a size that finishes.
pub const MAX_WORD_BYTES: usize = 32 << 20;

/// What a ZIP package (`.docx`, `.docm`) starts with.
const ZIP: &[u8] = b"PK\x03\x04";
/// The OLE compound-file signature of Word 97-2003 files (`.doc`) and of password-protected packages.
const OLE: &[u8] = &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    /// An OOXML package: `.docx`, `.docm`.
    Docx,
    /// A Word 97-2003 binary file, or a password-protected package (told apart when it is read).
    Doc,
}

/// The reader for `bytes` named `name`: it needs a Word extension (a ZIP is also an Excel, a PowerPoint
/// or just an archive) and the bytes of a package or an OLE file.
fn format(name: &str, bytes: &[u8]) -> Option<Format> {
    let (_, extension) = name.rsplit_once('.')?;
    if !WORD_EXTENSIONS.iter().any(|e| extension.eq_ignore_ascii_case(e)) {
        return None;
    }
    if bytes.starts_with(ZIP) {
        Some(Format::Docx)
    } else if bytes.starts_with(OLE) {
        Some(Format::Doc)
    } else {
        None
    }
}

/// Whether `bytes` named `name` is a Word file [`word_to_pdf`] converts.
pub fn is_word(name: &str, bytes: &[u8]) -> bool {
    format(name, bytes).is_some()
}

/// Convert the Word file `bytes` named `name` to PDF bytes. Nothing is written back to the Word file.
///
/// # Errors
/// A file that is not Word, is over [`MAX_WORD_BYTES`], is password-protected or does not read; and,
/// on a build that is not the browser's, always (see the module documentation).
pub fn word_to_pdf(name: &str, bytes: &[u8]) -> Result<Vec<u8>, CreateError> {
    let Some(format) = format(name, bytes) else {
        return Err(CreateError::Invalid(format!("{name}: not a Word file (.docx or .doc)")));
    };
    if bytes.len() > MAX_WORD_BYTES {
        return Err(CreateError::Invalid(format!("{name}: the Word file is over {} MiB", MAX_WORD_BYTES >> 20)));
    }
    convert(name, format, bytes)
}

#[cfg(any(target_arch = "wasm32", test))]
fn convert(name: &str, format: Format, bytes: &[u8]) -> Result<Vec<u8>, CreateError> {
    let fail = |what: &str, error: &dyn std::fmt::Display| CreateError::Invalid(format!("{name}: {what} ({error})"));
    let document = match format {
        Format::Docx => wordcraft_docx::read(bytes).map_err(|e| fail("the Word file could not be read", &e))?,
        Format::Doc => {
            if wordcraft_docx::is_encrypted(bytes) {
                return Err(CreateError::Invalid(format!("{name}: password-protected Word files can't be opened here")));
            }
            wordcraft_docbin::read(bytes).map_err(|e| fail("the Word file could not be read", &e))?
        }
    };
    wordcraft_pdf::export(&document, &wordcraft_pdf::PdfOptions::default()).map_err(|e| fail("the PDF could not be written", &e))
}

#[cfg(not(any(target_arch = "wasm32", test)))]
fn convert(name: &str, _format: Format, _bytes: &[u8]) -> Result<Vec<u8>, CreateError> {
    Err(CreateError::Invalid(format!("{name}: Word files open only in the web version, which converts them to PDF; save it as a PDF first")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WordCraft's own bundled fonts and nothing else, whatever the machine has installed. The first
    /// call decides for the whole test process.
    fn bundled_fonts_only() {
        wordcraft_fonts::FontDb::init_test_fonts();
    }

    fn crc32(bytes: &[u8]) -> u32 {
        let mut c = !0u32;
        for &b in bytes {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }

    /// A ZIP with the entries stored, not compressed.
    fn stored_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let (mut out, mut central) = (Vec::new(), Vec::new());
        for (name, data) in entries {
            let offset = out.len() as u32;
            let (crc, size, len) = (crc32(data), data.len() as u32, name.len() as u16);
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            for n in [crc, size, size] {
                out.extend_from_slice(&n.to_le_bytes());
            }
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);
            central.extend_from_slice(b"PK\x01\x02");
            central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            for n in [crc, size, size] {
                central.extend_from_slice(&n.to_le_bytes());
            }
            central.extend_from_slice(&len.to_le_bytes());
            central.extend_from_slice(&[0; 12]);
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let start = out.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(b"PK\x05\x06\0\0\0\0");
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(central.len() as u32).to_le_bytes());
        out.extend_from_slice(&start.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out
    }

    /// A one-section `.docx` with a paragraph for each line.
    fn docx(lines: &[String]) -> Vec<u8> {
        let paragraphs: String = lines.iter().map(|l| format!("<w:p><w:r><w:t xml:space=\"preserve\">{l}</w:t></w:r></w:p>")).collect();
        let document = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{paragraphs}</w:body></w:document>"
        );
        let types = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
        let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
        stored_zip(&[("[Content_Types].xml", types.as_bytes()), ("_rels/.rels", rels.as_bytes()), ("word/document.xml", document.as_bytes())])
    }

    fn page_count(pdf: &[u8]) -> usize {
        hayro_syntax::Pdf::new(pdf.to_vec()).expect("the converted file is a PDF the reader opens").pages().len()
    }

    #[test]
    fn a_word_file_is_told_by_its_extension_and_its_first_bytes() {
        for (name, bytes, word) in [
            ("a.docx", &b"PK\x03\x04rest"[..], true),
            ("A.DOCX", b"PK\x03\x04", true),
            ("macro.docm", b"PK\x03\x04", true),
            ("old.doc", OLE, true),
            ("really.doc", b"PK\x03\x04", true),
            ("sheet.xlsx", b"PK\x03\x04", false),
            ("deck.pptx", b"PK\x03\x04", false),
            ("archive.zip", b"PK\x03\x04", false),
            ("a.docx", b"%PDF-1.7", false),
            ("a.docx", b"", false),
            ("docx", b"PK\x03\x04", false),
            ("", b"PK\x03\x04", false),
        ] {
            assert_eq!(is_word(name, bytes), word, "{name:?}");
        }
    }

    #[test]
    fn a_docx_becomes_a_pdf_with_its_pages() {
        bundled_fonts_only();
        let short = word_to_pdf("letter.docx", &docx(&["Luogo e data".to_string(), "Firma".to_string()])).expect("converts");
        assert!(short.starts_with(b"%PDF-"));
        assert_eq!(page_count(&short), 1);
        // The layout engine ran: enough lines do not fit on one page.
        let lines: Vec<String> = (1..=200).map(|n| format!("Riga {n} del contratto")).collect();
        let long = word_to_pdf("contract.docx", &docx(&lines)).expect("converts");
        assert!(page_count(&long) >= 3, "{} pages", page_count(&long));
    }

    #[test]
    fn text_with_markup_characters_and_accents_converts() {
        bundled_fonts_only();
        let lines = ["Perché &amp; così &lt;sì&gt; — «virgolette» €".to_string()];
        assert!(word_to_pdf("accents.docx", &docx(&lines)).is_ok());
    }

    #[test]
    fn what_is_not_a_readable_word_file_is_refused_with_its_name() {
        bundled_fonts_only();
        let truncated = docx(&["Firma".to_string()]);
        for (name, bytes) in [
            ("notword.xlsx", b"PK\x03\x04".to_vec()),
            ("empty.docx", Vec::new()),
            ("zip.docx", b"PK\x03\x04 and then nothing a zip reader can use".to_vec()),
            ("cut.docx", truncated.get(..truncated.len() / 2).unwrap_or_default().to_vec()),
            ("ole.doc", [OLE, &[0u8; 600]].concat()),
        ] {
            match word_to_pdf(name, &bytes) {
                Err(CreateError::Invalid(message)) => assert!(message.starts_with(name), "{message}"),
                other => panic!("{name}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_file_over_the_limit_is_refused_before_it_is_read() {
        let mut huge = b"PK\x03\x04".to_vec();
        huge.resize(MAX_WORD_BYTES + 1, 0);
        match word_to_pdf("huge.docx", &huge) {
            Err(CreateError::Invalid(message)) => assert!(message.contains("over 32 MiB"), "{message}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn hostile_bytes_never_panic() {
        bundled_fonts_only();
        let seed = docx(&["Firma".to_string()]);
        assert!(word_to_pdf("seed.docx", &seed).is_ok(), "the file the mutations start from converts, so they reach the converter");
        let mut x = 0x9E37_79B9_u32;
        for round in 0..64 {
            // A valid package with a few bytes flipped, and noise after each header.
            let mut mutated = seed.clone();
            for _ in 0..=round % 4 {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let at = (x as usize) % mutated.len().max(1);
                if let Some(b) = mutated.get_mut(at) {
                    *b ^= (x >> 24) as u8 | 1;
                }
            }
            let _ = word_to_pdf("fuzz.docx", &mutated);
            let noise: Vec<u8> = (0..round * 16).map(|i| (x >> (i % 24)) as u8 ^ i as u8).collect();
            let _ = word_to_pdf("fuzz.docx", &[b"PK\x03\x04", &noise[..]].concat());
            let _ = word_to_pdf("fuzz.doc", &[OLE, &noise[..]].concat());
        }
    }
}
