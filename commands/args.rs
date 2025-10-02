use crate::protocol::parser::RespValue;
use geo::Geometry;
use crate::storage::geometry_utils::geojson_to_geometry;

/// 参数解析工具
pub struct ArgumentParser<'a> {
    args: &'a [RespValue],
    command_name: &'static str,
}

impl<'a> ArgumentParser<'a> {
    pub fn new(args: &'a [RespValue], command_name: &'static str) -> Self {
        Self { args, command_name }
    }

    /// 检查参数数量
    pub fn check_arg_count(&self, expected: usize) -> std::result::Result<(), String> {
        if self.args.len() != expected {
            return Err(format!(
                "ERR wrong number of arguments for '{}' command. Expected {}, got {}",
                self.command_name, expected, self.args.len()
            ));
        }
        Ok(())
    }

    /// 获取字符串参数
    pub fn get_string(&self, index: usize, param_name: &str) -> std::result::Result<&str, String> {
        match self.args.get(index) {
            Some(RespValue::BulkString(Some(s))) => Ok(s),
            Some(_) => Err(format!("ERR invalid {}: expected string", param_name)),
            None => Err(format!("ERR missing {} parameter", param_name)),
        }
    }

    /// 获取并解析 GeoJSON 参数
    pub fn get_geojson(&self, index: usize) -> std::result::Result<serde_json::Value, String> {
        let geojson_str = self.get_string(index, "GeoJSON")?;
        
        // 解析 JSON
        let geojson: serde_json::Value = serde_json::from_str(geojson_str)
            .map_err(|e| format!("ERR invalid GeoJSON: {}", e))?;
        
        // 基本格式验证
        self.validate_geojson(&geojson)?;
        
        Ok(geojson)
    }

    /// 获取并解析 Geometry 参数（直接转换，无中转）
    pub fn get_geometry(&self, index: usize) -> std::result::Result<Geometry, String> {
        let geojson_str = self.get_string(index, "GeoJSON")?;
        
        // 解析为 JSON
        // let geojson_value: serde_json::Value = serde_json::from_str(geojson_str)
        //     .map_err(|e| format!("ERR invalid GeoJSON: {}", e))?;
        
        // // 直接转换为 geo::Geometry
        // geojson_to_geometry(&geojson_value)
        //     .map_err(|e| format!("ERR invalid GeoJSON geometry: {}", e))

            geojson_to_geometry(&geojson_str)
            .map_err(|e| format!("ERR invalid GeoJSON geometry: {}", e))
    }

    /// 验证 GeoJSON 基本格式
    fn validate_geojson(&self, geojson: &serde_json::Value) -> std::result::Result<(), String> {
        if !geojson.is_object() {
            return Err("ERR invalid GeoJSON: must be an object".to_string());
        }
        
        if geojson.get("type").is_none() {
            return Err("ERR invalid GeoJSON: missing 'type' field".to_string());
        }
        
        Ok(())
    }

    /// 解析 SET 命令的参数
    pub fn parse_set_args(&self) -> std::result::Result<SetArgs, String> {
        self.check_arg_count(3)?;
        
        let collection_id = self.get_string(0, "collection ID")?;
        let item_id = self.get_string(1, "item ID")?;
        let geojson = self.get_string(2, "GeoJSON")?;
        
        Ok(SetArgs {
            collection_id: collection_id.to_string(),
            item_id: item_id.to_string(),
            geojson: geojson.to_string(),
        })
    }

    /// 解析 GET 命令的参数
    pub fn parse_get_args(&self) -> std::result::Result<GetArgs, String> {
        self.check_arg_count(2)?;
        
        let collection_id = self.get_string(0, "collection ID")?;
        let item_id = self.get_string(1, "item ID")?;
        
        Ok(GetArgs {
            collection_id: collection_id.to_string(),
            item_id: item_id.to_string(),
        })
    }

    /// 解析 INTERSECTS 命令的参数
    /// 语法: INTERSECTS collection geojson [WITHIN true|false] [LIMIT n]
    pub fn parse_intersects_args(&self) -> std::result::Result<IntersectsArgs, String> {
        // 至少需要2个参数: collection 和 geojson
        if self.args.len() < 2 {
            return Err(format!(
                "ERR wrong number of arguments for 'INTERSECTS' command. Expected at least 2, got {}",
                self.args.len()
            ));
        }
        
        let collection_id = self.get_string(0, "collection ID")?;
        let geometry = self.get_geometry(1)?;
        
        // 解析可选参数: WITHIN 和 LIMIT
        let mut within = false; // 默认为 false (相交查询)
        let mut limit = 0; // 默认无限制
        
        let mut i = 2;
        while i < self.args.len() {
            let key = self.get_string(i, "option key")?.to_uppercase();
            
            match key.as_str() {
                "WITHIN" => {
                    if i + 1 >= self.args.len() {
                        return Err("ERR WITHIN option requires a value (true or false)".to_string());
                    }
                    let value = self.get_string(i + 1, "WITHIN value")?;
                    within = match value.to_lowercase().as_str() {
                        "true" | "1" | "yes" => true,
                        "false" | "0" | "no" => false,
                        _ => return Err(format!("ERR invalid WITHIN value: expected true or false, got {}", value)),
                    };
                    i += 2;
                }
                "LIMIT" => {
                    if i + 1 >= self.args.len() {
                        return Err("ERR LIMIT option requires a value".to_string());
                    }
                    limit = self.get_integer(i + 1, "LIMIT value")?;
                    i += 2;
                }
                _ => {
                    // 向后兼容: 如果只有3个参数且第3个是数字，当作 limit
                    if self.args.len() == 3 && i == 2 {
                        if let Ok(parsed_limit) = self.get_integer(2, "limit") {
                            limit = parsed_limit;
                            break;
                        }
                    }
                    return Err(format!("ERR unknown option '{}' for INTERSECTS command", key));
                }
            }
        }

        Ok(IntersectsArgs {
            collection_id: collection_id.to_string(),
            geometry,
            limit,
            within,
        })
    }

