//! Matrix build support — reads `.ci/matrix.json`, generates Cartesian product of jobs.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Matrix configuration from `.ci/matrix.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    /// List of matrix dimensions
    #[serde(default)]
    pub include: Vec<MatrixInclude>,
    /// Optional matrix dimensions (alternative format)
    #[serde(default)]
    pub matrix: Option<HashMap<String, Vec<serde_json::Value>>>,
}

impl Default for MatrixConfig {
    fn default() -> Self {
        Self {
            include: vec![],
            matrix: None,
        }
    }
}

/// A single matrix include entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixInclude {
    /// Fixed key-value pairs for this matrix entry
    #[serde(default)]
    pub with: HashMap<String, serde_json::Value>,
}

impl MatrixConfig {
    /// Parse a matrix.json string. Returns None if parsing fails.
    pub fn parse(content: &str) -> Result<Self, String> {
        serde_json::from_str(content)
            .map_err(|e| format!("Failed to parse matrix.json: {}", e))
    }

    /// Generate the Cartesian product of all matrix variables.
    /// Returns a list of env overlays, one per combination.
    pub fn expand(&self) -> Vec<HashMap<String, String>> {
        let mut results = Vec::new();

        // Format 1: `include` list (explicit combinations)
        if !self.include.is_empty() {
            for entry in &self.include {
                let mut env = HashMap::new();
                for (k, v) in &entry.with {
                    env.insert(k.clone(), Self::val_to_string(v));
                }
                results.push(env);
            }
            return results;
        }

        // Format 2: `matrix` object (cartesian product)
        if let Some(ref matrix) = self.matrix {
            if matrix.is_empty() {
                return vec![HashMap::new()];
            }

            let keys: Vec<&String> = matrix.keys().collect();
            let values: Vec<&Vec<serde_json::Value>> = matrix.values().collect();

            Self::cartesian_recursive(&keys, &values, 0, &mut HashMap::new(), &mut results);
        }

        if results.is_empty() {
            results.push(HashMap::new());
        }

        results
    }

    fn cartesian_recursive(
        keys: &[&String],
        values: &[&Vec<serde_json::Value>],
        idx: usize,
        current: &mut HashMap<String, String>,
        results: &mut Vec<HashMap<String, String>>,
    ) {
        if idx >= keys.len() {
            results.push(current.clone());
            return;
        }

        let key = keys[idx];
        for val in values[idx] {
            current.insert(key.clone(), Self::val_to_string(val));
            Self::cartesian_recursive(keys, values, idx + 1, current, results);
            current.remove(key);
        }
    }

    fn val_to_string(v: &serde_json::Value) -> String {
        match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => String::new(),
            _ => v.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_include_format() {
        let json = r#"{
            "include": [
                { "with": { "PYTHON_VERSION": "3.10", "OS": "ubuntu" } },
                { "with": { "PYTHON_VERSION": "3.11", "OS": "debian" } }
            ]
        }"#;
        let config = MatrixConfig::parse(json).unwrap();
        let expanded = config.expand();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0].get("PYTHON_VERSION").unwrap(), "3.10");
        assert_eq!(expanded[1].get("OS").unwrap(), "debian");
    }

    #[test]
    fn test_matrix_cartesian_format() {
        let json = r#"{
            "matrix": {
                "PYTHON": ["3.10", "3.11"],
                "OS": ["ubuntu", "debian"]
            }
        }"#;
        let config = MatrixConfig::parse(json).unwrap();
        let expanded = config.expand();
        // 2 × 2 = 4 combinations
        assert_eq!(expanded.len(), 4);
    }

    #[test]
    fn test_matrix_empty() {
        let config = MatrixConfig::parse("{}").unwrap();
        assert_eq!(config.expand().len(), 1); // single empty env
        assert!(config.expand()[0].is_empty());
    }

    #[test]
    fn test_matrix_value_types() {
        let json = r#"{
            "include": [
                { "with": { "INT_VAL": 42, "BOOL_VAL": true, "NULL_VAL": null } }
            ]
        }"#;
        let config = MatrixConfig::parse(json).unwrap();
        let expanded = config.expand();
        assert_eq!(expanded[0].get("INT_VAL").unwrap(), "42");
        assert_eq!(expanded[0].get("BOOL_VAL").unwrap(), "true");
        assert_eq!(expanded[0].get("NULL_VAL").unwrap(), "");
    }
}
