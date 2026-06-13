use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use crate::address::CompositionAddress;
use crate::error::Result;
use crate::farga::FargaReader;
use crate::resolver::resolve;
use crate::tools::ToolRegistry;
use crate::tree::DefinitionTree;
use crate::types::ResolvedAgent;
use crate::watcher::{WatchHandle, watch};

pub struct Fondament {
    tree: Arc<RwLock<DefinitionTree>>,
    farga: Arc<dyn FargaReader>,
    org: String,
    definitions_path: PathBuf,
}

pub struct WatchedFondament {
    pub fondament: Fondament,
    pub handle: WatchHandle,
}

impl Fondament {
    pub fn load(definitions_path: &Path, farga: Arc<dyn FargaReader>, org: String) -> Result<Self> {
        let tree = DefinitionTree::load(definitions_path)?;
        Ok(Self {
            tree: Arc::new(RwLock::new(tree)),
            farga,
            org,
            definitions_path: definitions_path.to_path_buf(),
        })
    }

    pub fn watch(self) -> Result<WatchedFondament> {
        let handle = watch(&self.definitions_path, Arc::clone(&self.tree))
            .map_err(|e| crate::error::FondamentError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            ))?;
        Ok(WatchedFondament { fondament: self, handle })
    }

    pub async fn resolve(&self, address: &CompositionAddress) -> Result<ResolvedAgent> {
        let tree = self.tree.read().unwrap().clone();
        resolve(address, &tree, self.farga.as_ref(), &self.org).await
    }

    pub fn tool_registry(&self) -> ToolRegistry {
        let tree = self.tree.read().unwrap();
        let mut registry = ToolRegistry::default();
        for def in tree.all() {
            for tool in &def.tools.always_on {
                registry.register(tool.clone());
            }
            for tool in &def.tools.jit {
                registry.register(tool.clone());
            }
        }
        registry
    }
}
