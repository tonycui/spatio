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
        let geojson_value: serde_json::Value = serde_json::from_str(geojson_str)
            .map_err(|e| format!("ERR invalid GeoJSON: {}", e))?;
        
        // 直接转换为 geo::Geometry
        geojson_to_geometry(&geojson_value)
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
        let geometry = self.get_geometry(2)?;
        
        Ok(SetArgs {
            collection_id: collection_id.to_string(),
            item_id: item_id.to_string(),
            geometry,
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
    pub fn parse_intersects_args(&self) -> std::result::Result<IntersectsArgs, String> {
        // 支持2个或3个参数
        if self.args.len() < 2 || self.args.len() > 3 {
            return Err(format!(
                "ERR wrong number of arguments for 'INTERSECTS' command. Expected 2 or 3, got {}",
                self.args.len()
            ));
        }
        
        let collection_id = self.get_string(0, "collection ID")?;
        let geometry = self.get_geometry(1)?;
        
        // 解析可选的 limit 参数
        let limit = if self.args.len() == 3 {
            self.get_integer(2, "limit")?
        } else {
            0 // 默认值，表示无限制
        };

        Ok(IntersectsArgs {
            collection_id: collection_id.to_string(),
            geometry,
            limit,
        })
    }

    /// 获取整数参数
    pub fn get_integer(&self, index: usize, param_name: &str) -> std::result::Result<usize, String> {
        let str_val = self.get_string(index, param_name)?;
        str_val.parse::<usize>()
            .map_err(|_| format!("ERR invalid {}: expected positive integer", param_name))
    }
    
}

/// SET 命令的解析结果
#[derive(Debug)]
pub struct SetArgs {
    pub collection_id: String,
    pub item_id: String,
    pub geometry: Geometry,
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
        // 验证 geometry 而不是 geojson
        match parsed.geometry {
            Geometry::Point(_) => {},
            _ => panic!("Expected Point geometry"),
        }
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
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid GeoJSON"));
    }
}
