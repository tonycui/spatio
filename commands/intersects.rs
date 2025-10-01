use crate::protocol::parser::RespValue;
use crate::protocol::RespResponse;
use crate::storage::GeoDatabase;
use crate::commands::{Command, ArgumentParser};
use crate::Result;
use std::sync::Arc;
use serde_json;

pub struct IntersectsCommand {
    database: Arc<GeoDatabase>,
}

impl IntersectsCommand {
    pub fn new(database: Arc<GeoDatabase>) -> Self {
        Self { database }
    }
}

impl Command for IntersectsCommand {
    fn name(&self) -> &'static str {
        "INTERSECTS"
    }

    fn execute(&self, args: &[RespValue]) -> impl std::future::Future<Output = Result<String>> + Send {
        let database = Arc::clone(&self.database);
        
        // 同步解析参数
        let parse_result = ArgumentParser::new(args, "INTERSECTS").parse_intersects_args();
        
        async move {
            // 检查参数解析结果
            let parsed_args = match parse_result {
                Ok(args) => args,
                Err(err_msg) => {
                    return Ok(RespResponse::error(&err_msg));
                }
            };
            
            // 执行空间查询
            match database.intersects(&parsed_args.collection_id, &parsed_args.geometry, parsed_args.limit, parsed_args.within).await {
                Ok(results) => {
                    if results.is_empty() {
                        Ok(RespResponse::array(None))
                    } else {
                        // 优化：预分配容量，避免Vec动态扩容
                        let mut resp_values = Vec::with_capacity(results.len());
                        
                        for item in results {
                            // 优化：直接使用缓存的 GeoJSON 字符串，零序列化开销
                            resp_values.push(RespValue::BulkString(Some(item.geojson)));
                        }
                        
                        Ok(RespResponse::array(Some(&resp_values)))
                    }
                }
                Err(e) => Ok(RespResponse::error(&format!("ERR intersects query failed: {}", e))),
            }
        }
    }
}

/// INTERSECTS 命令的参数结构
#[derive(Debug)]
pub struct IntersectsArgs {
    pub collection_id: String,
    pub geometry: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_intersects_command_success() {
        let database = Arc::new(GeoDatabase::new());
        
        // 添加测试数据
        let point1 = json!({
            "type": "Point",
            "coordinates": [0.0, 0.0]
        });
        let point2 = json!({
            "type": "Point", 
            "coordinates": [5.0, 5.0]
        });
        let point3 = json!({
            "type": "Point",
            "coordinates": [15.0, 15.0]
        });
        
        database.set("fleet", "vehicle1", &point1.to_string()).await.unwrap();
        database.set("fleet", "vehicle2", &point2.to_string()).await.unwrap();
        database.set("fleet", "vehicle3", &point3.to_string()).await.unwrap();

        let cmd = IntersectsCommand::new(Arc::clone(&database));

        // 查询一个包含 point1 和 point2 的矩形区域
        let query_bbox = json!({
            "type": "Polygon",
            "coordinates": [[
                [-1.0, -1.0],
                [6.0, -1.0],
                [6.0, 6.0],
                [-1.0, 6.0],
                [-1.0, -1.0]
            ]]
        });

        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some(query_bbox.to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        
        // 应该包含两个结果
        assert!(result.starts_with("*2\r\n"));
        assert!(result.contains("0.0"));
        assert!(result.contains("5.0"));
        assert!(!result.contains("15.0")); // point3 应该不在结果中
    }

    #[tokio::test]
    async fn test_intersects_command_empty_result() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = IntersectsCommand::new(database);

        // 查询空集合
        let query_bbox = json!({
            "type": "Polygon",
            "coordinates": [[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, 0.0]
            ]]
        });

