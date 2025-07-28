use crate::protocol::parser::RespValue;

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
        let geojson = self.get_geojson(2)?;
        
        Ok(SetArgs {
            collection_id: collection_id.to_string(),
            item_id: item_id.to_string(),
            geojson,
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
}

/// SET 命令的解析结果
#[derive(Debug)]
pub struct SetArgs {
    pub collection_id: String,
    pub item_id: String,
    pub geojson: serde_json::Value,
}

/// GET 命令的解析结果
#[derive(Debug)]
pub struct GetArgs {
    pub collection_id: String,
    pub item_id: String,
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
        assert_eq!(parsed.geojson["type"], "Point");
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
