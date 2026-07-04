// Represents the matrix:
// [a  c  e]
// [b  d  f]
// [0  0  1]
pub type Transform = [f32; 6];

pub const IDENTITY_TRANSFORM: Transform = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// Multiplies matrix `child` by `parent`.
/// Result = parent * child
pub fn multiply_transforms(parent: &Transform, child: &Transform) -> Transform {
    [
        parent[0] * child[0] + parent[2] * child[1], // a
        parent[1] * child[0] + parent[3] * child[1], // b
        parent[0] * child[2] + parent[2] * child[3], // c
        parent[1] * child[2] + parent[3] * child[3], // d
        parent[0] * child[4] + parent[2] * child[5] + parent[4], // e
        parent[1] * child[4] + parent[3] * child[5] + parent[5], // f
    ]
}

pub fn make_translate(tx: f32, ty: f32) -> Transform {
    [1.0, 0.0, 0.0, 1.0, tx, ty]
}

pub fn make_scale(sx: f32, sy: f32) -> Transform {
    [sx, 0.0, 0.0, sy, 0.0, 0.0]
}

pub fn make_rotate(degrees: f32) -> Transform {
    let rad = degrees.to_radians();
    let (sin, cos) = rad.sin_cos();
    [cos, sin, -sin, cos, 0.0, 0.0]
}

pub fn make_skew_x(degrees: f32) -> Transform {
    let rad = degrees.to_radians();
    [1.0, 0.0, rad.tan(), 1.0, 0.0, 0.0]
}

pub fn make_skew_y(degrees: f32) -> Transform {
    let rad = degrees.to_radians();
    [1.0, rad.tan(), 0.0, 1.0, 0.0, 0.0]
}

pub struct TransformParser<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> TransformParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn skip_whitespace_and_commas(&mut self) {
        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            if c.is_ascii_whitespace() || c == b',' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Advances the cursor until a specific byte is found and passed
    fn skip_until(&mut self, target: u8) {
        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            self.pos += 1;
            if c == target {
                break;
            }
        }
    }

    /// Extracts the next alphabetical command (e.g., b"translate")
    pub fn next_command(&mut self) -> Option<&'a [u8]> {
        self.skip_whitespace_and_commas();
        let start = self.pos;
        
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_alphabetic() {
            self.pos += 1;
        }
        
        if start == self.pos {
            None
        } else {
            Some(&self.data[start..self.pos])
        }
    }

    /// Exact same number parser as Phase 3 (PathParser)
    pub fn next_number(&mut self) -> Option<f32> {
        self.skip_whitespace_and_commas();
        if self.pos >= self.data.len() { return None; }

        let start = self.pos;
        let mut has_dot = false;

        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            match c {
                b'-' | b'+' => {
                    if self.pos != start {
                        let prev = self.data[self.pos - 1];
                        if prev != b'e' && prev != b'E' { break; }
                    }
                }
                b'.' => {
                    if has_dot { break; }
                    has_dot = true;
                }
                b'0'..=b'9' | b'e' | b'E' => {}
                _ => break, 
            }
            self.pos += 1;
        }

        if start == self.pos { return None; }

        let slice = unsafe { std::str::from_utf8_unchecked(&self.data[start..self.pos]) };
        slice.parse::<f32>().ok()
    }
}

pub fn parse_transform(data: &[u8]) -> Transform {
    let mut parser = TransformParser::new(data);
    let mut result = IDENTITY_TRANSFORM;

    while let Some(command) = parser.next_command() {
        // Skip over the '(' opening bracket
        parser.skip_until(b'(');
        
        let mut local_transform = IDENTITY_TRANSFORM;
        
        match command {
            b"matrix" => {
                let a = parser.next_number().unwrap_or(1.0);
                let b = parser.next_number().unwrap_or(0.0);
                let c = parser.next_number().unwrap_or(0.0);
                let d = parser.next_number().unwrap_or(1.0);
                let e = parser.next_number().unwrap_or(0.0);
                let f = parser.next_number().unwrap_or(0.0);
                local_transform = [a, b, c, d, e, f];
            }
            b"translate" => {
                let tx = parser.next_number().unwrap_or(0.0);
                // If ty is not provided, SVG spec says it defaults to 0.0
                let ty = parser.next_number().unwrap_or(0.0);
                local_transform = make_translate(tx, ty);
            }
            b"scale" => {
                let sx = parser.next_number().unwrap_or(1.0);
                // If sy is not provided, SVG spec says it equals sx
                let sy = parser.next_number().unwrap_or(sx);
                local_transform = make_scale(sx, sy);
            }
            b"rotate" => {
                let angle = parser.next_number().unwrap_or(0.0);
                let cx = parser.next_number();
                let cy = parser.next_number();
                
                // SVG rotate can optionally take a center point of rotation: rotate(angle, cx, cy)
                // This translates to the point, rotates, and translates back.
                if let (Some(x), Some(y)) = (cx, cy) {
                    let t1 = make_translate(x, y);
                    let r = make_rotate(angle);
                    let t2 = make_translate(-x, -y);
                    
                    let temp = multiply_transforms(&t1, &r);
                    local_transform = multiply_transforms(&temp, &t2);
                } else {
                    local_transform = make_rotate(angle);
                }
            }
            b"skewX" => {
                let angle = parser.next_number().unwrap_or(0.0);
                local_transform = make_skew_x(angle);
            }
            b"skewY" => {
                let angle = parser.next_number().unwrap_or(0.0);
                local_transform = make_skew_y(angle);
            }
            _ => {} // Ignore unsupported commands
        }
        
        // Multiply the current aggregate matrix by the newly parsed matrix
        result = multiply_transforms(&result, &local_transform);
        
        // Skip over the ')' closing bracket before looking for the next command
        parser.skip_until(b')');
    }
    
    result
}