//! Anonymous, bounded transport only. Admission and replacement belong to Layer 2.
use std::time::Duration;

use mathesis_core::model::MATERIALS_INDEX_MAX_BYTES;

pub(super) async fn fetch_materials_index(raw: &str) -> Result<Vec<u8>, String> {
    let url = url::Url::parse(raw).map_err(|_| "materials index URL is invalid")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "materials index URL requires HTTPS without credentials, query or fragment".into(),
        );
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .user_agent("Mathesis/1.0 (materials index)")
        .build()
        .map_err(|_| "materials index client unavailable")?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| "materials index could not be reached")?;
    if !response.status().is_success() {
        return Err(format!(
            "materials index returned HTTP {}",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|n| n > MATERIALS_INDEX_MAX_BYTES as u64)
    {
        return Err("materials index exceeds its size bound".into());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "materials index download interrupted")?
    {
        if body.len().saturating_add(chunk.len()) > MATERIALS_INDEX_MAX_BYTES {
            return Err("materials index exceeds its size bound".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn refuses_unsafe_index_urls_before_transport() {
        for url in [
            "http://example.test/index.json",
            "https://user@example.test/index.json",
            "https://user:pass@example.test/index.json",
            "https://example.test/index.json?q=1",
            "https://example.test/index.json#x",
            "not a URL",
        ] {
            assert!(fetch_materials_index(url).await.is_err(), "{url}");
        }
    }
}
