//! 物理尺寸多屏布局引擎（纯几何，无系统调用）。
//!
//! 设计来源（公开资料，不含任何项目源码）：
//! - 大象漫步（Elephant Walk）公开说明：按真实物理毫米排列屏幕；拖动屏幕时 12 mm
//!   自动吸附，边缘接触容差 0.5 mm；只有边缘贴合（界面显示为“绿边”）时才允许指针
//!   穿越；旋转平板后沿相邻接缝重新锚定，而不是跳回左上角。
//! - Deskflow 公开协议参考：由主控端统一维护屏幕布局，只有到达“进入 / 离开”边界时
//!   才切换控制权。
//!
//! 本模块只实现这些公开机制对应的几何算法：坐标统一使用毫米，布局平面即桌面平面，
//! 因此跨屏映射可以 1:1 保持物理位置，不需要按分辨率缩放。这样既便于单元测试，也
//! 不与平台输入钩子耦合。

use std::cmp::Ordering;

use thiserror::Error;

/// 拖动屏幕时触发自动吸附的最大距离（毫米）。
pub const SNAP_DISTANCE_MM: f64 = 12.0;

/// 两个屏幕边缘被视为“贴合”的容差（毫米）。
pub const SEAM_TOLERANCE_MM: f64 = 0.5;

/// 穿越后指针在目标屏内的内缩距离（毫米），避免停在接缝上反复触发穿越。
pub const CROSS_INSET_MM: f64 = 0.5;

const EPSILON: f64 = 1e-9;

/// 布局错误。
#[derive(Debug, Error, PartialEq)]
pub enum LayoutError {
    #[error("设备 ID 不能为空")]
    EmptyId,
    #[error("设备 {0} 不存在")]
    UnknownDevice(String),
    #[error("设备尺寸必须是有限正数")]
    InvalidSize,
    #[error("设备坐标必须是有限数值")]
    InvalidPosition,
}

/// 屏幕旋转角度。只影响设备的有效宽高，不改变左上角坐标语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

impl Rotation {
    /// 90° / 270° 时宽高互换。
    pub fn swaps_axes(self) -> bool {
        matches!(self, Rotation::Deg90 | Rotation::Deg270)
    }
}

/// 屏幕边缘。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// 接缝方向：垂直接缝（左右相邻）或水平接缝（上下相邻）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
}

/// 以毫米为单位的尺寸。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeMm {
    pub width: f64,
    pub height: f64,
}

impl SizeMm {
    /// 创建并校验尺寸；非有限值或非正数会被拒绝。
    pub fn new(width: f64, height: f64) -> Result<Self, LayoutError> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(LayoutError::InvalidSize);
        }
        Ok(Self { width, height })
    }

    fn rotated(self, rotation: Rotation) -> Self {
        if rotation.swaps_axes() {
            Self {
                width: self.height,
                height: self.width,
            }
        } else {
            self
        }
    }
}

/// 以毫米为单位的平面坐标点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointMm {
    pub x: f64,
    pub y: f64,
}

