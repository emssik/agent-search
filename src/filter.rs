use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};

/// Path filter based on include/exclude glob patterns.
#[derive(Debug)]
pub struct PathFilter {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
}

impl Default for PathFilter {
    fn default() -> Self {
        Self {
            include: None,
            exclude: None,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
        }
    }
}

impl PathFilter {
    pub fn new(include: &[String], exclude: &[String]) -> Result<Self> {
        Ok(Self {
            include: build_glob_set(include)?,
            exclude: build_glob_set(exclude)?,
            include_patterns: include.to_vec(),
            exclude_patterns: exclude.to_vec(),
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

    /// Returns the original include glob patterns.
    pub fn include_patterns(&self) -> &[String] {
        &self.include_patterns
    }

    /// Returns the original exclude glob patterns.
    pub fn exclude_patterns(&self) -> &[String] {
        &self.exclude_patterns
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
