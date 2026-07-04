use crate::*;

#[derive(Clone, Copy, PartialEq)]
pub enum PathMode {
    Fill,
    Stroke(f32), // Contains the screen-space stroke width
}

#[inline(always)]
fn push_segment(p0: Point, p1: Point, rasterizer: &mut Rasterizer, mode: PathMode) {
    match mode {
        PathMode::Fill => {
            rasterizer.push_edge(p0, p1);
        }
        PathMode::Stroke(width) => {
            let dx = p1.x - p0.x;
            let dy = p1.y - p0.y;
            let len = (dx * dx + dy * dy).sqrt();

            if len <= f32::EPSILON {
                return;
            }

            // Calculate the perpendicular offset vector
            let half_w = width / 2.0;
            let ox = (-dy / len) * half_w;
            let oy = (dx / len) * half_w;

            // Generate the 4 corners of the thick line segment
            let a = Point {
                x: p0.x + ox,
                y: p0.y + oy,
            };
            let b = Point {
                x: p1.x + ox,
                y: p1.y + oy,
            };
            let c = Point {
                x: p1.x - ox,
                y: p1.y - oy,
            };
            let d = Point {
                x: p0.x - ox,
                y: p0.y - oy,
            };

            // Push the 4 edges of the rectangle into the rasterizer
            // We draw them in order to create a closed polygon
            rasterizer.push_edge(a, b);
            rasterizer.push_edge(b, c);
            rasterizer.push_edge(c, d);
            rasterizer.push_edge(d, a);
        }
    }
}

#[inline(always)]
fn push_quadratic(p0: Point, p1: Point, p2: Point, rasterizer: &mut Rasterizer, mode: PathMode) {
    const STEPS: usize = 20;
    let mut prev_point = p0;

    for step in 1..=STEPS {
        let t = step as f32 / STEPS as f32;
        let mt = 1.0 - t;
        let x = mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x;
        let y = mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y;
        let point = Point { x, y };

        push_segment(prev_point, point, rasterizer, mode);
        prev_point = point;
    }
}

