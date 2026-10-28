//! LaTeX sources through Layer 3: reading the structure for Layer 2, the
//! inlined text for review models, and compiling the PDF.

use std::{path::PathBuf, time::Duration};

use mathesis_core::{
    model::ClaimKind,
    ports::{PaperReader, ReadPaper, ReadStatement},
};
use mathesis_latex::{
    LatexError, StatementKind,
    archive::{Limits, unpack},
    compile::Compiler,
    parse, pdf,
};

fn kind(kind: StatementKind) -> ClaimKind {
    match kind {
        StatementKind::Theorem => ClaimKind::Theorem,
        StatementKind::Proposition => ClaimKind::Proposition,
        StatementKind::Lemma => ClaimKind::Lemma,
        StatementKind::Corollary => ClaimKind::Corollary,
        StatementKind::Claim => ClaimKind::Claim,
        StatementKind::Conjecture => ClaimKind::Conjecture,
        StatementKind::Question => ClaimKind::Question,
    }
}

fn reason(error: LatexError) -> String {
    match error {
        LatexError::Archive(e) => format!("the upload could not be unpacked: {e}"),
        LatexError::NoMainFile(e) => format!("no main .tex file: {e}"),
        LatexError::Compile(e) => e,
    }
}

pub struct LatexReader;

impl PaperReader for LatexReader {
    fn files(
        &self,
        bytes: &[u8],
        filename: &str,
    ) -> Result<std::collections::BTreeMap<String, Vec<u8>>, String> {
        unpack(bytes, filename, Limits::default()).map_err(reason)
    }
    fn read(&self, bytes: &[u8], filename: &str) -> Result<ReadPaper, String> {
        let files = unpack(bytes, filename, Limits::default()).map_err(reason)?;
        let paper = if let Some((path, bytes)) = pdf::manuscript(&files)? {
            pdf::read(bytes, path)?
        } else {
            parse::parse(&files).map_err(reason)?
        };
        Ok(ReadPaper {
            evidence: mathesis_core::model::evidence_inventory(&files)
                .map_err(|e| e.to_string())?,
            main_file: paper.main_file,
            title: paper.title,
            authors: paper.authors,
            abstract_text: paper.abstract_text,
            statements: paper
                .statements
                .into_iter()
                .map(|s| ReadStatement {
                    kind: kind(s.kind),
                    display_name: s.display_name,
                    title: s.title,
                    latex_label: s.label,
                    body: s.body,
                    has_proof: s.has_proof,
                    section: s.section,
                })
                .collect(),
            macros: paper.macros,
            warnings: paper.warnings,
        })
    }
}

/// The source with inputs inlined and comments removed, for review models.
pub fn source_text(bytes: &[u8], filename: &str) -> Result<String, String> {
    let files = unpack(bytes, filename, Limits::default()).map_err(reason)?;
    if let Some((_, bytes)) = pdf::manuscript(&files)? {
        return pdf::text(bytes);
    }
    let main = parse::main_file(&files).map_err(reason)?;
    let mut warnings = Vec::new();
    Ok(parse::strip_comments(&parse::expand(
        &files,
        &main,
        &mut warnings,
    )))
}

pub struct Compile {
    /// The TeX Live bin directory.
    pub tex_bin: PathBuf,
    /// Writable cache for TeX's fonts and formats.
    pub cache_dir: PathBuf,
    /// Extra texmf tree with packages beyond the distribution.
    pub texmf_home: Option<PathBuf>,
    pub timeout: Duration,
}

