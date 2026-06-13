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
    if let CompositionAddress::Composed { project, .. } = address {
        if let Ok(proj_ctx) = farga.project_layer(project).await {
            if !proj_ctx.content.is_empty() {
                layers.push(format!("## Project Context\n{}", proj_ctx.content));
            }
        }
    }

    // Layer 4+: Fondament definition layers
    let role_id = match address {
        CompositionAddress::Role { role, .. } => role.clone(),
        CompositionAddress::Composed { project, facet, stance } => {
            format!("roles/{}-{}", facet.as_deref().unwrap_or(project), stance)
        }
    };

    // Walk extends chain
    let mut to_visit = vec![role_id.clone()];
    let mut visited = std::collections::HashSet::new();

    while let Some(id) = to_visit.pop() {
        if visited.contains(&id) {
            return Err(FondamentError::CircularExtends(id));
        }
        visited.insert(id.clone());

        if let Some(def) = tree.get(&id) {
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

    // Layer: stance override
    if let CompositionAddress::Role { stance_override: Some(stance), .. } = address {
        if let Some(stance_def) = tree.get(&format!("stances/{}", stance)) {
            if let Some(ctx) = &stance_def.context {
                layers.push(ctx.clone());
            }
        }
    }

    Ok(ResolvedAgent {
        system_prompt: layers.join("\n\n"),
        tools: always_on,
        jit_tools,
        default_model,
    })
}
