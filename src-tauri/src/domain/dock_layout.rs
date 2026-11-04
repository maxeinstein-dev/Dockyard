//! The dock's geometry, computed instead of measured (spec 0011 H3): where
//! the bar, the items and their tiles sit in the dock window, how much each
//! item grows under the pointer, the window the dock needs and where a
//! dragged program would land. The same rules the page's dock followed
//! (`src/dock/layout.ts`, `magnification.ts` and `drag.ts`), with the same
//! numbers, in logical pixels:
//!
//! - the bar is `icon + 2 × pad` tall, flush with the window edge that faces
//!   the screen edge and centered across the window;
//! - each item is `icon × scale` wide (the magnification pushes its
//!   neighbours apart), `gap` apart, `pad` inside the bar;
//! - a tile grows from the middle of its screen-side edge and moves `lift`
//!   away from it;
//! - the label pill sits `TIP_GAP` beyond the tile, on the far side;
//! - a spacer (spec 0022) is an item with no tile of its own: a quarter or
//!   half of an icon of room (`SpacerSize::length`), that never grows and
//!   that the pointer never lands on (`hit`).

use super::settings::{DockEdge, HoverEffect, SizePreset};
use super::workspaces::SpacerSize;

/// A rectangle in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Area {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }

    pub fn center_x(&self) -> f64 {
        self.x + self.width / 2.0
    }

    pub fn center_y(&self) -> f64 {
        self.y + self.height / 2.0
    }

    /// The same rectangle with x and y swapped: how a lying bar's area
    /// becomes a standing one's (spec 0028 §5, step 1).
    pub fn transposed(self) -> Area {
        Area {
            x: self.y,
            y: self.x,
            width: self.height,
            height: self.width,
        }
    }

    /// The same rectangle flipped along y in a window `height` tall.
    fn flipped(self, height: f64) -> Area {
        Area {
            y: height - self.y - self.height,
            ..self
        }
    }
}

/// The screen edge the dock sits on, in the layout's terms (spec 0028): the
/// two of the settings today, and the two sides to come.
///
/// The sides are not laid out on their own. The bar is laid out lying, as
/// on the bottom or the top, in the viewport turned on its side, and every
/// area is then turned back (`Area::transposed`): each rule of the
/// magnification, the spacers, the stacks and the strip holds on the four
/// edges without being written twice. On the left the bar reads from the
/// top down, as the bottom reads from the left; on the right it is the
/// left's turned mirror, as on the PrimoDock (spec 0028 Q2): the clock on
/// top and the workspace dots at the foot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bottom,
    Top,
    // The settings reach the sides from step 2 of spec 0028; until then only
    // the tests lay them out.
    #[allow(dead_code)]
    Left,
    #[allow(dead_code)]
    Right,
}

impl Side {
    /// Whether the bar stands (a column) instead of lying (a row).
    pub fn standing(self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }

    /// The lying edge a side is laid out as before it is turned: the screen
    /// edge nearest the origin (the top, the left) is the top.
    fn lying(self) -> DockEdge {
        match self {
            Side::Bottom | Side::Right => DockEdge::Bottom,
            Side::Top | Side::Left => DockEdge::Top,
        }
    }
}

impl From<DockEdge> for Side {
    fn from(edge: DockEdge) -> Side {
        match edge {
            DockEdge::Bottom => Side::Bottom,
            DockEdge::Top => Side::Top,
        }
    }
}

/// The size presets the dock draws (DESIGN.md "Layout"): every icon size of
/// the settings is drawn as the nearest one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSize {
    Small,
    Medium,
    Large,
}

/// Icon, padding and gap of a size preset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeGeometry {
    pub icon: f64,
    pub pad: f64,
    pub gap: f64,
}

impl DockSize {
    /// The preset closest to `icon_size`; a tie goes to the smaller one.
    pub fn of(icon_size: u16) -> DockSize {
        let presets = [
            (SizePreset::Small, DockSize::Small),
            (SizePreset::Medium, DockSize::Medium),
            (SizePreset::Large, DockSize::Large),
        ];
        let mut best = DockSize::Medium;
        let mut best_distance = u16::MAX;
        for (preset, size) in presets {
            let distance = preset.icon_size().abs_diff(icon_size);
            if distance < best_distance {
                best = size;
                best_distance = distance;
            }
        }
        best
    }

    pub const fn geometry(self) -> SizeGeometry {
        match self {
            DockSize::Small => SizeGeometry {
                icon: 40.0,
                pad: 6.0,
                gap: 6.0,
            },
            DockSize::Medium => SizeGeometry {
                icon: 48.0,
                pad: 7.0,
                gap: 7.0,
            },
            DockSize::Large => SizeGeometry {
                icon: 64.0,
                pad: 9.0,
                gap: 9.0,
            },
        }
    }
}

/// Distance between a tile and its label pill (DESIGN.md "Tooltip").
pub const TIP_GAP: f64 = 10.0;

/// Height of the label pill: a 16 px line plus 4 px above and below.
pub const TIP_HEIGHT: f64 = 24.0;

/// Width of the clock and date at the end of the bar.
pub const STATUS_WIDTH: f64 = 76.0;

/// Width of the column of workspace dots at the start of the bar (spec
/// 0013), in logical pixels.
pub const SWITCHER_WIDTH: f64 = 12.0;

/// Radius of a workspace dot, and of the one shown.
pub const DOT_RADIUS: f64 = 3.0;
pub const ACTIVE_DOT_RADIUS: f64 = 4.0;

/// The centres of `count` workspace dots in the column `area`, top first:
/// spread over its height, at most 12 px apart.
pub fn switcher_dots(area: Area, count: usize) -> Vec<(f64, f64)> {
    if count == 0 {
        return Vec::new();
    }
    let pitch = (area.height / count as f64).min(12.0);
    let first = area.y + area.height / 2.0 - pitch * (count as f64 - 1.0) / 2.0;
    (0..count)
        .map(|index| (area.center_x(), first + pitch * index as f64))
        .collect()
}

/// The dot of the column `area` under the point, by its row: the one whose
/// centre is nearest across, within the column.
pub fn switcher_hit(area: Area, count: usize, x: f64, y: f64) -> Option<usize> {
    if !area.contains(x, y) {
        return None;
    }
    switcher_dots(area, count)
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (a.1 - y).abs().total_cmp(&(b.1 - y).abs()))
        .map(|(index, _)| index)
}

/// The side of a small icon's cell in the tray strip (two rows in the
/// height of an item), at most `MAX_TRAY_CELL`.
pub fn tray_cell(icon: f64) -> f64 {
    (icon / 2.0).floor().min(MAX_TRAY_CELL)
}

const MAX_TRAY_CELL: f64 = 28.0;

/// Width of a tray strip of `count` small icons, two to a column.
pub fn tray_width(icon: f64, count: usize) -> f64 {
    count.div_ceil(2) as f64 * tray_cell(icon)
}

/// The cells of a tray strip in `area` (as tall as an item): column by
/// column, the top one first.
pub fn tray_cells(area: Area, count: usize, icon: f64) -> Vec<Area> {
    let cell = tray_cell(icon);
    let top = area.y + (area.height - 2.0 * cell) / 2.0;
    (0..count)
        .map(|index| Area {
            x: area.x + (index / 2) as f64 * cell,
            y: top + (index % 2) as f64 * cell,
            width: cell,
            height: cell,
        })
        .collect()
}

/// The notification badge on a tile (spec 0016): a pill on its top right
/// corner, reaching a little beyond it, as on a phone. `text_width` is the
/// width of its number (0 for a badge with no number, which is a smaller
/// round dot).
pub fn badge_area(tile: Area, text_width: f64) -> Area {
    let height = (tile.height * BADGE_SHARE).clamp(BADGE_MIN, BADGE_MAX);
    let (width, height) = if text_width > 0.0 {
        (height.max(text_width + height * 0.6), height)
    } else {
        (height * 0.65, height * 0.65)
    };
    Area {
        x: tile.x + tile.width + BADGE_OUT - width,
        y: tile.y - BADGE_OUT,
        width,
        height,
    }
}

