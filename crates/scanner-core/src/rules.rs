//! Known-reclaimable rules database.
//!
//! Rules are glob patterns over lowercased, forward-slash paths (see
//! `classify::normalize`). The built-in set is embedded at compile time from
//! `rules/reclaimable.json`; a user-editable overlay file can be layered on
//! later without code changes.

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleAction {
    /// Contents can be deleted outright; the owner regenerates them.
    Safe,
    /// Reclaimable, but a human should confirm (project caches, VM disks).
    Review,
    /// Don't touch directly — deep-link the built-in tool (Disk Cleanup,
    /// Storage Sense, docker prune) that manages it.
    SystemTool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Globs matched against normalized paths. `**` spans separators.
    pub patterns: Vec<String>,
    pub action: RuleAction,
}

pub struct RuleSet {
    rules: Vec<Rule>,
    globs: GlobSet,
    /// glob index -> rule index
    glob_to_rule: Vec<usize>,
}

impl RuleSet {
    pub fn builtin() -> Self {
        let json = include_str!("../rules/reclaimable.json");
        Self::from_json(json).expect("built-in rules database must parse")
    }

    /// Name-based heuristics ("looks like a cache/temp/log folder"), applied
    /// only after every precise classification has failed.
    pub fn builtin_fallback() -> Self {
        let json = include_str!("../rules/fallback.json");
        Self::from_json(json).expect("built-in fallback rules must parse")
    }

    pub fn from_json(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let rules: Vec<Rule> = serde_json::from_str(json)?;
        let mut builder = GlobSetBuilder::new();
        let mut glob_to_rule = Vec::new();
        for (rule_index, rule) in rules.iter().enumerate() {
            for pattern in &rule.patterns {
                builder.add(
                    GlobBuilder::new(pattern)
                        .literal_separator(false)
                        .backslash_escape(false)
                        .build()?,
                );
                glob_to_rule.push(rule_index);
            }
        }
        Ok(Self {
            rules,
            globs: builder.build()?,
            glob_to_rule,
        })
    }

    /// `normalized` must already be lowercased with forward slashes.
    pub fn match_path(&self, normalized: &str) -> Option<&Rule> {
        let matches = self.globs.matches(normalized);
        let glob_index = matches.first()?;
        Some(&self.rules[self.glob_to_rule[*glob_index]])
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse_and_match() {
        let set = RuleSet::builtin();
        assert!(!set.rules().is_empty());
        assert!(set.match_path("c:/users/richa/appdata/local/temp").is_some());
        assert!(set.match_path("c:/users/richa/documents").is_none());
    }
}
