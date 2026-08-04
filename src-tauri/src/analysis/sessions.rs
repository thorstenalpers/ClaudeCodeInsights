use anyhow::Result;
use rusqlite::{Connection, ToSql};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionQuery {
    #[serde(default)]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub descending: bool,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub activities: Vec<String>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub branches: Vec<String>,
    /// Inclusive local dates, `YYYY-MM-DD`.
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    /// The window's price table, sent only to sort by cost.
    ///
    /// Cost is still never stored: it is worked out inside the one query that
    /// needs an order for it, from rates the window binds as parameters. The
    /// alternative — sorting the page the window already holds — would order
    /// twenty-five rows and call it the history.
    #[serde(default)]
    pub rates: Vec<FamilyRate>,
}

/// USD per million tokens for one model family, as the window has them.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FamilyRate {
    /// `opus`, `sonnet`, `haiku`, `fable` — matched against the model name.
    pub family: String,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

fn default_page_size() -> u32 {
    50
}

impl Default for SessionQuery {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: default_page_size(),
            sort: None,
            descending: true,
            search: None,
            activities: Vec::new(),
            models: Vec::new(),
            tags: Vec::new(),
            projects: Vec::new(),
            branches: Vec::new(),
            from: None,
            to: None,
            rates: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub session_id: String,
    pub topic: Option<String>,
    pub project_name: Option<String>,
    pub git_branch: Option<String>,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub duration_minutes: i64,
    pub model: Option<String>,
    pub turn_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub has_subagents: bool,
    pub activity: String,
    pub profile: HashMap<String, f64>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPage {
    pub rows: Vec<SessionRow>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

/// Sortable columns, mapped by hand.
///
/// The value that reaches SQL is chosen from this list and never taken from the
/// caller, so a sort column cannot become an injection point.
fn order_by(sort: Option<&str>) -> &'static str {
    match sort {
        Some("topic") => "s.topic",
        Some("project") => "s.project_name",
        Some("first") => "s.first_ts",
        Some("duration") => "duration_minutes",
        Some("turns") => "s.turn_count",
        Some("input") => "input_tokens",
        Some("output") => "output_tokens",
        Some("cacheRead") => "cache_read_tokens",
        Some("cacheWrite") => "cache_write_tokens",
        Some("tokens") => "(input_tokens + output_tokens + cache_read_tokens + cache_write_tokens)",
        Some("model") => "s.model",
        Some("activity") => "activity",
        _ => "s.last_ts",
    }
}

/// The cost of a row in SQL, from rates bound as parameters.
///
/// Every family the window sent becomes one `WHEN`, matched on the model name
/// the same way the window matches it. Only the placeholders are written into
/// the statement; the numbers travel as bound values, and a family name that
/// is not one of the four known ones never reaches the string at all.
fn cost_expression(rates: &[FamilyRate], first_index: usize) -> (String, Vec<Box<dyn ToSql>>) {
    const KNOWN: [&str; 4] = ["opus", "fable", "sonnet", "haiku"];

    let mut arms = String::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut index = first_index;

    for rate in rates
        .iter()
        .filter(|rate| KNOWN.contains(&rate.family.as_str()))
    {
        // `fable` and `mythos` are the same family under two names.
        let pattern = if rate.family == "fable" {
            "(s.model LIKE '%fable%' OR s.model LIKE '%mythos%')".to_owned()
        } else {
            format!("s.model LIKE '%{}%'", rate.family)
        };

        arms.push_str(&format!(
            " WHEN {pattern} THEN (input_tokens * ?{} + output_tokens * ?{} \
             + cache_read_tokens * ?{} + cache_write_tokens * ?{})",
            index,
            index + 1,
            index + 2,
            index + 3
        ));

        params.push(Box::new(rate.input));
        params.push(Box::new(rate.output));
        params.push(Box::new(rate.cache_read));
        params.push(Box::new(rate.cache_write));
        index += 4;
    }

    if arms.is_empty() {
        return ("0".to_owned(), params);
    }

    (format!("(CASE{arms} ELSE 0 END)"), params)
}

/// The join every session query starts from.
///
/// Token sums come from the turn rows rather than a stored column, so they
/// cannot drift out of step with the rows they describe.
const BASE: &str = r#"
FROM sessions s
LEFT JOIN (
    SELECT session_id,
           SUM(input_tokens)       AS input_tokens,
           SUM(output_tokens)      AS output_tokens,
           SUM(cache_read_tokens)  AS cache_read_tokens,
           SUM(cache_write_tokens) AS cache_write_tokens
    FROM turns GROUP BY session_id
) t ON t.session_id = s.session_id
LEFT JOIN session_activity a ON a.session_id = s.session_id
"#;

pub fn query(conn: &Connection, request: &SessionQuery) -> Result<SessionPage> {
    let mut where_parts: Vec<String> = vec!["s.turn_count > 0".to_owned()];
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(search) = request
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        where_parts
            .push("(s.topic LIKE ?  OR s.project_name LIKE ? OR s.git_branch LIKE ?)".to_owned());
        let pattern = format!("%{search}%");
        params.push(Box::new(pattern.clone()));
        params.push(Box::new(pattern.clone()));
        params.push(Box::new(pattern));
    }

