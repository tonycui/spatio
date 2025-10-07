use geo::Geometry;
use geojson::GeoJson;

// OLD CODE - NOT USED:
// pub fn geojson_to_geometry2(geojson: &serde_json::Value) -> Result<Geometry<f64>> {
//     // 1. 将 serde_json::Value 转换为 geojson::GeoJson
//     let geojson_str = geojson.to_string();
//     let parsed: GeoJson = geojson_str.parse()
//         .map_err(|e| format!("Invalid GeoJSON: {}", e))?;
//     // 2. 转换为 geo::Geometry
//     match parsed {
//         GeoJson::Geometry(geom) => {
//             geometry_from_geojson_geometry(geom)
//         }
//         GeoJson::Feature(feature) => {
//             if let Some(geom) = feature.geometry {
//                 geometry_from_geojson_geometry(geom)
//             } else {
//                 Err("Feature has no geometry".into())
//             }
//         }
//         _ => Err("Unsupported GeoJSON type".into()),
//     }
// }

/// 将 GeoJSON 字符串转为 geo::Geometry<f64>
/// 支持 GeoJSON 类型：Geometry 和 Feature
pub(crate) fn geojson_to_geometry(geojson_str: &str) -> crate::Result<Geometry<f64>> {
    // 解析 GeoJSON 字符串
    let geojson = geojson_str.parse::<GeoJson>()?;

    match geojson {
        GeoJson::Geometry(g) => Ok(g.try_into()?),
        GeoJson::Feature(f) => {
            let geometry = f.geometry.ok_or("Feature 没有 geometry 字段")?;
            Ok(geometry.try_into()?)
        }
        _ => Err("仅支持 GeoJSON Geometry 和 Feature 类型".into()),
    }
}

// fn geometry_from_geojson_geometry(geom: geojson::Geometry) -> Result<Geometry<f64>> {
//     match geom.value {
//         GeoJsonValue::Point(coords) => {
//             if coords.len() < 2 {
//                 return Err("Point coordinates must have at least 2 elements".into());
//             }
//             Ok(Geometry::Point(Point::new(coords[0], coords[1])))
//         }
//         GeoJsonValue::LineString(coords) => {
//             if coords.len() < 2 {
//                 return Err("LineString must have at least 2 points".into());
//             }
//             let line: LineString<f64> = coords.into_iter()
//                 .map(|coord| {
//                     if coord.len() < 2 {
//                         Coord { x: 0.0, y: 0.0 } // 默认值，实际上应该返回错误
//                     } else {
//                         Coord { x: coord[0], y: coord[1] }
//                     }
//                 })
//                 .collect();
//             Ok(Geometry::LineString(line))
//         }
//         GeoJsonValue::Polygon(coords) => {
//             if coords.is_empty() {
//                 return Err("Polygon must have at least one ring".into());
//             }

//             // 外环
//             let exterior: LineString<f64> = coords[0].iter()
//                 .map(|coord| {
//                     if coord.len() < 2 {
//                         Coord { x: 0.0, y: 0.0 } // 默认值
//                     } else {
//                         Coord { x: coord[0], y: coord[1] }
//                     }
//                 })
//                 .collect();

//             // 内环（如果有的话）
//             let interiors: Vec<LineString<f64>> = coords[1..].iter()
//                 .map(|ring| ring.iter()
//                     .map(|coord| {
//                         if coord.len() < 2 {
//                             Coord { x: 0.0, y: 0.0 } // 默认值
//                         } else {
//                             Coord { x: coord[0], y: coord[1] }
//                         }
//                     })
//                     .collect())
//                 .collect();