        let args = vec![
            RespValue::BulkString(Some("empty_fleet".to_string())),
            RespValue::BulkString(Some(query_bbox.to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert_eq!(result, RespResponse::array(None));
    }

    #[tokio::test]
    async fn test_intersects_command_invalid_args() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = IntersectsCommand::new(database);

        // 参数太少
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert!(result.contains("wrong number of arguments"));
    }

    #[tokio::test]
    async fn test_intersects_command_invalid_geometry() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = IntersectsCommand::new(database);

        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("invalid json".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        assert!(result.contains("ERR invalid GeoJSON"));
    }

    #[tokio::test]
    async fn test_intersects_command_with_within_true() {
        let database = Arc::new(GeoDatabase::new());
        
        // 添加测试数据：三个点
        // point1 完全在查询多边形内
        let point1 = json!({
            "type": "Point",
            "coordinates": [2.0, 2.0]
        });
        // point2 在多边形边界上
        let point2 = json!({
            "type": "Point", 
            "coordinates": [5.0, 0.0]
        });
        // point3 在多边形外
        let point3 = json!({
            "type": "Point",
            "coordinates": [15.0, 15.0]
        });
        
        database.set("test", "p1", &point1.to_string()).await.unwrap();
        database.set("test", "p2", &point2.to_string()).await.unwrap();
        database.set("test", "p3", &point3.to_string()).await.unwrap();

        let cmd = IntersectsCommand::new(Arc::clone(&database));

        // 查询多边形
        let query_polygon = json!({
            "type": "Polygon",
            "coordinates": [[
                [0.0, 0.0],
                [10.0, 0.0],
                [10.0, 10.0],
                [0.0, 10.0],
                [0.0, 0.0]
            ]]
        });

        // 使用 WITHIN true 查询 - 只返回完全在内部的点
        let args_within = vec![
            RespValue::BulkString(Some("test".to_string())),
            RespValue::BulkString(Some(query_polygon.to_string())),
            RespValue::BulkString(Some("WITHIN".to_string())),
            RespValue::BulkString(Some("true".to_string())),
        ];

        let result_within = cmd.execute(&args_within).await.unwrap();
        
        // 应该包含 point1 和 point2（边界上的点也算在内）
        assert!(result_within.contains("2.0"));
        
        // 使用 WITHIN false（或默认）查询 - 返回所有相交的点
        let args_intersects = vec![
            RespValue::BulkString(Some("test".to_string())),
            RespValue::BulkString(Some(query_polygon.to_string())),
            RespValue::BulkString(Some("WITHIN".to_string())),
            RespValue::BulkString(Some("false".to_string())),
        ];

        let result_intersects = cmd.execute(&args_intersects).await.unwrap();
        
        // 应该包含 point1 和 point2
        assert!(result_intersects.contains("2.0"));
    }

    #[tokio::test]
    async fn test_intersects_command_with_within_and_limit() {
        let database = Arc::new(GeoDatabase::new());
        
        // 添加多个在多边形内的点
        for i in 1..=5 {
            let point = json!({
                "type": "Point",
                "coordinates": [i as f64, i as f64]
            });
            database.set("test", &format!("p{}", i), &point.to_string()).await.unwrap();
        }

        let cmd = IntersectsCommand::new(Arc::clone(&database));

        // 查询多边形（覆盖所有点）
        let query_polygon = json!({
            "type": "Polygon",
            "coordinates": [[
                [0.0, 0.0],
                [10.0, 0.0],
                [10.0, 10.0],
                [0.0, 10.0],
                [0.0, 0.0]
            ]]
        });

        // 使用 WITHIN true 和 LIMIT 3
        let args = vec![
            RespValue::BulkString(Some("test".to_string())),
            RespValue::BulkString(Some(query_polygon.to_string())),
            RespValue::BulkString(Some("WITHIN".to_string())),
            RespValue::BulkString(Some("true".to_string())),
            RespValue::BulkString(Some("LIMIT".to_string())),
            RespValue::BulkString(Some("3".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();
        
        // 应该最多返回 3 个结果
        // 数组格式是 *3\r\n 开头
        assert!(result.starts_with("*3\r\n") || result.starts_with("*2\r\n") || result.starts_with("*1\r\n"));
    }
}