    /// 获取整数参数
    pub fn get_integer(&self, index: usize, param_name: &str) -> std::result::Result<usize, String> {
        let str_val = self.get_string(index, param_name)?;
        str_val.parse::<usize>()
            .map_err(|_| format!("ERR invalid {}: expected positive integer", param_name))
    }

    /// 解析 DROP 命令的参数
    pub fn parse_drop_args(&self) -> std::result::Result<DropArgs, String> {
        self.check_arg_count(1)?;
        
        let collection_id = self.get_string(0, "collection ID")?;
        
        Ok(DropArgs {
            collection_id: collection_id.to_string(),
        })
    }

    /// 解析 NEARBY 命令的参数
    /// 语法: NEARBY collection POINT lon lat COUNT k
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// NEARBY fleet POINT 116.4 39.9 COUNT 10
    /// ```
    pub fn parse_nearby_args(&self) -> std::result::Result<NearbyArgs, String> {
        // 至少需要 6 个参数: collection, POINT, lon, lat, COUNT, k
        if self.args.len() < 6 {
            return Err(format!(
                "ERR wrong number of arguments for 'NEARBY' command. Expected at least 6, got {}. Usage: NEARBY collection POINT lon lat COUNT k",
                self.args.len()
            ));
        }

        // 解析 collection ID
        let collection_id = self.get_string(0, "collection ID")?;

        // 验证 POINT 关键字
        let point_keyword = self.get_string(1, "POINT keyword")?;
        if point_keyword.to_uppercase() != "POINT" {
            return Err(format!(
                "ERR invalid syntax: expected 'POINT', got '{}'",
                point_keyword
            ));
        }

        // 解析经度和纬度
        let lon_str = self.get_string(2, "longitude")?;
        let lat_str = self.get_string(3, "latitude")?;

        let query_lon: f64 = lon_str.parse()
            .map_err(|_| format!("ERR invalid longitude: expected number, got '{}'", lon_str))?;
        let query_lat: f64 = lat_str.parse()
            .map_err(|_| format!("ERR invalid latitude: expected number, got '{}'", lat_str))?;

        // 验证经纬度范围
        if query_lon < -180.0 || query_lon > 180.0 {
            return Err(format!("ERR invalid longitude: must be between -180 and 180, got {}", query_lon));
        }
        if query_lat < -90.0 || query_lat > 90.0 {
            return Err(format!("ERR invalid latitude: must be between -90 and 90, got {}", query_lat));
        }

        // 验证 COUNT 关键字
        let count_keyword = self.get_string(4, "COUNT keyword")?;
        if count_keyword.to_uppercase() != "COUNT" {
            return Err(format!(
                "ERR invalid syntax: expected 'COUNT', got '{}'",
                count_keyword
            ));
        }

        // 解析 k（返回结果数量）
        let k = self.get_integer(5, "count")?;
        if k == 0 {
            return Err("ERR count must be greater than 0".to_string());
        }

        Ok(NearbyArgs {
            collection_id: collection_id.to_string(),
            query_lon,
            query_lat,
            k,
        })
    }
    
}

/// SET 命令的解析结果
#[derive(Debug)]
pub struct SetArgs {
    pub collection_id: String,
    pub item_id: String,
    pub geojson: String,
}

/// GET 命令的解析结果
#[derive(Debug)]
pub struct GetArgs {
    pub collection_id: String,
    pub item_id: String,
}

/// INTERSECTS 命令的解析结果
#[derive(Debug)]
pub struct IntersectsArgs {
    pub collection_id: String,
    pub geometry: Geometry,
    pub limit: usize,
    pub within: bool,  // true: 包含在内，false: 相交
}

/// DROP 命令的解析结果
#[derive(Debug)]
pub struct DropArgs {
    pub collection_id: String,
}

