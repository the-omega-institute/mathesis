//! Bounded literal citation extraction. Never evaluates TeX/BibTeX or fetches URLs.
use crate::{
    archive::Files,
    parse::{braced, bracketed, skip_space, strip_comments},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_TEXT: usize = 1_048_576;
const MAX_REFERENCES: usize = 100;
const MAX_RAW: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bibliography {
    pub method: String,
    pub references: Vec<Reference>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    pub key: String,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<String>,
    pub doi: Option<String>,
    pub arxiv: Option<String>,
    pub url: Option<String>,
    pub source_location: String,
    pub raw: String,
    pub raw_truncated: bool,
}
fn prefix(s: &str, n: usize) -> &str {
    let mut end = n.min(s.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
fn text(s: &str, n: usize) -> String {
    prefix(s.trim(), n).to_owned()
}
fn warn(notes: &mut BTreeSet<String>, s: &str) {
    notes.insert(s.into());
}
fn empty(key: &str, location: &str, raw: &str) -> Reference {
    Reference {
        key: text(key, 200),
        title: None,
        authors: vec![],
        year: None,
        doi: None,
        arxiv: None,
        url: None,
        source_location: text(location, 1000),
        raw: prefix(raw, MAX_RAW).into(),
        raw_truncated: raw.len() > MAX_RAW,
    }
}
impl Bibliography {
    pub fn pdf() -> Self {
        Self {
            method: "pdf_unindexed_v1".into(),
            references: vec![],
            limitations: vec![
                "PDF bibliography extraction is not supported; read the original PDF references."
                    .into(),
            ],
        }
    }
}
// Literal command scanner; callers advance beyond arguments to avoid nested fake citations.
fn command(s: &str, from: usize) -> Option<(usize, &str, usize)> {
    let at = from + s[from..].find('\\')?;
    let mut end = at + 1;
    while s.as_bytes().get(end).is_some_and(u8::is_ascii_alphabetic) {
        end += 1;
    }
    if end == at + 1 {
        return Some((
            at,
            "",
            end + s[end..].chars().next().map_or(0, char::len_utf8),
        ));
    }
    Some((at, &s[at + 1..end], end))
}
// Remove literal/comment-like regions and macro definitions. This is a bounded
// syntax reader, not a TeX interpreter; conditionals and custom macros stay unsupported.
fn reading_text(s: &str, omit_bibliography: bool) -> String {
    let s = strip_comments(s);
    let mut out = String::new();
    let mut from = 0;
    while let Some((at, name, end)) = command(&s, from) {
        out.push_str(&s[from..at]);
        let mut after = end;
        if name == "begin"
            && let Some((env, e)) = braced(&s, skip_space(&s, end))
            && (["verbatim", "verbatim*", "lstlisting", "minted", "comment"].contains(&env)
                || (omit_bibliography && env == "thebibliography"))
        {
            let mut scan = e;
            after = s.len();
            while let Some((_, word, end)) = command(&s, scan) {
                scan = end;
                if word == "end"
                    && let Some((closing, e)) = braced(&s, skip_space(&s, end))
                    && closing == env
                {
                    after = e;
                    break;
                }
            }
        } else if name == "verb" {
            let start = end + usize::from(s.as_bytes().get(end) == Some(&b'*'));
            if let Some(delimiter) = s[start..].chars().next() {
                let content = start + delimiter.len_utf8();
                after = s[content..]
                    .find(delimiter)
                    .map_or(s.len(), |n| content + n + delimiter.len_utf8());
            }
        } else if [
            "newcommand",
            "renewcommand",
            "providecommand",
            "DeclareRobustCommand",
        ]
        .contains(&name)
        {
            let mut p = skip_space(&s, end);
            if s.as_bytes().get(p) == Some(&b'*') {
                p = skip_space(&s, p + 1);
            }
            p = if let Some((_, e)) = braced(&s, p) {
                e
            } else {
                command(&s, p).map_or(p, |(_, _, e)| e)
            };
            for _ in 0..2 {
                if let Some((_, e)) = bracketed(&s, p) {
                    p = e;
                }
            }
            after = braced(&s, skip_space(&s, p)).map_or(p, |(_, e)| e);
        } else if ["def", "gdef", "edef", "xdef"].contains(&name) {
            after = s[end..]
                .find('{')
                .and_then(|n| braced(&s, end + n))
                .map_or(end, |(_, e)| e);
        } else {
            out.push_str(&s[at..end]);
        }
        from = after;
    }
    out.push_str(&s[from..]);
    out
}
fn arguments<'a>(s: &'a str, name: &str) -> Vec<&'a str> {
    let mut out = vec![];
    let mut from = 0;
    while let Some((_, word, end)) = command(s, from) {
        from = end;
        if word != name {
            continue;
        }
        let mut p = end;
        if s.as_bytes().get(p) == Some(&b'*') {
            p += 1;
        }
        for _ in 0..2 {
            if let Some((_, e)) = bracketed(s, p) {
                p = e;
            }
        }
        if let Some((arg, e)) = braced(s, skip_space(s, p)) {
            out.push(arg);
            from = e;
        }
    }
    out
}
fn valid_key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 200
        && s.chars()
            .all(|c| !c.is_whitespace() && !c.is_control() && !"\\{}%#*".contains(c))
}
/// Split a source-supplied author field at complete `and`-separated names.
///
/// A name is either retained in full or omitted.  In particular, never turn a
/// byte-boundary prefix into a different apparent author name; callers retain
/// the raw entry and publish an explicit coverage limitation when this bound
/// is reached.
fn names(s: &str) -> (Vec<String>, bool) {
    let mut depth: usize = 0;
    let mut start = 0;
    let mut out = vec![];
    let mut incomplete = false;
    let mut push = |part: &str| {
        let part = part.trim();
        if part.is_empty() {
            return;
        }
        if part.len() > 300 || out.len() >= 50 {
            incomplete = true;
            return;
        }
        out.push(part.to_owned());
    };
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 && s[i..].starts_with(" and ") {
            push(&s[start..i]);
            start = i + 5;
        }
    }
    push(&s[start..]);
    (out, incomplete)
}
// Parse only literal fields; macro concatenation/crossref/string expansion is not guessed.
fn fields(s: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut p = 0;
    while p < s.len() {
        p = skip_space(s, p);
        if s.as_bytes().get(p) == Some(&b',') {
            p += 1;
            continue;
        }
        let start = p;
        while s
            .as_bytes()
            .get(p)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            p += 1;
        }
        if p == start {
            break;
        }
        let name = s[start..p].to_ascii_lowercase();
        p = skip_space(s, p);
        if s.as_bytes().get(p) != Some(&b'=') {
            break;
        }
        p = skip_space(s, p + 1);
        let (value, end) = if let Some((v, e)) = braced(s, p) {
            (v, e)
        } else if s.as_bytes().get(p) == Some(&b'"') {
            let mut e = p + 1;
            let mut depth = 0;
            while e < s.len() {
                match s.as_bytes()[e] {
                    b'\\' => {
                        e += 2;
                        continue;
                    }
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b'"' if depth == 0 => break,
                    _ => {}
                }
                e += 1;
            }
            if e >= s.len() {
                break;
            }
            (&s[p + 1..e], e + 1)
        } else {
            let e = s[p..].find(',').map_or(s.len(), |n| p + n);
            let v = s[p..e].trim();
            if !v.bytes().all(|b| b.is_ascii_digit()) {
                p = e;
                continue;
            }
            (v, e)
        };
        p = skip_space(s, end);
        if p < s.len() && s.as_bytes()[p] != b',' {
            break;
        }
        // Duplicate fields are ambiguous; don't select an identity arbitrarily.
        out.entry(name)
            .and_modify(|v| *v = String::new())
            // The enclosing entry is already bounded by `MAX_RAW`.  Keep the
            // complete field here so an author list whose combined byte length
            // exceeds 1000 can still be split at complete names below.
            .or_insert_with(|| text(value, MAX_RAW));
    }
    out.retain(|_, v| !v.is_empty());
    out
}
fn bib_entries(
    s: &str,
    path: &str,
    wanted: &BTreeSet<String>,
    found: &mut BTreeMap<String, Vec<Reference>>,
    incomplete_authors: &mut bool,
) {
    let s = strip_comments(s);
    let mut p = 0;
    while let Some(n) = s[p..].find('@') {
        let start = p + n;
        p = start + 1;
        while s.as_bytes().get(p).is_some_and(u8::is_ascii_alphabetic) {
            p += 1;
        }
        let kind = s[start + 1..p].to_ascii_lowercase();
        let Some((body, end)) = braced(&s, skip_space(&s, p)) else {
            continue;
        };
        p = end;
        if ["comment", "preamble", "string"].contains(&kind.as_str()) {
            continue;
        }
        let Some((key, rest)) = body.split_once(',') else {
            continue;
        };
        let key = key.trim();
        if !wanted.contains(key) {
            continue;
        }
        let mut r = empty(key, path, &s[start..end]);
        if !r.raw_truncated {
            let mut f = fields(rest);
            r.title = f.remove("title").map(|v| text(&v, 1000));
            r.authors = f.remove("author").map_or_else(Vec::new, |a| {
                let (authors, incomplete) = names(&a);
                *incomplete_authors |= incomplete;
                authors
            });
            r.year = f.remove("year").map(|v| text(&v, 1000));
            r.doi = f.remove("doi").map(|v| text(&v, 1000));
            r.url = f.remove("url").map(|v| text(&v, 1000));
            r.arxiv = if f
                .get("archiveprefix")
                .is_some_and(|p| p.eq_ignore_ascii_case("arxiv"))
            {
                f.remove("eprint").map(|v| text(&v, 1000))
            } else {
                None
            };
        }
        let matches = found.entry(key.into()).or_default();
        if matches.len() < 2 {
            matches.push(r);
        }
    }
}
fn bibitems(
    s: &str,
    path: &str,
    wanted: &BTreeSet<String>,
    found: &mut BTreeMap<String, Vec<Reference>>,
) {
    let s = strip_comments(s);
    // Admit only complete literal environments. An absent/malformed closing
    // boundary must never turn later sections into reference evidence.
    let mut from = 0;
    while let Some((_, name, end)) = command(&s, from) {
        from = end;
        if name != "begin" {
            continue;
        }
        let Some(("thebibliography", e)) = braced(&s, skip_space(&s, end)) else {
            continue;
        };
        let Some((_, body_start)) = braced(&s, skip_space(&s, e)) else {
            continue;
        };
        let mut scan = body_start;
        let mut close = None;
        while let Some((at, name, end)) = command(&s, scan) {
            scan = end;
            if name == "end" {
                if let Some(("thebibliography", e)) = braced(&s, skip_space(&s, end)) {
                    close = Some((at, e));
                }
                break;
            }
            // Nested environments and sectioning are outside this literal
            // bibliography grammar; do not infer a missing terminator.
            if [
                "begin",
                "section",
                "subsection",
                "subsubsection",
                "chapter",
                "part",
                "appendix",
            ]
            .contains(&name)
            {
                break;
            }
        }
        let Some((body_end, after)) = close else {
            continue;
        };
        from = after;
        let body = &s[body_start..body_end];
        let mut positions = vec![];
        let mut scan = 0;
        while let Some((at, name, end)) = command(body, scan) {
            scan = end;
            if name != "bibitem" {
                continue;
            }
            let p = bracketed(body, skip_space(body, end)).map_or(end, |(_, e)| e);
            if let Some((key, e)) = braced(body, skip_space(body, p)) {
                positions.push((at, key.trim(), e));
                scan = e;
            }
        }
        for (i, (at, key, start)) in positions.iter().enumerate() {
            if !wanted.contains(*key) {
                continue;
            }
            let entry_end = positions.get(i + 1).map_or(body.len(), |p| p.0);
            let entry = &body[*start..entry_end];
            let mut r = empty(key, path, &body[*at..entry_end]);
            // No inferred author/title boundaries in inline/bbl prose.
            if !r.raw_truncated {
                let dois = arguments(entry, "doi");
                if dois.len() == 1 {
                    r.doi = Some(text(dois[0], 1000));
                }
                let urls: Vec<_> = arguments(entry, "url")
                    .into_iter()
                    .chain(arguments(entry, "href"))
                    .collect();
                if urls.len() == 1 {
                    r.url = Some(text(urls[0], 1000));
                }
            }
            let matches = found.entry((*key).into()).or_default();
            if matches.len() < 2 {
                matches.push(r);
            }
        }
    }
}

