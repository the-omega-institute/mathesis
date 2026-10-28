//! PDF intake preserves the original bytes and reads numbered statements from
//! the text layer. It never substitutes an invented theorem for missing text.
use crate::{ParsedPaper, Statement, StatementKind, archive::Files};
use std::{
    collections::BTreeMap,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub fn manuscript(files: &Files) -> Result<Option<(&str, &[u8])>, String> {
    if files
        .keys()
        .any(|name| !name.starts_with("evidence/") && name.to_ascii_lowercase().ends_with(".tex"))
    {
        return Ok(None);
    }
    let candidates: Vec<_> = files
        .iter()
        .filter(|(name, _)| {
            !name.starts_with("evidence/") && name.to_ascii_lowercase().ends_with(".pdf")
        })
        .collect();
    match candidates.as_slice() {
        [] => Ok(None),
        [(path, bytes)] if bytes.starts_with(b"%PDF-") => {
            Ok(Some((path.as_str(), bytes.as_slice())))
        }
        [_] => Err("The uploaded file is not a valid PDF.".into()),
        _ => Err("Choose one manuscript PDF. Add other PDFs as supplementary files.".into()),
    }
}

pub fn text(bytes: &[u8]) -> Result<String, String> {
    if !bytes.starts_with(b"%PDF-") {
        return Err("The uploaded file is not a valid PDF.".into());
    }
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input = dir.path().join("manuscript.pdf");
    let output = dir.path().join("manuscript.txt");
    std::fs::write(&input, bytes).map_err(|e| e.to_string())?;
    let mut process = Command::new("pdftotext")
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .args(["-q", "-layout", "-enc", "UTF-8", "-l", "501"])
        .arg(&input)
        .arg(&output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            "PDF reading is temporarily unavailable. Please try again or upload LaTeX source."
                .to_owned()
        })?;
    let started = Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(20)
            || output.metadata().is_ok_and(|m| m.len() > 4 * 1024 * 1024)
        {
            let _ = process.kill();
            let _ = process.wait();
            return Err("The PDF exceeds the text-reading limit. Upload the LaTeX source or a shorter manuscript.".into());
        }
        match process.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err("The PDF could not be read. Check that it is valid and not password-protected.".into());
                }
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                let _ = process.kill();
                let _ = process.wait();
                return Err(error.to_string());
            }
        }
    }
    if output.metadata().map_err(|e| e.to_string())?.len() > 4 * 1024 * 1024 {
        return Err("PDF text exceeds 4 MB.".into());
    }
    let text = std::fs::read_to_string(output).map_err(|e| e.to_string())?;
    if text.matches('\u{c}').count() > 500 {
        return Err("PDF manuscripts may contain at most 500 pages.".into());
    }
    if text.chars().filter(|c| c.is_alphanumeric()).count() < 20 {
        return Err("This PDF has no usable text layer. Export a searchable PDF or upload LaTeX source; scanned pages need OCR first.".into());
    }
    Ok(text)
}

fn heading(line: &str) -> Option<(StatementKind, String, String)> {
    let line = line.trim();
    let (word, rest) = line.split_once(char::is_whitespace)?;
    let name = word.trim_end_matches('.');
    let kind = match name.to_ascii_lowercase().as_str() {
        "theorem" => StatementKind::Theorem,
        "lemma" => StatementKind::Lemma,
        "proposition" => StatementKind::Proposition,
        "corollary" => StatementKind::Corollary,
        "claim" => StatementKind::Claim,
        "conjecture" => StatementKind::Conjecture,
        "question" | "problem" => StatementKind::Question,
        _ => return None,
    };
    let rest = rest.trim();
    let (number, body) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    // Require a printed number or capital letter, avoiding prose references such as “Theorem above”.
    let id = number.trim_end_matches(['.', ':']);
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c.is_ascii_uppercase())
    {
        return None;
    }
    Some((kind, format!("{name} {id}"), body.to_owned()))
}

