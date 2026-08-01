use super::categories::CategoryMap;
use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

/// What a session was mostly spent doing, derived from the tools it called.
///
/// A label alone would be misleading — most sessions do several things — so the
/// share per category travels with it and the UI shows both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Activity {
    Coding,
    Debugging,
    Exploration,
    Research,
    Planning,
    Delegation,
    Ops,
    Conversation,
}

impl Activity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Coding => "coding",
            Self::Debugging => "debugging",
            Self::Exploration => "exploration",
            Self::Research => "research",
            Self::Planning => "planning",
            Self::Delegation => "delegation",
            Self::Ops => "ops",
            Self::Conversation => "conversation",
        }
    }
}

/// Share of a session's tool calls per category. A call counted in two
/// categories contributes to both, so the shares can sum above 1.
pub type Profile = HashMap<String, f64>;

pub fn profile_from_counts(counts: &[(String, i64)], map: &CategoryMap) -> Profile {
    let total: i64 = counts.iter().map(|(_, calls)| *calls).sum();
    if total == 0 {
        return Profile::new();
    }

    let mut profile = Profile::new();
    for (tool, calls) in counts {
        for category in map.categories_for(tool) {
            *profile.entry(category.to_owned()).or_insert(0.0) += *calls as f64 / total as f64;
        }
    }
    profile
}

/// Picks the label from the shares.
///
/// The order matters: editing interleaved with running things is debugging, not
/// coding, and that distinction is the whole reason the label is derived rather
/// than just reporting the largest bucket.
pub fn classify(profile: &Profile) -> Activity {
    if profile.is_empty() {
        return Activity::Conversation;
    }

    let share = |name: &str| profile.get(name).copied().unwrap_or(0.0);

    let code = share("code_change");
    let exec = share("execution");

    if code >= 0.15 && exec >= 0.15 {
        return Activity::Debugging;
    }
    if code >= 0.15 {
        return Activity::Coding;
    }

    let ranked = [
        ("research", Activity::Research),
        ("planning", Activity::Planning),
        ("delegation", Activity::Delegation),
        ("exploration", Activity::Exploration),
        ("execution", Activity::Ops),
    ];

    ranked
        .iter()
        .max_by(|a, b| {
            share(a.0)
                .partial_cmp(&share(b.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .filter(|(name, _)| share(name) > 0.0)
        .map(|(_, activity)| *activity)
        .unwrap_or(Activity::Conversation)
}

/// Rebuilds the derived activity for every session.
///
/// Cheap enough to run whenever the mapping changes, which is what makes
/// "editing the mapping applies immediately" true without a rescan: the counts
/// it reads are raw tool names and never encode a mapping themselves.
pub fn recompute(conn: &Connection, map: &CategoryMap) -> Result<usize> {
    let mut per_session: HashMap<String, Vec<(String, i64)>> = HashMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT session_id, tool_name, calls FROM session_tool_counts")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (session_id, tool, calls) = row?;
            per_session.entry(session_id).or_default().push((tool, calls));
        }
    }

    // Sessions that called no tool at all still need a row, or they would
    // disappear from a filter on activity.
    let mut all_sessions: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT session_id FROM sessions")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            all_sessions.push(row?);
        }
    }

    conn.execute("DELETE FROM session_activity", [])?;
    let mut insert = conn.prepare(
        "INSERT INTO session_activity (session_id, activity, profile_json) VALUES (?1, ?2, ?3)",
    )?;

    let empty = Vec::new();
    for session_id in &all_sessions {
        let counts = per_session.get(session_id).unwrap_or(&empty);
        let profile = profile_from_counts(counts, map);
        let activity = classify(&profile);
        insert.execute((
            session_id,
            activity.as_str(),
            serde_json::to_string(&profile)?,
        ))?;
    }

    Ok(all_sessions.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(pairs: &[(&str, i64)]) -> Vec<(String, i64)> {
        pairs.iter().map(|(t, c)| ((*t).to_owned(), *c)).collect()
    }

    #[test]
    fn no_tool_calls_is_conversation() {
        let profile = profile_from_counts(&[], &CategoryMap::default());
        assert_eq!(classify(&profile), Activity::Conversation);
    }

    #[test]
    fn mostly_editing_is_coding() {
        let profile = profile_from_counts(
            &counts(&[("Edit", 8), ("Write", 4), ("Read", 2)]),
            &CategoryMap::default(),
        );
        assert_eq!(classify(&profile), Activity::Coding);
    }

    #[test]
    fn editing_interleaved_with_running_is_debugging() {
        // The distinction that makes the label worth deriving: the same tools in
        // different proportions mean different work.
        let profile = profile_from_counts(
            &counts(&[("Edit", 5), ("Bash", 5)]),
            &CategoryMap::default(),
        );
        assert_eq!(classify(&profile), Activity::Debugging);
    }

    #[test]
    fn reading_without_editing_is_exploration() {
        let profile = profile_from_counts(
            &counts(&[("Read", 10), ("Grep", 6), ("Glob", 3)]),
            &CategoryMap::default(),
        );
        assert_eq!(classify(&profile), Activity::Exploration);
    }

    #[test]
    fn running_things_without_editing_is_ops() {
        let profile =
            profile_from_counts(&counts(&[("Bash", 9), ("Read", 1)]), &CategoryMap::default());
        assert_eq!(classify(&profile), Activity::Ops);
    }

    #[test]
    fn web_lookups_are_research() {
        let profile = profile_from_counts(
            &counts(&[("WebSearch", 6), ("WebFetch", 4), ("Read", 1)]),
            &CategoryMap::default(),
        );
        assert_eq!(classify(&profile), Activity::Research);
    }

    #[test]
    fn a_profile_carries_every_category_not_just_the_winner() {
        let profile = profile_from_counts(
            &counts(&[("Edit", 5), ("Read", 5)]),
            &CategoryMap::default(),
        );
        assert!(profile.contains_key("code_change"));
        assert!(profile.contains_key("exploration"));
    }
}