pub fn extract(files: &Files, main: &str, expanded: &str) -> Bibliography {
    let mut notes=BTreeSet::from([
        "Only literal citation commands and selected local bibliography files are indexed. TeX conditionals, custom macros, BibTeX strings/crossrefs and BibLaTeX bbl formats are not evaluated. Inline/bbl entries require a complete literal thebibliography environment without nested environments or sectioning; unsupported or malformed boundaries are omitted. Coverage may be incomplete.".into(),
        "Bibliographic names and identifiers are source-declared, not independently authenticated. Citation is not proof use.".into(),
    ]);
    if expanded.len() > MAX_TEXT {
        warn(
            &mut notes,
            "Expanded source exceeded 1 MiB; only its prefix was indexed.",
        );
    }
    let expanded = prefix(expanded, MAX_TEXT);
    let body = expanded
        .find("\\begin{document}")
        .map_or(expanded, |p| &expanded[p..]);
    let reading = reading_text(body, true);
    let resources = reading_text(expanded, true);
    let mut keys = BTreeSet::new();
    let mut p = 0;
    while let Some((_, name, end)) = command(&reading, p) {
        p = end;
        if name == "nocite" {
            warn(&mut notes, "nocite entries do not create citation links.");
            continue;
        }
        if ![
            "cite",
            "citep",
            "citet",
            "citealp",
            "citealt",
            "citeauthor",
            "citeyear",
            "citeyearpar",
            "autocite",
            "parencite",
            "textcite",
            "footcite",
        ]
        .contains(&name)
        {
            continue;
        }
        let mut a = end;
        if reading.as_bytes().get(a) == Some(&b'*') {
            a += 1;
        }
        for _ in 0..2 {
            if let Some((_, e)) = bracketed(&reading, a) {
                a = e;
            }
        }
        if let Some((arg, e)) = braced(&reading, skip_space(&reading, a)) {
            p = e;
            for key in arg.split(',').map(str::trim) {
                if !valid_key(key) {
                    warn(
                        &mut notes,
                        "Nonliteral or oversized citation keys were omitted.",
                    );
                    continue;
                }
                if keys.len() < MAX_REFERENCES {
                    keys.insert(key.to_owned());
                } else if !keys.contains(key) {
                    warn(
                        &mut notes,
                        "At most 100 distinct citation keys are indexed per source version.",
                    );
                }
            }
        }
    }
    let mut found = BTreeMap::new();
    // Prefer the selected database over a generated bbl; don't double-count both.
    let mut selected = BTreeSet::new();
    for command in ["bibliography", "addbibresource"] {
        for arg in arguments(&resources, command) {
            for name in arg.split(',').map(str::trim) {
                if !valid_key(name) {
                    continue;
                }
                let name = if name.ends_with(".bib") {
                    name.to_owned()
                } else {
                    format!("{name}.bib")
                };
                let parent = main.rsplit_once('/').map(|(p, _)| p);
                let relative = parent.map_or_else(|| name.clone(), |p| format!("{p}/{name}"));
                if files.contains_key(&relative) {
                    selected.insert(relative);
                } else {
                    selected.insert(name);
                }
            }
        }
    }
    let has_database = selected.iter().any(|p| files.contains_key(p));
    if !has_database {
        selected.clear();
        selected.insert(
            main.strip_suffix(".tex")
                .map_or_else(|| format!("{main}.bbl"), |p| format!("{p}.bbl")),
        );
    }
    let mut budget = MAX_TEXT;
    let mut incomplete_authors = false;
    for path in selected.iter().take(16) {
        let Some(bytes) = files.get(path) else {
            continue;
        };
        let Ok(s) = std::str::from_utf8(bytes) else {
            warn(
                &mut notes,
                "A bibliography file was not UTF-8 and was omitted.",
            );
            continue;
        };
        let bounded = prefix(s, budget);
        if bounded.len() < s.len() {
            warn(
                &mut notes,
                "Bibliography files exceeded the shared 1 MiB extraction budget.",
            );
        }
        budget -= bounded.len();
        if path.ends_with(".bib") {
            bib_entries(bounded, path, &keys, &mut found, &mut incomplete_authors);
        } else {
            bibitems(bounded, path, &keys, &mut found);
        }
    }
    if selected.len() > 16 {
        warn(
            &mut notes,
            "At most 16 selected bibliography files are read.",
        );
    }
    bibitems(&reading_text(expanded, false), main, &keys, &mut found);
    let mut references = vec![];
    for key in keys {
        let matches = found.remove(&key).unwrap_or_default();
        let r = if matches.len() == 1 {
            matches.into_iter().next().unwrap()
        } else {
            warn(
                &mut notes,
                "Some cited keys are missing, unsupported or ambiguous; their source-local identities are retained.",
            );
            if matches.is_empty() {
                empty(&key, main, "Unresolved literal citation")
            } else {
                let raw = matches
                    .iter()
                    .map(|r| format!("Candidate in {}:\n{}", r.source_location, r.raw))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let locations = matches
                    .iter()
                    .map(|r| r.source_location.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                let mut unresolved = empty(&key, &locations, &raw);
                unresolved.raw_truncated |= matches.iter().any(|r| r.raw_truncated);
                unresolved
            }
        };
        if r.raw_truncated {
            warn(
                &mut notes,
                "A reference exceeded 4096 bytes; a raw prefix is retained without inferred metadata.",
            );
        }
        references.push(r);
    }
    if incomplete_authors {
        warn(
            &mut notes,
            "Some source author fields contained an omitted or oversized name; complete names are retained only when they fit the per-name and 50-name bounds. Consult the raw reference evidence.",
        );
    }
    Bibliography {
        method: "latex_literal_citations_v1".into(),
        references,
        limitations: notes.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(source: &str, bib: &str) -> Bibliography {
        extract(
            &BTreeMap::from([("refs.bib".into(), bib.as_bytes().to_vec())]),
            "main.tex",
            source,
        )
    }
    #[test]
    fn cites_only_used_entries_retains_credit_and_literal_fields() {
        let b = run(
            r"\begin{document}\citep[see][p. 3]{old}\nocite{unused}\bibliography{refs}",
            r#"@article{old, title={An {old} result}, author={A. Earlier and {Research Group}}, year=1901, doi="10.1234/OLD"}
            @article{unused, title={Never cited}}"#,
        );
        assert_eq!(b.references.len(), 1);
        assert_eq!(b.references[0].authors, ["A. Earlier", "{Research Group}"]);
        assert_eq!(b.references[0].doi.as_deref(), Some("10.1234/OLD"));
        assert!(b.references[0].raw.contains("year=1901"));
    }
    #[test]
    fn comments_macros_and_verbatim_do_not_invent_citations() {
        let b = run(
            r"\newcommand{\unused}{\cite{preamble}}\begin{document}
% \cite{comment}
\newcommand{\unused}{\cite{macro}}\def\other{\cite{def}}
\begin{verbatim}\cite{verbatim}\end{verbatim}\verb|\cite{verb}|\verbé\cite{unicode}é
\cite{actual}\bibliography{refs}",
            "",
        );
        assert_eq!(
            b.references
                .iter()
                .map(|r| r.key.as_str())
                .collect::<Vec<_>>(),
            ["actual"]
        );
    }
    #[test]
    fn ambiguity_and_string_expressions_never_select_an_identifier() {
        let b = run(
            r"\begin{document}\cite{a,b}\bibliography{refs}",
            r#"@book{a,doi={10.1234/one}} @book{a,doi={10.1234/two}}
@book{b,title={Prefix} # suffix,doi={10.1234/never}}"#,
        );
        assert!(b.references.iter().all(|r| r.doi.is_none()));
        assert!(b.limitations.iter().any(|s| s.contains("ambiguous")));
    }
    #[test]
    fn inline_and_generated_bbl_preserve_prose_without_guessing_names() {
        let b = extract(
            &Files::new(),
            "main.tex",
            r"\begin{document}\cite{old}\begin{thebibliography}{9}\bibitem{old} A. Earlier, Old result (1901). \doi{10.1234/old}\end{thebibliography}",
        );
        assert_eq!(b.references[0].doi.as_deref(), Some("10.1234/old"));
        assert!(b.references[0].authors.is_empty());
        assert!(b.references[0].raw.contains("A. Earlier"));
        let files = Files::from([(
            "main.bbl".into(),
            br"\begin{thebibliography}{9}\bibitem{a} Text.\end{thebibliography}".to_vec(),
        )]);
        let b = extract(
            &files,
            "main.tex",
            r"\begin{document}\cite{a}\bibliography{absent}",
        );
        assert_eq!(b.references[0].source_location, "main.bbl");
    }
    #[test]
    fn inline_identifier_extraction_stays_inside_the_bibliography_entry() {
        let b = extract(
            &Files::new(),
            "main.tex",
            r"\begin{document}\cite{old}\begin{thebibliography}{9}\bibitem{old} E. Earlier, Old result (1901).\end{thebibliography}
            \section{Supplementary data}\href{https://doi.org/10.9999/dataset}{Dataset}\doi{10.9999/other}",
        );
        let reference = &b.references[0];
        assert!(reference.doi.is_none());
        assert!(reference.url.is_none());
        assert!(!reference.raw.contains("dataset"));

        let b = extract(
            &Files::new(),
            "main.tex",
            r"\begin{document}\cite{old}\begin{thebibliography}{9}\bibitem{old} E. Earlier, Old result. \href{https://doi.org/10.1234/old}{DOI}\end{thebibliography}",
        );
        assert_eq!(
            b.references[0].url.as_deref(),
            Some("https://doi.org/10.1234/old")
        );
    }
    #[test]
    fn literal_occurrence_boundaries_isolate_every_identifier_across_whitespace_and_sources() {
        for closing in [
            r"\end{thebibliography}",
            r"\end {thebibliography}",
            "\\end\n\t{thebibliography}",
        ] {
            for inside in [
                "",
                r"\doi{10.1234/old}",
                r"\url{https://doi.org/10.1234/old}",
                r"\href{https://doi.org/10.1234/old}{Source}",
            ] {
                let env = format!(
                    r"\begin {{thebibliography}} {{9}}\bibitem {{first}} First.\doi{{10.1111/first}}\bibitem [Earlier 1901] {{old}} E. Earlier, \emph{{Old result}} (1901). {inside}{closing}"
                );
                let outside = r"\section{Appendix}\doi{10.9999/appendix}\url{https://doi.org/10.9999/url}\href{https://doi.org/10.9999/href}{Dataset}";
                for bbl in [false, true] {
                    let source = format!(
                        r"\begin{{document}}\cite{{first,old}}{}",
                        if bbl { r"\bibliography{absent}" } else { &env }
                    );
                    let files = if bbl {
                        Files::from([("main.bbl".into(), env.as_bytes().to_vec())])
                    } else {
                        Files::new()
                    };
                    let baseline = extract(&files, "main.tex", &source);
                    let changed_source = if bbl {
                        source.clone()
                    } else {
                        format!("{source}{outside}")
                    };
                    let changed_files = if bbl {
                        Files::from([("main.bbl".into(), format!("{env}{outside}").into_bytes())])
                    } else {
                        Files::new()
                    };
                    let changed = extract(&changed_files, "main.tex", &changed_source);
                    assert_eq!(
                        baseline.references, changed.references,
                        "{closing:?} {inside} bbl={bbl}"
                    );
                    let first = &changed.references[0];
                    let last = &changed.references[1];
                    assert_eq!(first.doi.as_deref(), Some("10.1111/first"));
                    assert_eq!(
                        last.doi.as_deref(),
                        inside.starts_with(r"\doi").then_some("10.1234/old")
                    );
                    assert_eq!(
                        last.url.as_deref(),
                        (inside.starts_with(r"\url") || inside.starts_with(r"\href"))
                            .then_some("https://doi.org/10.1234/old")
                    );
                    assert!(!last.raw.contains("Appendix"));
                }
            }
        }
    }
    #[test]
    fn missing_or_malformed_environment_does_not_admit_later_text_as_evidence() {
        for boundary in [
            "",
            r"\end thebibliography",
            r"\end{wrong}",
            r"\end{thebibliography",
            r"\end{thebibliographyExtra}",
        ] {
            let source = format!(
                r"\begin{{document}}\cite{{old}}\begin{{thebibliography}}{{9}}\bibitem{{old}}Earlier.{boundary}\section{{Later}}\doi{{10.9999/later}}\end{{document}}"
            );
            let b = extract(&Files::new(), "main.tex", &source);
            assert_eq!(b.references.len(), 1);
            assert!(b.references[0].doi.is_none());
            assert_eq!(b.references[0].raw, "Unresolved literal citation");
            assert!(
                b.limitations
                    .iter()
                    .any(|n| n.contains("malformed boundaries"))
            );
        }
    }
    #[test]
    fn long_author_fields_keep_complete_names_and_report_omissions() {
        let authors = (0..24)
            .map(|n| format!("Author{n:02} Givenname Middlename Surname"))
            .collect::<Vec<_>>();
        let bib = format!(
            "@article{{old,author={{{}}},title={{Long list}}}}",
            authors.join(" and ")
        );
        let b = run(r"\begin{document}\cite{old}\bibliography{refs}", &bib);
        assert_eq!(b.references[0].authors, authors);
        assert!(
            b.references[0]
                .authors
                .iter()
                .all(|author| author.ends_with("Surname"))
        );
        assert!(
            !b.limitations
                .iter()
                .any(|s| s.contains("omitted or oversized name"))
        );

        let oversized = format!("@article{{old,author={{{}}}}}", "A".repeat(301));
        let b = run(r"\begin{document}\cite{old}\bibliography{refs}", &oversized);
        assert!(b.references[0].authors.is_empty());
        assert!(
            b.limitations
                .iter()
                .any(|s| s.contains("omitted or oversized name"))
        );
        assert!(b.references[0].raw.contains(&"A".repeat(301)));
    }
    #[test]
    fn archive_inputs_and_preamble_biblatex_resource_are_read_without_new_author_metadata() {
        use std::io::{Cursor, Write};
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, contents) in [
            (
                "project/main.tex",
                r"\documentclass{article}\addbibresource{refs.bib}\begin{document}\input{body}\end{document}",
            ),
            (
                "project/body.tex",
                r"\textcite{old}\begin{theorem}A result.\end{theorem}",
            ),
            (
                "project/refs.bib",
                r"@book{old,title={Earlier foundation},author={E. Earlier},year={1901},archiveprefix={arXiv},eprint={math/0301234v2}}",
            ),
        ] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(contents.as_bytes()).unwrap();
        }
        let bytes = writer.finish().unwrap().into_inner();
        let files =
            crate::archive::unpack(&bytes, "synthetic.zip", crate::archive::Limits::default())
                .unwrap();
        let main = crate::parse::main_file(&files).unwrap();
        let mut warnings = vec![];
        let source = crate::parse::expand(&files, &main, &mut warnings);
        let b = extract(&files, &main, &source);
        assert_eq!(b.references.len(), 1);
        assert_eq!(b.references[0].authors, ["E. Earlier"]);
        assert_eq!(b.references[0].arxiv.as_deref(), Some("math/0301234v2"));
    }
    #[test]
    fn bounds_missing_and_pdf_coverage_are_explicit() {
        let many = (0..110)
            .map(|n| format!("k{n}"))
            .collect::<Vec<_>>()
            .join(",");
        let b = run(
            &format!("\\begin{{document}}\\cite{{{many}}}\\bibliography{{refs}}"),
            "",
        );
        assert_eq!(b.references.len(), 100);
        assert!(b.limitations.iter().any(|s| s.contains("100 distinct")));
        let b = run(
            r"\begin{document}\cite{a}\bibliography{refs}",
            &format!("@book{{a,title={{{}}}}}", "𝑥".repeat(5000)),
        );
        assert!(b.references[0].raw_truncated);
        assert!(b.references[0].raw.len() <= 4096);
        assert!(b.references[0].title.is_none());
        assert!(Bibliography::pdf().references.is_empty());
    }
}
