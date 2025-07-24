use std::sync::Arc;
use crate::commands::Command;
use crate::protocol::{RespResponse, parser::RespValue};
use crate::storage::GeoDatabase;
use crate::Result;

pub struct SetCommand {
    database: Arc<GeoDatabase>,
}

impl SetCommand {
    pub fn new(database: Arc<GeoDatabase>) -> Self {
        Self { database }
    }
}

impl Command for SetCommand {
    fn name(&self) -> &'static str {
        "SET"
    }

    fn execute(&self, args: &[RespValue]) -> impl std::future::Future<Output = Result<String>> + Send {
        let database = Arc::clone(&self.database);
        let args = args.to_vec(); // Clone args to move into async block
        
        async move {
            // SET collection_id item_id geojson
            if args.len() != 3 {
                return Ok(RespResponse::error("ERR wrong number of arguments for 'SET' command"));
            }

            let collection_id = match &args[0] {
                RespValue::BulkString(Some(s)) => s,
                _ => return Ok(RespResponse::error("ERR invalid collection ID")),
            };

            let item_id = match &args[1] {
                RespValue::BulkString(Some(s)) => s,
                _ => return Ok(RespResponse::error("ERR invalid item ID")),
            };

            let geojson_str = match &args[2] {
                RespValue::BulkString(Some(s)) => s,
                _ => return Ok(RespResponse::error("ERR invalid GeoJSON")),
            };

            // 解析 GeoJSON
            let geojson: serde_json::Value = match serde_json::from_str(geojson_str) {
                Ok(json) => json,
                Err(e) => return Ok(RespResponse::error(&format!("ERR invalid GeoJSON: {}", e))),
            };

            // 基本 GeoJSON 格式验证
            if !geojson.is_object() || geojson.get("type").is_none() {
                return Ok(RespResponse::error("ERR invalid GeoJSON: missing 'type' field"));
            }

            // 异步存储到数据库
            match database.set(collection_id, item_id, geojson).await {
                Ok(_) => Ok(RespResponse::simple_string("OK")),
                Err(e) => Ok(RespResponse::error(&format!("ERR failed to store: {}", e))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_set_command_success() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = SetCommand::new(Arc::clone(&database));

        let point_json = json!({
            "type": "Point",
            "coordinates": [-122.4194, 37.7749]
        });

        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("truck1".to_string())),
            RespValue::BulkString(Some(point_json.to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert_eq!(result, RespResponse::simple_string("OK"));

        // 验证数据已存储
        let item_result = database.get("fleet", "truck1").await.unwrap();
        let item = item_result.unwrap();
        assert_eq!(item.id, "truck1");
        assert_eq!(item.geojson["type"], "Point");
    }

    #[tokio::test]
    async fn test_set_command_invalid_args() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = SetCommand::new(database);

        // 参数太少
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("truck1".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert!(result.contains("wrong number of arguments"));
    }

    #[tokio::test]
    async fn test_set_command_invalid_geojson() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = SetCommand::new(database);

        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("truck1".to_string())),
            RespValue::BulkString(Some("invalid json".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert!(result.contains("invalid GeoJSON"));
    }
}
