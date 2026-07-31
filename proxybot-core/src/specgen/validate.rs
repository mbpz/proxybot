//! Validate LLM output against an embedded JSON schema.

use crate::specgen::error::SpecError;
use jsonschema::JSONSchema;
use serde_json::Value;

const OPENAPI_PATHS_SCHEMA: &str = "{
  \"type\": \"object\",
  \"required\": [\"paths\"],
  \"properties\": {
    \"paths\": {
      \"type\": \"object\",
      \"additionalProperties\": { \"$ref\": \"#/$defs/pathItem\" }
    }
  },
  \"$defs\": {
    \"pathItem\": {
      \"type\": \"object\",
      \"properties\": {
        \"get\":    { \"$ref\": \"#/$defs/operation\" },
        \"post\":   { \"$ref\": \"#/$defs/operation\" },
        \"put\":    { \"$ref\": \"#/$defs/operation\" },
        \"delete\": { \"$ref\": \"#/$defs/operation\" },
        \"patch\":  { \"$ref\": \"#/$defs/operation\" }
      }
    },
    \"operation\": {
      \"type\": \"object\",
      \"required\": [\"operationId\", \"summary\", \"responses\"],
      \"properties\": {
        \"operationId\": { \"type\": \"string\", \"pattern\": \"^[a-z][a-zA-Z0-9]+$\" },
        \"summary\":     { \"type\": \"string\" },
        \"tags\":        { \"type\": \"array\", \"items\": { \"type\": \"string\" } },
        \"parameters\":  { \"type\": \"array\" },
        \"requestBody\": { \"type\": \"object\" },
        \"responses\":   { \"type\": \"object\" }
      }
    }
  }
}";

pub fn validate_paths_object(candidate: &Value) -> Result<(), SpecError> {
    let schema: Value =
        serde_json::from_str(OPENAPI_PATHS_SCHEMA).expect("embedded schema is valid JSON");
    let compiled = JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .map_err(|e| SpecError::RenderFailed(format!("schema compile: {e}")))?;
    let result = compiled.validate(candidate);
    if let Err(errors) = result {
        let msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
        return Err(SpecError::RenderFailed(format!(
            "LLM output does not match schema: {}",
            msgs.join("; ")
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valid_paths_object_passes() {
        let v = json!({
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "listUsers",
                        "summary": "List users",
                        "responses": { "200": { "description": "ok" } }
                    }
                }
            }
        });
        assert!(validate_paths_object(&v).is_ok());
    }

    #[test]
    fn missing_paths_key_fails() {
        let v = json!({ "components": {} });
        assert!(validate_paths_object(&v).is_err());
    }

    #[test]
    fn operation_without_operationid_fails() {
        let v = json!({
            "paths": {
                "/x": {
                    "get": {
                        "summary": "no id",
                        "responses": {}
                    }
                }
            }
        });
        assert!(validate_paths_object(&v).is_err());
    }

    #[test]
    fn bad_operationid_pattern_fails() {
        let v = json!({
            "paths": {
                "/x": {
                    "get": {
                        "operationId": "BadId",
                        "summary": "x",
                        "responses": {}
                    }
                }
            }
        });
        assert!(validate_paths_object(&v).is_err());
    }
}
