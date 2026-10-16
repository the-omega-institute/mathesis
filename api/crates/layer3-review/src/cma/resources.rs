//! Typed CMA resource observations; eligibility is decided by the caller's domain.
use super::{Transport, TransportReply};
use crate::{ReviewError, ReviewResult};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProfilePin {
    pub id: String,
    pub revision: u64,
}
#[derive(Debug, Deserialize)]
struct Workspace {
    sandbox_id: String,
    status: String,
}
#[derive(Debug, Deserialize)]
struct Sandbox {
    owner: String,
    status: String,
    profile: ProfilePin,
    // Current CMA exposes the immutable instance setting even when its internal
    // profile is not independently readable. Older servers omit this field.
    model_access: Option<String>,
}
#[derive(Debug, Deserialize)]
struct Profile {
    content: ProfileContent,
}
#[derive(Debug, Deserialize)]
struct ProfileContent {
    model_access: String,
}
#[derive(Debug)]
pub struct WorkspaceFacts {
    pub sandbox_id: String,
    pub sandbox_profile: ProfilePin,
    pub owner: String,
    pub workspace_status: String,
    pub sandbox_status: String,
    pub model_access: String,
}
fn decode<T: serde::de::DeserializeOwned>(reply: TransportReply) -> ReviewResult<T> {
    serde_json::from_value(reply.json()?)
        .map_err(|_| ReviewError::Output("invalid CMA resource response".into()))
}
fn safe_id(id: &str) -> ReviewResult<()> {
    if id.is_empty()
        || id.len() > 200
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
    {
        return Err(ReviewError::Output(
            "invalid CMA resource identifier".into(),
        ));
    }
    Ok(())
}
pub async fn workspace_facts(
    transport: &dyn Transport,
    workspace: &str,
) -> ReviewResult<WorkspaceFacts> {
    safe_id(workspace)?;
    let w: Workspace = decode(
        transport
            .request("GET", &format!("api/v1/workspaces/{workspace}"), None, None)
            .await?,
    )?;
    safe_id(&w.sandbox_id)?;
    let s: Sandbox = decode(
        transport
            .request(
                "GET",
                &format!("api/v1/sandboxes/{}", w.sandbox_id),
                None,
                None,
            )
            .await?,
    )?;
    safe_id(&s.profile.id)?;
    if s.profile.revision == 0 {
        return Err(ReviewError::Output("invalid CMA profile revision".into()));
    }
    let model_access = match s.model_access {
        Some(access) => access,
        None => {
            let p: Profile = decode(
                transport
                    .request(
                        "GET",
                        &format!(
                            "api/v1/sandbox-profiles/{}/revisions/{}",
                            s.profile.id, s.profile.revision
                        ),
                        None,
                        None,
                    )
                    .await?,
            )?;
            p.content.model_access
        }
    };
    Ok(WorkspaceFacts {
        sandbox_id: w.sandbox_id,
        sandbox_profile: s.profile,
        owner: s.owner,
        workspace_status: w.status,
        sandbox_status: s.status,
        model_access,
    })
}
