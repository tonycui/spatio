use rtree::Rectangle;
use crate::Result;

/// 从 GeoJSON 中提取边界框
pub fn extract_bbox(geojson: &serde_json::Value) -> Result<Rectangle> {
    match geojson.get("type") {
        Some(serde_json::Value::String(geom_type)) => {
            match geom_type.as_str() {
                "Point" => extract_point_bbox(geojson),
                "LineString" | "MultiPoint" => extract_linestring_bbox(geojson),
                "Polygon" | "MultiLineString" => extract_polygon_bbox(geojson),
                "MultiPolygon" => extract_multipolygon_bbox(geojson),
                "GeometryCollection" => extract_geometry_collection_bbox(geojson),
                "Feature" => {
                    // 处理 Feature 类型，提取其 geometry 部分
                    if let Some(geometry) = geojson.get("geometry") {
                        extract_bbox(geometry)
                    } else {
                        Err("Feature missing geometry".into())
                    }
                }
                _ => Err(format!("Unsupported geometry type: {}", geom_type).into())
            }
        }
        _ => Err("Missing or invalid geometry type".into())
    }
}

/// 从 Point 几何体提取边界框
fn extract_point_bbox(geojson: &serde_json::Value) -> Result<Rectangle> {
    if let Some(coords) = geojson.get("coordinates").and_then(|c| c.as_array()) {
        if coords.len() >= 2 {
            let x = coords[0].as_f64().ok_or("Invalid X coordinate")?;
            let y = coords[1].as_f64().ok_or("Invalid Y coordinate")?;
            return Ok(Rectangle::from_point(x, y));
        }
    }
    Err("Invalid Point coordinates".into())
}

/// 从 LineString/MultiPoint 几何体提取边界框
fn extract_linestring_bbox(geojson: &serde_json::Value) -> Result<Rectangle> {
    if let Some(coords) = geojson.get("coordinates").and_then(|c| c.as_array()) {
        extract_bbox_from_coords_array(coords)
    } else {
        Err("Invalid coordinates for LineString/MultiPoint".into())
    }
}

/// 从 Polygon/MultiLineString 几何体提取边界框
fn extract_polygon_bbox(geojson: &serde_json::Value) -> Result<Rectangle> {
    if let Some(coords) = geojson.get("coordinates").and_then(|c| c.as_array()) {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        
        for ring in coords {
            if let Some(ring_coords) = ring.as_array() {
                let bbox = extract_bbox_from_coords_array(ring_coords)?;
                min_x = min_x.min(bbox.min[0]);
                min_y = min_y.min(bbox.min[1]);
                max_x = max_x.max(bbox.max[0]);
                max_y = max_y.max(bbox.max[1]);
            }
        }
        
        if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
            Ok(Rectangle::new(min_x, min_y, max_x, max_y))
        } else {
            Err("Invalid coordinates for Polygon/MultiLineString".into())
        }
    } else {
        Err("Invalid coordinates for Polygon/MultiLineString".into())
    }
}

/// 从 MultiPolygon 几何体提取边界框
fn extract_multipolygon_bbox(geojson: &serde_json::Value) -> Result<Rectangle> {
    if let Some(coords) = geojson.get("coordinates").and_then(|c| c.as_array()) {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        
        for polygon in coords {
            if let Some(polygon_coords) = polygon.as_array() {
                for ring in polygon_coords {
                    if let Some(ring_coords) = ring.as_array() {
                        let bbox = extract_bbox_from_coords_array(ring_coords)?;
                        min_x = min_x.min(bbox.min[0]);
                        min_y = min_y.min(bbox.min[1]);
                        max_x = max_x.max(bbox.max[0]);
                        max_y = max_y.max(bbox.max[1]);
                    }
                }
            }
        }
        
        if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
            Ok(Rectangle::new(min_x, min_y, max_x, max_y))
        } else {
            Err("Invalid coordinates for MultiPolygon".into())
        }
    } else {
        Err("Invalid coordinates for MultiPolygon".into())
    }
}

