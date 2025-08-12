use serde::{Deserialize, Serialize};

/// 矩形边界框 - 用于表示R-tree中的最小边界矩形(MBR)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rectangle {
    pub min: [f64; 2],  // [x_min, y_min]
    pub max: [f64; 2],  // [x_max, y_max]
}

impl Rectangle {
    /// 创建新的矩形
    pub fn new(x_min: f64, y_min: f64, x_max: f64, y_max: f64) -> Self {
        assert!(x_min <= x_max && y_min <= y_max, "Invalid rectangle bounds");
        Rectangle {
            min: [x_min, y_min],
            max: [x_max, y_max],
        }
    }

    /// 创建一个点矩形
    pub fn from_point(x: f64, y: f64) -> Self {
        Rectangle {
            min: [x, y],
            max: [x, y],
        }
    }

    /// 计算矩形面积
    pub fn area(&self) -> f64 {
        (self.max[0] - self.min[0]) * (self.max[1] - self.min[1])
    }

    /// 计算矩形周长
    pub fn perimeter(&self) -> f64 {
        2.0 * ((self.max[0] - self.min[0]) + (self.max[1] - self.min[1]))
    }

    /// 计算两个矩形的并集MBR
    pub fn union(&self, other: &Rectangle) -> Rectangle {
        Rectangle {
            min: [
                self.min[0].min(other.min[0]),
                self.min[1].min(other.min[1])
            ],
            max: [
                self.max[0].max(other.max[0]),
                self.max[1].max(other.max[1])
            ],
        }
    }

    /// 判断两个矩形是否相交
    pub fn intersects(&self, other: &Rectangle) -> bool {
        self.min[0] <= other.max[0] && self.max[0] >= other.min[0] &&
        self.min[1] <= other.max[1] && self.max[1] >= other.min[1]
    }

    /// 判断当前矩形是否包含另一个矩形
    pub fn contains(&self, other: &Rectangle) -> bool {
        self.min[0] <= other.min[0] && self.min[1] <= other.min[1] &&
        self.max[0] >= other.max[0] && self.max[1] >= other.max[1]
    }

    /// 判断当前矩形是否包含一个点
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        self.min[0] <= x && x <= self.max[0] &&
        self.min[1] <= y && y <= self.max[1]
    }

    /// 计算扩大到包含另一个矩形所需的面积增量
    pub fn enlargement(&self, other: &Rectangle) -> f64 {
        self.union(other).area() - self.area()
    }

    /// 计算两个矩形的交集面积
    pub fn intersection_area(&self, other: &Rectangle) -> f64 {
        if !self.intersects(other) {
            return 0.0;
        }
        
        let x_overlap = (self.max[0].min(other.max[0])) - (self.min[0].max(other.min[0]));
        let y_overlap = (self.max[1].min(other.max[1])) - (self.min[1].max(other.min[1]));
        
        x_overlap * y_overlap
    }

    /// 计算矩形中心点
    pub fn center(&self) -> [f64; 2] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
        ]
    }

    /// 判断矩形是否为空（面积为0）
    pub fn is_empty(&self) -> bool {
        self.area() == 0.0
    }

    /// 判断矩形是否为点（宽度和高度都为0）
    pub fn is_point(&self) -> bool {
        self.min[0] == self.max[0] && self.min[1] == self.max[1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rectangle_creation() {
        let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(rect.min, [0.0, 0.0]);
        assert_eq!(rect.max, [10.0, 10.0]);
    }

    #[test]
    fn test_rectangle_area() {
        let rect = Rectangle::new(0.0, 0.0, 10.0, 5.0);
        assert_eq!(rect.area(), 50.0);
    }

    #[test]
    fn test_rectangle_union() {
        let rect1 = Rectangle::new(0.0, 0.0, 5.0, 5.0);
        let rect2 = Rectangle::new(3.0, 3.0, 8.0, 8.0);
        let union = rect1.union(&rect2);
        assert_eq!(union, Rectangle::new(0.0, 0.0, 8.0, 8.0));
    }

    #[test]
    fn test_rectangle_intersects() {
        let rect1 = Rectangle::new(0.0, 0.0, 5.0, 5.0);
        let rect2 = Rectangle::new(3.0, 3.0, 8.0, 8.0);
        let rect3 = Rectangle::new(10.0, 10.0, 15.0, 15.0);
        
        assert!(rect1.intersects(&rect2));
        assert!(!rect1.intersects(&rect3));
    }

    #[test]
    fn test_rectangle_contains() {
        let rect1 = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        let rect2 = Rectangle::new(2.0, 2.0, 8.0, 8.0);
        let rect3 = Rectangle::new(5.0, 5.0, 15.0, 15.0);
        
        assert!(rect1.contains(&rect2));
        assert!(!rect1.contains(&rect3));
    }

    #[test]
    fn test_rectangle_contains_point() {
        let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        assert!(rect.contains_point(5.0, 5.0));
        assert!(!rect.contains_point(15.0, 15.0));
    }

    #[test]
    fn test_rectangle_enlargement() {
        let rect1 = Rectangle::new(0.0, 0.0, 5.0, 5.0);
        let rect2 = Rectangle::new(3.0, 3.0, 8.0, 8.0);
        let enlargement = rect1.enlargement(&rect2);
        assert_eq!(enlargement, 39.0); // 8*8 - 5*5 = 64 - 25 = 39
    }
}
