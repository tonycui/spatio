use rtree::Rectangle;
use crate::Result;

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

/// 从 geo::Geometry 计算边界框
pub fn geometry_to_bbox(geometry: &geo::Geometry) -> Result<Rectangle> {
    use geo::algorithm::bounding_rect::BoundingRect;
    
    match geometry.bounding_rect() {
        Some(rect) => {
            let min_x = rect.min().x;
            let min_y = rect.min().y;
            let max_x = rect.max().x;
            let max_y = rect.max().y;
            
            Ok(Rectangle {
                min: [min_x, min_y],
                max: [max_x, max_y],
            })
        }
        None => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Cannot calculate bounding box for empty geometry"
        )) as Box<dyn std::error::Error + Send + Sync>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_to_data_id() {
        let id1 = string_to_data_id("test");
        let id2 = string_to_data_id("test");
        let id3 = string_to_data_id("different");
        
        assert_eq!(id1, id2); // 相同字符串应该产生相同的ID
        assert_ne!(id1, id3); // 不同字符串应该产生不同的ID
    }

}

