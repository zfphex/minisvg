pub struct PathParser<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> PathParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Skips spaces, tabs, newlines, AND commas
    fn skip_separators(&mut self) {
        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            if c.is_ascii_whitespace() || c == b',' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
    /// Extracts the next f32 from the byte stream without allocating
    pub fn next_number(&mut self) -> Option<f32> {
        self.skip_separators();
        if self.pos >= self.data.len() {
            return None;
        }

        let start = self.pos;
        let mut has_dot = false;

        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            match c {
                b'-' | b'+' => {
                    // A sign is only valid as the VERY first character,
                    // OR immediately following an exponent (e or E).
                    if self.pos != start {
                        let prev = self.data[self.pos - 1];
                        if prev != b'e' && prev != b'E' {
                            break; // This sign belongs to the NEXT number!
                        }
                    }
                }
                b'.' => {
                    // If we already saw a decimal, a second one starts a new number
                    if has_dot {
                        break;
                    }
                    has_dot = true;
                }
                b'0'..=b'9' | b'e' | b'E' => {
                    // Valid number characters, keep going
                }
                _ => {
                    // We hit a command letter (like 'M' or 'Z') or something else
                    break;
                }
            }
            self.pos += 1;
        }

        if start == self.pos {
            return None;
        }

        // --- ZERO ALLOCATION MAGIC ---
        // 1. We take a slice of the bytes we just validated.
        // 2. Convert to &str (unchecked is safe here because we only allowed ascii math chars).
        // 3. Hand it to Rust's internal parser.
        let slice = unsafe { std::str::from_utf8_unchecked(&self.data[start..self.pos]) };
        slice.parse::<f32>().ok()
    }
}

#[derive(Debug, PartialEq)]
pub enum PathToken {
    Command(u8), // e.g., b'M', b'L', b'C'
    Number(f32),
}

impl<'a> PathParser<'a> {
    pub fn next_token(&mut self) -> Option<PathToken> {
        self.skip_separators();
        if self.pos >= self.data.len() {
            return None;
        }

        let c = self.data[self.pos];

        // If it is an alphabetical character, it's a command
        if c.is_ascii_alphabetic() {
            self.pos += 1;
            return Some(PathToken::Command(c));
        }

        // Otherwise, it must be a number
        if let Some(num) = self.next_number() {
            return Some(PathToken::Number(num));
        }

        None
    }
}