//             Ok(Geometry::Polygon(Polygon::new(exterior, interiors)))
//         }
//         GeoJsonValue::MultiPoint(coords) => {
//             let points: Vec<Point<f64>> = coords.into_iter()
//                 .filter_map(|coord| {
//                     if coord.len() >= 2 {
//                         Some(Point::new(coord[0], coord[1]))
//                     } else {
//                         None
//                     }
//                 })
//                 .collect();
//             Ok(Geometry::MultiPoint(MultiPoint::new(points)))
//         }
//         GeoJsonValue::MultiLineString(coords) => {
//             let lines: Vec<LineString<f64>> = coords.into_iter()
//                 .map(|line_coords| {
//                     line_coords.into_iter()
//                         .map(|coord| {
//                             if coord.len() >= 2 {
//                                 Coord { x: coord[0], y: coord[1] }
//                             } else {
//                                 Coord { x: 0.0, y: 0.0 }
//                             }
//                         })
//                         .collect()
//                 })
//                 .collect();
//             Ok(Geometry::MultiLineString(MultiLineString::new(lines)))
//         }
//         GeoJsonValue::MultiPolygon(coords) => {
//             let polygons: Vec<Polygon<f64>> = coords.into_iter()
//                 .map(|poly_coords| {
//                     if poly_coords.is_empty() {
//                         // 空多边形，创建一个默认的
//                         let coords = vec![
//                             Coord { x: 0.0, y: 0.0 },
//                             Coord { x: 0.0, y: 0.0 },
//                             Coord { x: 0.0, y: 0.0 },
//                             Coord { x: 0.0, y: 0.0 }
//                         ];
//                         Polygon::new(LineString::new(coords), vec![])
//                     } else {
//                         let exterior: LineString<f64> = poly_coords[0].iter()
//                             .map(|coord| {
//                                 if coord.len() >= 2 {
//                                     Coord { x: coord[0], y: coord[1] }
//                                 } else {
//                                     Coord { x: 0.0, y: 0.0 }
//                                 }
//                             })
//                             .collect();

//                         let interiors: Vec<LineString<f64>> = poly_coords[1..].iter()
//                             .map(|ring| ring.iter()
//                                 .map(|coord| {
//                                     if coord.len() >= 2 {
//                                         Coord { x: coord[0], y: coord[1] }
//                                     } else {
//                                         Coord { x: 0.0, y: 0.0 }
//                                     }
//                                 })
//                                 .collect())
//                             .collect();

//                         Polygon::new(exterior, interiors)
//                     }
//                 })
//                 .collect();
//             Ok(Geometry::MultiPolygon(MultiPolygon::new(polygons)))
//         }
//         GeoJsonValue::GeometryCollection(_) => {
//             Err("GeometryCollection not yet supported".into())
//         }
//     }
// }

/// 测试两个几何体是否相交
pub fn geometries_intersect(geom1: &Geometry<f64>, geom2: &Geometry<f64>) -> bool {
    use geo::algorithm::intersects::Intersects;
    geom1.intersects(geom2)
}

