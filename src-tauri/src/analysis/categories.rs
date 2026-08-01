use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tools that carry no category of their own.
pub const OTHER: &str = "other";

/// Assistant responses that called no tool at all. Not a tool category — a
/// residual, and named so it shows up in totals instead of quietly vanishing.
pub const REASONING: &str = "reasoning";

/// Category to tool patterns, deliberately in that direction: a tool can belong
/// to more than one category (Task is both execution and delegation), and the
/// reverse mapping cannot express that.
///
/// A pattern ending in `*` matches by prefix, anything else matches exactly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct CategoryMap(pub BTreeMap<String, Vec<String>>);

impl Default for CategoryMap {
    fn default() -> Self {
        let entries: &[(&str, &[&str])] = &[
            (
                "exploration",
                &["Read", "Glob", "Grep", "NotebookRead", "ToolSearch"],
            ),
            (
                "code_change",
                &["Edit", "MultiEdit", "Write", "NotebookEdit"],
            ),
            (
                "execution",
                &[
                    "Bash",
                    "PowerShell",
                    "BashOutput",
                    "KillShell",
                    "Task",
                    "Agent",
                ],
            ),
            ("delegation", &["Task", "Agent", "SendMessage"]),
            ("research", &["WebFetch", "WebSearch"]),
            (
                "planning",
                &[
                    "TodoWrite",
                    "TaskCreate",
                    "TaskUpdate",
                    "ExitPlanMode",
                    "EnterPlanMode",
                    "AskUserQuestion",
                ],
            ),
            ("mcp", &["mcp__*"]),
        ];

        Self(
            entries
                .iter()
                .map(|(name, tools)| {
                    (
                        (*name).to_owned(),
                        tools.iter().map(|t| (*t).to_owned()).collect(),
                    )
                })
                .collect(),
        )
    }
}

impl CategoryMap {
    /// Every category a tool belongs to, or `["other"]` when none claims it.
    pub fn categories_for(&self, tool: &str) -> Vec<&str> {
        let mut found: Vec<&str> = self
            .0
            .iter()
            .filter(|(_, patterns)| patterns.iter().any(|p| matches(p, tool)))
            .map(|(name, _)| name.as_str())
            .collect();

        if found.is_empty() {
            found.push(OTHER);
        }
        found
    }

    pub fn names(&self) -> Vec<&str> {
        self.0.keys().map(String::as_str).collect()
    }
}

fn matches(pattern: &str, tool: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => tool.starts_with(prefix),
        None => pattern == tool,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_patterns_match_exactly() {
        let map = CategoryMap::default();
        assert_eq!(map.categories_for("Read"), vec!["exploration"]);
        assert_eq!(map.categories_for("Reader"), vec![OTHER]);
    }

    #[test]
    fn trailing_star_matches_by_prefix() {
        let map = CategoryMap::default();
        assert_eq!(map.categories_for("mcp__github__create_issue"), vec!["mcp"]);
    }

    #[test]
    fn a_tool_can_belong_to_several_categories() {
        // Task both runs something and hands work to someone else; forcing it
        // into one bucket would misreport whichever question is being asked.
        let map = CategoryMap::default();
        let mut found = map.categories_for("Task");
        found.sort_unstable();
        assert_eq!(found, vec!["delegation", "execution"]);
    }

    #[test]
    fn an_unknown_tool_falls_back_to_other() {
        let map = CategoryMap::default();
        assert_eq!(map.categories_for("SomeFutureTool"), vec![OTHER]);
    }
}