impl PointMm {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// 轴对齐矩形（毫米）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectMm {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl RectMm {
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn center(&self) -> PointMm {
        PointMm::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// 判断点是否落在矩形内（含边界，带浮点容差）。
    pub fn contains(&self, point: PointMm) -> bool {
        point.x >= self.x - EPSILON
            && point.x <= self.right() + EPSILON
            && point.y >= self.y - EPSILON
            && point.y <= self.bottom() + EPSILON
    }

    /// 两个矩形是否存在真正的内部重叠；仅边缘相切不算重叠。
    fn overlaps_interior(&self, other: &RectMm) -> bool {
        self.x < other.right() - EPSILON
            && self.right() > other.x + EPSILON
            && self.y < other.bottom() - EPSILON
            && self.bottom() > other.y + EPSILON
    }
}

/// 布局平面上的一个物理屏幕。
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenDevice {
    pub id: String,
    /// 设备原生方向下的物理尺寸（毫米）。
    pub native_size: SizeMm,
    /// 旋转后矩形左上角在桌面平面上的坐标（毫米）。
    pub position: PointMm,
    pub rotation: Rotation,
}

impl ScreenDevice {
    pub fn new(
        id: impl Into<String>,
        native_size: SizeMm,
        position: PointMm,
    ) -> Result<Self, LayoutError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(LayoutError::EmptyId);
        }
        if !position.x.is_finite() || !position.y.is_finite() {
            return Err(LayoutError::InvalidPosition);
        }
        Ok(Self {
            id,
            native_size,
            position,
            rotation: Rotation::Deg0,
        })
    }

    pub fn with_rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// 旋转后的有效尺寸。
    pub fn effective_size(&self) -> SizeMm {
        self.native_size.rotated(self.rotation)
    }

    /// 旋转后的轴对齐矩形。
    pub fn rect(&self) -> RectMm {
        let size = self.effective_size();
        RectMm {
            x: self.position.x,
            y: self.position.y,
            width: size.width,
            height: size.height,
        }
    }
}

/// 两个屏幕之间的一条接缝。
#[derive(Debug, Clone, PartialEq)]
pub struct Seam {
    /// 接缝一侧的设备 ID。
    pub a: String,
    /// 接缝另一侧的设备 ID。
    pub b: String,
    pub a_edge: Edge,
    pub b_edge: Edge,
    pub axis: Axis,
    /// 接缝重叠区间在桌面平面上的起点（垂直缝为 y，水平缝为 x）。
    pub span_start: f64,
    /// 接缝重叠区间的终点。
    pub span_end: f64,
    /// 两屏边缘的间隙绝对值（毫米）。
    pub gap_mm: f64,
}

impl Seam {
    /// 只有间隙不超过容差的接缝才是可穿越的“绿边”。
    pub fn is_green(&self) -> bool {
        self.gap_mm <= SEAM_TOLERANCE_MM + EPSILON
    }

    pub fn span_len(&self) -> f64 {
        (self.span_end - self.span_start).max(0.0)
    }

    /// 判断沿接缝方向的坐标是否落在可穿越区间内。
    pub fn contains(&self, along: f64) -> bool {
        along >= self.span_start - EPSILON && along <= self.span_end + EPSILON
    }
}

/// 一次成功的跨屏穿越结果。`position` 是目标屏内的本地坐标（毫米）。
#[derive(Debug, Clone, PartialEq)]
pub struct Crossing {
    pub from: String,
    pub to: String,
    pub position: PointMm,
    pub seam: Seam,
}

/// 多屏物理布局。设备按插入顺序保存，保证算法输出可复现。
#[derive(Debug, Default, Clone)]
pub struct Layout {
    devices: Vec<ScreenDevice>,
}

