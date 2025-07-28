use std::sync::Arc;
use crate::commands::Command;
use crate::commands::args::ArgumentParser;
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
        
        // 同步解析参数（不需要在 async 块中）
        let parse_result = ArgumentParser::new(args, "SET").parse_set_args();
        
        async move {
            // 检查参数解析结果
            let parsed_args = match parse_result {
                Ok(args) => args,
                Err(err_msg) => {
                    return Ok(RespResponse::error(&err_msg));
                }
            };
            
            // 只有 I/O 操作需要异步
            match database.set(&parsed_args.collection_id, &parsed_args.item_id, parsed_args.geojson).await {
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
