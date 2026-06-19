use crate::address::CompositionAddress;
use crate::error::{FondamentError, Result};
use crate::farga::FargaReader;
use crate::tools::ToolDefinition;
use crate::tree::DefinitionTree;
use crate::types::{ModelId, ResolvedAgent};

pub async fn resolve(
    address: &CompositionAddress,
    tree: &DefinitionTree,
    farga: &dyn FargaReader,
    org: &str,
) -> Result<ResolvedAgent> {
    let mut layers: Vec<String> = Vec::new();
    let mut default_model = ModelId::default();
    let mut always_on: Vec<ToolDefinition> = Vec::new();
    let mut jit_tools: Vec<ToolDefinition> = Vec::new();
    let mut collected_parts: Vec<(String, String)> = Vec::new();

    let is_deconstructive = match address {
        CompositionAddress::Role { modifiers, .. } => modifiers.iter().any(|m| m == "deconstructive"),
        CompositionAddress::Composed { modifiers, .. } => modifiers.iter().any(|m| m == "deconstructive"),
    };

    // Layer 1: org context from Farga
    if let Ok(org_ctx) = farga.org_layer(org).await {
        if !org_ctx.content.is_empty() {
            layers.push(format!("## Organization Context\n{}", org_ctx.content));
        }
    }

    // Layer 2: initiative context from Farga
    if let Ok(initiatives) = farga.initiative_layer(org).await {
        for init in initiatives {
            if !init.content.is_empty() {
                layers.push(format!("## Strategic Initiative\n{}", init.content));
            }
        }
    }

    // Layer 3: project context (for Composed addresses)
    if let CompositionAddress::Composed { project, facet, .. } = address {
        if let Ok(proj_ctx) = farga.project_layer(project).await {
            if !proj_ctx.content.is_empty() {
                layers.push(format!("## Project Context\n{}", proj_ctx.content));
                let domain_name = facet.as_deref().unwrap_or(project.as_str()).to_string();
                collected_parts.push(("domain".into(), domain_name));
            }
        }
    }

    // Layer 4+: Fondament definition layers — walk extends chain
    let role_id = match address {
        CompositionAddress::Role { role, .. } => role.clone(),
        CompositionAddress::Composed { project, facet, stance, .. } => {
            format!("roles/{}-{}", facet.as_deref().unwrap_or(project), stance)
        }
    };

    let mut to_visit = vec![role_id];
    let mut visited = std::collections::HashSet::new();

    while let Some(id) = to_visit.pop() {
        if visited.contains(&id) {
            return Err(FondamentError::CircularExtends(id));
        }
        visited.insert(id.clone());

        if let Some(def) = tree.get(&id) {
            if def.kind == "discipline" && !def.modifier {
                let part_name = id.strip_prefix("disciplines/").unwrap_or(&id).to_string();
                collected_parts.push(("discipline".into(), part_name));
            }

            if let Some(ctx) = &def.context {
                if !ctx.is_empty() {
                    layers.push(ctx.clone());
                }
            }
            if let Some(model) = &def.default_model {
                default_model = model.clone();
            }
            always_on.extend(def.tools.always_on.clone());
            jit_tools.extend(def.tools.jit.clone());

            for parent in def.extends.iter().rev() {
                to_visit.push(parent.clone());
            }
        }
    }

    // Layer: stance (Role stance_override or Composed stance)
    let stance = match address {
        CompositionAddress::Role { stance_override: Some(s), .. } => Some(s.clone()),
        CompositionAddress::Composed { stance, .. } => Some(stance.clone()),
        _ => None,
    };
    if let Some(ref s) = stance {
        if let Some(stance_def) = tree.get(&format!("stances/{}", s)) {
            if let Some(ctx) = &stance_def.context {
                if !ctx.is_empty() {
                    layers.push(ctx.clone());
                    collected_parts.push(("stance".into(), s.clone()));
                }
            }
        }
    }

    // Deconstructive preamble: inject as layer[0] when modifier is active
    let thinking_budget = if is_deconstructive {
        let preamble = build_deconstructive_preamble(&collected_parts);
        layers.insert(0, preamble);
        let budget = (collected_parts.len() as u32 * 3_000).clamp(3_000, 10_000);
        Some(budget)
    } else {
        None
    };

    Ok(ResolvedAgent {
        system_prompt: layers.join("\n\n"),
        tools: always_on,
        jit_tools,
        default_model,
        thinking_budget,
    })
}

fn build_deconstructive_preamble(parts: &[(String, String)]) -> String {
    let mut preamble = String::from(
        "--- injected by deconstructive discipline ---\nYou are composed of the following parts:\n"
    );
    if parts.is_empty() {
        preamble.push_str("  - [role: this agent] — reason from your full context\n");
    } else {
        for (kind, name) in parts {
            preamble.push_str(&format!("  - [{}: {}]\n", kind, name));
        }
    }
    preamble.push_str(
        "\nBefore producing any response:\n\
         1. Become each part sequentially. Reason from its corpus alone.\n\
         2. Name the tensions between parts explicitly.\n\
         3. If a gap surfaces that no part of you owns, output it typed:\n\
            GAP { domain: \"...\", question: \"...\", blocking: true/false }\n\
         4. Recompose. Collapse to your public response from that synthesis.\n\
         \n\
         Your public response reflects the recomposed whole.\n\
         The internal debate is yours alone — it does not appear in output.\n\
         --- end injection ---"
    );
    preamble
}
