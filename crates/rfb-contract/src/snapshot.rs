// SPDX-License-Identifier: MPL-2.0

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const SNAPSHOT_NORMALIZATION_SCHEMA_VERSION: u16 = 1;

const VOLATILE_FIELDS: &[&str] = &[
    "createdAt",
    "diagnosticsId",
    "generatedAt",
    "generatedAtUnix",
    "localPath",
    "platform",
    "platformPath",
    "requestId",
    "savedAt",
    "sessionId",
    "sourceRelativePath",
    "timestamp",
    "viewport",
    "windowSize",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedSnapshot {
    pub normalization_schema_version: u16,
    pub snapshot: Value,
}

pub fn normalize_json(bytes: &[u8]) -> Result<NormalizedSnapshot, SnapshotNormalizationError> {
    normalize_value(serde_json::from_slice(bytes)?)
}

pub fn normalize_serializable(
    value: &impl Serialize,
) -> Result<NormalizedSnapshot, SnapshotNormalizationError> {
    normalize_value(serde_json::to_value(value)?)
}

pub fn normalize_value(mut value: Value) -> Result<NormalizedSnapshot, SnapshotNormalizationError> {
    normalize_node(&mut value, None)?;
    Ok(NormalizedSnapshot {
        normalization_schema_version: SNAPSHOT_NORMALIZATION_SCHEMA_VERSION,
        snapshot: value,
    })
}

pub fn canonical_bytes(
    snapshot: &NormalizedSnapshot,
) -> Result<Vec<u8>, SnapshotNormalizationError> {
    Ok(serde_json::to_vec(snapshot)?)
}

pub fn normalized_hash(
    snapshot: &NormalizedSnapshot,
) -> Result<String, SnapshotNormalizationError> {
    Ok(format!("{:x}", Sha256::digest(canonical_bytes(snapshot)?)))
}

fn normalize_node(
    value: &mut Value,
    parent_key: Option<&str>,
) -> Result<(), SnapshotNormalizationError> {
    match value {
        Value::Object(object) => {
            let original = std::mem::take(object);
            let mut entries = original
                .into_iter()
                .filter(|(key, _)| {
                    !(VOLATILE_FIELDS.contains(&key.as_str())
                        || parent_key == Some("source") && key == "path")
                })
                .collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut normalized = Map::new();
            for (key, mut child) in entries {
                normalize_node(&mut child, Some(&key))?;
                normalized.insert(key, child);
            }
            *object = normalized;
        }
        Value::Array(values) => {
            for child in values.iter_mut() {
                normalize_node(child, None)?;
            }
            match parent_key {
                Some("cells" | "changedCells") => values.sort_by(compare_position),
                Some("entities" | "items" | "statuses") => values.sort_by(compare_stable_id),
                Some("removedEntities" | "unorderedIds") => values.sort_by(compare_string),
                _ => {}
            }
        }
        Value::String(text) => {
            *text = text.replace("\r\n", "\n").replace('\r', "\n");
        }
        Value::Number(number) => {
            if number.as_i64().is_none() && number.as_u64().is_none() {
                return Err(SnapshotNormalizationError::FloatingPoint(
                    number.to_string(),
                ));
            }
        }
        Value::Null | Value::Bool(_) => {}
    }
    Ok(())
}

fn compare_position(left: &Value, right: &Value) -> Ordering {
    position(left)
        .cmp(&position(right))
        .then_with(|| canonical_value(left).cmp(&canonical_value(right)))
}

fn position(value: &Value) -> (i64, i64) {
    let position = value.get("position").unwrap_or(value);
    let x = position
        .get("x")
        .and_then(Value::as_i64)
        .unwrap_or(i64::MAX);
    let y = position
        .get("y")
        .and_then(Value::as_i64)
        .unwrap_or(i64::MAX);
    (y, x)
}

fn compare_stable_id(left: &Value, right: &Value) -> Ordering {
    stable_id(left)
        .cmp(stable_id(right))
        .then_with(|| canonical_value(left).cmp(&canonical_value(right)))
}

fn stable_id(value: &Value) -> &str {
    value.get("id").and_then(Value::as_str).unwrap_or("")
}

fn compare_string(left: &Value, right: &Value) -> Ordering {
    left.as_str()
        .unwrap_or("")
        .cmp(right.as_str().unwrap_or(""))
        .then_with(|| canonical_value(left).cmp(&canonical_value(right)))
}

fn canonical_value(value: &Value) -> String {
    serde_json::to_string(value).expect("serializing a JSON value should not fail")
}

#[derive(Debug, Error)]
pub enum SnapshotNormalizationError {
    #[error("authoritative snapshot contains floating-point value {0}")]
    FloatingPoint(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use rfb_core::Game;
    use serde_json::json;

    use super::*;

    #[test]
    fn object_and_unordered_collection_order_do_not_change_hash() {
        let left = json!({
            "entities": [{"id": "b", "hp": 2}, {"id": "a", "hp": 1}],
            "cells": [
                {"position": {"x": 2, "y": 1}, "terrainId": "floor"},
                {"position": {"x": 1, "y": 1}, "terrainId": "wall"}
            ],
            "removedEntities": ["z", "a"],
            "turn": 3
        });
        let right = json!({
            "turn": 3,
            "removedEntities": ["a", "z"],
            "cells": [
                {"terrainId": "wall", "position": {"y": 1, "x": 1}},
                {"terrainId": "floor", "position": {"y": 1, "x": 2}}
            ],
            "entities": [{"hp": 1, "id": "a"}, {"hp": 2, "id": "b"}]
        });

        let left = normalize_value(left).expect("left snapshot should normalize");
        let right = normalize_value(right).expect("right snapshot should normalize");
        assert_eq!(left, right);
        assert_eq!(
            normalized_hash(&left).expect("left hash should compute"),
            normalized_hash(&right).expect("right hash should compute")
        );
    }

    #[test]
    fn volatile_fields_and_line_endings_are_normalized_recursively() {
        let normalized = normalize_value(json!({
            "generatedAtUnix": 123,
            "message": "line one\r\nline two\rline three",
            "nested": {"localPath": "C:/private/save", "value": 7},
            "object": {"path": "semantic.route", "value": 9},
            "source": {"path": "C:/private/repository", "commit": "abc"},
            "sessionId": "random-session"
        }))
        .expect("snapshot should normalize");

        assert_eq!(
            normalized.snapshot,
            json!({
                "message": "line one\nline two\nline three",
                "nested": {"value": 7},
                "object": {"path": "semantic.route", "value": 9},
                "source": {"commit": "abc"}
            })
        );
    }

    #[test]
    fn event_order_remains_authoritative() {
        let first = normalize_value(json!({
            "events": [{"kind": "hit"}, {"kind": "slay"}]
        }))
        .expect("snapshot should normalize");
        let second = normalize_value(json!({
            "events": [{"kind": "slay"}, {"kind": "hit"}]
        }))
        .expect("snapshot should normalize");

        assert_ne!(first, second);
        assert_ne!(
            normalized_hash(&first).expect("first hash should compute"),
            normalized_hash(&second).expect("second hash should compute")
        );
    }

    #[test]
    fn floating_point_values_are_rejected() {
        assert!(matches!(
            normalize_value(json!({"authoritativeDamage": 1.5})),
            Err(SnapshotNormalizationError::FloatingPoint(_))
        ));
    }

    #[test]
    fn typed_game_snapshot_normalizes_stably() {
        let snapshot = Game::new(42).snapshot();
        let first = normalize_serializable(&snapshot).expect("snapshot should normalize");
        let second = normalize_serializable(&snapshot).expect("snapshot should normalize");
        assert_eq!(first, second);
        assert_eq!(
            normalized_hash(&first).expect("first hash should compute"),
            normalized_hash(&second).expect("second hash should compute")
        );
    }
}
