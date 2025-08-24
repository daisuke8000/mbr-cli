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

// Dashboard models
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Dashboard {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    #[serde(deserialize_with = "deserialize_collection_id", default)]
    pub collection_id: Option<u32>,
    pub creator_id: Option<u32>,
    pub created_at: String,
    pub updated_at: String,
    pub dashcards: Option<Vec<DashboardCard>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DashboardCard {
    pub id: u32,
    pub dashboard_id: u32,
    pub card_id: Option<u32>,
    pub col: i32,
    pub row: i32,
    pub size_x: i32,
    pub size_y: i32,
}

impl Dashboard {
    /// Validate dashboard data
    pub fn validate(&self) -> Result<(), String> {
        if self.id == 0 {
            return Err("Dashboard ID must be greater than 0".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("Dashboard name cannot be empty".to_string());
        }
        Ok(())
    }
}

impl DashboardCard {
    /// Validate dashboard card data
    pub fn validate(&self) -> Result<(), String> {
        if self.id == 0 {
            return Err("Dashboard card ID must be greater than 0".to_string());
        }
        if self.dashboard_id == 0 {
            return Err("Dashboard ID must be greater than 0".to_string());
        }
        if self.col < 0 {
            return Err("Column position cannot be negative".to_string());
        }
        if self.row < 0 {
            return Err("Row position cannot be negative".to_string());
        }
        if self.size_x <= 0 {
            return Err("Width must be greater than 0".to_string());
        }
        if self.size_y <= 0 {
            return Err("Height must be greater than 0".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_deserialization() {
        let json = r#"{
            "id": 1,
            "name": "Test Dashboard",
            "description": "A test dashboard",
            "collection_id": 123,
            "creator_id": 1,
            "created_at": "2023-01-01T00:00:00Z",
            "updated_at": "2023-01-01T00:00:00Z",
            "dashcards": []
        }"#;
        
        let dashboard: Dashboard = serde_json::from_str(json).unwrap();
        assert_eq!(dashboard.id, 1);
        assert_eq!(dashboard.name, "Test Dashboard");
        assert_eq!(dashboard.collection_id, Some(123));
    }

    #[test]
    fn test_dashboard_with_root_collection() {
        let json = r#"{
            "id": 2,
            "name": "Root Dashboard",
            "collection_id": "root",
            "creator_id": 1,
            "created_at": "2023-01-01T00:00:00Z",
            "updated_at": "2023-01-01T00:00:00Z"
        }"#;
        
        let dashboard: Dashboard = serde_json::from_str(json).unwrap();
        assert_eq!(dashboard.collection_id, None);
    }

    #[test]
    fn test_dashboard_card_deserialization() {
        let json = r#"{
            "id": 1,
            "dashboard_id": 1,
            "card_id": 123,
            "col": 0,
            "row": 0,
            "size_x": 4,
            "size_y": 4
        }"#;
        
        let card: DashboardCard = serde_json::from_str(json).unwrap();
        assert_eq!(card.id, 1);
        assert_eq!(card.dashboard_id, 1);
        assert_eq!(card.card_id, Some(123));
    }

    #[test]
    fn test_dashboard_validation() {
        // Valid dashboard
        let dashboard = Dashboard {
            id: 1,
            name: "Test Dashboard".to_string(),
            description: None,
            collection_id: None,
            creator_id: Some(1),
            created_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            dashcards: None,
        };
        assert!(dashboard.validate().is_ok());

        // Invalid: zero ID
        let mut invalid_dashboard = dashboard.clone();
        invalid_dashboard.id = 0;
        assert!(invalid_dashboard.validate().is_err());

        // Invalid: empty name
        let mut invalid_dashboard = dashboard.clone();
        invalid_dashboard.name = "".to_string();
        assert!(invalid_dashboard.validate().is_err());

        // Invalid: whitespace-only name
        let mut invalid_dashboard = dashboard;
        invalid_dashboard.name = "   ".to_string();
        assert!(invalid_dashboard.validate().is_err());
    }

    #[test]
    fn test_dashboard_card_validation() {
        // Valid dashboard card
        let card = DashboardCard {
            id: 1,
            dashboard_id: 1,
            card_id: Some(123),
            col: 0,
            row: 0,
            size_x: 4,
            size_y: 4,
        };
        assert!(card.validate().is_ok());

        // Invalid: zero ID
        let mut invalid_card = card.clone();
        invalid_card.id = 0;
        assert!(invalid_card.validate().is_err());

        // Invalid: negative position
        let mut invalid_card = card.clone();
        invalid_card.col = -1;
        assert!(invalid_card.validate().is_err());

        // Invalid: zero size
        let mut invalid_card = card;
        invalid_card.size_x = 0;
        assert!(invalid_card.validate().is_err());
    }

    #[test]
    fn test_dashboard_serialization() {
        let dashboard = Dashboard {
            id: 1,
            name: "Test Dashboard".to_string(),
            description: Some("A test dashboard".to_string()),
            collection_id: Some(123),
            creator_id: Some(1),
            created_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            dashcards: Some(vec![]),
        };

        let json = serde_json::to_string(&dashboard).unwrap();
        assert!(json.contains("Test Dashboard"));
        assert!(json.contains("A test dashboard"));
    }

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
