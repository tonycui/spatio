//! K-Nearest Neighbors (KNN) search algorithm for R-tree
//!
//! This module implements an efficient KNN search algorithm using a priority queue
//! (min-heap) to traverse the R-tree. The algorithm finds the K nearest items to
//! a query point, sorted by ascending distance.
//!
//! ## Algorithm Overview
//!
//! 1. Use a min-heap to maintain candidate nodes, ordered by minimum possible distance
//! 2. Start from the root node and add it to the heap
//! 3. Loop:
//!    - Pop the node with minimum distance from the heap
//!    - If it's a leaf entry (actual data), add to results
//!    - If it's an internal node, calculate distances to all children and add to heap
//!    - Continue until K results are found or heap is empty
//!
//! ## Performance
//!
//! - Time Complexity: O(K log N) for small K values
//! - Space Complexity: O(log N) for the heap
//! - Much more efficient than brute-force scan for large datasets

use super::super::rectangle::Rectangle;
use super::super::node::{Entry, Node};
use super::super::rtree::GeoItem;
use geo::Geometry;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Result of a KNN search: an item and its distance to the query point
#[derive(Debug, Clone)]
pub struct KnnResult {
    /// The geo item found
    pub item: GeoItem,
    /// Distance from the query point (in meters if using Haversine)
    pub distance: f64,
}

/// Entry in the priority queue for KNN search
///
/// This can represent either:
/// - A leaf entry (actual data item)
/// - An internal node with its children
#[derive(Debug)]
enum QueueEntry {
    /// A leaf entry containing actual data
    LeafEntry {
        min_distance: f64,
        item: GeoItem,
    },
    /// An internal node to be explored
    InternalNode {
        min_distance: f64,
        node: Node,
    },
}

impl QueueEntry {
    fn min_distance(&self) -> f64 {
        match self {
            QueueEntry::LeafEntry { min_distance, .. } => *min_distance,
            QueueEntry::InternalNode { min_distance, .. } => *min_distance,
        }
    }
}

// Implement Ord for BinaryHeap (min-heap behavior)
// Note: BinaryHeap is a max-heap by default, so we reverse the ordering
impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.min_distance() == other.min_distance()
    }
}

impl Eq for QueueEntry {}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior
        other.min_distance()
            .partial_cmp(&self.min_distance())
            .unwrap_or(Ordering::Equal)
    }
}

/// Calculate Haversine distance between two points on Earth's surface
///
/// This is the standard formula for calculating great-circle distances between
/// two points on a sphere given their longitudes and latitudes.
///
/// # Arguments
///
/// * `lon1`, `lat1` - First point (longitude, latitude in degrees)
/// * `lon2`, `lat2` - Second point (longitude, latitude in degrees)
///
/// # Returns
///
/// Distance in meters
///
/// # Reference
///
/// https://en.wikipedia.org/wiki/Haversine_formula
pub fn haversine_distance(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    const EARTH_RADIUS_METERS: f64 = 6_371_000.0; // Earth's mean radius in meters

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_METERS * c
}

/// Calculate minimum distance from a point to a rectangle (MBR)
///
/// This returns the distance to the closest point on the rectangle's boundary
/// or interior. If the point is inside the rectangle, returns 0.
///
/// # Arguments
///
/// * `point_lon`, `point_lat` - Query point coordinates (longitude, latitude)
/// * `rect` - The rectangle (MBR)
///
/// # Returns
///
/// Minimum distance in meters
pub fn point_to_rectangle_distance(point_lon: f64, point_lat: f64, rect: &Rectangle) -> f64 {
    // Find the closest point on the rectangle to the query point
    let closest_lon = point_lon.clamp(rect.min[0], rect.max[0]);
    let closest_lat = point_lat.clamp(rect.min[1], rect.max[1]);

    // Calculate distance to the closest point
    haversine_distance(point_lon, point_lat, closest_lon, closest_lat)
}

