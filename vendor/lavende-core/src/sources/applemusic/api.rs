use crate::sources::applemusic::token::AppleMusicTokenTracker;
use futures::future::join_all;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, warn};

pub const API_BASE: &str = "https://api.music.apple.com/v1";

pub async fn api_request(
    client: &reqwest::Client,
    token_tracker: &AppleMusicTokenTracker,
    path: &str,
) -> Option<Value> {
    let token = token_tracker.get_token().await?;
    let origin = token_tracker.get_origin().await;
    let url = if path.starts_with("http") {
        path.to_owned()
    } else {
        format!("{}{}", API_BASE, path)
    };
    let mut req = client.get(&url).bearer_auth(token);
    if let Some(o) = origin {
        req = req.header("Origin", format!("https://{}", o));
    }
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Apple Music API request failed: {}", e);
            return None;
        }
    };
    if !resp.status().is_success() {
        warn!("Apple Music API returned {} for {}", resp.status(), url);
        return None;
    }
    resp.json().await.ok()
}

pub async fn fetch_paginated_tracks(
    client: &reqwest::Client,
    token_tracker: &Arc<AppleMusicTokenTracker>,
    next_url: Option<String>,
    load_limit: usize,
    concurrency: usize,
) -> Vec<Value> {
    let initial_next = match next_url {
        Some(u) => u,
        None => return Vec::new(),
    };
    if initial_next.contains("offset=") {
        let base_url = initial_next
            .split("offset=")
            .next()
            .unwrap_or(&initial_next)
            .to_owned();
        let offset: usize = initial_next
            .split("offset=")
            .nth(1)
            .and_then(|s| s.split('&').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);
        let mut all_items = Vec::new();
        let mut current_offset = offset;
        let mut limit_reached = false;
        let mut pages_fetched = 1;
        while !limit_reached {
            let mut futs = Vec::new();
            let semaphore = Arc::new(Semaphore::new(concurrency));
            for _ in 0..concurrency {
                if load_limit > 0 && pages_fetched >= load_limit {
                    limit_reached = true;
                    break;
                }
                let url = format!("{}offset={}", base_url, current_offset);
                let sem = semaphore.clone();
                let c = client.clone();
                let tt = token_tracker.clone();
                futs.push(async move {
                    let _permit = sem.acquire().await.ok();
                    api_request(&c, &tt, &url).await
                });
                current_offset += 100;
                pages_fetched += 1;
            }
            if futs.is_empty() {
                break;
            }
            let results = join_all(futs).await;
            let mut added_on_this_step = 0;
            for res in results {
                if let Some(data) = res {
                    if let Some(items) = data.get("data").and_then(|v| v.as_array()) {
                        all_items.extend(items.iter().cloned());
                        added_on_this_step += items.len();
                        if items.len() < 100 {
                            limit_reached = true;
                        }
                    } else {
                        limit_reached = true;
                    }
                } else {
                    limit_reached = true;
                }
            }
            if added_on_this_step == 0 {
                break;
            }
        }
        return all_items;
    }
    let mut next = Some(initial_next);
    let mut all_items = Vec::new();
    let mut pages_fetched = 1;
    while let Some(url) = next {
        if load_limit > 0 && pages_fetched >= load_limit {
            break;
        }
        let data = match api_request(client, token_tracker, &url).await {
            Some(d) => d,
            None => break,
        };
        if let Some(items) = data.get("data").and_then(|v| v.as_array()) {
            all_items.extend(items.iter().cloned());
        }
        next = data
            .get("next")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        pages_fetched += 1;
    }
    all_items
}
