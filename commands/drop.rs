use crate::commands::args::ArgumentParser;
use crate::commands::Command;
use crate::protocol::{parser::RespValue, RespResponse};
use crate::storage::GeoDatabase;
use crate::Result;
use std::sync::Arc;

pub struct DropCommand {
    database: Arc<GeoDatabase>,
}

impl DropCommand {
    pub fn new(database: Arc<GeoDatabase>) -> Self {
        Self { database }
    }
}

impl Command for DropCommand {
    fn name(&self) -> &'static str {
        "DROP"
    }

    fn execute(
        &self,
        args: &[RespValue],
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let database = Arc::clone(&self.database);

        // 同步解析参数
        let parse_result = ArgumentParser::new(args, "DROP").parse_drop_args();

        async move {
            // 检查参数解析结果
            let parsed_args = match parse_result {
                Ok(args) => args,
                Err(err_msg) => {
                    return Ok(RespResponse::error(&err_msg));
                }
            };

            // 执行删除 collection 操作
            match database.drop_collection(&parsed_args.collection_id).await {
                Ok(count) => {
                    // 返回删除的项目数量
                    Ok(RespResponse::integer(count as i64))
                }
                Err(e) => Ok(RespResponse::error(&format!(
                    "ERR failed to drop collection: {}",
                    e
                ))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_drop_command_success() {
        let database = Arc::new(GeoDatabase::new());

        // 先添加一些数据
        let point_json = json!({
            "type": "Point",
            "coordinates": [-122.4194, 37.7749]
        });
        database
            .set("fleet", "truck1", &point_json.to_string())
            .await
            .unwrap();
        database
            .set("fleet", "truck2", &point_json.to_string())
            .await
            .unwrap();

        let cmd = DropCommand::new(Arc::clone(&database));

        let args = vec![RespValue::BulkString(Some("fleet".to_string()))];

        let result = cmd.execute(&args).await.unwrap();
        // 应该返回删除的项目数量（2个）
        assert!(result.contains("2"));

        // 验证数据已被删除
        let item_result = database.get("fleet", "truck1").await.unwrap();
        assert!(item_result.is_none());
    }

    #[tokio::test]
    async fn test_drop_command_empty_collection() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = DropCommand::new(database);

        let args = vec![RespValue::BulkString(Some("nonexistent".to_string()))];

        let result = cmd.execute(&args).await.unwrap();
        // 删除不存在的集合应该返回0
        assert!(result.contains("0"));
    }

    #[tokio::test]
    async fn test_drop_command_invalid_args() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = DropCommand::new(database);

        // 参数太少
        let args = vec![];

        let result = cmd.execute(&args).await.unwrap();
        assert!(result.contains("wrong number of arguments"));
    }
}
