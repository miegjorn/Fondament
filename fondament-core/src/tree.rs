use std::collections::HashMap;
use std::path::Path;
use crate::definition::DefinitionFile;
use crate::error::Result;

#[derive(Debug, Default, Clone)]
pub struct DefinitionTree {
    definitions: HashMap<String, DefinitionFile>,
}

impl DefinitionTree {
    pub fn load(root: &Path) -> Result<Self> {
        let mut tree = Self::default();
        tree.load_dir(root)?;
        Ok(tree)
    }

    fn load_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() { return Ok(()); }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.load_dir(&path)?;
            } else if path.extension().map_or(false, |e| e == "yaml") {
                let content = std::fs::read_to_string(&path)?;
                let def: DefinitionFile = serde_yaml::from_str(&content)?;
                self.definitions.insert(def.id.clone(), def);
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&DefinitionFile> {
        self.definitions.get(id)
    }

    pub fn all(&self) -> impl Iterator<Item = &DefinitionFile> {
        self.definitions.values()
    }

    pub fn reload_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let def: DefinitionFile = serde_yaml::from_str(&content)?;
        self.definitions.insert(def.id.clone(), def);
        Ok(())
    }
}
