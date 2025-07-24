use std::sync::Arc;
use crate::commands::Command;
use crate::protocol::{RespResponse, parser::RespValue};
use crate::storage::GeoDatabase;
use crate::Result;

pub struct GetCommand {
    database: Arc<GeoDatabase>,
}

impl GetCommand {
    pub fn new(database: Arc<GeoDatabase>) -> Self {
        Self { database }
    }
}

impl Command for GetCommand {
    fn name(&self) -> &'static str {
        "GET"
    }

    fn execute(&self, args: &[RespValue]) -> impl std::future::Future<Output = Result<String>> + Send {
        let database = Arc::clone(&self.database);
        let args = args.to_vec(); // Clone args to move into async block
        
        async move {
            // GET collection_id item_id
            if args.len() != 2 {
                return Ok(RespResponse::error("ERR wrong number of arguments for 'GET' command"));
            }

            let collection_id = match &args[0] {
                RespValue::BulkString(Some(s)) => s,
                _ => return Ok(RespResponse::error("ERR invalid collection ID")),
            };

            let item_id = match &args[1] {
                RespValue::BulkString(Some(s)) => s,
                _ => return Ok(RespResponse::error("ERR invalid item ID")),
            };

            // 异步从数据库获取
            match database.get(collection_id, item_id).await {
                Ok(Some(item)) => {
                    // 返回 GeoJSON 字符串
                    let geojson_str = item.geojson.to_string();
                    Ok(RespResponse::bulk_string(Some(&geojson_str)))
                }
                Ok(None) => Ok(RespResponse::bulk_string(None)),
                Err(e) => Ok(RespResponse::error(&format!("ERR failed to get: {}", e))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_get_command_success() {
        let database = Arc::new(GeoDatabase::new());
        let point_json = json!({
            "type": "Point",
            "coordinates": [-122.4194, 37.7749]
        });

        // 先存储数据
        database.set("fleet", "truck1", point_json.clone()).await.unwrap();

        let cmd = GetCommand::new(Arc::clone(&database));

        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("truck1".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert!(result.contains("Point"));
        assert!(result.contains("-122.4194"));
    }

    #[tokio::test]
    async fn test_get_command_not_found() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = GetCommand::new(database);

        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("nonexistent".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert_eq!(result, RespResponse::bulk_string(None));
    }

    #[tokio::test]
    async fn test_get_command_invalid_args() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = GetCommand::new(database);

        // 参数太少
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert!(result.contains("wrong number of arguments"));
    }
}