/// NEARBY 命令的解析结果
#[derive(Debug)]
pub struct NearbyArgs {
    pub collection_id: String,
    pub query_lon: f64,
    pub query_lat: f64,
    pub k: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_argument_parser_set_success() {
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("truck1".to_string())),
            RespValue::BulkString(Some(json!({"type": "Point", "coordinates": [1.0, 2.0]}).to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "SET");
        let result = parser.parse_set_args();
        
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.collection_id, "fleet");
        assert_eq!(parsed.item_id, "truck1");
        // 验证 geojson 字符串而不是 geometry
        assert!(parsed.geojson.contains("Point"));
        assert!(parsed.geojson.contains("1.0"));
        assert!(parsed.geojson.contains("2.0"));
    }

    #[test]
    fn test_argument_parser_get_success() {
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("truck1".to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "GET");
        let result = parser.parse_get_args();
        
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.collection_id, "fleet");
        assert_eq!(parsed.item_id, "truck1");
    }

    #[test]
    fn test_argument_parser_invalid_arg_count() {
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "SET");
        let result = parser.parse_set_args();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("wrong number of arguments"));
    }

    #[test]
    fn test_argument_parser_invalid_geojson() {
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("truck1".to_string())),
            RespValue::BulkString(Some("invalid json".to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "SET");
        let result = parser.parse_set_args();
        
        // 现在 parse_set_args 只获取字符串，不验证 GeoJSON 格式
        // 验证会在后续的存储过程中进行
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.geojson, "invalid json");
    }

    #[test]
    fn test_get_geometry_point() {
        let args = vec![
            RespValue::BulkString(Some(json!({"type": "Point", "coordinates": [10.5, 20.7]}).to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "TEST");
        let result = parser.get_geometry(0);
        
        assert!(result.is_ok());
        let geometry = result.unwrap();
        match geometry {
            Geometry::Point(point) => {
                assert_eq!(point.x(), 10.5);
                assert_eq!(point.y(), 20.7);
            }
            _ => panic!("Expected Point geometry"),
        }
    }

    #[test]
    fn test_get_geometry_polygon() {
        let polygon_geojson = json!({
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
            RespValue::BulkString(Some(polygon_geojson.to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "TEST");
        let result = parser.get_geometry(0);
        
        assert!(result.is_ok());
        let geometry = result.unwrap();
        match geometry {
            Geometry::Polygon(polygon) => {
                assert_eq!(polygon.exterior().coords().count(), 5); // 包括闭合点
            }
            _ => panic!("Expected Polygon geometry"),
        }
    }

    #[test]
    fn test_get_geometry_feature() {
        let feature_geojson = json!({
            "type": "Feature",
            "geometry": {
                "type": "Point",
                "coordinates": [5.5, 15.3]
            },
            "properties": {
                "name": "test point"
            }
        });
        
        let args = vec![
            RespValue::BulkString(Some(feature_geojson.to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "TEST");
        let result = parser.get_geometry(0);
        
        assert!(result.is_ok());
        let geometry = result.unwrap();
        match geometry {
            Geometry::Point(point) => {
                assert_eq!(point.x(), 5.5);
                assert_eq!(point.y(), 15.3);
            }
            _ => panic!("Expected Point geometry"),
        }
    }

    #[test]
    fn test_get_geometry_invalid_json() {
        let args = vec![
            RespValue::BulkString(Some("invalid json string".to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "TEST");
        let result = parser.get_geometry(0);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid GeoJSON geometry"));
    }

    #[test]
    fn test_get_geometry_missing_parameter() {
        let args = vec![];
        
        let parser = ArgumentParser::new(&args, "TEST");
        let result = parser.get_geometry(0);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing GeoJSON parameter"));
    }

    #[test]
    fn test_get_geometry_invalid_geometry_type() {
        let invalid_geojson = json!({
            "type": "InvalidType",
            "coordinates": [1.0, 2.0]
        });
        
        let args = vec![
            RespValue::BulkString(Some(invalid_geojson.to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "TEST");
        let result = parser.get_geometry(0);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid GeoJSON geometry"));
    }

    #[test]
    fn test_parse_nearby_args_success() {
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("116.4".to_string())),
            RespValue::BulkString(Some("39.9".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("10".to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "NEARBY");
        let result = parser.parse_nearby_args();
        
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.collection_id, "fleet");
        assert_eq!(parsed.query_lon, 116.4);
        assert_eq!(parsed.query_lat, 39.9);
        assert_eq!(parsed.k, 10);
    }

    #[test]
    fn test_parse_nearby_args_invalid_longitude() {
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("200.0".to_string())), // 无效经度
            RespValue::BulkString(Some("39.9".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("10".to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "NEARBY");
        let result = parser.parse_nearby_args();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid longitude"));
    }

    #[test]
    fn test_parse_nearby_args_missing_point_keyword() {
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("NOTPOINT".to_string())), // 错误关键字
            RespValue::BulkString(Some("116.4".to_string())),
            RespValue::BulkString(Some("39.9".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("10".to_string())),
        ];
        
        let parser = ArgumentParser::new(&args, "NEARBY");
        let result = parser.parse_nearby_args();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected 'POINT'"));
    }

    #[test]
    fn test_parse_nearby_args_zero_count() {
        let args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("116.4".to_string())),
            RespValue::BulkString(Some("39.9".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("0".to_string())), // k = 0
        ];
        
        let parser = ArgumentParser::new(&args, "NEARBY");
        let result = parser.parse_nearby_args();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("count must be greater than 0"));
    }
}