/// The badge's height, as a share of the tile's, within its limits.
const BADGE_SHARE: f64 = 0.38;
const BADGE_MIN: f64 = 14.0;
const BADGE_MAX: f64 = 22.0;
/// How far the badge reaches beyond the tile's corner.
const BADGE_OUT: f64 = 3.0;

/// The size of a badge's number for a badge `height` tall.
pub fn badge_text_size(height: f64) -> f64 {
    height * 0.68
}

/// Transparent room the window keeps beyond the label and on both sides of
/// the widest bar, for the pill's shadow and the bar's.
pub const ROOM: f64 = 8.0;

/// Width a separator takes between two groups, its margins included (a 1 px
/// line with `gap × 0.4` on each side), besides the gap that follows it.
pub fn separator_width(gap: f64) -> f64 {
    1.0 + gap * 0.8
}

/// How much an item grows: its tile is scaled by `scale` from the middle of
/// its screen-side edge, then moved `lift` away from it. The item's box
/// widens by the same `scale`, so the growth pushes the neighbours apart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Magnification {
    pub scale: f64,
    pub lift: f64,
}

const REST: Magnification = Magnification {
    scale: 1.0,
    lift: 0.0,
};

/// The "magnify" effect (DESIGN.md "Layout", Ampliação B1): the hovered item
/// and each direct neighbour.
pub const MAGNIFY_HOVERED: Magnification = Magnification {
    scale: 1.4,
    lift: 6.0,
};
pub const MAGNIFY_NEIGHBOR: Magnification = Magnification {
    scale: 1.15,
    lift: 0.0,
};

/// What an item does under the pointer, and how much the "scale" effect
/// grows it (a ratio: 1.1 is 110 %).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hover {
    pub effect: HoverEffect,
    pub scale: f64,
}

impl Hover {
    pub fn of(effect: HoverEffect, percent: u16) -> Hover {
        Hover {
            effect,
            scale: f64::from(percent) / 100.0,
        }
    }

    /// The growth of the hovered item at its peak: what the window must
    /// leave room for.
    pub fn peak(self) -> Magnification {
        match self.effect {
            HoverEffect::None => REST,
            HoverEffect::Scale => Magnification {
                scale: self.scale,
                lift: 0.0,
            },
            HoverEffect::Magnify => MAGNIFY_HOVERED,
        }
    }

    /// The growth of the item at `index` while the item at `magnified` is
    /// hovered.
    pub fn of_item(self, index: usize, magnified: Option<usize>) -> Magnification {
        let Some(magnified) = magnified else {
            return REST;
        };
        match self.effect {
            HoverEffect::None => REST,
            _ if index == magnified => self.peak(),
            HoverEffect::Scale => REST,
            HoverEffect::Magnify if index.abs_diff(magnified) == 1 => MAGNIFY_NEIGHBOR,
            HoverEffect::Magnify => REST,
        }
    }
}

/// What the dock shows: its size, the number of items in each group (a
/// separator stands between groups that have items), what an item does
/// under the pointer and whether the clock ends the bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub size: DockSize,
    pub groups: Vec<usize>,
    pub hover: Hover,
    pub status: bool,
    /// The small icons at the end (the tray's arrow, the battery, the
    /// network, the volume, the tray icons kept in the dock): 0 for none.
    pub tray: usize,
    /// The workspaces (spec 0013): with two or more, a column of dots starts
    /// the bar.
    pub switcher: usize,
    /// How many icons wide each item is, in order (spec 0017: a widget
    /// takes 1 to 4); an item past the list is one icon wide.
    pub spans: Vec<usize>,
    /// Which items are spacers (spec 0022), and their size, in order; an
    /// item past the list is not one.
    pub spacers: Vec<Option<SpacerSize>>,
}

impl Shape {
    /// The size of the item at `index` when it is a spacer.
    pub fn spacer(&self, index: usize) -> Option<SpacerSize> {
        self.spacers.get(index).copied().flatten()
    }
}

/// The width of an item `span` icons wide, at rest: the icons and the gaps
/// between them, so a wide item lines up with the icons it replaces.
pub fn span_width(icon: f64, gap: f64, span: usize) -> f64 {
    let span = span.clamp(1, 4) as f64;
    icon * span + gap * (span - 1.0)
}

/// One item along the bar: its left edge from the bar's outer left edge,
/// its width and its growth.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RowItem {
    x: f64,
    width: f64,
    magnification: Magnification,
}

/// Lays the items out along the bar with each item at `scales` (as it is
/// now, mid-animation included): the bar's width, each item and where the
/// separators and the clock sit (from the bar's outer left edge).
#[derive(Debug, Clone, PartialEq)]
struct Row {
    width: f64,
    items: Vec<RowItem>,
    separators: Vec<f64>,
    switcher: Option<f64>,
    tray: Option<f64>,
    status: Option<f64>,
}

fn row(shape: &Shape, growth: &dyn Fn(usize) -> Magnification) -> Row {
    let SizeGeometry { icon, pad, gap } = shape.size.geometry();
    let mut items = Vec::new();
    let mut separators = Vec::new();
    let mut x = pad;
    // [switcher][gap][separator][gap][items]: the dots, then the items.
    let switcher = (shape.switcher >= 2).then(|| {
        let at = x;
        x += SWITCHER_WIDTH + gap;
        separators.push(x + gap * 0.4);
        x += separator_width(gap) + gap;
        at
    });
    for &count in shape.groups.iter().filter(|&&count| count > 0) {
        if !items.is_empty() {
            separators.push(x + gap * 0.4);
            x += separator_width(gap) + gap;
        }
        for _ in 0..count {
            let index = items.len();
            // A spacer keeps its room whatever is magnified beside it.
            let spacer = shape.spacer(index);
            let magnification = if spacer.is_some() {
                REST
            } else {
                growth(index)
            };
            let span = shape.spans.get(index).copied().unwrap_or(1);
            let width = match spacer {
                Some(size) => size.length(icon),
                None => span_width(icon, gap, span) * magnification.scale,
            };
            items.push(RowItem {
                x,
                width,
                magnification,
            });
            x += width + gap;
        }
    }
    let mut status = None;
    let mut tray = None;
    if shape.status || shape.tray > 0 {
        // [item][gap][separator][gap][tray][gap][status][gap]: the way two
        // groups meet.
        if !items.is_empty() {
            separators.push(x + gap * 0.4);
            x += separator_width(gap) + gap;
        }
        if shape.tray > 0 {
            tray = Some(x);
            x += tray_width(icon, shape.tray) + gap;
        }
        if shape.status {
            status = Some(x);
            x += STATUS_WIDTH + gap;
        }
    }
    let width = if items.is_empty() && !shape.status && shape.tray == 0 && switcher.is_none() {
        2.0 * pad
    } else {
        x - gap + pad
    };
    Row {
        width,
        items,
        separators,
        switcher,
        tray,
        status,
    }
}

/// Rounds up to whole pixels, ignoring floating-point noise.
fn ceil(value: f64) -> f64 {
    (value - 1e-6).ceil()
}

/// The window the dock needs, whatever is magnified or labelled: the bar,
/// the magnified tile and the label pill beyond it, plus `ROOM`; and,
/// across, the widest the bar gets or the widest label, plus `ROOM` on each
/// side. `label_widths` are the pills of every label the dock can show: its
/// items', its strip cells' and its workspace dots'. A pill sits over what
/// it names and moves in to stay inside the window (`tip_center`), so the
/// window never grows past the widest of them.
pub fn window_size(
    shape: &Shape,
    side: impl Into<Side>,
    motion: bool,
    label_widths: &[f64],
) -> (f64, f64) {
    let (along, across) = lying_window(shape, motion, label_widths);
    // On a side, the lying window turned on its side (spec 0028).
    if side.into().standing() {
        (across, along)
    } else {
        (along, across)
    }
}

