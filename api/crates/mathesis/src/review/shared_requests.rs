//! Fixed, read-only GitHub adapter. A failed refresh never replaces the last snapshot.
use chrono::{Timelike, Utc};
use mathesis_core::{app::App, model::*};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

const API: &str = "https://api.github.com";
const LIBRARY: &str = "the-omega-institute/trureturing";

struct GitHub {
    client: reqwest::Client,
    token: Option<String>,
}
impl GitHub {
    async fn get(&self, path: &str) -> Result<Value, String> {
        let mut request = self.client.get(format!("{API}{path}"));
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| "shared request transport failed")?;
        if !response.status().is_success() {
            // A request can mention a normal issue number in the library repo.
            // It is not merged-PR evidence; other transport/auth failures stop refresh.
            if response.status() == reqwest::StatusCode::NOT_FOUND && path.contains("/pulls/") {
                return Ok(Value::Null);
            }
            return Err(format!(
                "shared request GitHub HTTP {} at {path}",
                response.status()
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "shared request read interrupted")?
        {
            if bytes.len() + chunk.len() > 8 * 1024 * 1024 {
                return Err("shared request page exceeds 8 MiB".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| "invalid shared request GitHub JSON".into())
    }
    async fn pages(&self, path: &str) -> Result<Vec<Value>, String> {
        let mut rows = vec![];
        for page in 1..=100 {
            let separator = if path.contains('?') { '&' } else { '?' };
            let value = self
                .get(&format!("{path}{separator}per_page=100&page={page}"))
                .await?;
            let items = value.as_array().ok_or("invalid shared request list")?;
            rows.extend(items.iter().cloned());
            if items.len() < 100 {
                return Ok(rows);
            }
        }
        Err("shared request list exceeds its page bound".into())
    }
}

fn contributor(value: &Value) -> Option<RequestContributor> {
    let login = value["login"].as_str()?;
    if login.is_empty()
        || login.len() > 100
        || !login
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-[]".contains(&b))
    {
        return None;
    }
    Some(RequestContributor {
        login: login.into(),
        url: format!("https://github.com/{login}"),
        agent: value["type"] == "Bot" || login.ends_with("[bot]"),
    })
}

fn references(text: &str) -> BTreeSet<u64> {
    let mut found = BTreeSet::new();
    for prefix in [
        "https://github.com/the-omega-institute/trureturing/pull/",
        "the-omega-institute/trureturing#",
    ] {
        for suffix in text.split(prefix).skip(1) {
            let number: String = suffix.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(number) = number.parse::<u64>()
                && number > 0
            {
                found.insert(number);
            }
        }
    }
    found
}

fn link(text: &str, prefix: &str) -> Option<String> {
    let suffix = text.split_once(prefix)?.1;
    let tail: String = suffix
        .chars()
        .take_while(|c| !c.is_whitespace() && !"<>)]`\"".contains(*c))
        .collect();
    let tail = tail.trim_end_matches(['.', ',', ';']);
    if tail.is_empty() {
        return None;
    }
    Some(format!("{prefix}{tail}"))
}

fn claimants(issue: &Value, comments: &[Value]) -> Vec<RequestContributor> {
    let mut people: BTreeMap<String, RequestContributor> = issue["assignees"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(contributor)
        .map(|p| (p.login.to_lowercase(), p))
        .collect();
    // Only an explicit self-claim changes ownership. Mentioning somebody is not a claim.
    let mut ordered: Vec<_> = comments.iter().collect();
    ordered.sort_by_key(|c| c["created_at"].as_str().unwrap_or_default());
    for comment in ordered {
        let Some(person) = contributor(&comment["user"]) else {
            continue;
        };
        match comment["body"].as_str().unwrap_or_default().trim() {
            "/claim" => {
                people.insert(person.login.to_lowercase(), person);
            }
            "/unclaim" | "/release" => {
                let assigned = issue["assignees"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|a| {
                        a["login"]
                            .as_str()
                            .is_some_and(|s| s.eq_ignore_ascii_case(&person.login))
                    });
                if !assigned {
                    people.remove(&person.login.to_lowercase());
                }
            }
            _ => {}
        }
    }
    people.into_values().collect()
}

fn discussion(comments: &[Value]) -> Result<Vec<RequestComment>, String> {
    comments
        .iter()
        .map(|comment| {
            Ok(RequestComment {
                id: comment["id"]
                    .as_u64()
                    .filter(|id| *id > 0)
                    .ok_or("invalid request comment id")?,
                author: contributor(&comment["user"]).ok_or("request comment author missing")?,
                body: comment["body"].as_str().unwrap_or_default().into(),
                created_at: comment["created_at"]
                    .as_str()
                    .ok_or("request comment date missing")?
                    .parse()
                    .map_err(|_| "invalid request comment date")?,
                updated_at: comment["updated_at"]
                    .as_str()
                    .ok_or("request comment update missing")?
                    .parse()
                    .map_err(|_| "invalid request comment update")?,
            })
        })
        .collect()
}

fn refers_to_request(body: &str, number: u64) -> bool {
    [
        format!("https://github.com/{REQUEST_REPOSITORY}/issues/{number}"),
        format!("{REQUEST_REPOSITORY}#{number}"),
    ]
    .iter()
    .any(|reference| {
        body.match_indices(reference).any(|(at, _)| {
            !body[at + reference.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        })
    })
}

fn ranked(
    person: &RequestContributor,
    requester: &str,
    excluded: &BTreeSet<String>,
    eligible: &BTreeSet<String>,
) -> bool {
    eligible.contains(&person.login.to_lowercase())
        && !excluded.contains(&person.login.to_lowercase())
        && !person.login.eq_ignore_ascii_case(requester)
}

fn completion(
    pr: &Value,
    candidate: u64,
    number: u64,
    requester: &str,
    excluded: &BTreeSet<String>,
    eligible: &BTreeSet<String>,
) -> Option<RequestCompletion> {
    if pr["merged"] != true || !refers_to_request(pr["body"].as_str().unwrap_or_default(), number) {
        return None;
    }
    let person = contributor(&pr["user"])?;
    let revision = pr["merge_commit_sha"]
        .as_str()
        .filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))?;
    Some(RequestCompletion {
        url: format!("https://github.com/{LIBRARY}/pull/{candidate}"),
        revision: revision.into(),
        merged_at: pr["merged_at"].as_str()?.parse().ok()?,
        ranked: ranked(&person, requester, excluded, eligible),
        contributor: person,
    })
}

pub(crate) async fn fetch(
    previous: &SharedRequestsSnapshot,
    token: Option<String>,
    excluded: &BTreeSet<String>,
    eligible: &BTreeSet<String>,
) -> Result<SharedRequestsSnapshot, String> {
    let github = GitHub {
        client: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(45))
            .user_agent("Mathesis/1.0 (canonical formalization requests)")
            .build()
            .map_err(|_| "shared request client unavailable")?,
        token,
    };
    let issues = github
        .pages(&format!(
            "/repos/{REQUEST_REPOSITORY}/issues?state=all&sort=updated&direction=desc"
        ))
        .await?;
    let old: BTreeMap<_, _> = previous.requests.iter().map(|r| (r.number, r)).collect();
    let mut requests = vec![];
    let mut pulls = BTreeMap::<u64, Value>::new();
    for issue in issues {
        if issue.get("pull_request").is_some() {
            continue;
        }
        let title = issue["title"].as_str().ok_or("request title is missing")?;
        if !title.starts_with("[Formalization]") {
            continue;
        }
        let number = issue["number"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("invalid request number")?;
        let updated_at = issue["updated_at"]
            .as_str()
            .ok_or("request update time missing")?
            .parse()
            .map_err(|_| "invalid request update time")?;
        if let Some(saved) = old.get(&number)
            && saved.updated_at == updated_at
            // Comment edits need not change the issue timestamp. Always read
            // discussions again; only an issue with no comments can reuse them.
            && saved.comments.is_empty()
            && issue["comments"].as_u64().unwrap_or(0) == 0
            // A PR can merge after its issue was closed without updating the issue.
            // Recheck unresolved closures daily, staggered across the anonymous budget.
            && !(saved.state == "closed" && number % 24 == u64::from(Utc::now().hour()))
        {
            let mut saved = (*saved).clone();
            saved.body = issue["body"].as_str().unwrap_or_default().into();
            for done in &mut saved.completions {
                done.ranked = ranked(&done.contributor, &saved.requested_by, excluded, eligible);
            }
            requests.push(saved);
            continue;
        }
        let comments = if issue["comments"].as_u64().unwrap_or(0) > 0 {
            github
                .pages(&format!(
                    "/repos/{REQUEST_REPOSITORY}/issues/{number}/comments"
                ))
                .await?
        } else {
            vec![]
        };
        let body = issue["body"].as_str().unwrap_or_default();
        let mut candidates = references(body);
        for comment in &comments {
            candidates.extend(references(comment["body"].as_str().unwrap_or_default()));
        }
        let requested_by = contributor(&issue["user"])
            .ok_or("request author missing")?
            .login;
        let mut completions = vec![];
        if issue["state"] == "closed" && issue["state_reason"] != "not_planned" {
            for candidate in candidates {
                if let std::collections::btree_map::Entry::Vacant(entry) = pulls.entry(candidate) {
                    entry.insert(
                        github
                            .get(&format!("/repos/{LIBRARY}/pulls/{candidate}"))
                            .await?,
                    );
                }
                let pr = &pulls[&candidate];
                if let Some(done) =
                    completion(pr, candidate, number, &requested_by, excluded, eligible)
                {
                    completions.push(done);
                }
            }
        }
        let claimants = claimants(&issue, &comments);
        let state = if issue["state"] == "closed" {
            if completions.is_empty() {
                "closed"
            } else {
                "merged"
            }
        } else if claimants.is_empty() {
            "open"
        } else {
            "claimed"
        };
        requests.push(SharedFormalizationRequest {
            number,
            title: title.trim_start_matches("[Formalization]").trim().into(),
            body: body.into(),
            comments: discussion(&comments)?,
            url: format!("https://github.com/{REQUEST_REPOSITORY}/issues/{number}"),
            state: state.into(),
            updated_at,
            requested_by,
            claimants,
            completions,
            paper_repository: link(body, "https://github.com/mathesis-papers/")
                .map(|s| s.split("/blob/").next().unwrap_or(&s).to_owned()),
            lean_source: link(
                body,
                "https://github.com/the-omega-institute/trureturing/blob/",
            ),
        });
    }
    requests.sort_by_key(|r| std::cmp::Reverse(r.number));
    Ok(SharedRequestsSnapshot {
        revision: previous.revision + 1,
        observed_at: Some(Utc::now()),
        requests,
    })
}

pub(super) async fn refresh(
    app: &App,
    token: Option<String>,
    excluded: &BTreeSet<String>,
    eligible: &BTreeSet<String>,
) -> Result<(), String> {
    let previous = app
        .shared_formalization_requests()
        .await
        .map_err(|e| e.to_string())?;
    let snapshot = fetch(&previous, token, excluded, eligible).await?;
    app.replace_shared_formalization_requests(&snapshot, previous.revision)
        .await
        .map_err(|e| e.to_string())
}

/// Request synchronization cannot delay reviews, archive discovery or material publication.
pub(crate) async fn run(
    app: std::sync::Arc<App>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    token_file: Option<String>,
    excluded: BTreeSet<String>,
    eligible: BTreeSet<String>,
) {
    loop {
        let token = match &token_file {
            Some(path) => match tokio::fs::read_to_string(path).await {
                Ok(token) => Some(token.trim().to_owned()),
                Err(_) => {
                    tracing::warn!("shared request token file unavailable");
                    None
                }
            },
            None => None,
        };
        tokio::select! {
            result = refresh(&app, token, &excluded, &eligible) => match result {
                Ok(()) => tracing::info!("refreshed canonical formalization requests and contributions"),
                Err(error) => tracing::warn!(%error, "shared requests retain their previous snapshot"),
            },
            _ = shutdown.changed() => break,
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(3600)) => {},
            _ = shutdown.changed() => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn discussion_keeps_authorship_markdown_and_comment_edits() {
        let comment = json!({"id": 123, "user": {"login": "alice", "type": "User"},
            "body": "A **proof** of $x = x$", "created_at": "2026-11-24T00:00:00Z", "updated_at": "2026-11-24T01:00:00Z"});
        let saved = discussion(&[comment.clone()]).unwrap();
        assert_eq!(saved[0].author.login, "alice");
        assert_eq!(saved[0].body, "A **proof** of $x = x$");
        assert!(saved[0].updated_at > saved[0].created_at);
        let mut invalid = comment;
        invalid["user"]["login"] = json!("invalid/link");
        assert!(discussion(&[invalid]).is_err());
    }
    #[test]
    fn claims_are_self_authored_explicit_and_release_does_not_erase_an_assignee() {
        let person = |login| json!({"login":login,"type":"User"});
        let comment = |login, body, at| json!({"user":person(login),"body":body,"created_at":at});
        let issue = json!({"assignees":[person("assigned")]});
        let comments = vec![
            comment("alice", "/claim", "1"),
            comment("bob", "please assign @alice", "2"),
            comment("alice", "/release", "3"),
            comment("assigned", "/unclaim", "4"),
        ];
        assert_eq!(
            claimants(&issue, &comments)
                .iter()
                .map(|p| p.login.as_str())
                .collect::<Vec<_>>(),
            vec!["assigned"]
        );
    }
    #[test]
    fn proof_links_and_backlinks_are_exact_and_repository_scoped() {
        assert_eq!(
            references(
                "trureturing#2 unrelated#4 https://github.com/the-omega-institute/trureturing/pull/42; the-omega-institute/trureturing#31"
            ),
            BTreeSet::from([31, 42])
        );
        assert!(refers_to_request(
            "Fixes the-omega-institute/trureturing-requests#459.",
            459
        ));
        assert!(!refers_to_request(
            "Fixes the-omega-institute/trureturing-requests#4590.",
            459
        ));
        assert!(!refers_to_request("Closes #459", 459));
    }
    #[test]
    fn completion_requires_merged_exact_request_evidence_and_keeps_unranked_credit() {
        let mut pr = json!({"merged":true,"body":"Fixes the-omega-institute/trureturing-requests#459",
            "user":{"login":"external","type":"User"}, "merge_commit_sha":"a".repeat(40), "merged_at":"2026-11-23T00:00:00Z"});
        let excluded = BTreeSet::from(["platform".into()]);
        let eligible = BTreeSet::from([
            "external".into(),
            "author".into(),
            "platform".into(),
            "automation[bot]".into(),
        ]);
        assert!(
            completion(&pr, 32816, 459, "author", &excluded, &eligible)
                .unwrap()
                .ranked
        );
        assert!(completion(&pr, 32816, 4590, "author", &excluded, &eligible).is_none());
        assert!(
            !completion(&pr, 32816, 459, "author", &excluded, &BTreeSet::new())
                .unwrap()
                .ranked,
            "an unresolved author identity cannot earn competitive credit"
        );
        pr["merged"] = json!(false);
        assert!(completion(&pr, 32816, 459, "author", &excluded, &eligible).is_none());
        pr["merged"] = json!(true);
        for login in ["author", "platform"] {
            pr["user"]["login"] = json!(login);
            let done = completion(&pr, 32816, 459, "author", &excluded, &eligible).unwrap();
            assert!(!done.ranked);
            assert_eq!(done.contributor.login, login);
        }
        pr["user"]["login"] = json!("automation[bot]");
        assert!(
            completion(&pr, 32816, 459, "author", &excluded, &eligible)
                .unwrap()
                .ranked,
            "verified external people and agents follow the same credit rule"
        );
        pr["merge_commit_sha"] = json!("short");
        assert!(completion(&pr, 32816, 459, "author", &excluded, &eligible).is_none());
    }
}
