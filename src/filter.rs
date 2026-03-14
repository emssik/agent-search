use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};

/// Path filter based on include/exclude glob patterns.
#[derive(Debug)]
pub struct PathFilter {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
}

impl Default for PathFilter {
    fn default() -> Self {
        Self {
            include: None,
            exclude: None,
        }
    }
}

impl PathFilter {
    pub fn new(include: &[String], exclude: &[String]) -> Result<Self> {
        Ok(Self {
            include: build_glob_set(include)?,
            exclude: build_glob_set(exclude)?,
        })
    }

    /// Returns true if the path passes both include and exclude filters.
    pub fn matches(&self, path: &str) -> bool {
        if let Some(ref inc) = self.include {
            if !inc.is_match(path) {
                return false;
            }
        }
        if let Some(ref exc) = self.exclude {
            if exc.is_match(path) {
                return false;
            }
        }
        true
    }
}

fn build_glob_set(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(Some(builder.build()?))
}
