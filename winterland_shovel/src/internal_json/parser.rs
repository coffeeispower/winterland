//! Line-oriented parser for Nix internal JSON output.

use super::types::{Event, Field, Message};
use std::collections::BTreeMap;

/// Parses one line from a Nix process using `--log-format internal-json`.
///
/// Nix prefixes structured records with `@nix `. Non-prefixed lines are
/// returned as `Ok(None)` so callers can preserve ordinary diagnostics. A
/// prefixed line with invalid JSON returns the original serde error.
pub fn parse_line(line: &str) -> Result<Option<Event>, serde_json::Error> {
    let Some(json) = line.strip_prefix("@nix ") else {
        return Ok(None);
    };

    serde_json::from_str(json).map(Some)
}

/// Stateful interpreter for Nix internal JSON records.
#[derive(Debug, Default)]
pub struct Processor {
    active_downloads: BTreeMap<u64, String>,
}

impl Processor {
    /// Creates a processor with no active activities.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses and interprets one line from Nix stderr.
    pub fn process_line(&mut self, line: &str) -> Result<Message, serde_json::Error> {
        let Some(event) = parse_line(line)? else {
            return Ok(Message::Text(line.to_owned()));
        };

        Ok(match event.action.as_str() {
            "start" => self.process_start(event),
            "stop" => self.process_stop(event),
            "result" => self.process_result(event),
            _ => Message::Text(line.to_owned()),
        })
    }

    fn process_start(&mut self, event: Event) -> Message {
        let Some(text) = event.text else {
            return Message::ActivityStarted {
                id: event.id,
                description: None,
            };
        };

        let lower = text.to_ascii_lowercase();
        if lower.starts_with("evaluating ") {
            return Message::EvaluationStarted {
                id: event.id,
                path: text["evaluating ".len()..].to_owned(),
            };
        }

        if event.event_type == Some(ACT_FILE_TRANSFER) || is_input_download(&lower) {
            if let Some(id) = event.id {
                self.active_downloads.insert(id, text.clone());
            }
            return Message::InputDownloadStarted {
                id: event.id,
                description: text,
            };
        }

        Message::ActivityStarted {
            id: event.id,
            description: Some(text),
        }
    }

    fn process_result(&self, event: Event) -> Message {
        let Some(id) = event.id else {
            return Message::ActivityResult {
                id: None,
                event_type: event.event_type,
                fields: event.fields,
            };
        };

        let Some(description) = self.active_downloads.get(&id) else {
            return Message::ActivityResult {
                id: Some(id),
                event_type: event.event_type,
                fields: event.fields,
            };
        };

        match (event.event_type, event.fields.as_deref()) {
            (Some(RES_PROGRESS), Some(fields)) => {
                if let Some([done, expected, running, failed]) = four_numbers(fields) {
                    return Message::DownloadProgress {
                        id,
                        description: description.clone(),
                        done,
                        expected,
                        running,
                        failed,
                    };
                }
            }
            (
                Some(RES_SET_EXPECTED),
                Some([Field::Number(activity_type), Field::Number(expected)]),
            ) if *activity_type == ACT_FILE_TRANSFER => {
                return Message::DownloadExpected {
                    id,
                    description: description.clone(),
                    expected: *expected,
                };
            }
            _ => {}
        }

        Message::ActivityResult {
            id: Some(id),
            event_type: event.event_type,
            fields: event.fields,
        }
    }

    fn process_stop(&mut self, event: Event) -> Message {
        if let Some(id) = event.id {
            if let Some(description) = self.active_downloads.remove(&id) {
                return Message::InputDownloadFinished {
                    id: Some(id),
                    description,
                };
            }
        }

        Message::ActivityStopped { id: event.id }
    }
}

fn is_input_download(text: &str) -> bool {
    text.contains("fetch")
        || text.contains("download")
        || text.contains("copying path")
        || text.contains("copying closure")
}

const ACT_FILE_TRANSFER: u64 = 101;
const RES_PROGRESS: u64 = 105;
const RES_SET_EXPECTED: u64 = 106;

fn four_numbers(fields: &[Field]) -> Option<[u64; 4]> {
    let [Field::Number(done), Field::Number(expected), Field::Number(running), Field::Number(failed)] =
        fields
    else {
        return None;
    };
    Some([*done, *expected, *running, *failed])
}

#[cfg(test)]
mod tests {
    use super::{parse_line, Message};

    #[test]
    fn parses_nix_records() {
        let event = parse_line(
            r#"@nix {"action":"start","id":42,"parent":0,"text":"evaluating 'packages'","type":0}"#,
        )
        .unwrap()
        .unwrap();

        assert_eq!(event.action, "start");
        assert_eq!(event.id, Some(42));
        assert_eq!(event.parent, Some(0));
        assert_eq!(event.text.as_deref(), Some("evaluating 'packages'"));
        assert_eq!(event.event_type, Some(0));
    }

    #[test]
    fn leaves_unstructured_lines_to_the_caller() {
        assert!(parse_line("error: evaluation failed").unwrap().is_none());
    }

    #[test]
    fn interprets_and_correlates_download_messages() {
        let mut processor = super::Processor::new();
        let started = processor
            .process_line(r#"@nix {"action":"start","id":42,"text":"fetching input 'nixpkgs'"}"#)
            .unwrap();
        assert!(matches!(
            started,
            super::Message::InputDownloadStarted { id: Some(42), .. }
        ));

        let finished = processor
            .process_line(r#"@nix {"action":"stop","id":42}"#)
            .unwrap();
        assert!(matches!(
            finished,
            super::Message::InputDownloadFinished { id: Some(42), .. }
        ));
    }

    #[test]
    fn interprets_download_byte_progress() {
        let mut processor = super::Processor::new();
        processor
            .process_line(
                r#"@nix {"action":"start","id":42,"type":101,"text":"GET 'https://example'"}"#,
            )
            .unwrap();

        let progress = processor
            .process_line(r#"@nix {"action":"result","id":42,"type":105,"fields":[512,1024,0,0]}"#)
            .unwrap();

        assert!(matches!(
            progress,
            Message::DownloadProgress {
                id: 42,
                done: 512,
                expected: 1024,
                running: 0,
                failed: 0,
                ..
            }
        ));
    }
}
