use crate::commands::args::ArgumentParser;
use crate::commands::Command;
use crate::protocol::{parser::RespValue, RespResponse};
use crate::storage::GeoDatabase;
use crate::Result;
use std::sync::Arc;

pub struct DeleteCommand {
    database: Arc<GeoDatabase>,
}

impl DeleteCommand {
    pub fn new(database: Arc<GeoDatabase>) -> Self {
        Self { database }
    }
}

impl Command for DeleteCommand {
    fn name(&self) -> &'static str {
        "DELETE"
    }

    fn execute(
        &self,
        args: &[RespValue],
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let database = Arc::clone(&self.database);

        // 同步解析参数
        let parse_result = ArgumentParser::new(args, "DELETE").parse_delete_args();

        async move {
            // 检查参数解析结果
            let parsed_args = match parse_result {
                Ok(args) => args,
                Err(err_msg) => {
                    return Ok(RespResponse::error(&err_msg));
                }
            };

            // 调用数据库的 delete 方法
            match database
                .delete(&parsed_args.collection_id, &parsed_args.item_id)
                .await
            {
                Ok(true) => {
                    // 成功删除，返回 1
                    Ok(RespResponse::integer(1))
                }
                Ok(false) => {
                    // 未找到项目，返回 0
                    Ok(RespResponse::integer(0))
                }
                Err(e) => Ok(RespResponse::error(&format!("ERR failed to delete: {}", e))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_delete_command_success() {
        let database = Arc::new(GeoDatabase::new());
        let point_json = json!({
            "type": "Point",
            "coordinates": [-122.4194, 37.7749]
        });

        // 先存储数据
        database
            .set("fleet", "truck1", &point_json.to_string())
            .await
            .unwrap();

        let cmd = DeleteCommand::new(Arc::clone(&database));

        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("truck1".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert_eq!(result, RespResponse::integer(1));

        // 验证已删除
        let item = database.get("fleet", "truck1").await.unwrap();
        assert!(item.is_none());
    }

    #[tokio::test]
    async fn test_delete_command_not_found() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = DeleteCommand::new(database);

        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("nonexistent".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert_eq!(result, RespResponse::integer(0));
    }

    #[tokio::test]
    async fn test_delete_command_invalid_args() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = DeleteCommand::new(database);

        // 参数太少
        let args = vec![RespValue::BulkString(Some("fleet".to_string()))];

        let result = cmd.execute(&args).await.unwrap();
        assert!(result.contains("ERR"));
    }

    #[tokio::test]
    async fn test_delete_command_empty_args() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = DeleteCommand::new(database);

        let args = vec![];
        let result = cmd.execute(&args).await.unwrap();
        assert!(result.contains("ERR"));
    }

    #[tokio::test]
    async fn test_delete_command_twice() {
        let database = Arc::new(GeoDatabase::new());
        let point_json = json!({
            "type": "Point",
            "coordinates": [-122.4194, 37.7749]
        });

        // 先存储数据
        database
            .set("fleet", "truck1", &point_json.to_string())
            .await
            .unwrap();

        let cmd = DeleteCommand::new(Arc::clone(&database));

        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("truck1".to_string())),
        ];

        // 第一次删除，应该返回 1
        let result = cmd.execute(&args).await.unwrap();
        assert_eq!(result, RespResponse::integer(1));

        // 第二次删除同一个 item，应该返回 0
        let result = cmd.execute(&args).await.unwrap();
        assert_eq!(result, RespResponse::integer(0));
    }
}
