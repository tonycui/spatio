use std::sync::Arc;
use crate::commands::Command;
use crate::protocol::{RespResponse, parser::RespValue};
use crate::storage::GeoDatabase;
use crate::Result;

pub struct KeysCommand {
    database: Arc<GeoDatabase>,
}

impl KeysCommand {
    pub fn new(database: Arc<GeoDatabase>) -> Self {
        Self { database }
    }
}

impl Command for KeysCommand {
    fn name(&self) -> &'static str {
        "KEYS"
    }

    fn execute(&self, args: &[RespValue]) -> impl std::future::Future<Output = Result<String>> + Send {
        let database = Arc::clone(&self.database);
        
        async move {
            // KEYS 命令不接受参数
            if !args.is_empty() {
                return Ok(RespResponse::error("ERR wrong number of arguments for 'KEYS' command"));
            }
            
            // 获取所有 collection 名称
            let collection_names = database.collection_names().await;
            
            if collection_names.is_empty() {
                // 返回空数组
                Ok(RespResponse::array(None))
            } else {
                // 将 collection 名称转换为 RespValue
                let resp_values: Vec<RespValue> = collection_names
                    .into_iter()
                    .map(|name| RespValue::BulkString(Some(name)))
                    .collect();
                
                Ok(RespResponse::array(Some(&resp_values)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_keys_command_empty() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = KeysCommand::new(database);

        let args = vec![];

        let result = cmd.execute(&args).await.unwrap();
        // 应该返回空数组
        assert!(result.contains("*0") || result.contains("*-1"));
    }

    #[tokio::test]
    async fn test_keys_command_with_collections() {
        let database = Arc::new(GeoDatabase::new());
        
        // 添加一些数据到不同的 collections
        let point_json = json!({
            "type": "Point",
            "coordinates": [-122.4194, 37.7749]
        });
        database.set("fleet", "truck1", &point_json.to_string()).await.unwrap();
        database.set("zones", "zone1", &point_json.to_string()).await.unwrap();
        database.set("buildings", "bldg1", &point_json.to_string()).await.unwrap();

        let cmd = KeysCommand::new(Arc::clone(&database));

        let args = vec![];

        let result = cmd.execute(&args).await.unwrap();
        
        // 应该返回包含3个元素的数组
        assert!(result.contains("*3"));
        assert!(result.contains("fleet"));
        assert!(result.contains("zones"));
        assert!(result.contains("buildings"));
    }

    #[tokio::test]
    async fn test_keys_command_with_args_error() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = KeysCommand::new(database);

        // KEYS 不应该接受参数
        let args = vec![
            RespValue::BulkString(Some("invalid".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert!(result.contains("wrong number of arguments"));
    }

    #[tokio::test]
    async fn test_keys_command_after_drop() {
        let database = Arc::new(GeoDatabase::new());
        
        // 添加数据
        let point_json = json!({
            "type": "Point",
            "coordinates": [1.0, 2.0]
        });
        database.set("collection1", "item1", &point_json.to_string()).await.unwrap();
        database.set("collection2", "item2", &point_json.to_string()).await.unwrap();

        let cmd = KeysCommand::new(Arc::clone(&database));

        // 验证有2个 collections
        let result = cmd.execute(&[]).await.unwrap();
        assert!(result.contains("*2"));

        // 删除一个 collection
        database.drop_collection("collection1").await.unwrap();

        // 应该只剩1个 collection
        let result = cmd.execute(&[]).await.unwrap();
        assert!(result.contains("*1"));
        assert!(result.contains("collection2"));
        assert!(!result.contains("collection1"));
    }
}
