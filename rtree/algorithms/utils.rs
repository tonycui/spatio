use super::super::node::{Entry, Node};
use super::super::rectangle::Rectangle;
use crate::rtree::RTree;
// use std::result::Result;
use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

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
            "Cannot calculate bounding box for empty geometry",
        )) as Box<dyn std::error::Error + Send + Sync>),
    }
}

/// R-tree工具函数实现
impl RTree {
    /// 向上调整树 - 更新MBR
    ///
    /// 从给定路径的叶子节点向上更新每一层的MBR，确保树的MBR信息正确
    pub(crate) fn adjust_tree_upward(&mut self, mut path: Vec<usize>) {
        // 从叶子节点向上更新每一层的MBR
        while !path.is_empty() {
            // 获取当前节点,如果当前节点为空，则跳过当前节点
            let node = match self.get_last_node_mut(&path) {
                Some(node) => node,
                None => {
                    path.pop().unwrap();
                    break;
                }
            };

            // 更新当前节点的MBR
            node.update_mbr();

            // 将当前节点从 path 中移除
            let current_node_index = path[path.len() - 1];
            path.pop().unwrap();

            let current_mbr = node.mbr;

            // 更新父节点中指向当前节点的条目的MBR
            if path.is_empty() {
                // 当前节点是根节点的直接子节点，更新根节点中的条目
                if let Some(root) = self.root_mut() {
                    if let Some(Entry::Node { mbr, .. }) = root.entries.get_mut(current_node_index)
                    {
                        *mbr = current_mbr;
                    }
                }
            } else {
                // 更新中间层的父节点
                let parent = match self.get_last_node_mut(&path) {
                    Some(node) => node,
                    None => {
                        println!(
                            "Warning: Failed to get parent node (Empty Node) during MBR update"
                        );
                        break;
                    }
                };
                if let Some(Entry::Node { mbr, .. }) = parent.entries.get_mut(current_node_index) {
                    *mbr = current_mbr;
                }
            }
        }
    }

    /// 获取路径中最后一个节点的可变引用
    ///
    /// 根据给定的路径从根节点开始遍历，返回路径末端节点的可变引用
    pub(crate) fn get_last_node_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        let mut current = self.root_mut().as_mut()?;

        for &index in path {
            if let Some(Entry::Node { node, .. }) = current.entries.get_mut(index) {
                current = node;
            } else {
                return None;
            }
        }

        Some(current)
    }

    /// 计算扩大成本
    ///
    /// 计算将一个矩形添加到另一个矩形时所需的面积扩大量
    #[allow(dead_code)]
    pub(crate) fn enlargement_cost(&self, mbr: &Rectangle, rect: &Rectangle) -> f64 {
        mbr.enlargement(rect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enlargement_cost() {
        let rtree = RTree::new(4);
        let mbr1 = Rectangle::new(0.0, 0.0, 10.0, 10.0); // 面积100
        let rect1 = Rectangle::new(5.0, 5.0, 15.0, 15.0); // 会扩展到(0,0,15,15)，面积225

        let cost = rtree.enlargement_cost(&mbr1, &rect1);
        assert_eq!(cost, 125.0); // 225 - 100 = 125
    }

    #[test]
    fn test_get_last_node_mut() {
        let mut rtree = RTree::new(4);

        // 插入一些数据以创建多层结构
        rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), "1".to_string());
        rtree.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), "2".to_string());
        rtree.insert(Rectangle::new(4.0, 4.0, 5.0, 5.0), "3".to_string());
        rtree.insert(Rectangle::new(6.0, 6.0, 7.0, 7.0), "4".to_string());
        rtree.insert(Rectangle::new(8.0, 8.0, 9.0, 9.0), "5".to_string());

        // 测试空路径 - 应该返回根节点
        assert!(rtree.get_last_node_mut(&[]).is_some());

        // 测试有效路径
        if let Some(root) = rtree.root_ref() {
            if !root.entries.is_empty() {
                let path = vec![0];
                let node = rtree.get_last_node_mut(&path);
                assert!(node.is_some());
            }
        }
    }

    #[test]
    fn test_adjust_tree_upward() {
        let mut rtree = RTree::new(3);

        // 插入数据
        rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), "1".to_string());
        rtree.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), "2".to_string());
        rtree.insert(Rectangle::new(4.0, 4.0, 5.0, 5.0), "3".to_string());
        rtree.insert(Rectangle::new(6.0, 6.0, 7.0, 7.0), "4".to_string());

        // 获取根节点的MBR作为参考
        let _original_mbr = if let Some(root) = rtree.root_ref() {
            root.mbr
        } else {
            return;
        };

        // 调用adjust_tree_upward应该不会改变正确的MBR
        // 这里我们传入一个空路径，应该不会造成任何变化
        rtree.adjust_tree_upward(vec![]);

        // 验证根节点MBR没有被错误修改
        if let Some(root) = rtree.root_ref() {
            // MBR应该仍然包含所有数据点
            assert!(root.mbr.contains(&Rectangle::new(0.0, 0.0, 1.0, 1.0)));
            assert!(root.mbr.contains(&Rectangle::new(6.0, 6.0, 7.0, 7.0)));
        }
    }
}