    if !request.activities.is_empty() {
        where_parts.push(format!(
            "COALESCE(a.activity, 'conversation') IN ({})",
            placeholders(request.activities.len())
        ));
        for value in &request.activities {
            params.push(Box::new(value.clone()));
        }
    }

    if !request.models.is_empty() {
        where_parts.push(format!(
            "COALESCE(s.model, '') IN ({})",
            placeholders(request.models.len())
        ));
        for value in &request.models {
            params.push(Box::new(value.clone()));
        }
    }

    if !request.tags.is_empty() {
        where_parts.push(format!(
            "EXISTS (SELECT 1 FROM session_tags st
                     WHERE st.session_id = s.session_id AND st.tag IN ({}))",
            placeholders(request.tags.len())
        ));
        for value in &request.tags {
            params.push(Box::new(value.clone()));
        }
    }

    if !request.projects.is_empty() {
        // Either spelling counts. The filter chips carry the short name the
        // facets are built from, while a project's own page knows only the full
        // path it was registered under — matching one column would leave the
        // other asking for rows that cannot answer. The path is compared
        // spelling-blind: `~/.claude.json` holds the same directory twice, once
        // with each separator, and only one of them is what was scanned.
        where_parts.push(format!(
            "(COALESCE(s.project_name, '') IN ({0}) OR {1} IN ({0}))",
            placeholders(request.projects.len()),
            crate::analysis::SAME_PATH
        ));
        for value in &request.projects {
            params.push(Box::new(value.clone()));
        }
        for value in &request.projects {
            params.push(Box::new(crate::analysis::same_path(value)));
        }
    }

    if !request.branches.is_empty() {
        where_parts.push(format!(
            "COALESCE(s.git_branch, '') IN ({})",
            placeholders(request.branches.len())
        ));
        for value in &request.branches {
            params.push(Box::new(value.clone()));
        }
    }

    // The window is on last activity, which is what the column shows and what
    // "the past seven days" means to someone reading the table.
    if let Some(from) = request.from.as_deref().filter(|value| !value.is_empty()) {
        where_parts.push("date(s.last_ts, 'localtime') >= ?".to_owned());
        params.push(Box::new(from.to_owned()));
    }
    if let Some(to) = request.to.as_deref().filter(|value| !value.is_empty()) {
        where_parts.push("date(s.last_ts, 'localtime') <= ?".to_owned());
        params.push(Box::new(to.to_owned()));
    }

    let where_sql = format!("WHERE {}", where_parts.join(" AND "));

    let total: i64 = {
        let sql = format!("SELECT COUNT(*) {BASE} {where_sql}");
        let refs: Vec<&dyn ToSql> = params.iter().map(AsRef::as_ref).collect();
        conn.query_row(&sql, refs.as_slice(), |row| row.get(0))?
    };

    let direction = if request.descending { "DESC" } else { "ASC" };
    let page_size = request.page_size.clamp(1, 500);
    let offset = request.page as i64 * page_size as i64;

    // Cost is the one order the database cannot know on its own, so the rates
    // come with the request and are bound after the filters.
    let order = if request.sort.as_deref() == Some("cost") {
        let (expression, rate_params) = cost_expression(&request.rates, params.len() + 1);
        params.extend(rate_params);
        expression
    } else {
        order_by(request.sort.as_deref()).to_owned()
    };

    let sql = format!(
        r#"
        SELECT
            s.session_id,
            s.topic,
            s.project_name,
            s.git_branch,
            s.first_ts,
            s.last_ts,
            CAST((julianday(s.last_ts) - julianday(s.first_ts)) * 24 * 60 AS INTEGER) AS duration_minutes,
            s.model,
            s.turn_count,
            COALESCE(t.input_tokens, 0)       AS input_tokens,
            COALESCE(t.output_tokens, 0)      AS output_tokens,
            COALESCE(t.cache_read_tokens, 0)  AS cache_read_tokens,
            COALESCE(t.cache_write_tokens, 0) AS cache_write_tokens,
            s.has_subagents,
            COALESCE(a.activity, 'conversation') AS activity,
            COALESCE(a.profile_json, '{{}}')     AS profile_json
        {BASE}
        {where_sql}
        ORDER BY {order} {direction}
        LIMIT ?{limit_index} OFFSET ?{offset_index}
        "#,
        limit_index = params.len() + 1,
        offset_index = params.len() + 2,
    );

    params.push(Box::new(page_size as i64));
    params.push(Box::new(offset));

    let refs: Vec<&dyn ToSql> = params.iter().map(AsRef::as_ref).collect();
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(refs.as_slice())?;

    let mut result = Vec::new();
    while let Some(row) = rows.next()? {
        let profile_json: String = row.get(15)?;
        result.push(SessionRow {
            session_id: row.get(0)?,
            topic: row.get(1)?,
            project_name: row.get(2)?,
            git_branch: row.get(3)?,
            first_ts: row.get(4)?,
            last_ts: row.get(5)?,
            duration_minutes: row.get::<_, Option<i64>>(6)?.unwrap_or(0),
            model: row.get(7)?,
            turn_count: row.get(8)?,
            input_tokens: row.get(9)?,
            output_tokens: row.get(10)?,
            cache_read_tokens: row.get(11)?,
            cache_write_tokens: row.get(12)?,
            has_subagents: row.get::<_, i64>(13)? != 0,
            activity: row.get(14)?,
            profile: serde_json::from_str(&profile_json).unwrap_or_default(),
            tags: Vec::new(),
        });
    }
    drop(rows);
    drop(stmt);

    attach_tags(conn, &mut result)?;

    Ok(SessionPage {
        rows: result,
        total,
        page: request.page,
        page_size,
    })
}