/// The window of a lying bar: along it, then across.
fn lying_window(shape: &Shape, motion: bool, label_widths: &[f64]) -> (f64, f64) {
    let SizeGeometry { icon, pad, .. } = shape.size.geometry();
    let peak = shape.hover.peak();
    let grows = motion && peak.scale != 1.0;
    let Magnification { scale, lift } = if grows { peak } else { REST };
    let height = pad + icon * scale + lift + TIP_GAP + TIP_HEIGHT + ROOM;

    let count = row(shape, &|_| REST).items.len();
    let mut states = vec![None];
    if grows {
        states.extend((0..count).map(Some));
    }
    let bar = states
        .into_iter()
        .map(|state| row(shape, &|index| shape.hover.of_item(index, state)).width)
        .fold(0.0, f64::max);
    let label = label_widths.iter().copied().fold(0.0, f64::max);
    (ceil(bar.max(label) + 2.0 * ROOM), ceil(height))
}

/// Where everything sits in a window of `viewport`.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// The visible bar.
    pub bar: Area,
    /// Each item's box.
    pub items: Vec<Area>,
    /// Each item's tile, grown.
    pub tiles: Vec<Area>,
    /// Each item's scale.
    pub scales: Vec<f64>,
    /// The x of each separator's line.
    pub separators: Vec<f64>,
    /// The column of workspace dots, when it starts the bar.
    pub switcher: Option<Area>,
    /// The strip of small icons, when there is one.
    pub tray: Option<Area>,
    /// The clock's box, when it ends the bar.
    pub status: Option<Area>,
    /// The edge it was laid out for: on a side, `separators` are the lines'
    /// y, and `column_at` reads the bar along y.
    pub side: Side,
}

/// Where the bar, the items and the tiles are when each item has grown as
/// `growth` says, on any of the four edges (spec 0028): on the bottom and
/// the top laid out lying; on a side, laid out lying in the viewport turned
/// on its side and turned back, the right read from the bottom up.
pub fn place_with(
    shape: &Shape,
    side: impl Into<Side>,
    viewport: (f64, f64),
    growth: &dyn Fn(usize) -> Magnification,
) -> Placement {
    let side = side.into();
    if !side.standing() {
        return lying(shape, side.lying(), viewport, growth);
    }
    let turned = Placement {
        side,
        ..lying(shape, side.lying(), (viewport.1, viewport.0), growth).transposed()
    };
    match side {
        Side::Right => turned.flipped(viewport.1),
        _ => turned,
    }
}

/// The bar laid out lying, on the bottom or the top of `viewport`.
fn lying(
    shape: &Shape,
    edge: DockEdge,
    viewport: (f64, f64),
    growth: &dyn Fn(usize) -> Magnification,
) -> Placement {
    let SizeGeometry { icon, pad, .. } = shape.size.geometry();
    let row = row(shape, growth);
    let height = icon + 2.0 * pad;
    let bar = Area {
        x: (viewport.0 - row.width) / 2.0,
        y: match edge {
            DockEdge::Bottom => viewport.1 - height,
            DockEdge::Top => 0.0,
        },
        width: row.width,
        height,
    };
    let top = bar.y + pad;
    Placement {
        bar,
        items: row
            .items
            .iter()
            .map(|item| Area {
                x: bar.x + item.x,
                y: top,
                width: item.width,
                height: icon,
            })
            .collect(),
        tiles: row
            .items
            .iter()
            .map(|item| {
                // A wide tile grows as tall as a square one does.
                let tall = icon * item.magnification.scale;
                Area {
                    x: bar.x + item.x,
                    y: match edge {
                        DockEdge::Bottom => top + icon - item.magnification.lift - tall,
                        DockEdge::Top => top + item.magnification.lift,
                    },
                    width: item.width,
                    height: tall,
                }
            })
            .collect(),
        scales: row
            .items
            .iter()
            .map(|item| item.magnification.scale)
            .collect(),
        separators: row.separators.iter().map(|x| bar.x + x).collect(),
        switcher: row.switcher.map(|x| Area {
            x: bar.x + x,
            y: top,
            width: SWITCHER_WIDTH,
            height: icon,
        }),
        tray: row.tray.map(|x| Area {
            x: bar.x + x,
            y: top,
            width: tray_width(icon, shape.tray),
            height: icon,
        }),
        status: row.status.map(|x| Area {
            x: bar.x + x,
            y: top,
            width: STATUS_WIDTH,
            height: icon,
        }),
        side: edge.into(),
    }
}

impl Placement {
    /// Every area with x and y swapped; the separators are positions along
    /// the bar, so they stay as they are.
    fn transposed(self) -> Placement {
        Placement {
            bar: self.bar.transposed(),
            items: self.items.into_iter().map(Area::transposed).collect(),
            tiles: self.tiles.into_iter().map(Area::transposed).collect(),
            scales: self.scales,
            separators: self.separators,
            switcher: self.switcher.map(Area::transposed),
            tray: self.tray.map(Area::transposed),
            status: self.status.map(Area::transposed),
            side: self.side,
        }
    }

    /// Everything flipped along y in a window `height` tall: the order of
    /// the bar read from the bottom up.
    fn flipped(self, height: f64) -> Placement {
        let flip = |area: Area| area.flipped(height);
        Placement {
            bar: flip(self.bar),
            items: self.items.into_iter().map(flip).collect(),
            tiles: self.tiles.into_iter().map(flip).collect(),
            scales: self.scales,
            separators: self.separators.iter().map(|y| height - y).collect(),
            switcher: self.switcher.map(flip),
            tray: self.tray.map(flip),
            status: self.status.map(flip),
            side: self.side,
        }
    }
}

/// Where everything ends up once the magnification of `magnified` settles.
pub fn place(
    shape: &Shape,
    side: impl Into<Side>,
    viewport: (f64, f64),
    magnified: Option<usize>,
) -> Placement {
    place_with(shape, side, viewport, &|index| {
        shape.hover.of_item(index, magnified)
    })
}

/// Where the label pill of `tile` is anchored: the middle of its edge nearest
/// the tile, `TIP_GAP` beyond the tile on the far side.
///
/// On a side (spec 0028), the middle of the tile's edge away from the screen
/// edge, `TIP_GAP` beyond it.
pub fn tip_anchor(tile: Area, side: impl Into<Side>) -> (f64, f64) {
    match side.into() {
        Side::Bottom => (tile.center_x(), tile.y - TIP_GAP),
        Side::Top => (tile.center_x(), tile.y + tile.height + TIP_GAP),
        Side::Left => (tile.x + tile.width + TIP_GAP, tile.center_y()),
        Side::Right => (tile.x - TIP_GAP, tile.center_y()),
    }
}

/// Where across a label pill `width` wide is centred, in a window `window`
/// wide, for what it names centred at `x`: over it, moved in just enough to
/// keep `ROOM` between the pill and each side of the window; in the middle
/// of a window too narrow for it.
pub fn tip_center(x: f64, width: f64, window: f64) -> f64 {
    let low = ROOM + width / 2.0;
    let high = window - low;
    if low > high {
        window / 2.0
    } else {
        x.clamp(low, high)
    }
}

/// The item whose column holds the point (logical, in the window), as
/// placed in `placement`: inside the bar or on a grown tile, from half a gap
/// before the item to half a gap after it. Spacers included: what the right
/// button and a drag find.
pub fn column_at(shape: &Shape, placement: &Placement, x: f64, y: f64) -> Option<usize> {
    let gap = shape.size.geometry().gap;
    let on_tile = placement
        .tiles
        .iter()
        .enumerate()
        .any(|(index, tile)| shape.spacer(index).is_none() && tile.contains(x, y));
    if !placement.bar.contains(x, y) && !on_tile {
        return None;
    }
    // On a side (spec 0028) the column is a row of the bar, along y.
    let standing = placement.side.standing();
    placement.items.iter().position(|item| {
        let (at, start, length) = if standing {
            (y, item.y, item.height)
        } else {
            (x, item.x, item.width)
        };
        at >= start - gap / 2.0 && at < start + length + gap / 2.0
    })
}