/// 将 geo::Geometry 转换为 serde_json::Value (GeoJSON)
pub fn geometry_to_geojson(geometry: &Geometry<f64>) -> serde_json::Value {
    use serde_json::json;

    match geometry {
        Geometry::Point(point) => {
            json!({
                "type": "Point",
                "coordinates": [point.x(), point.y()]
            })
        }
        Geometry::LineString(line) => {
            let coords: Vec<Vec<f64>> = line.coords().map(|coord| vec![coord.x, coord.y]).collect();
            json!({
                "type": "LineString",
                "coordinates": coords
            })
        }
        Geometry::Polygon(polygon) => {
            let mut rings: Vec<Vec<Vec<f64>>> = Vec::new();

            // 外环
            let exterior: Vec<Vec<f64>> = polygon
                .exterior()
                .coords()
                .map(|coord| vec![coord.x, coord.y])
                .collect();
            rings.push(exterior);

            // 内环
            for interior in polygon.interiors() {
                let interior_coords: Vec<Vec<f64>> = interior
                    .coords()
                    .map(|coord| vec![coord.x, coord.y])
                    .collect();
                rings.push(interior_coords);
            }

            json!({
                "type": "Polygon",
                "coordinates": rings
            })
        }
        Geometry::MultiPoint(multi_point) => {
            let coords: Vec<Vec<f64>> = multi_point
                .iter()
                .map(|point| vec![point.x(), point.y()])
                .collect();
            json!({
                "type": "MultiPoint",
                "coordinates": coords
            })
        }
        Geometry::MultiLineString(multi_line) => {
            let coords: Vec<Vec<Vec<f64>>> = multi_line
                .iter()
                .map(|line| line.coords().map(|coord| vec![coord.x, coord.y]).collect())
                .collect();
            json!({
                "type": "MultiLineString",
                "coordinates": coords
            })
        }
        Geometry::MultiPolygon(multi_polygon) => {
            let coords: Vec<Vec<Vec<Vec<f64>>>> = multi_polygon
                .iter()
                .map(|polygon| {
                    let mut rings: Vec<Vec<Vec<f64>>> = Vec::new();

                    // 外环
                    let exterior: Vec<Vec<f64>> = polygon
                        .exterior()
                        .coords()
                        .map(|coord| vec![coord.x, coord.y])
                        .collect();
                    rings.push(exterior);

                    // 内环
                    for interior in polygon.interiors() {
                        let interior_coords: Vec<Vec<f64>> = interior
                            .coords()
                            .map(|coord| vec![coord.x, coord.y])
                            .collect();
                        rings.push(interior_coords);
                    }

                    rings
                })
                .collect();
            json!({
                "type": "MultiPolygon",
                "coordinates": coords
            })
        }
        _ => {
            // 对于其他几何类型，返回一个占位符
            json!({
                "type": "GeometryCollection",
                "geometries": []
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_point_conversion() {
        let point_json = json!({
            "type": "Point",
            "coordinates": [0.0, 0.0]
        });

        let geometry = geojson_to_geometry(&point_json.to_string()).unwrap();
        match geometry {
            Geometry::Point(p) => {
                assert_eq!(p.x(), 0.0);
                assert_eq!(p.y(), 0.0);
            }
            _ => panic!("Expected Point geometry"),
        }
    }

    #[test]
    fn test_linestring_conversion() {
        let linestring_json = json!({
            "type": "LineString",
            "coordinates": [[0.0, 0.0], [1.0, 1.0]]
        });

        let geometry = geojson_to_geometry(&linestring_json.to_string()).unwrap();
        match geometry {
            Geometry::LineString(line) => {
                assert_eq!(line.0.len(), 2);
            }
            _ => panic!("Expected LineString geometry"),
        }
    }

    #[test]
    fn test_polygon_conversion() {
        let polygon_json = json!({
            "type": "Polygon",
            "coordinates": [[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, 0.0]
            ]]
        });

        let geometry = geojson_to_geometry(&polygon_json.to_string()).unwrap();
        match geometry {
            Geometry::Polygon(poly) => {
                assert_eq!(poly.exterior().0.len(), 5);
            }
            _ => panic!("Expected Polygon geometry"),
        }
    }

    #[test]
    fn test_point_polygon_intersection() {
        let point_json = json!({
            "type": "Point",
            "coordinates": [0.5, 0.5]
        });

        let polygon_json = json!({
            "type": "Polygon",
            "coordinates": [[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, 0.0]
            ]]
        });

        let point_geom = geojson_to_geometry(&point_json.to_string()).unwrap();
        let polygon_geom = geojson_to_geometry(&polygon_json.to_string()).unwrap();

        assert!(geometries_intersect(&point_geom, &polygon_geom));
    }

    #[test]
    fn test_point_polygon_no_intersection() {
        let point_json = json!({
            "type": "Point",
            "coordinates": [2.0, 2.0]
        });

        let polygon_json = json!({
            "type": "Polygon",
            "coordinates": [[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, 0.0]
            ]]
        });

        let point_geom = geojson_to_geometry(&point_json.to_string()).unwrap();
        let polygon_geom = geojson_to_geometry(&polygon_json.to_string()).unwrap();

        assert!(!geometries_intersect(&point_geom, &polygon_geom));
    }

    #[test]
    fn test_invalid_geojson() {
        let invalid_json = json!({
            "type": "Point",
            "coordinates": [0.0] // 缺少 y 坐标
        });

        let result = geojson_to_geometry(&invalid_json.to_string());
        assert!(result.is_err());
    }
}