impl Compile {
    /// Compile an uploaded source in a fresh temporary directory. Runs on a
    /// blocking thread. `Err` carries the message for the author.
    pub async fn pdf(&self, bytes: Vec<u8>, filename: String) -> Result<Vec<u8>, String> {
        let compiler = Compiler {
            bin_dir: self.tex_bin.clone(),
            cache_dir: self.cache_dir.clone(),
            texmf_home: self.texmf_home.clone(),
            timeout: self.timeout,
        };
        tokio::task::spawn_blocking(move || {
            let files = unpack(&bytes, &filename, Limits::default()).map_err(reason)?;
            if let Some((_, bytes)) = pdf::manuscript(&files)? {
                // The uploaded PDF is the reading copy; never re-render or substitute it.
                pdf::text(bytes)?;
                return Ok(bytes.to_vec());
            }
            let main = parse::main_file(&files).map_err(reason)?;
            let work = tempfile::tempdir().map_err(|e| e.to_string())?;
            compiler
                .compile(&files, &main, work.path())
                .map(|c| c.pdf)
                .map_err(reason)
        })
        .await
        .map_err(|e| format!("the compiler task failed: {e}"))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pdf_intake_preserves_original_bytes_metadata_and_review_text() {
        if std::process::Command::new("pdftotext")
            .arg("-v")
            .output()
            .is_err()
        {
            eprintln!("PDF integration test needs Poppler pdftotext");
            return;
        }
        use mathesis_core::{
            app::{App, Upload},
            memory::MemoryStores,
            model::*,
            policy::Policy,
            ports::SystemClock,
        };
        use std::{collections::BTreeSet, sync::Arc};
        let bytes = include_bytes!("../../layer3-latex/tests/fixtures/manuscript.pdf").to_vec();
        let read = LatexReader.read(&bytes, "paper.pdf").unwrap();
        assert_eq!(read.main_file, "paper.pdf");
        assert_eq!(read.statements.len(), 2);
        assert!(
            source_text(&bytes, "paper.pdf")
                .unwrap()
                .contains("Proof. This follows")
        );
        let cache = tempfile::tempdir().unwrap();
        let compiler = Compile {
            tex_bin: "/nonexistent".into(),
            cache_dir: cache.path().into(),
            texmf_home: None,
            timeout: std::time::Duration::from_secs(1),
        };
        assert_eq!(
            compiler
                .pdf(bytes.clone(), "paper.pdf".into())
                .await
                .unwrap(),
            bytes
        );
        let stores = Arc::new(MemoryStores::default());
        let app = App::new(
            stores.ports(Arc::new(SystemClock), Arc::new(LatexReader)),
            Policy::default(),
            BTreeSet::new(),
        );
        let author = app
            .ensure_service_account(&"pdf-author".into(), "A. Author", Role::Endorser)
            .await
            .unwrap();
        let metadata: NewPaper = serde_json::from_value(serde_json::json!({
            "title":"Explicit PDF title", "abstract_text":"Abstract supplied by the author.",
            "purpose":"review_only", "processing_consent":"ai-review-2026-10-11",
            "ai_disclosure":{"level":"none","statement":"No AI used."}, "authors":[{"name":"A. Author","affiliation":"Example University"}]
        })).unwrap();
        let paper = app
            .submit_paper(
                &author,
                metadata,
                Upload {
                    filename: "paper.pdf".into(),
                    bytes: bytes.clone(),
                },
            )
            .await
            .unwrap();
        assert_eq!(paper.title, "Explicit PDF title");
        assert_eq!(paper.abstract_text, "Abstract supplied by the author.");
        assert_eq!(
            paper.authors[0].affiliation.as_deref(),
            Some("Example University")
        );
        assert_eq!(paper.purpose, SubmissionPurpose::ReviewOnly);
        assert_eq!(paper.analysis_visibility, Visibility::Private);
        assert_eq!(paper.status, SubmissionStatus::Draft);
        let saved = app
            .paper_file(Some(&author), &paper.id, None, false)
            .await
            .unwrap();
        assert_eq!(saved.bytes, bytes);
    }

    #[test]
    fn reads_a_single_file_paper() {
        let source = br"\documentclass{article}
\newtheorem{theorem}{Theorem}
\title{Gaps}\author{A. Author}
\begin{document}\maketitle
\begin{theorem}\label{thm:main} Every gap is finite. \end{theorem}
\begin{proof} Trivial. \end{proof}
\end{document}";
        let read = LatexReader.read(source, "paper.tex").unwrap();
        assert_eq!(read.title.as_deref(), Some("Gaps"));
        assert_eq!(read.statements.len(), 1);
        assert_eq!(read.statements[0].kind, ClaimKind::Theorem);
        assert!(read.statements[0].has_proof);
        assert!(LatexReader.read(b"not latex", "paper.tex").is_err());
    }
    #[tokio::test]
    async fn typed_conjecture_generates_stored_tex_and_a_confirmable_main_claim() {
        use mathesis_core::{
            app::{App, Upload},
            memory::MemoryStores,
            model::*,
            policy::Policy,
            ports::SystemClock,
        };
        use std::{collections::BTreeSet, sync::Arc};
        let stores = Arc::new(MemoryStores::default());
        let app = App::new(
            stores.ports(Arc::new(SystemClock), Arc::new(LatexReader)),
            Policy::default(),
            BTreeSet::new(),
        );
        let author = app
            .ensure_service_account(&"author".into(), "A. Author", Role::Endorser)
            .await
            .unwrap();
        let new: NewPaper = serde_json::from_value(serde_json::json!({
            "kind":"conjecture", "processing_consent":"ai-review-2026-10-11", "ai_disclosure":{"level":"none","statement":"No AI used."}, "authors":[{"name":"A. Author"}],
            "typed_conjecture":{"title":"A conjecture: 100% & x_y","statement":"For every $n > 1$, $f(n) > 0$.", "background":"Let $f$ be the given function.", "origin":"my own"}
        })).unwrap();
        let paper = app
            .submit_paper(
                &author,
                new,
                Upload {
                    filename: String::new(),
                    bytes: vec![],
                },
            )
            .await
            .unwrap();
        assert_eq!(paper.title, "A conjecture: 100% & x_y");
        assert_eq!(paper.extracted.len(), 1);
        assert_eq!(paper.extracted[0].kind, ClaimKind::Conjecture);
        assert_eq!(paper.extracted[0].role, ClaimRole::Main);
        assert_eq!(paper.status, SubmissionStatus::Draft);
        let source = app
            .paper_file(Some(&author), &paper.id, None, false)
            .await
            .unwrap();
        assert_eq!(source.filename, "conjecture.tex");
        let text = String::from_utf8(source.bytes.clone()).unwrap();
        assert!(text.contains(r"\begin{conjecture}"));
        assert!(text.contains("my own"));
        let reread = LatexReader.read(&source.bytes, &source.filename).unwrap();
        assert_eq!(reread.statements.len(), 1);
        assert_eq!(reread.statements[0].kind, ClaimKind::Conjecture);
        let confirmed = app
            .confirm_claims(
                &author,
                &paper.id,
                vec![ClaimConfirmation {
                    depends_on_conjectures: vec![],
                    id: "C1".into(),
                    kind: ClaimKind::Conjecture,
                    role: ClaimRole::Main,
                    depends_on: vec![],
                    settles: None,
                    excluded: false,
                }],
            )
            .await
            .unwrap();
        assert_eq!(
            confirmed.claims[0].statement,
            "For every $n > 1$, $f(n) > 0$."
        );
        assert_eq!(confirmed.status, SubmissionStatus::InReview);
        if let Ok(bin) = std::env::var("WISHPOOL_TEST_TEX_BIN") {
            let cache = tempfile::tempdir().unwrap();
            let compile = Compile {
                tex_bin: bin.into(),
                cache_dir: cache.path().into(),
                texmf_home: None,
                timeout: Duration::from_secs(60),
            };
            assert!(
                compile
                    .pdf(source.bytes, source.filename)
                    .await
                    .unwrap()
                    .starts_with(b"%PDF")
            );
        }
    }
}