/// The item under the point: what the pointer hovers, magnifies, labels
/// and clicks. A spacer is none of these (spec 0022): its room is the
/// dock's own.
pub fn hit(shape: &Shape, placement: &Placement, x: f64, y: f64) -> Option<usize> {
    column_at(shape, placement, x, y).filter(|&index| shape.spacer(index).is_none())
}

/// The spacer under the point: its own menu on the right button, and its
/// drag.
pub fn spacer_hit(shape: &Shape, placement: &Placement, x: f64, y: f64) -> Option<usize> {
    column_at(shape, placement, x, y).filter(|&index| shape.spacer(index).is_some())
}

/// How far the pointer goes before a press becomes a drag: a click that
/// moves a little is still a click.
pub const DRAG_THRESHOLD: f64 = 6.0;

/// Where a pinned program would go, among the pinned programs, if the
/// pointer were let go at `x`: the number of the other programs whose centre
/// is before it.
pub fn drop_index(centers: &[f64], from: usize, x: f64) -> usize {
    centers
        .iter()
        .enumerate()
        .filter(|&(index, &center)| index != from && center < x)
        .count()
}

/// How far the dragged item rises off the bar, in logical pixels.
pub const DRAG_LIFT: f64 = 6.0;
/// How much it grows while held.
pub const DRAG_GROW: f64 = 1.08;

/// How far the others step aside for the dragged item: its width and the
/// gap after it, read from where the items of its group rest.
pub fn drag_pitch(items: &[Area], from: usize) -> f64 {
    let Some(item) = items.get(from) else {
        return 0.0;
    };
    let gap = match (
        items.get(from + 1),
        from.checked_sub(1).and_then(|i| items.get(i)),
    ) {
        (Some(next), _) => next.x - (item.x + item.width),
        (None, Some(before)) => item.x - (before.x + before.width),
        (None, None) => 0.0,
    };
    item.width + gap
}

/// How far each item of a group moves aside while the one at `from` is
/// dragged to `target`: those in between make room for it, by `pitch`. The
/// dragged one is 0 here: it follows the pointer.
pub fn make_room(count: usize, from: usize, target: usize, pitch: f64) -> Vec<f64> {
    (0..count)
        .map(|index| {
            if from < index && index <= target {
                -pitch
            } else if target <= index && index < from {
                pitch
            } else {
                0.0
            }
        })
        .collect()
}

/// One frame of an item sliding from `now` toward `target`: a part of the
/// way, and there once it is close.
pub fn slide_step(now: f64, target: f64) -> f64 {
    let next = now + (target - now) * 0.3;
    if (target - next).abs() < 0.25 {
        target
    } else {
        next
    }
}

/// Keeps the dragged tile inside the bar: `x` is the pointer, `start` where
/// the drag began, and the tile can go from `min` to `max`.
pub fn drag_offset(x: f64, start: f64, rest_left: f64, width: f64, min: f64, max: f64) -> f64 {
    (x - start)
        .max(min - rest_left)
        .min(max - width - rest_left)
}

/// The three buttons of a Tocando agora three icons wide (spec 0017 F4),
/// left to right: previous, play or pause, next. On the lower line of the
/// column right of the cover, centered in it (the title and the artist are
/// on the upper one); the dock draws them and takes their clicks.
pub fn media_buttons(tile: Area) -> [Area; 3] {
    let u = tile.height;
    let side = u * 0.3;
    let step = side * 1.7;
    let column = tile.x + u;
    let center = (column + tile.x + tile.width - u * 0.12) / 2.0;
    let left = center - side / 2.0 - step;
    let y = tile.y + u * 0.68 - side / 2.0;
    std::array::from_fn(|index| Area {
        x: left + index as f64 * step,
        y,
        width: side,
        height: side,
    })
}

/// Seconds a long text stands at its start, and at its end, before it moves.
const MARQUEE_WAIT: f64 = 3.0;
const MARQUEE_REST: f64 = 2.0;
/// How fast it moves, in logical pixels a second.
const MARQUEE_SPEED: f64 = 22.0;
/// A frame while it moves: 30 a second.
const MARQUEE_FRAME: f64 = 1.0 / 30.0;

