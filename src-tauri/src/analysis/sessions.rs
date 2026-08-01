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

    if let Some(search) = request.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        where_parts.push(
            "(s.topic LIKE ?  OR s.project_name LIKE ? OR s.git_branch LIKE ?)".to_owned(),
        );
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

    let where_sql = format!("WHERE {}", where_parts.join(" AND "));

    let total: i64 = {
        let sql = format!("SELECT COUNT(*) {BASE} {where_sql}");
        let refs: Vec<&dyn ToSql> = params.iter().map(AsRef::as_ref).collect();
        conn.query_row(&sql, refs.as_slice(), |row| row.get(0))?
    };

    let direction = if request.descending { "DESC" } else { "ASC" };
    let page_size = request.page_size.clamp(1, 500);
    let offset = request.page as i64 * page_size as i64;

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
        order = order_by(request.sort.as_deref()),
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
    let ids: Vec<&dyn ToSql> = rows
        .iter()
        .map(|r| &r.session_id as &dyn ToSql)
        .collect();

    let mut by_session: HashMap<String, Vec<String>> = HashMap::new();
    let mut stmt = conn.prepare(&sql)?;
    let mut result = stmt.query(ids.as_slice())?;
    while let Some(row) = result.next()? {
        by_session
            .entry(row.get(0)?)
            .or_default()
            .push(row.get(1)?);
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
    std::iter::repeat_n("?", count).collect::<Vec<_>>().join(",")
}

/// The distinct values the filter controls offer, so the UI never invents one.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFacets {
    pub models: Vec<String>,
    pub activities: Vec<String>,
    pub tags: Vec<String>,
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
    })
}
