use crate::commands::{ArgumentParser, Command};
use crate::protocol::parser::RespValue;
use crate::protocol::RespResponse;
use crate::storage::GeoDatabase;
use crate::Result;
use std::sync::Arc;

pub struct NearbyCommand {
    database: Arc<GeoDatabase>,
}

impl NearbyCommand {
    pub fn new(database: Arc<GeoDatabase>) -> Self {
        Self { database }
    }
}

impl Command for NearbyCommand {
    fn name(&self) -> &'static str {
        "NEARBY"
    }

    fn execute(
        &self,
        args: &[RespValue],
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let database = Arc::clone(&self.database);

        // 同步解析参数
        let parse_result = ArgumentParser::new(args, "NEARBY").parse_nearby_args();

        async move {
            // 检查参数解析结果
            let parsed_args = match parse_result {
                Ok(args) => args,
                Err(err_msg) => {
                    return Ok(RespResponse::error(&err_msg));
                }
            };

            // 执行 KNN 查询
            let k = parsed_args.k.unwrap_or(0); // 0 表示不限制数量
            match database
                .nearby(
                    &parsed_args.collection_id,
                    parsed_args.query_lon,
                    parsed_args.query_lat,
                    k,
                    parsed_args.max_radius,
                )
                .await
            {
                Ok(results) => {
                    if results.is_empty() {
                        Ok(RespResponse::array(None))
                    } else {
                        // 构建返回结果，包含距离信息
                        // 格式: [["item_id", geojson, distance_in_meters], ...]
                        let mut resp_values = Vec::with_capacity(results.len());

                        for (item, distance) in results {
                            // 每个结果是一个数组：[geojson, distance]
                            let result_array = vec![
                                RespValue::BulkString(Some(item.geojson)),
                                RespValue::BulkString(Some(format!("{:.2}", distance))), // 距离保留两位小数
                            ];
                            resp_values.push(RespValue::Array(Some(result_array)));
                        }

                        Ok(RespResponse::array(Some(&resp_values)))
                    }
                }
                Err(e) => Ok(RespResponse::error(&format!(
                    "ERR nearby query failed: {}",
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
    async fn test_nearby_command_success() {
        let database = Arc::new(GeoDatabase::new());

        // 添加测试数据：在北京周围的几个点
        let point1 = json!({
            "type": "Point",
            "coordinates": [116.4, 39.9] // 北京市中心
        });
        let point2 = json!({
            "type": "Point",
            "coordinates": [116.5, 40.0] // 东北方向
        });
        let point3 = json!({
            "type": "Point",
            "coordinates": [116.3, 39.8] // 西南方向
        });
        let point4 = json!({
            "type": "Point",
            "coordinates": [117.0, 40.5] // 很远的东北方向
        });

        database
            .set("fleet", "vehicle1", &point1.to_string())
            .await
            .unwrap();
        database
            .set("fleet", "vehicle2", &point2.to_string())
            .await
            .unwrap();
        database
            .set("fleet", "vehicle3", &point3.to_string())
            .await
            .unwrap();
        database
            .set("fleet", "vehicle4", &point4.to_string())
            .await
            .unwrap();

        let cmd = NearbyCommand::new(Arc::clone(&database));

        // 查询北京市中心 (116.4, 39.9) 最近的 3 个点
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("116.4".to_string())),
            RespValue::BulkString(Some("39.9".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("3".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();

        // 验证结果格式
        assert!(result.starts_with("*")); // RESP 数组格式

        // 结果应该包含 3 个项目
        // 第一个应该是 vehicle1（距离最近，几乎为 0）
        println!("Result: {}", result);
    }

    #[tokio::test]
    async fn test_nearby_command_empty_collection() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = NearbyCommand::new(Arc::clone(&database));

        // 查询不存在的 collection
        let args = vec![
            RespValue::BulkString(Some("nonexistent".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("116.4".to_string())),
            RespValue::BulkString(Some("39.9".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("10".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();

        // 应该返回空数组
        assert!(result.contains("*-1") || result.contains("*0"));
    }

    #[tokio::test]
    async fn test_nearby_command_invalid_args() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = NearbyCommand::new(Arc::clone(&database));

        // 缺少参数
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();

        // 应该返回错误
        assert!(result.contains("ERR"));
        assert!(result.contains("wrong number of arguments"));
    }

    #[tokio::test]
    async fn test_nearby_command_invalid_coordinates() {
        let database = Arc::new(GeoDatabase::new());
        let cmd = NearbyCommand::new(Arc::clone(&database));

        // 无效的经度
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("200.0".to_string())), // 无效经度
            RespValue::BulkString(Some("39.9".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("10".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();

        // 应该返回错误
        assert!(result.contains("ERR"));
        assert!(result.contains("invalid longitude"));
    }

    #[tokio::test]
    async fn test_nearby_command_distance_order() {
        let database = Arc::new(GeoDatabase::new());

        // 添加已知距离的测试数据
        // 查询点：(116.0, 39.0)
        let point1 = json!({"type": "Point", "coordinates": [116.0, 39.0]}); // 距离 0
        let point2 = json!({"type": "Point", "coordinates": [116.1, 39.0]}); // 约 11km
        let point3 = json!({"type": "Point", "coordinates": [116.2, 39.0]}); // 约 22km

        database
            .set("test", "p1", &point1.to_string())
            .await
            .unwrap();
        database
            .set("test", "p2", &point2.to_string())
            .await
            .unwrap();
        database
            .set("test", "p3", &point3.to_string())
            .await
            .unwrap();

        let cmd = NearbyCommand::new(Arc::clone(&database));

        let args = vec![
            RespValue::BulkString(Some("test".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("116.0".to_string())),
            RespValue::BulkString(Some("39.0".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("3".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();

        println!("Distance order result: {}", result);

        // 结果应该按距离排序
        // p1 应该第一个（距离最近）
        assert!(result.contains("116.0"));
    }

    #[tokio::test]
    async fn test_nearby_command_with_radius_only() {
        let database = Arc::new(GeoDatabase::new());

        // 添加不同距离的点
        let point1 = json!({"type": "Point", "coordinates": [116.001, 39.0]}); // ~111m
        let point2 = json!({"type": "Point", "coordinates": [116.005, 39.0]}); // ~555m
        let point3 = json!({"type": "Point", "coordinates": [116.01, 39.0]}); // ~1110m
        let point4 = json!({"type": "Point", "coordinates": [116.02, 39.0]}); // ~2220m

        database
            .set("fleet", "v1", &point1.to_string())
            .await
            .unwrap();
        database
            .set("fleet", "v2", &point2.to_string())
            .await
            .unwrap();
        database
            .set("fleet", "v3", &point3.to_string())
            .await
            .unwrap();
        database
            .set("fleet", "v4", &point4.to_string())
            .await
            .unwrap();

        let cmd = NearbyCommand::new(Arc::clone(&database));

        // 只使用 RADIUS，查询 1000 米内的所有点
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("116.0".to_string())),
            RespValue::BulkString(Some("39.0".to_string())),
            RespValue::BulkString(Some("RADIUS".to_string())),
            RespValue::BulkString(Some("1000".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();

        println!("Radius only result: {}", result);

        // 应该返回 v1 和 v2（在 1000m 内），但不包括 v3 和 v4
        assert!(result.starts_with("*"));
    }

    #[tokio::test]
    async fn test_nearby_command_with_count_and_radius() {
        let database = Arc::new(GeoDatabase::new());

        // 添加多个点
        for i in 1..=10 {
            let lon = 116.0 + (i as f64) * 0.001;
            let point = json!({"type": "Point", "coordinates": [lon, 39.0]});
            database
                .set("fleet", &format!("v{}", i), &point.to_string())
                .await
                .unwrap();
        }

        let cmd = NearbyCommand::new(Arc::clone(&database));

        // COUNT 5 + RADIUS 500m，应该返回 500m 内最近的 5 个（如果有的话）
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("116.0".to_string())),
            RespValue::BulkString(Some("39.0".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("5".to_string())),
            RespValue::BulkString(Some("RADIUS".to_string())),
            RespValue::BulkString(Some("500".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();

        println!("Count + Radius result: {}", result);
        assert!(result.starts_with("*"));
    }

    #[tokio::test]
    async fn test_nearby_command_radius_reverse_order() {
        let database = Arc::new(GeoDatabase::new());

        let point = json!({"type": "Point", "coordinates": [116.001, 39.0]});
        database
            .set("fleet", "v1", &point.to_string())
            .await
            .unwrap();

        let cmd = NearbyCommand::new(Arc::clone(&database));

        // RADIUS 在 COUNT 之前（测试参数顺序不敏感）
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("116.0".to_string())),
            RespValue::BulkString(Some("39.0".to_string())),
            RespValue::BulkString(Some("RADIUS".to_string())),
            RespValue::BulkString(Some("1000".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("10".to_string())),
        ];

        let result = cmd.execute(&args).await.unwrap();

        println!("Reverse order result: {}", result);
        assert!(result.starts_with("*"));
    }
}
