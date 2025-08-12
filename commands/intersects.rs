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
            match database.intersects(&parsed_args.collection_id, &parsed_args.geometry, parsed_args.limit).await {
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
        
        // 转换为 geo::Geometry
        use crate::storage::geometry_utils::geojson_to_geometry;
        let geom1 = geojson_to_geometry(&point1).unwrap();
        let geom2 = geojson_to_geometry(&point2).unwrap();
        let geom3 = geojson_to_geometry(&point3).unwrap();
        
        database.set("fleet", "vehicle1", geom1).await.unwrap();
        database.set("fleet", "vehicle2", geom2).await.unwrap();
        database.set("fleet", "vehicle3", geom3).await.unwrap();

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
}