/// Tags are fetched for the page's sessions in one query rather than per row.
fn attach_tags(conn: &Connection, rows: &mut [SessionRow]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let sql = format!(
        "SELECT session_id, tag FROM session_tags WHERE session_id IN ({}) ORDER BY tag",
        placeholders(rows.len())
    );
    let ids: Vec<&dyn ToSql> = rows.iter().map(|r| &r.session_id as &dyn ToSql).collect();

    let mut by_session: HashMap<String, Vec<String>> = HashMap::new();
    let mut stmt = conn.prepare(&sql)?;
    let mut result = stmt.query(ids.as_slice())?;
    while let Some(row) = result.next()? {
        by_session.entry(row.get(0)?).or_default().push(row.get(1)?);
    }

    for row in rows.iter_mut() {
        if let Some(tags) = by_session.remove(&row.session_id) {
            row.tags = tags;
        }
    }
    Ok(())
}

/// `?,?,?` for a variable-length IN clause. The count comes from the number of
/// bound values, never from caller text.
fn placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(",")
}

/// The distinct values the filter controls offer, so the UI never invents one.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFacets {
    pub models: Vec<String>,
    pub activities: Vec<String>,
    pub tags: Vec<String>,
    pub projects: Vec<String>,
    pub branches: Vec<String>,
}

pub fn facets(conn: &Connection) -> Result<SessionFacets> {
    let collect = |sql: &str| -> Result<Vec<String>> {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.filter_map(Result::ok).collect())
    };

    Ok(SessionFacets {
        models: collect(
            "SELECT DISTINCT model FROM sessions WHERE model IS NOT NULL AND model <> '' ORDER BY model",
        )?,
        activities: collect("SELECT DISTINCT activity FROM session_activity ORDER BY activity")?,
        tags: collect("SELECT name FROM tags ORDER BY name")?,
        projects: collect(
            "SELECT DISTINCT project_name FROM sessions
             WHERE project_name IS NOT NULL AND project_name <> '' AND turn_count > 0
             ORDER BY project_name",
        )?,
        branches: collect(
            "SELECT DISTINCT git_branch FROM sessions
             WHERE git_branch IS NOT NULL AND git_branch <> '' AND turn_count > 0
             ORDER BY git_branch",
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema;

    fn seeded() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO scan_files (id, path, mtime_ms, size_bytes, line_count, scanned_at)
            VALUES (1, 'f.jsonl', 0, 0, 0, '2026-01-01T00:00:00Z');

            INSERT INTO sessions (session_id, project_name, project_path, turn_count)
            VALUES ('s1', 'work/a', 'C:\Sources\work\a', 1);

            INSERT INTO turns (id, session_id, message_id, ts_utc, model, input_tokens,
                               output_tokens, cache_read_tokens, cache_write_tokens, scan_file_id)
            VALUES (1, 's1', 'm1', '2026-01-01T10:00:00Z', 'claude-opus-5', 100, 10, 1000, 5, 1);
            "#,
        )
        .unwrap();
        conn
    }

    fn request(projects: Vec<String>) -> SessionQuery {
        SessionQuery {
            projects,
            ..SessionQuery::default()
        }
    }

    #[test]
    fn a_project_is_found_by_its_short_name_and_by_its_full_path() {
        // The filter chips send the one, a project's own page sends the other.
        let conn = seeded();
        assert_eq!(
            query(&conn, &request(vec!["work/a".to_owned()]))
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            query(&conn, &request(vec![r"C:\Sources\work\a".to_owned()]))
                .unwrap()
                .total,
            1
        );
    }
}
