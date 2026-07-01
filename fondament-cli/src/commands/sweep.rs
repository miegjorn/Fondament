use anyhow::anyhow;

// ── Public entry point ────────────────────────────────────────────────────────

pub async fn run(defs: &std::path::Path, path_filter: Option<&str>) -> anyhow::Result<()> {
    let tree = fondament_core::tree::DefinitionTree::load(defs)?;

    let entries: Vec<&fondament_core::definition::DefinitionFile> = tree
        .all()
        .filter(|def| {
            if let Some(filter) = path_filter {
                def.id.starts_with(filter)
            } else {
                true
            }
        })
        .collect();

    let mut results: Vec<(String, AssessResult)> = Vec::new();

    for def in &entries {
        let context = match &def.context {
            Some(ctx) if !ctx.trim().is_empty() => ctx.as_str(),
            _ => continue,
        };

        let result = assess_definition(&def.id, &def.kind, context).await?;
        results.push((def.id.clone(), result));
    }

    for (id, result) in &results {
        let prefix = verdict_prefix(&result.verdict);
        if result.verdict == "invalid" {
            eprintln!("{} {} — {}", prefix, id, result.reason);
        } else if result.verdict == "warning" {
            println!("{} {} — {}", prefix, id, result.reason);
        } else {
            println!("{} {}", prefix, id);
        }
    }

    let invalid_count = count_invalids(&results);
    if invalid_count > 0 {
        Err(anyhow!("{} definition(s) failed semantic lint", invalid_count))
    } else {
        Ok(())
    }
}

// ── Verdict helpers ───────────────────────────────────────────────────────────

pub fn verdict_prefix(verdict: &str) -> &'static str {
    match verdict {
        "valid" => "✓",
        "warning" => "⚠",
        _ => "✗",
    }
}

pub fn count_invalids(results: &[(String, AssessResult)]) -> usize {
    results.iter().filter(|(_, r)| r.verdict == "invalid").count()
}

// ── API types & call ──────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct AssessResult {
    pub verdict: String, // "valid" | "warning" | "invalid"
    pub reason: String,
}

async fn assess_definition(id: &str, kind: &str, context: &str) -> anyhow::Result<AssessResult> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
    let xai_key = std::env::var("XAI_API_KEY").ok();

    let model = if xai_key.is_some() { "grok-3" } else { "claude-sonnet-4-6" };

    let prompt = format!(
        "You are reviewing an agent definition file for semantic consistency.\n\n\
         Kind: {kind}\n\
         ID: {id}\n\
         Context:\n{context}\n\n\
         Does this context actually match what is claimed? A \"{kind}\" definition with id \"{id}\" should focus on that exact topic.\n\n\
         Respond ONLY with a JSON object, no markdown:\n\
         {{\"verdict\": \"valid\"|\"warning\"|\"invalid\", \"reason\": \"one sentence\"}}\n\
         - valid: context clearly matches the declared kind and id\n\
         - warning: context is related but has drift or gaps from the claimed focus\n\
         - invalid: context clearly doesn't match the kind/id claim"
    );

    let client = reqwest::Client::new();
    let resp = if model.starts_with("grok") || model.starts_with("xai") {
        let key = xai_key.ok_or_else(|| anyhow::anyhow!("XAI_API_KEY not set"))?;
        client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "max_tokens": 256,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await?
            .error_for_status()?
    } else {
        let key = api_key.ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;
        client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "max_tokens": 256,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await?
            .error_for_status()?
    };

    let json: serde_json::Value = resp.json().await?;
    let text = if model.starts_with("grok") || model.starts_with("xai") {
        json["choices"][0]["message"]["content"].as_str()
            .ok_or_else(|| anyhow::anyhow!("empty response from Grok"))?
    } else {
        json["content"]
            .as_array()
            .and_then(|blocks| blocks.iter().find(|b| b["type"].as_str() == Some("text")))
            .and_then(|b| b["text"].as_str())
            .ok_or_else(|| anyhow::anyhow!("empty response from Claude"))?
    };

    let result: AssessResult = serde_json::from_str(text.trim())
        .map_err(|e| anyhow::anyhow!("could not parse assessment JSON '{}': {}", text, e))?;
    Ok(result)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_prefix_valid() {
        assert_eq!(verdict_prefix("valid"), "✓");
    }

    #[test]
    fn verdict_prefix_warning() {
        assert_eq!(verdict_prefix("warning"), "⚠");
    }

    #[test]
    fn verdict_prefix_invalid() {
        assert_eq!(verdict_prefix("invalid"), "✗");
    }

    #[test]
    fn count_invalids_returns_correct_count() {
        let results = vec![
            (
                "disciplines/a".to_string(),
                AssessResult {
                    verdict: "valid".into(),
                    reason: "".into(),
                },
            ),
            (
                "disciplines/b".to_string(),
                AssessResult {
                    verdict: "invalid".into(),
                    reason: "wrong".into(),
                },
            ),
            (
                "disciplines/c".to_string(),
                AssessResult {
                    verdict: "warning".into(),
                    reason: "drift".into(),
                },
            ),
        ];
        assert_eq!(count_invalids(&results), 1);
    }
}