impl Layout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn devices(&self) -> &[ScreenDevice] {
        &self.devices
    }

    pub fn get(&self, id: &str) -> Option<&ScreenDevice> {
        self.devices.iter().find(|device| device.id == id)
    }

    /// 插入或替换设备。设备 ID 必须非空，尺寸与坐标必须有效。
    pub fn upsert(&mut self, device: ScreenDevice) -> Result<(), LayoutError> {
        if device.id.trim().is_empty() {
            return Err(LayoutError::EmptyId);
        }
        SizeMm::new(device.native_size.width, device.native_size.height)?;
        if !device.position.x.is_finite() || !device.position.y.is_finite() {
            return Err(LayoutError::InvalidPosition);
        }
        match self.devices.iter_mut().find(|slot| slot.id == device.id) {
            Some(slot) => *slot = device,
            None => self.devices.push(device),
        }
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Option<ScreenDevice> {
        let index = self.devices.iter().position(|device| device.id == id)?;
        Some(self.devices.remove(index))
    }

    /// 计算拖动中的屏幕应吸附到的位置。
    ///
    /// 规则对应大象漫步公开说明：当移动屏与邻居的贴合位置相差不超过 12 mm，且两者在
    /// 接缝方向上有重叠时，把贴合一侧精确对齐到 0 mm；同时把另一轴吸附到顶部 / 底部
    /// 或居中对齐，便于形成整齐的桌面布局。超过 12 mm 或会与其他屏幕重叠时不吸附。
    pub fn snap_position(&self, moving: &ScreenDevice) -> PointMm {
        let size = moving.effective_size();
        let proposed = moving.position;
        let mut best: Option<(f64, PointMm)> = None;

        for other in &self.devices {
            if other.id == moving.id {
                continue;
            }
            let neighbor = other.rect();

            // 水平贴合：移动屏位于邻居右侧或左侧。
            for target_x in [neighbor.right(), neighbor.x - size.width] {
                let gap = (target_x - proposed.x).abs();
                if gap > SNAP_DISTANCE_MM + EPSILON {
                    continue;
                }
                if overlap_len(proposed.y, size.height, neighbor.y, neighbor.height) <= EPSILON {
                    continue;
                }
                let y = snap_parallel(proposed.y, size.height, neighbor.y, neighbor.bottom());
                let candidate = RectMm {
                    x: target_x,
                    y,
                    width: size.width,
                    height: size.height,
                };
                if self.overlaps_any(&moving.id, &candidate) {
                    continue;
                }
                consider(&mut best, gap, PointMm::new(target_x, y));
            }

            // 垂直贴合：移动屏位于邻居下方或上方。
            for target_y in [neighbor.bottom(), neighbor.y - size.height] {
                let gap = (target_y - proposed.y).abs();
                if gap > SNAP_DISTANCE_MM + EPSILON {
                    continue;
                }
                if overlap_len(proposed.x, size.width, neighbor.x, neighbor.width) <= EPSILON {
                    continue;
                }
                let x = snap_parallel(proposed.x, size.width, neighbor.x, neighbor.right());
                let candidate = RectMm {
                    x,
                    y: target_y,
                    width: size.width,
                    height: size.height,
                };
                if self.overlaps_any(&moving.id, &candidate) {
                    continue;
                }
                consider(&mut best, gap, PointMm::new(x, target_y));
            }
        }

        best.map(|(_, point)| point).unwrap_or(proposed)
    }

    /// 拖拽结束后写入吸附位置。
    pub fn snap_device(&mut self, id: &str, proposed: PointMm) -> Result<PointMm, LayoutError> {
        let index = self
            .devices
            .iter()
            .position(|device| device.id == id)
            .ok_or_else(|| LayoutError::UnknownDevice(id.to_string()))?;

        let mut moving = self.devices[index].clone();
        moving.position = proposed;
        let snapped = self.snap_position(&moving);
        self.devices[index].position = snapped;
        Ok(snapped)
    }

    /// 返回所有真实贴合的接缝（间隙不超过 0.5 mm 且存在重叠区间）。
    pub fn seams(&self) -> Vec<Seam> {
        let mut seams = Vec::new();
        for (index, a) in self.devices.iter().enumerate() {
            let rect_a = a.rect();
            for b in self.devices.iter().skip(index + 1) {
                let rect_b = b.rect();
                push_vertical(&mut seams, a, b, rect_a, rect_b);
                push_vertical(&mut seams, b, a, rect_b, rect_a);
                push_horizontal(&mut seams, a, b, rect_a, rect_b);
                push_horizontal(&mut seams, b, a, rect_b, rect_a);
            }
        }
        seams
    }

    /// 返回可穿越的“绿边”接缝。
    pub fn green_seams(&self) -> Vec<Seam> {
        self.seams().into_iter().filter(Seam::is_green).collect()
    }

    /// 尝试把指针从 `from_id` 的指定边缘移出。
    ///
    /// 只有同时满足以下条件才允许穿越，从机制上避免误触：
    /// 1. 指针确实位于该屏幕的边缘容差内；
    /// 2. 该边缘存在绿边接缝；
    /// 3. 指针的沿边坐标落在接缝的重叠区间内。
    ///
    /// 穿越后按物理毫米 1:1 保持沿边位置，并在进入方向内缩 0.5 mm。
    pub fn try_cross(&self, from_id: &str, point: PointMm, exit_edge: Edge) -> Option<Crossing> {
        let from = self.get(from_id)?;
        let from_rect = from.rect();

        let (normal_offset, along, along_start, along_end) = match exit_edge {
            Edge::Left => (
                point.x - from_rect.x,
                point.y,
                from_rect.y,
                from_rect.bottom(),
            ),
            Edge::Right => (
                point.x - from_rect.right(),
                point.y,
                from_rect.y,
                from_rect.bottom(),
            ),
            Edge::Top => (
                point.y - from_rect.y,
                point.x,
                from_rect.x,
                from_rect.right(),
            ),
            Edge::Bottom => (
                point.y - from_rect.bottom(),
                point.x,
                from_rect.x,
                from_rect.right(),
            ),
        };

        if normal_offset.abs() > SEAM_TOLERANCE_MM + EPSILON {
            return None;
        }
        if along < along_start - SEAM_TOLERANCE_MM || along > along_end + SEAM_TOLERANCE_MM {
            return None;
        }

        for seam in self.seams() {
            if !seam.is_green() {
                continue;
            }
            let is_from_a = seam.a == from_id && seam.a_edge == exit_edge;
            let is_from_b = seam.b == from_id && seam.b_edge == exit_edge;
            if !is_from_a && !is_from_b {
                continue;
            }
            if !seam.contains(along) {
                continue;
            }

            let target_id = if is_from_a { &seam.b } else { &seam.a };
            let target = self.get(target_id)?;
            let target_rect = target.rect();

            let position = match seam.axis {
                Axis::Vertical => {
                    let y = (along - target_rect.y).clamp(0.0, target_rect.height);
                    let x = match exit_edge {
                        // 从右侧移出 → 进入目标屏左边缘。
                        Edge::Right => CROSS_INSET_MM,
                        // 从左侧移出 → 进入目标屏右边缘。
                        Edge::Left => target_rect.width - CROSS_INSET_MM,
                        _ => return None,
                    };
                    PointMm::new(x, y)
                }
                Axis::Horizontal => {
                    let x = (along - target_rect.x).clamp(0.0, target_rect.width);
                    let y = match exit_edge {
                        Edge::Bottom => CROSS_INSET_MM,
                        Edge::Top => target_rect.height - CROSS_INSET_MM,
                        _ => return None,
                    };
                    PointMm::new(x, y)
                }
            };

            return Some(Crossing {
                from: from_id.to_string(),
                to: target_id.clone(),
                position,
                seam,
            });
        }

        None
    }

    /// 旋转设备，并沿相邻接缝重新锚定。
    ///
    /// 对应大象漫步公开说明里的行为：旋转平板后，设备不会跳回左上角，而是保持与
    /// 邻居的接缝接触，并以原接缝重叠区间的中点作为沿边锚点。没有邻居时保持中心不动。
    pub fn rotate_device(&mut self, id: &str, rotation: Rotation) -> Result<(), LayoutError> {
        let index = self
            .devices
            .iter()
            .position(|device| device.id == id)
            .ok_or_else(|| LayoutError::UnknownDevice(id.to_string()))?;

        let device = self.devices[index].clone();
        let old_rect = device.rect();
        let new_size = device.native_size.rotated(rotation);

        let anchor = self
            .seams()
            .into_iter()
            .filter(|seam| seam.a == id || seam.b == id)
            .min_by(|left, right| {
                left.gap_mm
                    .partial_cmp(&right.gap_mm)
                    .unwrap_or(Ordering::Equal)
            });

        let new_position = if let Some(seam) = anchor {
            let (my_edge, neighbor_id) = if seam.a == id {
                (seam.a_edge, seam.b.clone())
            } else {
                (seam.b_edge, seam.a.clone())
            };
            let neighbor_rect = self
                .get(&neighbor_id)
                .expect("接缝必然引用布局内的设备")
                .rect();
            let along = (seam.span_start + seam.span_end) / 2.0;
            anchor_along_seam(new_size, my_edge, &neighbor_rect, along)
        } else {
            PointMm::new(
                old_rect.center().x - new_size.width / 2.0,
                old_rect.center().y - new_size.height / 2.0,
            )
        };

        let device = &mut self.devices[index];
        device.rotation = rotation;
        device.position = new_position;
        Ok(())
    }

    fn overlaps_any(&self, exclude_id: &str, rect: &RectMm) -> bool {
        self.devices
            .iter()
            .filter(|device| device.id != exclude_id)
            .any(|device| rect.overlaps_interior(&device.rect()))
    }
}

