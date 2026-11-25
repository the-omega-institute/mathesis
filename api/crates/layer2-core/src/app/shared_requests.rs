use super::App;
use crate::{CoreError, CoreResult, model::*};

fn valid_login(login: &str) -> bool {
    !login.is_empty()
        && login.len() <= 100
        && login
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-[]".contains(&b))
}
fn valid_person(person: &RequestContributor) -> bool {
    valid_login(&person.login)
        && person.url == format!("https://github.com/{}", person.login)
        && (!person.login.ends_with("[bot]") || person.agent)
}
fn safe_link(value: &str, prefix: &str) -> bool {
    value.len() <= 2048
        && value
            .strip_prefix(prefix)
            .is_some_and(|tail| !tail.is_empty())
        && !value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "?<>\"".contains(c))
}

impl App {
    pub async fn shared_formalization_requests(&self) -> CoreResult<SharedRequestsSnapshot> {
        self.ports.materials.load_shared_requests().await
    }

    pub async fn replace_shared_formalization_requests(
        &self,
        snapshot: &SharedRequestsSnapshot,
        expected: u64,
    ) -> CoreResult<()> {
        if snapshot.requests.len() > 10_000
            || snapshot
                .observed_at
                .is_none_or(|at| at > self.ports.clock.now() + chrono::Duration::minutes(5))
        {
            return Err(CoreError::invalid("invalid shared request snapshot"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for request in &snapshot.requests {
            if request.number == 0
                || !seen.insert(request.number)
                || request.url
                    != format!(
                        "https://github.com/{REQUEST_REPOSITORY}/issues/{}",
                        request.number
                    )
                || !["open", "claimed", "merged", "closed"].contains(&request.state.as_str())
                || request.title.trim().is_empty()
                || request.title.len() > 1024
                || request.body.len() > 262_144
                || request.comments.len() > 10_000
                || !valid_login(&request.requested_by)
                || request.claimants.len() > 100
                || request.completions.len() > 100
                || request.claimants.iter().any(|p| !valid_person(p))
                || (request.state == "merged") == request.completions.is_empty()
                || (request.state == "claimed" && request.claimants.is_empty())
                || (request.state == "open" && !request.claimants.is_empty())
                || request
                    .paper_repository
                    .as_ref()
                    .is_some_and(|url| !safe_link(url, "https://github.com/mathesis-papers/"))
                || request.lean_source.as_ref().is_some_and(|url| {
                    !safe_link(
                        url,
                        "https://github.com/the-omega-institute/trureturing/blob/",
                    )
                })
            {
                return Err(CoreError::invalid(
                    "invalid canonical formalization request",
                ));
            }
            let mut comment_ids = std::collections::BTreeSet::new();
            for comment in &request.comments {
                if comment.id == 0
                    || !comment_ids.insert(comment.id)
                    || !valid_person(&comment.author)
                    || comment.body.len() > 262_144
                    || comment.updated_at < comment.created_at
                {
                    return Err(CoreError::invalid("invalid shared request comment"));
                }
            }
            for completion in &request.completions {
                let number = completion
                    .url
                    .strip_prefix("https://github.com/the-omega-institute/trureturing/pull/");
                if !number.is_some_and(|n| n.parse::<u64>().is_ok_and(|n| n > 0))
                    || !valid_person(&completion.contributor)
                    || (completion.ranked
                        && (completion
                            .contributor
                            .login
                            .eq_ignore_ascii_case(&request.requested_by)))
                    || snapshot
                        .observed_at
                        .is_some_and(|at| completion.merged_at > at)
                    || completion.revision.len() != 40
                    || !completion.revision.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err(CoreError::invalid("invalid request completion evidence"));
                }
            }
        }
        self.ports
            .materials
            .replace_shared_requests(snapshot, expected)
            .await
    }
}