fn statements(text: &str) -> Result<Vec<Statement>, String> {
    let mut out = Vec::new();
    let mut current: Option<Statement> = None;
    let mut in_proof = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some((kind, label, body)) = heading(line) {
            if let Some(previous) = current.take() {
                out.push(previous);
            }
            current = Some(Statement {
                index: out.len() + 1,
                kind,
                environment: "pdf".into(),
                display_name: label
                    .split_whitespace()
                    .next()
                    .unwrap_or("Statement")
                    .into(),
                title: None,
                label: label.split_once(' ').map(|(_, number)| number.to_owned()),
                body,
                has_proof: false,
                section: None,
            });
            in_proof = false;
        } else if let Some(statement) = current.as_mut() {
            if line == "Proof"
                || line.starts_with("Proof.")
                || line.starts_with("Proof ")
                || line.starts_with("Proof:")
            {
                statement.has_proof = !statement.kind.is_open();
                in_proof = true;
            } else if !in_proof && !line.is_empty() {
                statement.body.push('\n');
                statement.body.push_str(line);
                if statement.body.chars().count() > 40_000 {
                    return Err("A PDF statement could not be separated reliably. Please upload LaTeX source.".into());
                }
            }
        }
        if out.len() > 500 {
            return Err("The PDF contains too many statements. Please upload LaTeX source.".into());
        }
    }
    if let Some(last) = current {
        out.push(last);
    }
    out.retain(|s| !s.body.trim().is_empty());
    if out.is_empty() {
        return Err("No numbered mathematical statements could be read from this PDF. Use selectable text with headings such as Theorem 1, or upload LaTeX source.".into());
    }
    Ok(out)
}

pub fn read(bytes: &[u8], filename: &str) -> Result<ParsedPaper, String> {
    let text = text(bytes)?;
    Ok(ParsedPaper { main_file: filename.into(), title: None, authors: vec![], abstract_text: None, statements: statements(&text)?, macros: BTreeMap::new(), warnings: vec!["Statements were read from the PDF text layer; mathematical symbols and statement boundaries may need correction. The original PDF is preserved for review.".into()] })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn separates_proved_statements_and_open_questions_without_inventing_results() {
        let result = statements("Theorem 1. Every even integer is divisible by two.\nProof. By definition.\nLemma A. A supporting result.\nProof. Details.\nConjecture 2. All objects have property P.\nQuestion 3. Is the bound sharp?").unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].body, "Every even integer is divisible by two.");
        assert!(result[0].has_proof && result[1].has_proof);
        assert!(!result[2].has_proof && !result[3].has_proof);
        assert!(statements("A paragraph with no identified statement.").is_err());
        assert!(heading("Theorem above implies the claim.").is_none());
    }
    #[test]
    fn distinguishes_primary_pdf_from_supplements_and_source_projects() {
        let mut files = Files::new();
        files.insert("paper.pdf".into(), b"%PDF-1.7".to_vec());
        files.insert("evidence/supplement.pdf".into(), b"%PDF-1.7".to_vec());
        files.insert("evidence/supplement.tex".into(), b"supplement".to_vec());
        assert_eq!(manuscript(&files).unwrap().unwrap().0, "paper.pdf");
        files.insert("another.pdf".into(), b"%PDF-1.7".to_vec());
        assert!(manuscript(&files).is_err());
        files.insert("main.tex".into(), vec![]);
        assert!(manuscript(&files).unwrap().is_none());
    }
    #[test]
    fn reads_actual_pdf_text_and_refuses_unreadable_inputs() {
        if Command::new("pdftotext").arg("-v").output().is_err() {
            eprintln!("PDF integration test needs Poppler pdftotext");
            return;
        }
        let bytes = include_bytes!("../tests/fixtures/manuscript.pdf");
        let result = read(bytes, "paper.pdf").unwrap();
        assert_eq!(result.main_file, "paper.pdf");
        assert_eq!(result.statements.len(), 2);
        assert_eq!(result.statements[0].label.as_deref(), Some("1"));
        assert_eq!(
            result.statements[0].body,
            "Every even integer is divisible by two."
        );
        assert!(result.statements[0].has_proof);
        assert!(!result.statements[1].has_proof);
        assert!(!result.warnings.is_empty());
        assert!(
            text(include_bytes!("../tests/fixtures/no-text.pdf"))
                .unwrap_err()
                .contains("no usable text")
        );
        assert!(
            text(b"%PDF-1.4 broken")
                .unwrap_err()
                .contains("could not be read")
        );
        assert!(text(b"not pdf").unwrap_err().contains("not a valid PDF"));
    }
}
