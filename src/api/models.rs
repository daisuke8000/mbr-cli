use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Custom deserializer: converts "root" string to None for collection_id
fn deserialize_collection_id<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(n) => {
            if let Some(id) = n.as_u64() {
                Ok(Some(id as u32))
            } else {
                Ok(None)
            }
        }
        Value::String(s) => {
            // NOTE: "root" is a special case that should return None
            if s == "root" {
                Ok(None)
            } else if let Ok(id) = s.parse::<u32>() {
                Ok(Some(id))
            } else {
                Ok(None)
            }
        }
        Value::Null => Ok(None),
        _ => Ok(None),
    }
}

// Authentication models
#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub id: String, // session token
}

// Question/Card models
#[derive(Debug, Deserialize, Clone)]
pub struct Question {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    #[serde(deserialize_with = "deserialize_collection_id", default)]
    pub collection_id: Option<u32>,
    pub collection: Option<Collection>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Collection {
    #[serde(deserialize_with = "deserialize_collection_id", default)]
    pub id: Option<u32>,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QueryResult {
    pub data: QueryData,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QueryData {
    pub cols: Vec<Column>,
    pub rows: Vec<Vec<Value>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Column {
    pub name: String,
    pub display_name: String,
    pub base_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_collection_id_with_question() {
        // Test with a "root" string
        let json = r#"{
            "id": 1,
            "name": "Test Question",
            "collection_id": "root"
        }"#;
        let question: Question = serde_json::from_str(json).unwrap();
        assert_eq!(question.collection_id, None);

        // Test with number
        let json = r#"{
            "id": 2,
            "name": "Test Question",
            "collection_id": 123
        }"#;
        let question: Question = serde_json::from_str(json).unwrap();
        assert_eq!(question.collection_id, Some(123));

        // Test with string number
        let json = r#"{
            "id": 3,
            "name": "Test Question",
            "collection_id": "456"
        }"#;
        let question: Question = serde_json::from_str(json).unwrap();
        assert_eq!(question.collection_id, Some(456));

        // Test with null
        let json = r#"{
            "id": 4,
            "name": "Test Question",
            "collection_id": null
        }"#;
        let question: Question = serde_json::from_str(json).unwrap();
        assert_eq!(question.collection_id, None);

        // Test with missing field (uses default)
        let json = r#"{
            "id": 5,
            "name": "Test Question"
        }"#;
        let question: Question = serde_json::from_str(json).unwrap();
        assert_eq!(question.collection_id, None);
    }

    #[test]
    fn test_deserialize_collection_with_root() {
        let json = r#"{
            "id": "root",
            "name": "Our Analytics"
        }"#;
        let collection: Collection = serde_json::from_str(json).unwrap();
        assert_eq!(collection.id, None);
        assert_eq!(collection.name, "Our Analytics");
    }

    #[test]
    fn test_login_request_serialization() {
        let request = LoginRequest {
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test_user"));
        assert!(json.contains("test_pass"));
    }

    #[test]
    fn test_query_result_deserialization() {
        let json = r#"{
            "data": {
                "cols": [
                    {
                        "name": "id",
                        "display_name": "ID",
                        "base_type": "type/Integer"
                    },
                    {
                        "name": "name",
                        "display_name": "Name",
                        "base_type": "type/Text"
                    }
                ],
                "rows": [
                    [1, "Alice"],
                    [2, "Bob"]
                ]
            }
        }"#;

        let result: QueryResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.data.cols.len(), 2);
        assert_eq!(result.data.rows.len(), 2);
        assert_eq!(result.data.cols[0].name, "id");
        assert_eq!(result.data.cols[1].display_name, "Name");
    }
}
