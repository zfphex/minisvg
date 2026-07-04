#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    /// Applies the 6-element affine matrix [a, b, c, d, e, f] to this point
    pub fn transform(&self, matrix: &[f32; 6]) -> Self {
        Self {
            x: self.x * matrix[0] + self.y * matrix[2] + matrix[4],
            y: self.x * matrix[1] + self.y * matrix[3] + matrix[5],
        }
    }
}

pub struct CubicFlattenIter {
    pub p0: Point,
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
    pub steps: usize,
    pub current_step: usize,
}

impl CubicFlattenIter {
    pub fn new(p0: Point, p1: Point, p2: Point, p3: Point, steps: usize) -> Self {
        Self {
            p0,
            p1,
            p2,
            p3,
            steps,
            current_step: 0,
        }
    }
}

impl Iterator for CubicFlattenIter {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_step > self.steps {
            return None;
        }

        // Calculate t (from 0.0 to 1.0)
        let t = self.current_step as f32 / self.steps as f32;
        self.current_step += 1;

        // The Bernstein polynomial coefficients
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        let t2 = t * t;
        let t3 = t2 * t;

        // B(t) = (1-t)^3*P0 + 3*(1-t)^2*t*P1 + 3*(1-t)*t^2*P2 + t^3*P3
        let x = mt3 * self.p0.x
            + 3.0 * mt2 * t * self.p1.x
            + 3.0 * mt * t2 * self.p2.x
            + t3 * self.p3.x;

        let y = mt3 * self.p0.y
            + 3.0 * mt2 * t * self.p1.y
            + 3.0 * mt * t2 * self.p2.y
            + t3 * self.p3.y;

        Some(Point { x, y })
    }
}