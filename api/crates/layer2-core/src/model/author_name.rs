//! One written form of an author name for public projections, and one rule for
//! matching it. Both are purely syntactic: stored names are never rewritten,
//! and a matching name never identifies a researcher.

use std::borrow::Cow;

use super::Author;

/// A token showing that a single comma does not separate a family name from
/// given names: generational suffixes, name lists, degrees and company forms.
/// Compared in ASCII lower case, without one trailing period.
const NOT_A_PERSONAL_NAME: [&str; 17] = [
    "jr", "sr", "jnr", "snr", "junior", "senior", "ii", "iii", "iv", "and", "et", "phd", "ph.d",
    "dphil", "inc", "ltd", "llc",
];

/// Latin, Greek and Cyrillic letters. Names in family-first scripts, such as
/// CJK, are never reordered.
fn name_letter(c: char) -> bool {
    c.is_alphabetic()
        && matches!(
            c,
            'A'..='Z'
                | 'a'..='z'
                | '\u{C0}'..='\u{24F}'
                | '\u{370}'..='\u{3FF}'
                | '\u{400}'..='\u{52F}'
                | '\u{1E00}'..='\u{1EFF}'
        )
}

/// The two parts of an unambiguous "Family, Given" name.
fn family_given(name: &str) -> Option<(&str, &str)> {
    let (family, given) = name.split_once(',')?;
    if given.contains(',')
        || !name
            .chars()
            .all(|c| name_letter(c) || matches!(c, ' ' | ',' | '.' | '-' | '\'' | '\u{2019}'))
    {
        return None;
    }
    let (family, given) = (family.trim_matches(' '), given.trim_matches(' '));
    // A bare "V" may be the fifth of a name; "V." is an initial.
    let excluded = |token: &str| {
        token.eq_ignore_ascii_case("v")
            || NOT_A_PERSONAL_NAME.contains(
                &token
                    .strip_suffix('.')
                    .unwrap_or(token)
                    .to_ascii_lowercase()
                    .as_str(),
            )
    };
    ([family, given]
        .iter()
        .all(|part| part.chars().any(name_letter))
        && !family.split(' ').chain(given.split(' ')).any(excluded))
    .then_some((family, given))
}

/// The canonical written form, "Given Family", as source-derived author lists
/// already use. Only the unambiguous single-comma "Family, Given" form is
/// rewritten; every other name is returned exactly as stored.
pub fn canonical_author_name(name: &str) -> Cow<'_, str> {
    match family_given(name) {
        Some((family, given)) => Cow::Owned(format!("{given} {family}")),
        None => Cow::Borrowed(name),
    }
}

/// Whether a stored author name matches a query: a literal, case-insensitive
/// substring of the name as written or of its canonical form.
pub fn author_name_matches(name: &str, query: &str) -> bool {
    name.to_lowercase().contains(&query.to_lowercase())
        || canonical_author_name(name)
            .to_lowercase()
            .contains(&canonical_author_name(query).to_lowercase())
}

/// Literal terms that every name matching `query` contains, ignoring case.
/// A store may preselect candidates with them; [`author_name_matches`] decides.
pub fn author_name_terms(query: &str) -> Vec<String> {
    canonical_author_name(query)
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

impl Author {
    /// The author as shown in public projections.
    pub fn canonical(&self) -> Self {
        Self {
            name: canonical_author_name(&self.name).into_owned(),
            ..self.clone()
        }
    }
}

/// Authors as shown in public projections; the stored names are unchanged.
pub fn canonical_authors(authors: &[Author]) -> Vec<Author> {
    authors.iter().map(Author::canonical).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The web helper `web/src/lib/authorName.ts` is tested against the same cases.
    fn cases(section: &str) -> Vec<serde_json::Value> {
        let all: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/author-names.json"
        )))
        .unwrap();
        all[section].as_array().unwrap().clone()
    }

    #[test]
    fn only_the_single_comma_family_given_form_is_rewritten() {
        for case in cases("canonical") {
            let stored = case["stored"].as_str().unwrap();
            let canonical = canonical_author_name(stored);
            assert_eq!(canonical, case["canonical"].as_str().unwrap(), "{stored:?}");
            assert_eq!(
                canonical_author_name(&canonical),
                canonical,
                "{stored:?} is not stable"
            );
        }
        assert_eq!(canonical_author_name("Ma, Haobo"), "Haobo Ma");
        assert_eq!(canonical_author_name("de la Cruz, Ana"), "Ana de la Cruz");
        assert_eq!(canonical_author_name("  Ma,  Haobo "), "Haobo Ma");
        for stored in [
            "Smith, John, Jr.",
            "Doe, Jane, Roe, Richard",
            "John Smith, Jr.",
            "{Research Group}, The",
            "Euclid",
            "张文林",
            "张, 文林",
        ] {
            assert!(
                matches!(canonical_author_name(stored), Cow::Borrowed(s) if s == stored),
                "{stored:?}"
            );
        }
    }

    #[test]
    fn either_written_form_matches_the_same_names() {
        for case in cases("matching") {
            let (stored, query) = (
                case["stored"].as_str().unwrap(),
                case["query"].as_str().unwrap(),
            );
            let matches = case["matches"].as_bool().unwrap();
            assert_eq!(
                author_name_matches(stored, query),
                matches,
                "{stored:?} / {query:?}"
            );
            // A store's preselection must never drop a matching name.
            if matches {
                for term in author_name_terms(query) {
                    assert!(
                        stored.to_lowercase().contains(&term.to_lowercase()),
                        "{stored:?} lacks {term:?} of {query:?}"
                    );
                }
            }
        }
        for stored in ["Ma, Haobo", "Haobo Ma"] {
            assert_eq!(
                author_name_matches(stored, "Haobo Ma"),
                author_name_matches(stored, "Ma, Haobo")
            );
        }
        assert_eq!(author_name_terms("Ma, Haobo"), ["Haobo", "Ma"]);
        assert_eq!(
            author_name_terms("Smith, John, Jr."),
            ["Smith,", "John,", "Jr."]
        );
    }

    #[test]
    fn projection_keeps_every_other_author_field() {
        let stored = Author {
            name: "Ma, Haobo".into(),
            person: Some("person".into()),
            orcid: Some("0000-0002-1825-0097".into()),
            affiliation: Some("Institute".into()),
            google_scholar: None,
            website: None,
        };
        let shown = canonical_authors(std::slice::from_ref(&stored));
        assert_eq!(shown[0].name, "Haobo Ma");
        assert_eq!(
            Author {
                name: stored.name.clone(),
                ..shown[0].clone()
            },
            stored
        );
    }
}