/// Calculate distance from a point to a geometry
///
/// This function calculates the true minimum distance from a query point to any type of
/// geometry, including perpendicular distances to line segments and polygon edges.
/// 
/// Uses geo crate's ClosestPoint trait to find the nearest point on the geometry surface,
/// then calculates the Haversine distance. This approach gives accurate results for
/// local-scale applications (< 100km) with acceptable error for geodetic distances.
///
/// # Arguments
///
/// * `point_lon`, `point_lat` - Query point coordinates (longitude, latitude)
/// * `geometry` - The geometry to measure distance to
///
/// # Returns
///
/// Distance in meters (using Haversine formula)
///
/// # Notes
///
/// - For points inside polygons, returns 0.0
/// - Uses planar approximation for finding closest points, then Haversine for distance
/// - Suitable for local-scale queries; large-scale queries may have minor geodetic errors
pub fn point_to_geometry_distance(point_lon: f64, point_lat: f64, geometry: &Geometry) -> f64 {
    use geo::algorithm::closest_point::ClosestPoint;
    
    let query_point = geo::Point::new(point_lon, point_lat);
    
    match geometry {
        Geometry::Point(p) => {
            // Point to point: direct distance
            haversine_distance(point_lon, point_lat, p.x(), p.y())
        }
        Geometry::Line(line) => {
            // Point to line segment: may be perpendicular distance or distance to endpoint
            match line.closest_point(&query_point) {
                geo::Closest::Intersection(_) => 0.0,  // Point is on the line
                geo::Closest::SinglePoint(closest) => {
                    haversine_distance(point_lon, point_lat, closest.x(), closest.y())
                }
                geo::Closest::Indeterminate => f64::INFINITY,
            }
        }
        Geometry::LineString(ls) => {
            // Point to polyline: finds closest point on any segment
            match ls.closest_point(&query_point) {
                geo::Closest::Intersection(_) => 0.0,
                geo::Closest::SinglePoint(closest) => {
                    haversine_distance(point_lon, point_lat, closest.x(), closest.y())
                }
                geo::Closest::Indeterminate => f64::INFINITY,
            }
        }
        Geometry::Polygon(poly) => {
            // Point to polygon: 0 if inside, otherwise distance to boundary
            use geo::algorithm::contains::Contains;
            if poly.contains(&query_point) {
                return 0.0;
            }
            
            // Find closest point on exterior ring
            let mut min_distance = match poly.exterior().closest_point(&query_point) {
                geo::Closest::SinglePoint(closest) => {
                    haversine_distance(point_lon, point_lat, closest.x(), closest.y())
                }
                geo::Closest::Intersection(_) => 0.0,
                geo::Closest::Indeterminate => f64::INFINITY,
            };
            
            // Check interior rings (holes) - point might be closest to a hole boundary
            for interior in poly.interiors() {
                let dist = match interior.closest_point(&query_point) {
                    geo::Closest::SinglePoint(closest) => {
                        haversine_distance(point_lon, point_lat, closest.x(), closest.y())
                    }
                    geo::Closest::Intersection(_) => 0.0,
                    geo::Closest::Indeterminate => f64::INFINITY,
                };
                min_distance = min_distance.min(dist);
            }
            
            min_distance
        }
        Geometry::MultiPoint(mp) => {
            // Find nearest point in the collection
            mp.iter()
                .map(|p| haversine_distance(point_lon, point_lat, p.x(), p.y()))
                .fold(f64::INFINITY, f64::min)
        }
        Geometry::MultiLineString(mls) => {
            // Find nearest line in the collection
            mls.iter()
                .map(|ls| {
                    match ls.closest_point(&query_point) {
                        geo::Closest::Intersection(_) => 0.0,
                        geo::Closest::SinglePoint(closest) => {
                            haversine_distance(point_lon, point_lat, closest.x(), closest.y())
                        }
                        geo::Closest::Indeterminate => f64::INFINITY,
                    }
                })
                .fold(f64::INFINITY, f64::min)
        }
        Geometry::MultiPolygon(mp) => {
            // Find nearest polygon in the collection
            mp.iter()
                .map(|poly| {
                    // Check if point is inside any polygon
                    use geo::algorithm::contains::Contains;
                    if poly.contains(&query_point) {
                        return 0.0;
                    }
                    
                    // Find minimum distance to this polygon's boundary
                    let mut min_dist = match poly.exterior().closest_point(&query_point) {
                        geo::Closest::SinglePoint(closest) => {
                            haversine_distance(point_lon, point_lat, closest.x(), closest.y())
                        }
                        geo::Closest::Intersection(_) => 0.0,
                        geo::Closest::Indeterminate => f64::INFINITY,
                    };
                    
                    for interior in poly.interiors() {
                        let dist = match interior.closest_point(&query_point) {
                            geo::Closest::SinglePoint(closest) => {
                                haversine_distance(point_lon, point_lat, closest.x(), closest.y())
                            }
                            geo::Closest::Intersection(_) => 0.0,
                            geo::Closest::Indeterminate => f64::INFINITY,
                        };
                        min_dist = min_dist.min(dist);
                    }
                    
                    min_dist
                })
                .fold(f64::INFINITY, f64::min)
        }
        Geometry::GeometryCollection(gc) => {
            // Recursively find nearest geometry in the collection
            gc.iter()
                .map(|geom| point_to_geometry_distance(point_lon, point_lat, geom))
                .fold(f64::INFINITY, f64::min)
        }
        _ => {
            // For other types (Rect, Triangle), use bounding rectangle as fallback
            if let Some(rect) = geometry_to_rectangle(geometry) {
                point_to_rectangle_distance(point_lon, point_lat, &rect)
            } else {
                f64::INFINITY
            }
        }
    }
}