pub fn process_path(
    d_bytes: &[u8],
    active_matrix: &[f32; 6],
    rasterizer: &mut Rasterizer,
    mode: PathMode,
) {
    let mut parser = PathParser::new(d_bytes);

    let mut current_pos = Point { x: 0.0, y: 0.0 };
    let mut subpath_start = Point { x: 0.0, y: 0.0 };

    // Track this for the 'S' and 's' reflection math
    let mut last_control_point = Point { x: 0.0, y: 0.0 };
    let mut active_cmd = b'M';
    let mut last_curve_cmd = b'M';

    while let Some(token) = parser.next_token() {
        match token {
            PathToken::Command(c) => {
                active_cmd = c;

                if active_cmd == b'Z' || active_cmd == b'z' {
                    let t_start = current_pos.transform(active_matrix);
                    let t_end = subpath_start.transform(active_matrix);
                    push_segment(t_start, t_end, rasterizer, mode);

                    current_pos = subpath_start;
                    last_control_point = current_pos; // Reset control point
                    last_curve_cmd = active_cmd.to_ascii_uppercase();
                }
            }
            PathToken::Number(first_num) => {
                let is_relative = active_cmd.is_ascii_lowercase();

                match active_cmd.to_ascii_uppercase() {
                    b'M' | b'L' => {
                        let n1 = first_num;
                        let n2 = parser.next_number().unwrap_or(0.0);

                        let mut target = Point { x: n1, y: n2 };
                        if is_relative {
                            target.x += current_pos.x;
                            target.y += current_pos.y;
                        }

                        if active_cmd.to_ascii_uppercase() == b'M' {
                            subpath_start = target;
                            active_cmd = if is_relative { b'l' } else { b'L' };
                        } else {
                            let t_start = current_pos.transform(active_matrix);
                            let t_end = target.transform(active_matrix);
                            push_segment(t_start, t_end, rasterizer, mode);
                        }

                        current_pos = target;
                        last_control_point = current_pos; // Reset control point
                        last_curve_cmd = active_cmd.to_ascii_uppercase();
                    }
                    b'H' => {
                        let mut target = current_pos;
                        if is_relative {
                            target.x += first_num;
                        } else {
                            target.x = first_num;
                        }

                        let t_start = current_pos.transform(active_matrix);
                        let t_end = target.transform(active_matrix);
                        push_segment(t_start, t_end, rasterizer, mode);

                        current_pos = target;
                        last_control_point = current_pos; // Reset control point
                        last_curve_cmd = active_cmd.to_ascii_uppercase();
                    }
                    b'V' => {
                        let mut target = current_pos;
                        if is_relative {
                            target.y += first_num;
                        } else {
                            target.y = first_num;
                        }

                        let t_start = current_pos.transform(active_matrix);
                        let t_end = target.transform(active_matrix);
                        push_segment(t_start, t_end, rasterizer, mode);

                        current_pos = target;
                        last_control_point = current_pos; // Reset control point
                        last_curve_cmd = active_cmd.to_ascii_uppercase();
                    }
                    b'C' => {
                        let c1y = parser.next_number().unwrap_or(0.0);
                        let c2x = parser.next_number().unwrap_or(0.0);
                        let c2y = parser.next_number().unwrap_or(0.0);
                        let ex = parser.next_number().unwrap_or(0.0);
                        let ey = parser.next_number().unwrap_or(0.0);

                        let mut c1 = Point {
                            x: first_num,
                            y: c1y,
                        };
                        let mut c2 = Point { x: c2x, y: c2y };
                        let mut end = Point { x: ex, y: ey };

                        if is_relative {
                            c1.x += current_pos.x;
                            c1.y += current_pos.y;
                            c2.x += current_pos.x;
                            c2.y += current_pos.y;
                            end.x += current_pos.x;
                            end.y += current_pos.y;
                        }

                        let t_p0 = current_pos.transform(active_matrix);
                        let t_p1 = c1.transform(active_matrix);
                        let t_p2 = c2.transform(active_matrix);
                        let t_p3 = end.transform(active_matrix);

                        let flattener = CubicFlattenIter::new(t_p0, t_p1, t_p2, t_p3, 20);
                        let mut prev_point = t_p0;
                        for point in flattener {
                            push_segment(prev_point, point, rasterizer, mode);
                            prev_point = point;
                        }

                        current_pos = end;
                        last_control_point = c2; // Save c2 for potential following 'S'
                        last_curve_cmd = active_cmd.to_ascii_uppercase();
                    }
                    b'S' => {
                        let c2y = parser.next_number().unwrap_or(0.0);
                        let ex = parser.next_number().unwrap_or(0.0);
                        let ey = parser.next_number().unwrap_or(0.0);

                        let mut c2 = Point {
                            x: first_num,
                            y: c2y,
                        };
                        let mut end = Point { x: ex, y: ey };

                        if is_relative {
                            c2.x += current_pos.x;
                            c2.y += current_pos.y;
                            end.x += current_pos.x;
                            end.y += current_pos.y;
                        }

                        let c1 = if last_curve_cmd == b'C' || last_curve_cmd == b'S' {
                            Point {
                                x: current_pos.x + (current_pos.x - last_control_point.x),
                                y: current_pos.y + (current_pos.y - last_control_point.y),
                            }
                        } else {
                            current_pos
                        };

                        let t_p0 = current_pos.transform(active_matrix);
                        let t_p1 = c1.transform(active_matrix);
                        let t_p2 = c2.transform(active_matrix);
                        let t_p3 = end.transform(active_matrix);

                        let flattener = CubicFlattenIter::new(t_p0, t_p1, t_p2, t_p3, 20);
                        let mut prev_point = t_p0;
                        for point in flattener {
                            push_segment(prev_point, point, rasterizer, mode);
                            prev_point = point;
                        }

                        current_pos = end;
                        last_control_point = c2; // Save c2 for potential following 'S'
                        last_curve_cmd = active_cmd.to_ascii_uppercase();
                    }
                    b'Q' => {
                        let c1y = parser.next_number().unwrap_or(0.0);
                        let ex = parser.next_number().unwrap_or(0.0);
                        let ey = parser.next_number().unwrap_or(0.0);

                        let mut c1 = Point {
                            x: first_num,
                            y: c1y,
                        };
                        let mut end = Point { x: ex, y: ey };

                        if is_relative {
                            c1.x += current_pos.x;
                            c1.y += current_pos.y;
                            end.x += current_pos.x;
                            end.y += current_pos.y;
                        }

                        let t_p0 = current_pos.transform(active_matrix);
                        let t_p1 = c1.transform(active_matrix);
                        let t_p2 = end.transform(active_matrix);

                        push_quadratic(t_p0, t_p1, t_p2, rasterizer, mode);

                        current_pos = end;
                        last_control_point = c1;
                        last_curve_cmd = active_cmd.to_ascii_uppercase();
                    }
                    b'T' => {
                        let ey = parser.next_number().unwrap_or(0.0);
                        let mut end = Point {
                            x: first_num,
                            y: ey,
                        };

                        if is_relative {
                            end.x += current_pos.x;
                            end.y += current_pos.y;
                        }

                        let c1 = if last_curve_cmd == b'Q' || last_curve_cmd == b'T' {
                            Point {
                                x: current_pos.x + (current_pos.x - last_control_point.x),
                                y: current_pos.y + (current_pos.y - last_control_point.y),
                            }
                        } else {
                            current_pos
                        };

                        let t_p0 = current_pos.transform(active_matrix);
                        let t_p1 = c1.transform(active_matrix);
                        let t_p2 = end.transform(active_matrix);

                        push_quadratic(t_p0, t_p1, t_p2, rasterizer, mode);

                        current_pos = end;
                        last_control_point = c1;
                        last_curve_cmd = active_cmd.to_ascii_uppercase();
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Edge {
    pub y_min: i32,
    pub y_max: i32,
    pub x: f32,         // The current X position
    pub dx_per_dy: f32, // How much X changes for every 1 step in Y
    pub dir: i32,       // +1 for downward edge, -1 for upward edge (Winding rule)
}

impl Edge {
    /// Converts a line segment (Point A to Point B) into a scanline Edge
    pub fn new(p0: Point, p1: Point) -> Option<Self> {
        let y0 = p0.y.round() as i32;
        let y1 = p1.y.round() as i32;

        // Ignore perfectly horizontal lines (they don't cross scanlines)
        if y0 == y1 {
            return None;
        }

        let (dir, y_min, y_max, start_x, end_x) = if y0 < y1 {
            (1, y0, y1, p0.x, p1.x)
        } else {
            (-1, y1, y0, p1.x, p0.x)
        };

        let dx_per_dy = (end_x - start_x) / (y_max - y_min) as f32;

        Some(Self {
            y_min,
            y_max,
            x: start_x,
            dx_per_dy,
            dir,
        })
    }
}

const MAX_EDGES: usize = 8192; // Max segments per shape
const MAX_AET: usize = 256; // Max overlapping edges on a single horizontal line

pub struct Rasterizer<'a> {
    pub width: usize,
    pub height: usize,
    pub pixels: &'a mut [u32], // The screen! (Pre-allocated RGBA buffer)

    edges: [Edge; MAX_EDGES],
    edge_count: usize,

    // The Active Edge Table just holds indices pointing to the `edges` array
    aet: [usize; MAX_AET],
    aet_count: usize,
}

impl<'a> Rasterizer<'a> {
    pub fn new(pixels: &'a mut [u32], width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels,
            edges: [Edge::default(); MAX_EDGES],
            edge_count: 0,
            aet: [0; MAX_AET],
            aet_count: 0,
        }
    }

    /// Takes a transformed line segment, converts it to a scanline edge,
    /// and pushes it directly into the pre-allocated buffer.
    pub fn push_edge(&mut self, p0: Point, p1: Point) {
        if self.edge_count >= MAX_EDGES {
            return; // Safety bounds check to prevent panic
        }

        if let Some(edge) = Edge::new(p0, p1) {
            self.edges[self.edge_count] = edge;
            self.edge_count += 1;
        }
    }

    pub fn rasterize(&mut self, color: u32, fill_rule: FillRule) {
        self.aet_count = 0;

        if self.edge_count == 0 {
            return;
        }

        let active_edges = &mut self.edges[0..self.edge_count];
        active_edges.sort_unstable_by_key(|e| e.y_min);

        // Allow negative minimums so the AET doesn't stall on off-screen edges
        let min_y = active_edges[0].y_min;
        let max_y = active_edges
            .iter()
            .map(|e| e.y_max)
            .max()
            .unwrap_or(0)
            .min(self.height as i32); // Safe to clamp the max to save loop cycles

        let mut current_edge_idx = 0;

        for y in min_y..max_y {
            // Add new edges
            while current_edge_idx < self.edge_count && self.edges[current_edge_idx].y_min == y {
                if self.aet_count < MAX_AET {
                    self.aet[self.aet_count] = current_edge_idx;
                    self.aet_count += 1;
                }
                current_edge_idx += 1;
            }

            // Remove ended edges
            let mut i = 0;
            while i < self.aet_count {
                let edge = &self.edges[self.aet[i]];
                if edge.y_max <= y {
                    self.aet.swap(i, self.aet_count - 1);
                    self.aet_count -= 1;
                } else {
                    i += 1;
                }
            }

            self.aet[0..self.aet_count]
                .sort_unstable_by(|&a, &b| self.edges[a].x.partial_cmp(&self.edges[b].x).unwrap());

            // Only fill pixels if the current scanline is actually on the screen
            if y >= 0 && y < self.height as i32 {
                let y_usize = y as usize;
                let mut winding_number: i32 = 0;
                let mut fill_start_x = 0;

                for i in 0..self.aet_count {
                    let edge_idx = self.aet[i];
                    let edge = self.edges[edge_idx];

                    let was_inside = match fill_rule {
                        FillRule::NonZero => winding_number != 0,
                        FillRule::EvenOdd => winding_number % 2 != 0,
                    };

                    winding_number += edge.dir;

                    let is_inside = match fill_rule {
                        FillRule::NonZero => winding_number != 0,
                        FillRule::EvenOdd => winding_number % 2 != 0,
                    };

                    if !was_inside && is_inside {
                        fill_start_x = edge.x as i32;
                    }

                    if was_inside && !is_inside {
                        let fill_end_x = edge.x as i32;
                        self.fill_span(y_usize, fill_start_x, fill_end_x, color);
                    }
                }
            }

            for i in 0..self.aet_count {
                let edge_idx = self.aet[i];
                self.edges[edge_idx].x += self.edges[edge_idx].dx_per_dy;
            }
        }

        self.edge_count = 0;
        self.aet_count = 0;
    }

    /// Writes directly into the pre-allocated pixel slice
    #[inline(always)]
    fn fill_span(&mut self, y: usize, x0: i32, x1: i32, color: u32) {
        // 1. Clamp both coordinates to the screen bounds strictly as i32
        let start = x0.max(0).min(self.width as i32);
        let end = x1.max(0).min(self.width as i32);

        // 2. Safely evaluate the bounds
        if start >= end {
            return;
        }

        // 3. Now that they are guaranteed >= 0, cast them to usize
        let start_idx = start as usize;
        let end_idx = end as usize;

        let offset = y * self.width;
        let row = &mut self.pixels[offset + start_idx..offset + end_idx];

        for pixel in row.iter_mut() {
            *pixel = color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_stroke_expands_flattened_segments() {
        let mut pixels = [0xFFFFFFFF; 64 * 64];
        let mut rasterizer = Rasterizer::new(&mut pixels, 64, 64);

        process_path(
            b"M 5 5 C 10 40 50 40 55 5",
            &IDENTITY_TRANSFORM,
            &mut rasterizer,
            PathMode::Stroke(4.0),
        );

        assert!(
            rasterizer.edge_count > 40,
            "stroked cubic should emit stroke polygons, not only fill edges"
        );
    }

    #[test]
    fn rasterize_clears_active_edge_table_between_shapes() {
        let mut pixels = [0xFFFFFFFF; 32 * 32];
        let mut rasterizer = Rasterizer::new(&mut pixels, 32, 32);

        process_path(
            b"M -10 -10 L 20 -10 L 20 20 L -10 20 z",
            &IDENTITY_TRANSFORM,
            &mut rasterizer,
            PathMode::Fill,
        );
        rasterizer.rasterize(0xFF0000FF, FillRule::NonZero);
        assert_eq!(rasterizer.aet_count, 0);

        process_path(
            b"M 22 22 L 28 22 L 28 28 L 22 28 z",
            &IDENTITY_TRANSFORM,
            &mut rasterizer,
            PathMode::Fill,
        );
        rasterizer.rasterize(0x0000FFFF, FillRule::NonZero);
        assert_eq!(rasterizer.aet_count, 0);
        assert_eq!(pixels[24 * 32 + 24], 0x0000FFFF);
    }
}