/// 从 GeometryCollection 提取边界框
fn extract_geometry_collection_bbox(geojson: &serde_json::Value) -> Result<Rectangle> {
    if let Some(geometries) = geojson.get("geometries").and_then(|g| g.as_array()) {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        
        for geometry in geometries {
            let bbox = extract_bbox(geometry)?;
            min_x = min_x.min(bbox.min[0]);
            min_y = min_y.min(bbox.min[1]);
            max_x = max_x.max(bbox.max[0]);
            max_y = max_y.max(bbox.max[1]);
        }
        
        if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
            Ok(Rectangle::new(min_x, min_y, max_x, max_y))
        } else {
            Err("Invalid coordinates for GeometryCollection".into())
        }
    } else {
        Err("Invalid geometries for GeometryCollection".into())
    }
}

/// 从坐标数组中提取边界框
pub fn extract_bbox_from_coords_array(coords: &[serde_json::Value]) -> Result<Rectangle> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    
    for coord in coords {
        if let Some(coord_array) = coord.as_array() {
            if coord_array.len() >= 2 {
                let x = coord_array[0].as_f64().ok_or("Invalid X coordinate")?;
                let y = coord_array[1].as_f64().ok_or("Invalid Y coordinate")?;
                
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    
    if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
        Ok(Rectangle::new(min_x, min_y, max_x, max_y))
    } else {
        Err("No valid coordinates found".into())
    }
}

/// 生成字符串的简单哈希值，用作 R-tree 的数据 ID
pub fn string_to_data_id(s: &str) -> i32 {
    s.bytes().fold(0i32, |acc, b| acc.wrapping_add(b as i32))
}

/// 验证 GeoJSON 格式是否有效
pub fn validate_geojson(geojson: &serde_json::Value) -> Result<()> {
    // 基本验证：检查是否有 type 字段
    match geojson.get("type") {
        Some(serde_json::Value::String(_)) => {
            // 尝试提取边界框以验证坐标有效性
            extract_bbox(geojson)?;
            Ok(())
        }
        _ => Err("Invalid GeoJSON: missing or invalid type field".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_point_bbox() {
        let point = json!({
            "type": "Point",
            "coordinates": [1.0, 2.0]
        });
        
        let bbox = extract_bbox(&point).unwrap();
        assert_eq!(bbox.min[0], 1.0);
        assert_eq!(bbox.min[1], 2.0);
        assert_eq!(bbox.max[0], 1.0);
        assert_eq!(bbox.max[1], 2.0);
    }

    #[test]
    fn test_extract_linestring_bbox() {
        let linestring = json!({
            "type": "LineString",
            "coordinates": [[0.0, 0.0], [2.0, 2.0], [1.0, 3.0]]
        });
        
        let bbox = extract_bbox(&linestring).unwrap();
        assert_eq!(bbox.min[0], 0.0);
        assert_eq!(bbox.min[1], 0.0);
        assert_eq!(bbox.max[0], 2.0);
        assert_eq!(bbox.max[1], 3.0);
    }

    #[test]
    fn test_extract_polygon_bbox() {
        let polygon = json!({
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]
        });
        
        let bbox = extract_bbox(&polygon).unwrap();
        assert_eq!(bbox.min[0], 0.0);
        assert_eq!(bbox.min[1], 0.0);
        assert_eq!(bbox.max[0], 2.0);
        assert_eq!(bbox.max[1], 2.0);
    }

    #[test]
    fn test_extract_feature_bbox() {
        let feature = json!({
            "type": "Feature",
            "geometry": {
                "type": "Point",
                "coordinates": [1.0, 2.0]
            },
            "properties": {
                "name": "test point"
            }
        });
        
        let bbox = extract_bbox(&feature).unwrap();
        assert_eq!(bbox.min[0], 1.0);
        assert_eq!(bbox.min[1], 2.0);
    }

    #[test]
    fn test_string_to_data_id() {
        let id1 = string_to_data_id("test");
        let id2 = string_to_data_id("test");
        let id3 = string_to_data_id("different");
        
        assert_eq!(id1, id2); // 相同字符串应该产生相同的ID
        assert_ne!(id1, id3); // 不同字符串应该产生不同的ID
    }

    #[test]
    fn test_validate_geojson() {
        let valid_point = json!({
            "type": "Point",
            "coordinates": [1.0, 2.0]
        });
        
        let invalid_geojson = json!({
            "coordinates": [1.0, 2.0]
        });
        
        assert!(validate_geojson(&valid_point).is_ok());
        assert!(validate_geojson(&invalid_geojson).is_err());
    }
}