/// 记录当前最优吸附候选：优先选择贴合距离最小的位置。
fn consider(best: &mut Option<(f64, PointMm)>, gap: f64, point: PointMm) {
    if best.map_or(true, |(best_gap, _)| gap < best_gap - EPSILON) {
        *best = Some((gap, point));
    }
}

/// 沿边方向吸附到顶部 / 底部 / 居中之一，距离超过 12 mm 时保持原值。
fn snap_parallel(value: f64, size: f64, low: f64, high: f64) -> f64 {
    let center = (low + high) / 2.0 - size / 2.0;
    [low, high - size, center]
        .into_iter()
        .filter(|candidate| (candidate - value).abs() <= SNAP_DISTANCE_MM + EPSILON)
        .min_by(|left, right| {
            (left - value)
                .abs()
                .partial_cmp(&(right - value).abs())
                .unwrap_or(Ordering::Equal)
        })
        .unwrap_or(value)
}

/// 两个一维区间重叠长度。
fn overlap_len(a_start: f64, a_len: f64, b_start: f64, b_len: f64) -> f64 {
    let a_end = a_start + a_len;
    let b_end = b_start + b_len;
    (a_end.min(b_end) - a_start.max(b_start)).max(0.0)
}

fn push_vertical(
    seams: &mut Vec<Seam>,
    a: &ScreenDevice,
    b: &ScreenDevice,
    rect_a: RectMm,
    rect_b: RectMm,
) {
    let gap = rect_b.x - rect_a.right();
    if gap.abs() > SEAM_TOLERANCE_MM + EPSILON {
        return;
    }
    let span_start = rect_a.y.max(rect_b.y);
    let span_end = rect_a.bottom().min(rect_b.bottom());
    if span_end - span_start <= EPSILON {
        return;
    }
    seams.push(Seam {
        a: a.id.clone(),
        b: b.id.clone(),
        a_edge: Edge::Right,
        b_edge: Edge::Left,
        axis: Axis::Vertical,
        span_start,
        span_end,
        gap_mm: gap.abs(),
    });
}