/// Convert a geometry to its bounding rectangle
fn geometry_to_rectangle(geometry: &Geometry) -> Option<Rectangle> {
    use geo::algorithm::bounding_rect::BoundingRect;
    
    geometry.bounding_rect().map(|rect| {
        Rectangle::new(rect.min().x, rect.min().y, rect.max().x, rect.max().y)
    })
}

/// Perform KNN search on an R-tree
///
/// This function finds the K nearest items to a query point using an efficient
/// priority queue-based algorithm.
///
/// # Arguments
///
/// * `root` - Optional root node of the R-tree
/// * `query_lon`, `query_lat` - Query point coordinates (longitude, latitude)
/// * `k` - Number of nearest neighbors to find
/// * `items_map` - HashMap mapping item IDs to GeoItems (for retrieving full data)
///
/// # Returns
///
/// Vector of KnnResult, sorted by ascending distance (nearest first)
///
/// # Examples
///
/// ```ignore
/// let results = knn_search(
///     tree.root.as_ref(),
///     116.3,  // Beijing longitude
///     39.9,   // Beijing latitude
///     10,     // Find 10 nearest
///     &items_map
/// );
/// ```
pub fn knn_search(
    root: Option<&Node>,
    query_lon: f64,
    query_lat: f64,
    k: usize,
    items_map: &std::collections::HashMap<String, GeoItem>,
) -> Vec<KnnResult> {
    // Early return if tree is empty or k is 0
    if root.is_none() || k == 0 {
        return Vec::new();
    }

    let mut results: Vec<KnnResult> = Vec::with_capacity(k);
    let mut heap: BinaryHeap<QueueEntry> = BinaryHeap::new();

    // Start with the root node
    let root_node = root.unwrap();
    let root_distance = if root_node.entries.is_empty() {
        f64::INFINITY
    } else {
        // Calculate minimum distance to root's MBR
        let root_mbr = &root_node.mbr;
        point_to_rectangle_distance(query_lon, query_lat, root_mbr)
    };

    heap.push(QueueEntry::InternalNode {
        min_distance: root_distance,
        node: root_node.clone(),
    });

    // Process the heap until we have K results or heap is empty
    while let Some(entry) = heap.pop() {
        // Early termination: if we have K results and the next entry's
        // minimum distance is greater than our furthest result, we're done
        if results.len() >= k {
            let furthest_distance = results.last().unwrap().distance;
            if entry.min_distance() > furthest_distance {
                break;
            }
        }

        match entry {
            QueueEntry::LeafEntry { min_distance, item } => {
                // This is an actual data item
                results.push(KnnResult {
                    item,
                    distance: min_distance,
                });

                // Keep results sorted by distance
                results.sort_by(|a, b| {
                    a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal)
                });

                // Keep only K nearest
                if results.len() > k {
                    results.truncate(k);
                }
            }
            QueueEntry::InternalNode { node, .. } => {
                // Process all entries in this node
                for entry in &node.entries {
                    match entry {
                        Entry::Data { mbr: _, data } => {
                            // This is a leaf entry - retrieve the full item
                            if let Some(item) = items_map.get(data) {
                                let distance = point_to_geometry_distance(
                                    query_lon,
                                    query_lat,
                                    &item.geometry,
                                );

                                heap.push(QueueEntry::LeafEntry {
                                    min_distance: distance,
                                    item: item.clone(),
                                });
                            }
                        }
                        Entry::Node { mbr, node } => {
                            // This is an internal node - calculate distance to its MBR
                            let distance = point_to_rectangle_distance(query_lon, query_lat, mbr);

                            heap.push(QueueEntry::InternalNode {
                                min_distance: distance,
                                node: (**node).clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance() {
        // Test distance between Beijing and Shanghai (roughly 1067 km)
        let beijing_lon = 116.4074;
        let beijing_lat = 39.9042;
        let shanghai_lon = 121.4737;
        let shanghai_lat = 31.2304;

        let distance = haversine_distance(beijing_lon, beijing_lat, shanghai_lon, shanghai_lat);

        // Should be approximately 1067 km = 1,067,000 meters
        assert!((distance - 1_067_000.0).abs() < 10_000.0, 
                "Distance should be approximately 1,067,000 meters, got {}", distance);
    }

    #[test]
    fn test_haversine_distance_same_point() {
        // Distance from a point to itself should be 0
        let distance = haversine_distance(116.4, 39.9, 116.4, 39.9);
        assert_eq!(distance, 0.0);
    }

    #[test]
    fn test_point_to_rectangle_distance_inside() {
        // Point inside rectangle should have distance 0
        let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        let distance = point_to_rectangle_distance(5.0, 5.0, &rect);
        assert_eq!(distance, 0.0);
    }

    #[test]
    fn test_point_to_rectangle_distance_outside() {
        // Point outside rectangle
        let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        let distance = point_to_rectangle_distance(15.0, 15.0, &rect);
        
        // Distance should be from (15, 15) to closest corner (10, 10)
        let expected = haversine_distance(15.0, 15.0, 10.0, 10.0);
        assert!((distance - expected).abs() < 1.0);
    }

    #[test]
    fn test_point_to_rectangle_distance_edge() {
        // Point on the edge of rectangle should have distance 0
        let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        let distance = point_to_rectangle_distance(5.0, 0.0, &rect);
        assert_eq!(distance, 0.0);
    }

    #[test]
    fn test_point_to_geometry_distance_point() {
        let geometry = Geometry::Point(geo::Point::new(116.4, 39.9));
        let distance = point_to_geometry_distance(116.5, 40.0, &geometry);
        
        let expected = haversine_distance(116.5, 40.0, 116.4, 39.9);
        assert!((distance - expected).abs() < 1.0);
    }

    #[test]
    fn test_point_to_line_perpendicular_distance() {
        // Test perpendicular distance to a horizontal line segment
        // Line: from (0, 0) to (10, 0)
        // Query point: (5, 3) - should be perpendicular to middle of line
        let line = geo::Line::new(
            geo::coord! { x: 0.0, y: 0.0 },
            geo::coord! { x: 10.0, y: 0.0 },
        );
        let geometry = Geometry::Line(line);
        
        let distance = point_to_geometry_distance(5.0, 3.0, &geometry);
        
        // The closest point should be (5, 0), so distance is from (5, 3) to (5, 0)
        let expected = haversine_distance(5.0, 3.0, 5.0, 0.0);
        
        // Should be much less than distance to endpoints
        let dist_to_start = haversine_distance(5.0, 3.0, 0.0, 0.0);
        let dist_to_end = haversine_distance(5.0, 3.0, 10.0, 0.0);
        
        assert!(distance < dist_to_start, "Perpendicular distance should be less than distance to start");
        assert!(distance < dist_to_end, "Perpendicular distance should be less than distance to end");
        assert!((distance - expected).abs() < 1.0, "Distance should match perpendicular projection");
    }

    #[test]
    fn test_point_to_linestring_perpendicular() {
        // Test with a simple L-shaped linestring
        let ls = geo::LineString::from(vec![
            geo::coord! { x: 0.0, y: 0.0 },
            geo::coord! { x: 10.0, y: 0.0 },
            geo::coord! { x: 10.0, y: 10.0 },
        ]);
        let geometry = Geometry::LineString(ls);
        
        // Query point near the horizontal segment
        let distance = point_to_geometry_distance(5.0, 2.0, &geometry);
        
        // Should be perpendicular distance to (5, 0)
        let expected = haversine_distance(5.0, 2.0, 5.0, 0.0);
        assert!((distance - expected).abs() < 1.0);
    }

    #[test]
    fn test_point_inside_polygon_returns_zero() {
        // Create a square polygon
        let poly = geo::Polygon::new(
            geo::LineString::from(vec![
                geo::coord! { x: 0.0, y: 0.0 },
                geo::coord! { x: 10.0, y: 0.0 },
                geo::coord! { x: 10.0, y: 10.0 },
                geo::coord! { x: 0.0, y: 10.0 },
                geo::coord! { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let geometry = Geometry::Polygon(poly);
        
        // Point inside the polygon
        let distance = point_to_geometry_distance(5.0, 5.0, &geometry);
        assert_eq!(distance, 0.0, "Point inside polygon should have distance 0");
    }

    #[test]
    fn test_point_outside_polygon_perpendicular() {
        // Create a square polygon
        let poly = geo::Polygon::new(
            geo::LineString::from(vec![
                geo::coord! { x: 0.0, y: 0.0 },
                geo::coord! { x: 10.0, y: 0.0 },
                geo::coord! { x: 10.0, y: 10.0 },
                geo::coord! { x: 0.0, y: 10.0 },
                geo::coord! { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let geometry = Geometry::Polygon(poly);
        
        // Point outside, directly below the bottom edge
        let distance = point_to_geometry_distance(5.0, -2.0, &geometry);
        
        // Should be perpendicular distance to bottom edge at (5, 0)
        let expected = haversine_distance(5.0, -2.0, 5.0, 0.0);
        
        // Distance to corners should be larger
        let dist_to_corner = haversine_distance(5.0, -2.0, 0.0, 0.0);
        
        assert!(distance < dist_to_corner, "Perpendicular distance should be less than distance to corner");
        assert!((distance - expected).abs() < 1.0);
    }

    #[test]
    fn test_knn_search_empty_tree() {
        let items_map = std::collections::HashMap::new();
        let results = knn_search(None, 116.4, 39.9, 10, &items_map);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_knn_search_k_zero() {
        let items_map = std::collections::HashMap::new();
        let results = knn_search(None, 116.4, 39.9, 0, &items_map);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_knn_search_basic() {
        use crate::rtree::RTree;
        use std::collections::HashMap;

        // Create a tree and insert some points around Beijing (116.4, 39.9)
        let mut tree = RTree::new(4);
        let mut items_map = HashMap::new();

        // Define test points with known distances from query point (116.4, 39.9)
        let test_data = vec![
            ("p1", 116.5, 40.0),   // Northeast
            ("p2", 116.3, 39.8),   // Southwest
            ("p3", 116.4, 39.9),   // Exact match (distance = 0)
            ("p4", 117.0, 40.5),   // Far northeast
            ("p5", 116.2, 39.7),   // Far southwest
        ];

        for (id, lon, lat) in test_data.iter() {
            let geojson = format!(
                r#"{{"type":"Feature","properties":{{"id":"{}"}},"geometry":{{"type":"Point","coordinates":[{},{}]}}}}"#,
                id, lon, lat
            );
            let geometry = Geometry::Point(geo::Point::new(*lon, *lat));
            
            tree.insert_geojson(id.to_string(), &geojson);
            
            items_map.insert(
                id.to_string(),
                GeoItem {
                    id: id.to_string(),
                    geometry,
                    geojson,
                },
            );
        }

        // Search for 3 nearest neighbors to (116.4, 39.9)
        let results = knn_search(
            tree.get_root(),
            116.4,
            39.9,
            3,
            &items_map,
        );

        // Should return 3 results
        assert_eq!(results.len(), 3);

        // Results should be sorted by distance
        assert_eq!(results[0].item.id, "p3"); // Exact match, distance = 0
        assert!(results[0].distance < 1.0); // Very close to 0

        // Second should be either p1 or p2 (both relatively close)
        assert!(results[1].distance < results[2].distance);

        // Verify distances are in ascending order
        for i in 0..results.len() - 1 {
            assert!(
                results[i].distance <= results[i + 1].distance,
                "Results not sorted by distance"
            );
        }
    }

    #[test]
    fn test_knn_search_k_greater_than_items() {
        use crate::rtree::RTree;
        use std::collections::HashMap;

        let mut tree = RTree::new(4);
        let mut items_map = HashMap::new();

        // Insert only 3 items
        for i in 0..3 {
            let id = format!("item_{}", i);
            let lon = 116.0 + i as f64 * 0.1;
            let lat = 39.0 + i as f64 * 0.1;
            let geojson = format!(
                r#"{{"type":"Point","coordinates":[{},{}]}}"#,
                lon, lat
            );
            let geometry = Geometry::Point(geo::Point::new(lon, lat));

            tree.insert_geojson(id.clone(), &geojson);
            items_map.insert(
                id.clone(),
                GeoItem {
                    id: id.clone(),
                    geometry,
                    geojson,
                },
            );
        }

        // Request 10 neighbors but only 3 exist
        let results = knn_search(
            tree.get_root(),
            116.0,
            39.0,
            10,
            &items_map,
        );

        // Should return only 3 results
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_knn_search_correctness() {
        use crate::rtree::RTree;
        use std::collections::HashMap;

        // Create a grid of points
        let mut tree = RTree::new(4);
        let mut items_map = HashMap::new();
        let mut all_items = Vec::new();

        for x in 0..5 {
            for y in 0..5 {
                let id = format!("grid_{}_{}", x, y);
                let lon = 116.0 + x as f64 * 0.1;
                let lat = 39.0 + y as f64 * 0.1;
                let geojson = format!(
                    r#"{{"type":"Point","coordinates":[{},{}]}}"#,
                    lon, lat
                );
                let geometry = Geometry::Point(geo::Point::new(lon, lat));

                tree.insert_geojson(id.clone(), &geojson);
                
                let item = GeoItem {
                    id: id.clone(),
                    geometry: geometry.clone(),
                    geojson: geojson.clone(),
                };
                items_map.insert(id.clone(), item.clone());
                all_items.push((id, lon, lat, geometry));
            }
        }

        // Query point
        let query_lon = 116.15;
        let query_lat = 39.15;
        let k = 5;

        // KNN search
        let knn_results = knn_search(
            tree.get_root(),
            query_lon,
            query_lat,
            k,
            &items_map,
        );

        // Brute force: calculate all distances and sort
        let mut brute_force_results: Vec<(String, f64)> = all_items
            .iter()
            .map(|(id, lon, lat, _)| {
                let dist = haversine_distance(query_lon, query_lat, *lon, *lat);
                (id.clone(), dist)
            })
            .collect();
        brute_force_results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        brute_force_results.truncate(k);

        // Verify KNN results match brute force
        assert_eq!(knn_results.len(), brute_force_results.len());
        
        for i in 0..k {
            assert_eq!(knn_results[i].item.id, brute_force_results[i].0);
            assert!(
                (knn_results[i].distance - brute_force_results[i].1).abs() < 1.0,
                "Distance mismatch: KNN={}, Brute={}",
                knn_results[i].distance,
                brute_force_results[i].1
            );
        }
    }

    #[test]
    fn test_knn_performance_comparison() {
        use crate::rtree::RTree;
        use std::collections::HashMap;
        use std::time::Instant;

        // Create a larger dataset for performance testing
        let mut tree = RTree::new(10);
        let mut items_map = HashMap::new();
        let mut all_items = Vec::new();

        let grid_size = 20; // 20x20 = 400 points
        for x in 0..grid_size {
            for y in 0..grid_size {
                let id = format!("perf_{}_{}", x, y);
                let lon = 115.0 + x as f64 * 0.1;
                let lat = 38.0 + y as f64 * 0.1;
                let geojson = format!(
                    r#"{{"type":"Point","coordinates":[{},{}]}}"#,
                    lon, lat
                );
                let geometry = Geometry::Point(geo::Point::new(lon, lat));

                tree.insert_geojson(id.clone(), &geojson);
                
                let item = GeoItem {
                    id: id.clone(),
                    geometry: geometry.clone(),
                    geojson: geojson.clone(),
                };
                items_map.insert(id.clone(), item);
                all_items.push((lon, lat));
            }
        }

        let query_lon = 116.5;
        let query_lat = 39.5;
        let k = 10;

        // Measure KNN search time
        let start = Instant::now();
        let knn_results = knn_search(
            tree.get_root(),
            query_lon,
            query_lat,
            k,
            &items_map,
        );
        let knn_duration = start.elapsed();

        // Measure brute force time
        let start = Instant::now();
        let mut brute_force_results: Vec<f64> = all_items
            .iter()
            .map(|(lon, lat)| haversine_distance(query_lon, query_lat, *lon, *lat))
            .collect();
        brute_force_results.sort_by(|a, b| a.partial_cmp(b).unwrap());
        brute_force_results.truncate(k);
        let brute_force_duration = start.elapsed();

        println!("\n=== KNN Performance Test ===");
        println!("Dataset size: {} points", grid_size * grid_size);
        println!("K: {}", k);
        println!("KNN search time: {:?}", knn_duration);
        println!("Brute force time: {:?}", brute_force_duration);
        
        if brute_force_duration > knn_duration {
            let speedup = brute_force_duration.as_nanos() as f64 / knn_duration.as_nanos() as f64;
            println!("KNN is {:.2}x faster than brute force", speedup);
        } else {
            println!("Note: For small datasets, overhead may make KNN slower");
        }

        // Verify correctness
        assert_eq!(knn_results.len(), k);

        // For larger datasets (1000+ points), KNN should be faster
        // But for 400 points, it might be similar or slower due to overhead
        // The important thing is that it's correct
    }
}
