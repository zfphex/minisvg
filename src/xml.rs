#[derive(Debug, PartialEq)]
pub enum XmlEvent<'a> {
    StartTag(&'a [u8]),
    EndTag(&'a [u8]),
    Attribute(&'a [u8], &'a [u8]),
    Eof,
}

#[derive(Debug)]
pub struct XmlParser<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub in_tag: bool,
    pub current_tag: &'a [u8], // Track the active tag
}

impl<'a> XmlParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            in_tag: false,
            current_tag: b"",
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn skip_until_byte(&mut self, target: u8) {
        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            self.pos += 1;
            if c == target {
                break;
            }
        }
    }

    fn skip_until_sequence(&mut self, seq: &[u8]) {
        if seq.is_empty() {
            return;
        }

        while self.pos + seq.len() <= self.data.len() {
            if &self.data[self.pos..self.pos + seq.len()] == seq {
                self.pos += seq.len();
                return;
            }
            self.pos += 1;
        }

        self.pos = self.data.len();
    }

    pub fn next(&mut self) -> XmlEvent<'a> {
        self.skip_whitespace();

        if self.pos >= self.data.len() {
            return XmlEvent::Eof;
        }

        if self.in_tag {
            match self.data[self.pos] {
                b'>' => {
                    self.pos += 1;
                    self.in_tag = false;
                    return self.next();
                }
                b'/' => {
                    // Properly emit EndTag for self-closing elements
                    if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'>' {
                        self.pos += 2;
                        self.in_tag = false;
                        return XmlEvent::EndTag(self.current_tag);
                    }
                }
                _ => {
                    let key_start = self.pos;
                    while self.pos < self.data.len() && self.data[self.pos] != b'=' {
                        self.pos += 1;
                    }
                    let key = &self.data[key_start..self.pos];

                    self.pos += 1; // Skip '='
                    self.pos += 1; // Skip opening '"'

                    let val_start = self.pos;
                    while self.pos < self.data.len() && self.data[self.pos] != b'"' {
                        self.pos += 1;
                    }
                    let val = &self.data[val_start..self.pos];

                    self.pos += 1; // Skip closing '"'
                    return XmlEvent::Attribute(key, val);
                }
            }
        }

        if self.data[self.pos] == b'<' {
            self.pos += 1;

            if self.pos >= self.data.len() {
                return XmlEvent::Eof;
            }

            if self.data[self.pos] == b'?' {
                self.pos += 1;
                self.skip_until_sequence(b"?>");
                return self.next();
            }

            if self.data[self.pos] == b'!' {
                self.pos += 1;
                if self.pos + 2 <= self.data.len() && &self.data[self.pos..self.pos + 2] == b"--" {
                    self.pos += 2;
                    self.skip_until_sequence(b"-->");
                } else {
                    self.skip_until_byte(b'>');
                }
                return self.next();
            }

            if self.data[self.pos] == b'/' {
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.data.len() && self.data[self.pos] != b'>' {
                    self.pos += 1;
                }
                let tag = &self.data[start..self.pos];
                self.pos += 1;
                return XmlEvent::EndTag(tag);
            }

            let start = self.pos;
            while self.pos < self.data.len()
                && !self.data[self.pos].is_ascii_whitespace()
                && self.data[self.pos] != b'>'
                && self.data[self.pos] != b'/'
            {
                self.pos += 1;
            }
            let tag = &self.data[start..self.pos];
            self.in_tag = true;
            self.current_tag = tag; // Store it for self-closing tags
            return XmlEvent::StartTag(tag);
        }

        self.pos += 1;
        self.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_xml_declaration_doctype_and_comments() {
        let svg = br#"<?xml version="1.0"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<!-- ignored -->
<svg><path d="M0 0L1 1"/></svg>"#;
        let mut parser = XmlParser::new(svg);

        assert_eq!(parser.next(), XmlEvent::StartTag(b"svg"));
        assert_eq!(parser.next(), XmlEvent::StartTag(b"path"));
        assert_eq!(parser.next(), XmlEvent::Attribute(b"d", b"M0 0L1 1"));
        assert_eq!(parser.next(), XmlEvent::EndTag(b"path"));
        assert_eq!(parser.next(), XmlEvent::EndTag(b"svg"));
        assert_eq!(parser.next(), XmlEvent::Eof);
    }
}

pub struct StyleParser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> StyleParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    /// Yields the next (key, value) pair from a style string like "fill: #f00; stroke: none"
    pub fn next_kv(&mut self) -> Option<(&'a [u8], &'a [u8])> {
        self.skip_whitespace();
        if self.pos >= self.data.len() {
            return None;
        }

        // 1. Find the key (up to the ':')
        let key_start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != b':' {
            self.pos += 1;
        }

        // Trim trailing whitespace from the key
        let mut key_end = self.pos;
        while key_end > key_start && self.data[key_end - 1].is_ascii_whitespace() {
            key_end -= 1;
        }
        let key = &self.data[key_start..key_end];

        if self.pos < self.data.len() {
            self.pos += 1; // Skip the ':'
        }

        // 2. Find the value (up to the ';')
        self.skip_whitespace();
        let val_start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != b';' {
            self.pos += 1;
        }

        // Trim trailing whitespace from the value
        let mut val_end = self.pos;
        while val_end > val_start && self.data[val_end - 1].is_ascii_whitespace() {
            val_end -= 1;
        }
        let val = &self.data[val_start..val_end];

        if self.pos < self.data.len() {
            self.pos += 1; // Skip the ';'
        }

        Some((key, val))
    }
}