fn push_horizontal(
    seams: &mut Vec<Seam>,
    a: &ScreenDevice,
    b: &ScreenDevice,
    rect_a: RectMm,
    rect_b: RectMm,
) {
    let gap = rect_b.y - rect_a.bottom();
    if gap.abs() > SEAM_TOLERANCE_MM + EPSILON {
        return;
    }
    let span_start = rect_a.x.max(rect_b.x);
    let span_end = rect_a.right().min(rect_b.right());
    if span_end - span_start <= EPSILON {
        return;
    }
    seams.push(Seam {
        a: a.id.clone(),
        b: b.id.clone(),
        a_edge: Edge::Bottom,
        b_edge: Edge::Top,
        axis: Axis::Horizontal,
        span_start,
        span_end,
        gap_mm: gap.abs(),
    });
}

/// 按“我的哪条边贴住邻居”计算旋转后的左上角，沿边方向以 `along` 为中心。
fn anchor_along_seam(size: SizeMm, my_edge: Edge, neighbor: &RectMm, along: f64) -> PointMm {
    match my_edge {
        Edge::Left => PointMm::new(neighbor.right(), along - size.height / 2.0),
        Edge::Right => PointMm::new(neighbor.x - size.width, along - size.height / 2.0),
        Edge::Top => PointMm::new(along - size.width / 2.0, neighbor.bottom()),
        Edge::Bottom => PointMm::new(along - size.width / 2.0, neighbor.y - size.height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(id: &str, width: f64, height: f64, x: f64, y: f64) -> ScreenDevice {
        ScreenDevice::new(
            id,
            SizeMm::new(width, height).expect("尺寸有效"),
            PointMm::new(x, y),
        )
        .expect("设备有效")
    }

    fn layout(devices: Vec<ScreenDevice>) -> Layout {
        let mut layout = Layout::new();
        for device in devices {
            layout.upsert(device).expect("布局写入成功");
        }
        layout
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "期望 {expected}，实际 {actual}"
        );
    }

    #[test]
    fn snaps_to_neighbor_within_twelve_millimeters() {
        let layout = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 306.0, 4.0),
        ]);
        let moving = layout.get("b").expect("存在 b").clone();

        let snapped = layout.snap_position(&moving);

        assert_close(snapped.x, 300.0);
        assert_close(snapped.y, 0.0);
    }

    #[test]
    fn does_not_snap_beyond_twelve_millimeters() {
        let layout = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 313.0, 100.0),
        ]);
        let moving = layout.get("b").expect("存在 b").clone();

        assert_eq!(layout.snap_position(&moving), PointMm::new(313.0, 100.0));
    }

    #[test]
    fn seam_is_green_only_within_half_millimeter() {
        let green = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 300.4, 0.0),
        ]);
        let seams = green.seams();
        assert_eq!(seams.len(), 1);
        assert!(seams[0].is_green());
        assert_eq!(seams[0].a_edge, Edge::Right);
        assert_eq!(seams[0].b_edge, Edge::Left);

        let not_green = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 300.6, 0.0),
        ]);
        assert!(not_green.seams().is_empty());
    }

    #[test]
    fn crossing_requires_green_seam() {
        let green = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 300.0, 0.0),
        ]);
        let crossing = green
            .try_cross("a", PointMm::new(300.0, 100.0), Edge::Right)
            .expect("绿边可穿越");
        assert_eq!(crossing.to, "b");
        assert_close(crossing.position.x, CROSS_INSET_MM);
        assert_close(crossing.position.y, 100.0);

        let gap = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 300.6, 0.0),
        ]);
        assert!(gap
            .try_cross("a", PointMm::new(300.0, 100.0), Edge::Right)
            .is_none());
    }

    #[test]
    fn crossing_is_blocked_without_overlap() {
        let layout = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 300.0, 250.0),
        ]);

        assert!(layout
            .try_cross("a", PointMm::new(300.0, 100.0), Edge::Right)
            .is_none());
    }

    #[test]
    fn crossing_keeps_physical_position_on_partial_overlap() {
        let layout = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 300.0, 150.0),
        ]);

        let crossing = layout
            .try_cross("a", PointMm::new(300.0, 170.0), Edge::Right)
            .expect("落在重叠区间内");
        assert_close(crossing.position.y, 20.0);

        assert!(layout
            .try_cross("a", PointMm::new(300.0, 100.0), Edge::Right)
            .is_none());
    }

    #[test]
    fn crossing_back_enters_from_target_far_edge() {
        let layout = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 300.0, 0.0),
        ]);

        let crossing = layout
            .try_cross("b", PointMm::new(300.0, 170.0), Edge::Left)
            .expect("反向穿越");

        assert_eq!(crossing.to, "a");
        assert_close(crossing.position.x, 300.0 - CROSS_INSET_MM);
        assert_close(crossing.position.y, 170.0);
    }

    #[test]
    fn crossing_between_vertically_stacked_screens() {
        let layout = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 0.0, 200.0),
        ]);

        let crossing = layout
            .try_cross("a", PointMm::new(150.0, 200.0), Edge::Bottom)
            .expect("上下相邻可穿越");

        assert_eq!(crossing.to, "b");
        assert_close(crossing.position.x, 150.0);
        assert_close(crossing.position.y, CROSS_INSET_MM);
    }

    #[test]
    fn snap_device_writes_snapped_position_back() {
        let mut layout = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 308.0, 3.0),
        ]);

        let snapped = layout
            .snap_device("b", PointMm::new(308.0, 3.0))
            .expect("吸附成功");

        assert_eq!(snapped, PointMm::new(300.0, 0.0));
        assert_eq!(
            layout.get("b").expect("存在 b").position,
            PointMm::new(300.0, 0.0)
        );
        assert_eq!(layout.green_seams().len(), 1);
    }

    #[test]
    fn rotating_reanchors_along_neighbor_seam() {
        let mut layout = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 400.0, 300.0, 300.0, 0.0),
        ]);

        layout
            .rotate_device("b", Rotation::Deg90)
            .expect("旋转成功");

        let b = layout.get("b").expect("存在 b");
        // 有效尺寸变为 300×400，沿接缝重叠区间中点 y=100 居中。
        assert_close(b.position.x, 300.0);
        assert_close(b.position.y, -100.0);
        assert_eq!(b.rect().width, 300.0);
        assert_eq!(b.rect().height, 400.0);
        assert!(layout
            .green_seams()
            .iter()
            .any(|seam| { (seam.a == "a" && seam.b == "b") || (seam.a == "b" && seam.b == "a") }));
    }

    #[test]
    fn rotating_without_neighbor_keeps_center() {
        let mut layout = layout(vec![device("a", 300.0, 200.0, 10.0, 20.0)]);

        layout
            .rotate_device("a", Rotation::Deg90)
            .expect("旋转成功");

        let a = layout.get("a").expect("存在 a");
        assert_close(a.position.x, 60.0);
        assert_close(a.position.y, -30.0);
        assert_close(a.rect().center().x, 160.0);
        assert_close(a.rect().center().y, 120.0);
    }

    #[test]
    fn upsert_replaces_and_remove_clears_seams() {
        let mut layout = layout(vec![
            device("a", 300.0, 200.0, 0.0, 0.0),
            device("b", 300.0, 200.0, 300.0, 0.0),
        ]);
        assert_eq!(layout.seams().len(), 1);

        layout
            .upsert(device("b", 300.0, 200.0, 900.0, 0.0))
            .expect("替换成功");
        assert!(layout.seams().is_empty());

        assert!(layout.remove("a").is_some());
        assert!(layout.get("a").is_none());
    }

    #[test]
    fn rejects_invalid_devices() {
        let mut layout = Layout::new();
        let invalid_size = ScreenDevice {
            id: "a".into(),
            native_size: SizeMm {
                width: 0.0,
                height: 200.0,
            },
            position: PointMm::new(0.0, 0.0),
            rotation: Rotation::Deg0,
        };
        assert_eq!(layout.upsert(invalid_size), Err(LayoutError::InvalidSize));

        let invalid_position = ScreenDevice {
            id: "a".into(),
            native_size: SizeMm::new(300.0, 200.0).expect("尺寸有效"),
            position: PointMm::new(f64::NAN, 0.0),
            rotation: Rotation::Deg0,
        };
        assert_eq!(
            layout.upsert(invalid_position),
            Err(LayoutError::InvalidPosition)
        );
    }
}