/// Where a text `overflow` pixels too long for its room is, `t` seconds
/// after it showed (spec 0017 F4): it waits at its start, rolls to its end,
/// rests there and rolls back, over and over. The offset to draw it at
/// (0 to `overflow`), and the seconds until it moves again (a frame while
/// it moves).
pub fn marquee(t: f64, overflow: f64) -> (f64, f64) {
    if overflow <= 0.0 {
        return (0.0, f64::INFINITY);
    }
    let travel = overflow / MARQUEE_SPEED;
    let cycle = MARQUEE_WAIT + travel + MARQUEE_REST + travel;
    let at = t.max(0.0) % cycle;
    if at < MARQUEE_WAIT {
        (0.0, MARQUEE_WAIT - at)
    } else if at < MARQUEE_WAIT + travel {
        ((at - MARQUEE_WAIT) * MARQUEE_SPEED, MARQUEE_FRAME)
    } else if at < MARQUEE_WAIT + travel + MARQUEE_REST {
        (overflow, MARQUEE_WAIT + travel + MARQUEE_REST - at)
    } else {
        let back = at - MARQUEE_WAIT - travel - MARQUEE_REST;
        ((overflow - back * MARQUEE_SPEED).max(0.0), MARQUEE_FRAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Configurações and Sobre, one group, with the Fase 1 magnification.
    fn shape(size: DockSize) -> Shape {
        Shape {
            size,
            groups: vec![2],
            hover: Hover {
                effect: HoverEffect::Magnify,
                scale: 1.4,
            },
            status: false,
            tray: 0,
            switcher: 0,
            spans: vec![],
            spacers: vec![],
        }
    }

    const LABELS: [f64; 2] = [110.0, 60.0];

    fn area(x: f64, y: f64, width: f64, height: f64) -> Area {
        Area {
            x,
            y,
            width,
            height,
        }
    }

    fn close(actual: Area, expected: Area) {
        for (a, e) in [
            (actual.x, expected.x),
            (actual.y, expected.y),
            (actual.width, expected.width),
            (actual.height, expected.height),
        ] {
            assert!((a - e).abs() < 1e-6, "{actual:?} != {expected:?}");
        }
    }

    #[test]
    fn a_wide_item_takes_its_icons_and_the_gaps_between_them() {
        assert_eq!(span_width(48.0, 7.0, 1), 48.0);
        assert_eq!(span_width(48.0, 7.0, 2), 103.0);
        assert_eq!(span_width(48.0, 7.0, 4), 213.0);
        // Out of range: one to four.
        assert_eq!(span_width(48.0, 7.0, 0), 48.0);
        assert_eq!(span_width(48.0, 7.0, 9), 213.0);
        let shape = Shape {
            groups: vec![3],
            hover: Hover::of(HoverEffect::None, 100),
            spans: vec![1, 2],
            ..shape(DockSize::Medium)
        };
        let wide = place(&shape, DockEdge::Bottom, (500.0, 123.0), None);
        let square = place(
            &Shape {
                spans: vec![],
                ..shape.clone()
            },
            DockEdge::Bottom,
            (500.0, 123.0),
            None,
        );
        // The bar grows by one icon and one gap; the wide tile is as tall as
        // the others, and the third item moves over.
        assert!((wide.bar.width - (square.bar.width + 55.0)).abs() < 1e-9);
        assert!((wide.items[1].width - 103.0).abs() < 1e-9);
        assert!((wide.tiles[1].height - 48.0).abs() < 1e-9);
        assert!((wide.tiles[1].width - 103.0).abs() < 1e-9);
        assert!((wide.items[2].x - (wide.items[1].x + 103.0 + 7.0)).abs() < 1e-9);
    }

    /// A program, a small spacer, a program, a large spacer, a program.
    fn spaced() -> Shape {
        Shape {
            groups: vec![5],
            spacers: vec![None, Some(SpacerSize::Small), None, Some(SpacerSize::Large)],
            ..shape(DockSize::Medium)
        }
    }

    #[test]
    fn spacers_leave_a_quarter_or_half_an_icon_on_both_edges() {
        for edge in [DockEdge::Bottom, DockEdge::Top] {
            let placement = place(&spaced(), edge, (500.0, 123.0), None);
            let items = &placement.items;
            // Their room along the bar, the gaps on each side as any item.
            assert!((items[1].width - 12.0).abs() < 1e-9, "{edge:?}");
            assert!((items[3].width - 24.0).abs() < 1e-9, "{edge:?}");
            assert!((items[1].x - (items[0].x + 48.0 + 7.0)).abs() < 1e-9);
            assert!((items[2].x - (items[1].x + 12.0 + 7.0)).abs() < 1e-9);
            assert!((items[4].x - (items[3].x + 24.0 + 7.0)).abs() < 1e-9);
            // The bar: the pads, three icons, the two spacers, four gaps.
            let width = 7.0 + 3.0 * 48.0 + 12.0 + 24.0 + 4.0 * 7.0 + 7.0;
            assert!((placement.bar.width - width).abs() < 1e-9);
            // As tall as an icon, in the bar like the others.
            assert_eq!(items[1].y, items[0].y);
            assert_eq!(items[1].height, 48.0);
        }
        // On the large size, a quarter and half of 64.
        let large = place(
            &Shape {
                size: DockSize::Large,
                ..spaced()
            },
            DockEdge::Bottom,
            (600.0, 147.0),
            None,
        );
        assert!((large.items[1].width - 16.0).abs() < 1e-9);
        assert!((large.items[3].width - 32.0).abs() < 1e-9);
    }

    #[test]
    fn a_spacer_never_grows_and_stops_the_magnification() {
        for edge in [DockEdge::Bottom, DockEdge::Top] {
            let rest = place(&spaced(), edge, (500.0, 123.0), None);
            // The program before the small spacer magnified: its neighbour
            // on that side is the spacer, which keeps its room.
            let magnified = place(&spaced(), edge, (500.0, 123.0), Some(0));
            assert_eq!(magnified.scales[0], 1.4);
            assert_eq!(magnified.scales[1], 1.0);
            assert!((magnified.items[1].width - rest.items[1].width).abs() < 1e-9);
            assert_eq!(magnified.tiles[1].y, rest.tiles[1].y);
            // The program past it does not grow either.
            assert_eq!(magnified.scales[2], 1.0);
        }
    }

    #[test]
    fn the_pointer_never_lands_on_a_spacer() {
        let shape = spaced();
        let placement = place(&shape, DockEdge::Bottom, (500.0, 123.0), None);
        let y = placement.items[0].y + 10.0;
        let middle = |index: usize| placement.items[index].center_x();

        assert_eq!(hit(&shape, &placement, middle(0), y), Some(0));
        assert_eq!(hit(&shape, &placement, middle(2), y), Some(2));
        assert_eq!(hit(&shape, &placement, middle(1), y), None);
        assert_eq!(hit(&shape, &placement, middle(3), y), None);
        // But its own menu and its drag find it.
        assert_eq!(spacer_hit(&shape, &placement, middle(1), y), Some(1));
        assert_eq!(spacer_hit(&shape, &placement, middle(3), y), Some(3));
        assert_eq!(spacer_hit(&shape, &placement, middle(2), y), None);
        assert_eq!(column_at(&shape, &placement, middle(3), y), Some(3));
        // Off the bar, nothing.
        assert_eq!(column_at(&shape, &placement, middle(1), 0.0), None);
        // On the top edge too.
        let top = place(&shape, DockEdge::Top, (500.0, 123.0), None);
        let y = top.items[0].y + 10.0;
        assert_eq!(hit(&shape, &top, top.items[1].center_x(), y), None);
        assert_eq!(
            spacer_hit(&shape, &top, top.items[1].center_x(), y),
            Some(1)
        );
    }

    #[test]
    fn the_badge_sits_on_the_top_right_corner_and_reaches_beyond_it() {
        let tile = area(100.0, 20.0, 48.0, 48.0);

        let one = badge_area(tile, 7.0);

        // As tall as 38 % of the tile, never narrower than tall.
        close(one, area(148.0 + 3.0 - 18.24, 17.0, 18.24, 18.24));
        // A long number widens it to the left, keeping the corner.
        let many = badge_area(tile, 20.0);
        assert!(many.width > one.width);
        assert!((many.x + many.width - (one.x + one.width)).abs() < 1e-9);
        assert_eq!(many.y, one.y);
    }

    #[test]
    fn the_badge_keeps_a_size_that_reads_on_every_tile() {
        assert_eq!(badge_area(area(0.0, 0.0, 20.0, 20.0), 5.0).height, 14.0);
        assert_eq!(badge_area(area(0.0, 0.0, 120.0, 120.0), 5.0).height, 22.0);
        // With no number, a smaller round dot.
        let dot = badge_area(area(0.0, 0.0, 48.0, 48.0), 0.0);
        assert_eq!(dot.width, dot.height);
        assert!(dot.height < 18.24);
    }

    #[test]
    fn icon_sizes_draw_the_nearest_preset() {
        assert_eq!(DockSize::of(32), DockSize::Small);
        assert_eq!(DockSize::of(40), DockSize::Small);
        assert_eq!(DockSize::of(44), DockSize::Small);
        assert_eq!(DockSize::of(45), DockSize::Medium);
        assert_eq!(DockSize::of(56), DockSize::Medium);
        assert_eq!(DockSize::of(57), DockSize::Large);
        assert_eq!(DockSize::of(96), DockSize::Large);
    }

    #[test]
    fn sizes_match_the_presets_of_the_design() {
        let geometry = |size: DockSize| {
            let SizeGeometry { icon, pad, gap } = size.geometry();
            (icon, pad, gap)
        };
        assert_eq!(geometry(DockSize::Small), (40.0, 6.0, 6.0));
        assert_eq!(geometry(DockSize::Medium), (48.0, 7.0, 7.0));
        assert_eq!(geometry(DockSize::Large), (64.0, 9.0, 9.0));
    }

    #[test]
    fn window_fits_each_size_magnified_with_its_labels() {
        for (size, expected) in [
            (DockSize::Small, (136.0, 110.0)),
            (DockSize::Medium, (160.0, 123.0)),
            (DockSize::Large, (207.0, 147.0)),
        ] {
            assert_eq!(
                window_size(&shape(size), DockEdge::Bottom, true, &LABELS),
                expected
            );
        }
    }

    #[test]
    fn window_fits_each_size_at_rest_with_reduced_motion() {
        for (size, expected) in [
            (DockSize::Small, (126.0, 88.0)),
            (DockSize::Medium, (133.0, 97.0)),
            (DockSize::Large, (171.0, 115.0)),
        ] {
            assert_eq!(
                window_size(&shape(size), DockEdge::Bottom, false, &LABELS),
                expected
            );
        }
    }

    #[test]
    fn window_fits_the_widest_bar_when_the_labels_are_narrow() {
        assert_eq!(
            window_size(
                &shape(DockSize::Medium),
                DockEdge::Bottom,
                true,
                &[0.0, 0.0]
            ),
            (160.0, 123.0)
        );
    }

    #[test]
    fn window_grows_with_a_label_wider_than_the_bar() {
        let narrow = window_size(&shape(DockSize::Medium), DockEdge::Bottom, true, &LABELS);
        let wide = window_size(
            &shape(DockSize::Medium),
            DockEdge::Bottom,
            true,
            &[200.0, 60.0],
        );

        // As wide as the label and ROOM on each side, not more: the pill
        // moves in to stay inside.
        assert_eq!(wide.0, 200.0 + 2.0 * ROOM);
        assert!(wide.0 > narrow.0);
        assert_eq!(wide.1, narrow.1);
    }

    #[test]
    fn the_bar_at_rest_sits_on_the_bottom_edge_centered() {
        let placement = place(
            &shape(DockSize::Medium),
            DockEdge::Bottom,
            (189.0, 123.0),
            None,
        );

        close(placement.bar, area(36.0, 61.0, 117.0, 62.0));
        close(placement.items[0], area(43.0, 68.0, 48.0, 48.0));
        close(placement.items[1], area(98.0, 68.0, 48.0, 48.0));
        close(placement.tiles[1], area(98.0, 68.0, 48.0, 48.0));
        assert_eq!(placement.scales, vec![1.0, 1.0]);
    }

    #[test]
    fn magnifying_pushes_the_neighbour_apart_and_lifts_the_tile() {
        let placement = place(
            &shape(DockSize::Medium),
            DockEdge::Bottom,
            (189.0, 123.0),
            Some(0),
        );

        close(placement.bar, area(22.8, 61.0, 143.4, 62.0));
        close(placement.items[0], area(29.8, 68.0, 67.2, 48.0));
        close(placement.items[1], area(104.0, 68.0, 55.2, 48.0));
        close(placement.tiles[0], area(29.8, 42.8, 67.2, 67.2));
        close(placement.tiles[1], area(104.0, 60.8, 55.2, 55.2));
        assert_eq!(placement.scales, vec![1.4, 1.15]);
    }

    #[test]
    fn everything_is_mirrored_on_the_top_edge() {
        let placement = place(
            &shape(DockSize::Medium),
            DockEdge::Top,
            (189.0, 123.0),
            Some(0),
        );

        close(placement.bar, area(22.8, 0.0, 143.4, 62.0));
        close(placement.tiles[0], area(29.8, 13.0, 67.2, 67.2));
        close(placement.tiles[1], area(104.0, 7.0, 55.2, 55.2));
    }

    #[test]
    fn a_separator_stands_between_groups() {
        let shape = Shape {
            groups: vec![2, 1],
            ..shape(DockSize::Medium)
        };
        let placement = place(&shape, DockEdge::Bottom, (400.0, 123.0), None);

        let third = 7.0 + 2.0 * (48.0 + 7.0) + separator_width(7.0) + 7.0;
        assert!((separator_width(7.0) - 6.6).abs() < 1e-9);
        assert!((placement.bar.width - (third + 48.0 + 7.0)).abs() < 1e-9);
        assert!((placement.items[2].x - (placement.bar.x + third)).abs() < 1e-9);
        assert_eq!(placement.separators.len(), 1);
        // The line is in the middle of its room, between the two gaps.
        let line = placement.separators[0];
        let second_end = placement.items[1].x + placement.items[1].width;
        assert!((line - (second_end + 7.0 + 2.8)).abs() < 1e-9);
    }

    #[test]
    fn empty_groups_add_no_separator() {
        let shape = Shape {
            groups: vec![0, 1, 0, 2],
            ..shape(DockSize::Medium)
        };
        let placement = place(&shape, DockEdge::Bottom, (400.0, 123.0), None);

        assert_eq!(placement.items.len(), 3);
        assert_eq!(placement.separators.len(), 1);
    }

    #[test]
    fn the_clock_adds_a_separator_and_its_width_and_moves_no_item() {
        let without = place(
            &shape(DockSize::Medium),
            DockEdge::Bottom,
            (400.0, 123.0),
            Some(0),
        );
        let with = place(
            &Shape {
                status: true,
                ..shape(DockSize::Medium)
            },
            DockEdge::Bottom,
            (400.0, 123.0),
            Some(0),
        );

        let added = with.bar.width - without.bar.width;
        assert!((added - (2.0 * 7.0 + separator_width(7.0) + STATUS_WIDTH)).abs() < 1e-9);
        for (a, b) in without.items.iter().zip(&with.items) {
            assert!(((a.x - without.bar.x) - (b.x - with.bar.x)).abs() < 1e-9);
        }
        let status = with.status.expect("the clock is placed");
        assert!((status.x + STATUS_WIDTH + 7.0 - (with.bar.x + with.bar.width)).abs() < 1e-9);
    }

    #[test]
    fn the_tray_strip_comes_before_the_clock_two_icons_a_column() {
        let shape = Shape {
            status: true,
            tray: 5,
            ..shape(DockSize::Medium)
        };
        let placement = place(&shape, DockEdge::Bottom, (600.0, 123.0), None);
        let tray = placement.tray.expect("the strip is placed");
        let status = placement.status.expect("the clock is placed");

        // Three columns of 24 px for five icons on the medium size.
        assert_eq!(tray_cell(48.0), 24.0);
        assert!((tray.width - 72.0).abs() < 1e-9);
        assert!((status.x - (tray.x + tray.width + 7.0)).abs() < 1e-9);
        let cells = tray_cells(tray, 5, 48.0);
        assert_eq!(cells.len(), 5);
        assert!((cells[1].x - cells[0].x).abs() < 1e-9);
        assert!((cells[1].y - cells[0].y - 24.0).abs() < 1e-9);
        assert!((cells[4].x - cells[0].x - 48.0).abs() < 1e-9);
        // The two rows fill the item's height, centered.
        assert!((cells[0].y - tray.y).abs() < 1e-9);
        assert_eq!(tray_cell(96.0), 28.0);
    }

    #[test]
    fn the_clock_alone_is_a_bar_of_its_own() {
        let shape = Shape {
            groups: vec![],
            status: true,
            ..shape(DockSize::Medium)
        };
        let placement = place(&shape, DockEdge::Bottom, (300.0, 100.0), None);

        assert!((placement.bar.width - (14.0 + STATUS_WIDTH)).abs() < 1e-9);
    }

    #[test]
    fn the_label_hangs_beyond_the_tile() {
        let tile = area(29.8, 42.8, 67.2, 67.2);
        let (x, y) = tip_anchor(tile, DockEdge::Bottom);
        assert!((x - 63.4).abs() < 1e-9 && (y - 32.8).abs() < 1e-9);
        let (_, y) = tip_anchor(area(29.8, 13.0, 67.2, 67.2), DockEdge::Top);
        assert!((y - 90.2).abs() < 1e-9);
    }

    #[test]
    fn the_window_fits_everything_the_dock_paints() {
        for size in [DockSize::Small, DockSize::Medium, DockSize::Large] {
            for edge in [DockEdge::Bottom, DockEdge::Top] {
                for motion in [true, false] {
                    let shape = shape(size);
                    let window = window_size(&shape, DockEdge::Bottom, motion, &LABELS);
                    let states: &[Option<usize>] = if motion {
                        &[None, Some(0), Some(1)]
                    } else {
                        &[None]
                    };
                    let within = |inner: Area| {
                        inner.x >= -1e-9
                            && inner.y >= -1e-9
                            && inner.x + inner.width <= window.0 + 1e-9
                            && inner.y + inner.height <= window.1 + 1e-9
                    };
                    for &magnified in states {
                        let placement = place(&shape, edge, window, magnified);
                        assert!(within(placement.bar));
                        for (index, tile) in placement.tiles.iter().enumerate() {
                            assert!(within(*tile));
                            if magnified.is_some_and(|m| m != index) {
                                continue;
                            }
                            let (x, y) = tip_anchor(*tile, edge);
                            let width = LABELS[index];
                            let x = tip_center(x, width, window.0);
                            let pill_y = match edge {
                                DockEdge::Bottom => y - TIP_HEIGHT - ROOM,
                                DockEdge::Top => y,
                            };
                            assert!(
                                within(area(
                                    x - width / 2.0 - ROOM,
                                    pill_y,
                                    width + 2.0 * ROOM,
                                    TIP_HEIGHT + ROOM,
                                )),
                                "{size:?} {edge:?} {motion} item {index} magnified {magnified:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Whether the pill `width` wide of what sits in `named`, in a window
    /// of `window`, is whole inside it, with `ROOM` around.
    fn pill_fits(named: Area, width: f64, edge: DockEdge, window: (f64, f64)) -> bool {
        let (x, y) = tip_anchor(named, edge);
        let x = tip_center(x, width, window.0);
        let top = match edge {
            DockEdge::Bottom => y - TIP_HEIGHT,
            DockEdge::Top => y,
        };
        x - width / 2.0 >= ROOM - 1e-9
            && x + width / 2.0 <= window.0 - ROOM + 1e-9
            && top >= -1e-9
            && top + TIP_HEIGHT <= window.1 + 1e-9
    }

    #[test]
    fn the_label_of_a_tray_icon_fits_a_narrow_dock() {
        // One program, the tray with its arrow, the FortiClient and the
        // network, volume and battery, the clock: a long tray tip
        // ("FortiClient 7.4.3 hotfix 1.8758") wider than the bar.
        let shape = Shape {
            groups: vec![1],
            status: true,
            tray: 5,
            ..shape(DockSize::Small)
        };
        let forticlient = 230.0;
        let labels = [60.0, 90.0, forticlient, 50.0, 70.0, 80.0];
        for edge in [DockEdge::Bottom, DockEdge::Top] {
            for motion in [true, false] {
                let window = window_size(&shape, DockEdge::Bottom, motion, &labels);
                let placement = place(&shape, edge, window, None);
                let icon = shape.size.geometry().icon;
                let cells = tray_cells(placement.tray.unwrap(), shape.tray, icon);
                for (index, cell) in cells.iter().enumerate() {
                    assert!(
                        pill_fits(*cell, labels[1 + index], edge, window),
                        "{edge:?} {motion} cell {index} in {window:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_labels_of_a_new_workspace_with_few_items_fit() {
        // A workspace just made, with two programs: the dots start the bar,
        // the tray ends it, and every name is wider than the bar.
        let shape = Shape {
            groups: vec![2],
            status: true,
            tray: 2,
            switcher: 2,
            ..shape(DockSize::Small)
        };
        let items = [180.0, 210.0];
        let strip = [150.0, 240.0];
        let dots = [120.0, 200.0];
        let labels: Vec<f64> = items.iter().chain(&strip).chain(&dots).copied().collect();
        for edge in [DockEdge::Bottom, DockEdge::Top] {
            for motion in [true, false] {
                let window = window_size(&shape, DockEdge::Bottom, motion, &labels);
                let states: &[Option<usize>] = if motion {
                    &[None, Some(0), Some(1)]
                } else {
                    &[None]
                };
                for &magnified in states {
                    let placement = place(&shape, edge, window, magnified);
                    for (index, tile) in placement.tiles.iter().enumerate() {
                        if magnified.is_some_and(|m| m != index) {
                            continue;
                        }
                        assert!(
                            pill_fits(*tile, items[index], edge, window),
                            "{edge:?} {motion} item {index} magnified {magnified:?}"
                        );
                    }
                    let icon = shape.size.geometry().icon;
                    let cells = tray_cells(placement.tray.unwrap(), shape.tray, icon);
                    for (index, cell) in cells.iter().enumerate() {
                        assert!(pill_fits(*cell, strip[index], edge, window));
                    }
                    let side = 2.0 * ACTIVE_DOT_RADIUS;
                    let centers = switcher_dots(placement.switcher.unwrap(), shape.switcher);
                    for (index, &(x, y)) in centers.iter().enumerate() {
                        let dot = area(x - side / 2.0, y - side / 2.0, side, side);
                        assert!(pill_fits(dot, dots[index], edge, window));
                    }
                }
            }
        }
    }

    #[test]
    fn a_label_moves_in_only_near_the_sides() {
        // Room for it over what it names: it stays there.
        assert_eq!(tip_center(100.0, 60.0, 200.0), 100.0);
        // Near a side, in just enough to keep ROOM.
        assert_eq!(tip_center(10.0, 60.0, 200.0), ROOM + 30.0);
        assert_eq!(tip_center(195.0, 60.0, 200.0), 200.0 - ROOM - 30.0);
        // Wider than the window: in its middle.
        assert_eq!(tip_center(10.0, 300.0, 200.0), 100.0);
    }

    #[test]
    fn hover_effects_grow_the_items_they_say() {
        let scale = Hover::of(HoverEffect::Scale, 120);
        let magnify = Hover::of(HoverEffect::Magnify, 120);
        let none = Hover::of(HoverEffect::None, 120);

        assert_eq!(scale.of_item(1, Some(1)).scale, 1.2);
        assert_eq!(scale.of_item(0, Some(1)), REST);
        assert_eq!(magnify.of_item(1, Some(1)), MAGNIFY_HOVERED);
        assert_eq!(magnify.of_item(0, Some(1)), MAGNIFY_NEIGHBOR);
        assert_eq!(magnify.of_item(2, Some(1)), MAGNIFY_NEIGHBOR);
        assert_eq!(magnify.of_item(3, Some(1)), REST);
        assert_eq!(none.of_item(1, Some(1)), REST);
        assert_eq!(magnify.of_item(1, None), REST);
        assert_eq!(none.peak(), REST);
    }

    #[test]
    fn a_drag_lands_after_the_programs_whose_centre_it_passed() {
        let centers = [10.0, 50.0, 90.0, 130.0];

        assert_eq!(drop_index(&centers, 0, 5.0), 0);
        assert_eq!(drop_index(&centers, 0, 60.0), 1);
        assert_eq!(drop_index(&centers, 0, 200.0), 3);
        assert_eq!(drop_index(&centers, 3, 0.0), 0);
        assert_eq!(drop_index(&centers, 1, 60.0), 1);
    }

    #[test]
    fn the_others_make_room_where_the_dragged_item_would_land() {
        // Dragged right, from the first to the third place: the second and
        // the third step left.
        assert_eq!(make_room(4, 0, 2, 50.0), [0.0, -50.0, -50.0, 0.0]);
        // Dragged left, from the last to the second place.
        assert_eq!(make_room(4, 3, 1, 50.0), [0.0, 50.0, 50.0, 0.0]);
        // Still over its own place: nobody moves.
        assert_eq!(make_room(4, 2, 2, 50.0), [0.0; 4]);
    }

    #[test]
    fn the_room_is_the_dragged_item_and_the_gap_after_it() {
        let area = |x: f64, width: f64| Area {
            x,
            y: 0.0,
            width,
            height: 48.0,
        };
        // 48 px items, 8 px apart; a wide widget in the middle.
        let items = [area(0.0, 48.0), area(56.0, 112.0), area(176.0, 48.0)];

        assert_eq!(drag_pitch(&items, 0), 56.0);
        assert_eq!(drag_pitch(&items, 1), 120.0);
        // The last: the gap before it.
        assert_eq!(drag_pitch(&items, 2), 56.0);
        assert_eq!(drag_pitch(&items[..1], 0), 48.0);
        assert_eq!(drag_pitch(&[], 0), 0.0);
    }

    #[test]
    fn a_slide_eases_in_and_arrives() {
        let mut now = 0.0;
        let mut frames = 0;
        while now != -56.0 {
            let next = slide_step(now, -56.0);
            // Always toward the target, never past it.
            assert!(next < now && next >= -56.0);
            now = next;
            frames += 1;
        }
        // About a quarter of a second at 60 frames a second.
        assert!((8..=20).contains(&frames), "{frames} frames");
        assert_eq!(slide_step(-56.0, -56.0), -56.0);
    }

    #[test]
    fn the_dragged_tile_stays_inside_the_bar() {
        // Rest at 50, 40 wide, the programs from 10 to 200.
        assert_eq!(drag_offset(100.0, 70.0, 50.0, 40.0, 10.0, 200.0), 30.0);
        assert_eq!(drag_offset(0.0, 70.0, 50.0, 40.0, 10.0, 200.0), -40.0);
        assert_eq!(drag_offset(500.0, 70.0, 50.0, 40.0, 10.0, 200.0), 110.0);
    }

    #[test]
    fn the_media_buttons_sit_side_by_side_beside_the_cover() {
        let tile = Area {
            x: 100.0,
            y: 10.0,
            width: 180.0,
            height: 56.0,
        };
        let [previous, toggle, next] = media_buttons(tile);
        // Right of the cover, on the lower line.
        assert!(previous.x > tile.x + tile.height);
        assert!(previous.y > tile.y + tile.height / 2.0 - previous.height);
        assert!(previous.y + previous.height < tile.y + tile.height);
        assert!(previous.x + previous.width < toggle.x);
        assert!(toggle.x + toggle.width < next.x);
        assert!(next.x + next.width < tile.x + tile.width);
    }

    #[test]
    fn a_long_text_waits_rolls_to_its_end_and_back() {
        // 44 px too long: 2 s to roll at 22 px/s.
        assert_eq!(marquee(1.0, 44.0), (0.0, 2.0));
        let (offset, next) = marquee(4.0, 44.0);
        assert!((offset - 22.0).abs() < 1e-9);
        assert!(next < 0.1);
        assert_eq!(marquee(5.5, 44.0), (44.0, 1.5));
        let (offset, _) = marquee(8.0, 44.0);
        assert!((offset - 22.0).abs() < 1e-9);
        // Then over again.
        assert_eq!(marquee(9.0 + 1.0, 44.0), (0.0, 2.0));
        // A text that fits never moves.
        assert_eq!(marquee(4.0, 0.0), (0.0, f64::INFINITY));
    }

    /// Configurações and Sobre plus a two-icon widget, a spacer, the
    /// workspace dots, the tray and the clock: every kind of thing on the bar.
    fn full_shape() -> Shape {
        Shape {
            groups: vec![2, 2],
            status: true,
            tray: 3,
            switcher: 2,
            spans: vec![1, 1, 2, 1],
            spacers: vec![None, None, None, Some(SpacerSize::Small)],
            ..shape(DockSize::Medium)
        }
    }

    const SIDES: [Side; 4] = [Side::Bottom, Side::Top, Side::Left, Side::Right];

    #[test]
    fn the_bottom_and_the_top_are_laid_out_as_before() {
        let shape = full_shape();
        for edge in [DockEdge::Bottom, DockEdge::Top] {
            let placed = place(&shape, edge, (900.0, 120.0), Some(1));
            let lying = lying(&shape, edge, (900.0, 120.0), &|index| {
                shape.hover.of_item(index, Some(1))
            });
            assert_eq!(placed, lying, "{edge:?}");
            assert_eq!(placed.side, Side::from(edge));
        }
    }

    #[test]
    fn a_side_is_the_bottom_or_the_top_turned_on_its_side() {
        let shape = full_shape();
        let (wide, tall) = window_size(&shape, DockEdge::Bottom, true, &LABELS);
        assert_eq!(window_size(&shape, Side::Left, true, &LABELS), (tall, wide));
        assert_eq!(
            window_size(&shape, Side::Right, true, &LABELS),
            (tall, wide)
        );
        let viewport = (tall, wide);
        let left = place(&shape, Side::Left, viewport, Some(1));
        let top = place(&shape, DockEdge::Top, (wide, tall), Some(1));
        close(left.bar, top.bar.transposed());
        for (standing, lying) in left.tiles.iter().zip(&top.tiles) {
            close(*standing, lying.transposed());
        }
        assert_eq!(left.separators, top.separators);
    }

    #[test]
    fn the_bar_touches_its_screen_edge_and_is_centred_along_it() {
        let shape = full_shape();
        for side in SIDES {
            let viewport = match side {
                Side::Bottom | Side::Top => (900.0, 120.0),
                Side::Left | Side::Right => (120.0, 900.0),
            };
            let placed = place(&shape, side, viewport, None);
            let bar = placed.bar;
            let touches = match side {
                Side::Bottom => (bar.y + bar.height - viewport.1).abs() < 1e-6,
                Side::Top => bar.y.abs() < 1e-6,
                Side::Left => bar.x.abs() < 1e-6,
                Side::Right => (bar.x + bar.width - viewport.0).abs() < 1e-6,
            };
            assert!(touches, "{side:?} {bar:?}");
            let centred = if side.standing() {
                (bar.center_y() - viewport.1 / 2.0).abs() < 1e-6
            } else {
                (bar.center_x() - viewport.0 / 2.0).abs() < 1e-6
            };
            assert!(centred, "{side:?} {bar:?}");
            for item in &placed.items {
                assert!(
                    bar.contains(item.center_x(), item.center_y()),
                    "{side:?} {item:?}"
                );
            }
        }
    }

    #[test]
    fn the_left_reads_down_and_the_right_reads_up() {
        let shape = full_shape();
        let viewport = (120.0, 900.0);
        let left = place(&shape, Side::Left, viewport, None);
        let right = place(&shape, Side::Right, viewport, None);
        let ys = |placed: &Placement| placed.items.iter().map(|item| item.y).collect::<Vec<_>>();
        assert!(ys(&left).windows(2).all(|pair| pair[0] < pair[1]));
        assert!(ys(&right).windows(2).all(|pair| pair[0] > pair[1]));
        // The workspace dots start the bar, the clock ends it: on the left
        // the dots are on top; on the right, the clock is.
        let (dots, clock) = (left.switcher.unwrap(), left.status.unwrap());
        assert!(dots.y < clock.y);
        let (dots, clock) = (right.switcher.unwrap(), right.status.unwrap());
        assert!(dots.y > clock.y);
        // Each separator's line lies between the items it parts.
        for placed in [&left, &right] {
            for line in &placed.separators {
                assert!(*line > placed.bar.y && *line < placed.bar.y + placed.bar.height);
            }
        }
    }

    #[test]
    fn a_wide_widget_stands_tall_on_a_side() {
        let shape = full_shape();
        let left = place(&shape, Side::Left, (120.0, 900.0), None);
        let square = left.items[0];
        let wide = left.items[2];
        assert!((wide.width - square.width).abs() < 1e-6);
        assert!(wide.height > 1.9 * square.height);
    }

    #[test]
    fn a_grown_tile_moves_away_from_its_screen_edge() {
        let shape = full_shape();
        for side in SIDES {
            let viewport = if side.standing() {
                (120.0, 900.0)
            } else {
                (900.0, 120.0)
            };
            let rest = place(&shape, side, viewport, None).tiles[1];
            let grown = place(&shape, side, viewport, Some(1)).tiles[1];
            let away = match side {
                Side::Bottom => grown.y < rest.y,
                Side::Top => grown.y + grown.height > rest.y + rest.height,
                Side::Left => grown.x + grown.width > rest.x + rest.width,
                Side::Right => grown.x < rest.x,
            };
            assert!(away, "{side:?}: {rest:?} -> {grown:?}");
        }
    }

    #[test]
    fn the_pointer_finds_the_same_item_on_every_edge() {
        let shape = full_shape();
        for side in SIDES {
            let viewport = if side.standing() {
                (120.0, 900.0)
            } else {
                (900.0, 120.0)
            };
            let placed = place(&shape, side, viewport, None);
            for (index, item) in placed.items.iter().enumerate() {
                let at = (item.center_x(), item.center_y());
                assert_eq!(
                    column_at(&shape, &placed, at.0, at.1),
                    Some(index),
                    "{side:?}"
                );
                let expected = shape.spacer(index).is_none().then_some(index);
                assert_eq!(hit(&shape, &placed, at.0, at.1), expected, "{side:?}");
            }
            assert_eq!(column_at(&shape, &placed, -5.0, -5.0), None, "{side:?}");
        }
    }

    #[test]
    fn a_label_sits_beyond_the_tile_away_from_the_screen_edge() {
        let tile = area(100.0, 200.0, 48.0, 48.0);
        assert_eq!(tip_anchor(tile, Side::Bottom), (124.0, 200.0 - TIP_GAP));
        assert_eq!(tip_anchor(tile, Side::Top), (124.0, 248.0 + TIP_GAP));
        assert_eq!(tip_anchor(tile, Side::Left), (148.0 + TIP_GAP, 224.0));
        assert_eq!(tip_anchor(tile, Side::Right), (100.0 - TIP_GAP, 224.0));
    }
}
