pub mod reader;
pub mod record;
pub mod scanner;

#[cfg(test)]
mod tests {
    use super::reader::parse_file;
    use std::io::Write;
    use std::path::PathBuf;

    /// Writes a transcript to a unique temp path. `dir_hint` lets a test place
    /// the file under a `subagents/` directory.
    fn transcript(name: &str, dir_hint: Option<&str>, lines: &[&str]) -> PathBuf {
        let mut dir = std::env::temp_dir().join(format!("cua-test-{}", std::process::id()));
        if let Some(hint) = dir_hint {
            dir = dir.join(hint);
        }
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join(format!("{name}.jsonl"));
        let mut file = std::fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        path
    }

    fn assistant(message_id: &str, uuid: &str, output: i64, ts: &str) -> String {
        format!(
            r#"{{"type":"assistant","sessionId":"s1","uuid":"{uuid}","timestamp":"{ts}",
                 "message":{{"id":"{message_id}","model":"claude-opus-4-8",
                 "usage":{{"input_tokens":10,"output_tokens":{output}}}}}}}"#
        )
        .replace('\n', "")
    }

    #[test]
    fn last_record_for_a_message_id_wins() {
        // Claude Code emits several streaming records per response and only the
        // last carries the final tally. Taking the first would under-report.
        let path = transcript(
            "streaming",
            None,
            &[
                &assistant("msg-1", "u1", 5, "2026-01-01T10:00:00Z"),
                &assistant("msg-1", "u2", 50, "2026-01-01T10:00:01Z"),
                &assistant("msg-1", "u3", 500, "2026-01-01T10:00:02Z"),
            ],
        );

        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(parsed.turns.len(), 1, "three records, one response");
        assert_eq!(parsed.turns[0].output_tokens, 500, "the last tally wins");
    }

    #[test]
    fn zero_token_turns_are_dropped() {
        let path = transcript(
            "zero-tokens",
            None,
            &[
                r#"{"type":"assistant","sessionId":"s1","uuid":"u1","timestamp":"2026-01-01T10:00:00Z","message":{"id":"m1","usage":{"input_tokens":0,"output_tokens":0}}}"#,
                &assistant("m2", "u2", 20, "2026-01-01T10:00:01Z"),
            ],
        );

        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(parsed.turns.len(), 1);
        assert_eq!(parsed.turns[0].message_id.as_deref(), Some("m2"));
    }

    #[test]
    fn repeated_uuids_are_counted_once() {
        // A session resumed headlessly and reopened in the CLI rewrites the same
        // line range verbatim.
        let line = assistant("m1", "same-uuid", 30, "2026-01-01T10:00:00Z");
        let other = assistant("m2", "same-uuid", 40, "2026-01-01T10:00:01Z");
        let path = transcript("resumed", None, &[&line, &other]);

        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(parsed.turns.len(), 1, "the second uuid repeat is skipped");
    }

    #[test]
    fn turns_without_a_message_id_are_all_kept() {
        // They are the reasoning bucket and must not collapse into one another.
        let path = transcript(
            "anonymous",
            None,
            &[
                r#"{"type":"assistant","sessionId":"s1","uuid":"u1","timestamp":"2026-01-01T10:00:00Z","message":{"usage":{"output_tokens":11}}}"#,
                r#"{"type":"assistant","sessionId":"s1","uuid":"u2","timestamp":"2026-01-01T10:00:01Z","message":{"usage":{"output_tokens":22}}}"#,
            ],
        );

        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(parsed.turns.len(), 2);
        assert!(parsed.turns.iter().all(|t| t.message_id.is_none()));
    }

    #[test]
    fn subagents_are_detected_from_any_of_the_three_signals() {
        let sidechain = r#"{"type":"assistant","sessionId":"s1","uuid":"a","timestamp":"2026-01-01T10:00:00Z","isSidechain":true,"message":{"id":"m1","usage":{"output_tokens":5}}}"#;
        let by_agent_id = r#"{"type":"assistant","sessionId":"s1","uuid":"b","timestamp":"2026-01-01T10:00:01Z","agentId":"agent-7","message":{"id":"m2","usage":{"output_tokens":5}}}"#;

        let parsed = parse_file(&transcript("flags", None, &[sidechain, by_agent_id]), 0).unwrap();
        assert!(parsed.turns.iter().all(|t| t.is_subagent));
        assert!(parsed.sessions["s1"].has_subagents);

        // Third signal: the path itself.
        let plain = assistant("m3", "c", 5, "2026-01-01T10:00:02Z");
        let nested = parse_file(&transcript("agent-3", Some("subagents"), &[&plain]), 0).unwrap();
        assert!(nested.turns[0].is_subagent, "path under subagents/ counts");
    }

    #[test]
    fn nested_agent_id_is_found() {
        let line = r#"{"type":"assistant","sessionId":"s1","uuid":"a","timestamp":"2026-01-01T10:00:00Z","data":{"agentId":"nested-9"},"message":{"id":"m1","usage":{"output_tokens":5}}}"#;
        let parsed = parse_file(&transcript("nested-agent", None, &[line]), 0).unwrap();
        assert_eq!(parsed.turns[0].agent_id.as_deref(), Some("nested-9"));
    }

    #[test]
    fn a_written_title_outranks_an_inferred_one() {
        let ai = r#"{"type":"ai-title","sessionId":"s1","aiTitle":"Guessed"}"#;
        let custom = r#"{"type":"custom-title","sessionId":"s1","customTitle":"Written"}"#;

        let after_ai_then_custom = parse_file(&transcript("t1", None, &[ai, custom]), 0).unwrap();
        assert_eq!(after_ai_then_custom.sessions["s1"].topic.as_deref(), Some("Written"));

        // And the other way round: an inferred title must not overwrite one the
        // user wrote, whatever the order in the file.
        let after_custom_then_ai = parse_file(&transcript("t2", None, &[custom, ai]), 0).unwrap();
        assert_eq!(after_custom_then_ai.sessions["s1"].topic.as_deref(), Some("Written"));
    }

    #[test]
    fn tool_calls_are_read_in_order() {
        let line = r#"{"type":"assistant","sessionId":"s1","uuid":"a","timestamp":"2026-01-01T10:00:00Z","message":{"id":"m1","usage":{"output_tokens":5},"content":[{"type":"text","text":"hi"},{"type":"tool_use","name":"Read"},{"type":"tool_use","name":"Edit"}]}}"#;
        let parsed = parse_file(&transcript("tools", None, &[line]), 0).unwrap();
        assert_eq!(parsed.turns[0].tools, vec!["Read", "Edit"]);
    }

    #[test]
    fn subagent_dispatches_are_collected() {
        let line = r#"{"type":"user","sessionId":"s1","uuid":"a","timestamp":"2026-01-01T10:00:00Z","toolUseResult":{"agentId":"agent-1","agentType":"Explore","status":"completed","totalTokens":1234,"totalDurationMs":5000,"totalToolUseCount":7}}"#;
        let parsed = parse_file(&transcript("dispatch", None, &[line]), 0).unwrap();

        assert_eq!(parsed.agents.len(), 1);
        let agent = &parsed.agents[0];
        assert_eq!(agent.agent_id, "agent-1");
        assert_eq!(agent.agent_type.as_deref(), Some("Explore"));
        assert_eq!(agent.total_tokens, Some(1234));
    }

    #[test]
    fn a_tool_use_result_that_is_not_an_object_is_ignored() {
        // Real transcripts sometimes carry a bare string here; it must not abort
        // the file.
        let line = r#"{"type":"user","sessionId":"s1","uuid":"a","timestamp":"2026-01-01T10:00:00Z","toolUseResult":"done"}"#;
        let parsed = parse_file(&transcript("odd-result", None, &[line]), 0).unwrap();
        assert!(parsed.agents.is_empty());
        assert_eq!(parsed.malformed_lines, 0);
    }

    #[test]
    fn malformed_lines_are_counted_not_fatal() {
        let path = transcript(
            "malformed",
            None,
            &[
                "{not json at all",
                &assistant("m1", "u1", 42, "2026-01-01T10:00:00Z"),
            ],
        );

        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(parsed.malformed_lines, 1);
        assert_eq!(parsed.turns.len(), 1, "the good line still lands");
    }

    #[test]
    fn skipping_lines_reads_only_the_tail() {
        // This is what makes rescanning an append-only transcript cheap.
        let path = transcript(
            "appended",
            None,
            &[
                &assistant("m1", "u1", 10, "2026-01-01T10:00:00Z"),
                &assistant("m2", "u2", 20, "2026-01-01T10:00:01Z"),
                &assistant("m3", "u3", 30, "2026-01-01T10:00:02Z"),
            ],
        );

        let parsed = parse_file(&path, 2).unwrap();
        assert_eq!(parsed.line_count, 3, "the count still covers the whole file");
        assert_eq!(parsed.turns.len(), 1);
        assert_eq!(parsed.turns[0].message_id.as_deref(), Some("m3"));
    }
}
