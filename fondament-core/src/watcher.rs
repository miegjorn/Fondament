use notify::{Event, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use std::path::Path;
use std::sync::{Arc, RwLock};
use crate::lint::fast::run_fast;
use crate::tree::DefinitionTree;

pub struct WatchHandle {
    _watcher: RecommendedWatcher,
}

pub fn watch(
    root: &Path,
    tree: Arc<RwLock<DefinitionTree>>,
) -> NotifyResult<WatchHandle> {
    let root = root.to_path_buf();
    let mut watcher = notify::recommended_watcher(move |res: NotifyResult<Event>| {
        if let Ok(event) = res {
            for path in event.paths {
                if path.extension().map_or(false, |e| e == "yaml") {
                    let mut t = tree.write().unwrap();
                    match t.reload_file(&path) {
                        Ok(_) => {
                            let results = run_fast(&t);
                            let failures: Vec<_> = results.iter()
                                .filter(|r| matches!(r, crate::lint::fast::LintResult::Fail { .. }))
                                .collect();
                            if failures.is_empty() {
                                tracing::info!("hot-reload: {} reloaded OK", path.display());
                            } else {
                                tracing::warn!("hot-reload: lint failed for {}, keeping previous tree", path.display());
                            }
                        }
                        Err(e) => tracing::error!("hot-reload error: {}", e),
                    }
                }
            }
        }
    })?;
    watcher.watch(&root, RecursiveMode::Recursive)?;
    Ok(WatchHandle { _watcher: watcher })
}
