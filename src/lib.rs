pub mod xml;
pub use xml::*;

pub mod state;
pub use state::*;

pub mod path;
pub use path::*;

pub mod cubic;
pub use cubic::*;

pub mod rasteriser;
pub use rasteriser::*;

pub mod transform;
pub use transform::*;

pub fn parse_svg(svg_data: &[u8], pixels: &mut [u32], width: usize, height: usize) {
    let mut xml_parser = XmlParser::new(svg_data);
    let mut state_stack = StateStack::new();
    let mut rasterizer = Rasterizer::new(pixels, width, height);

    let mut current_tag: &[u8] = b"";
    let mut current_d: Option<&[u8]> = None;

    loop {
        match xml_parser.next() {
            XmlEvent::StartTag(tag) => {
                current_tag = tag;
                state_stack.push(); // Push state for EVERY tag to prevent leaks
                current_d = None; // Reset path data for the new tag
            }
            XmlEvent::Attribute(key, val) => {
                if current_tag == b"svg" && key == b"viewBox" {
                    apply_view_box_attribute(val, width, height, &mut state_stack);
                } else if key == b"style" {
                    let mut style_parser = StyleParser::new(val);
                    while let Some((s_key, s_val)) = style_parser.next_kv() {
                        apply_style_attribute(s_key, s_val, &mut state_stack);
                    }
                } else {
                    apply_style_attribute(key, val, &mut state_stack);
                }

                if current_tag == b"path" && key == b"d" {
                    current_d = Some(val); // Save it to process on EndTag
                }
            }
            XmlEvent::EndTag(tag) => {
                if tag == b"path" {
                    if let Some(d_bytes) = current_d {
                        let current_state = state_stack.current();

                        if let Some(fill_color) = current_state.fill {
                            process_path(
                                d_bytes,
                                &current_state.transform,
                                &mut rasterizer,
                                PathMode::Fill,
                            );
                            rasterizer.rasterize(fill_color, current_state.fill_rule);
                        }

                        if let Some(stroke_color) = current_state.stroke {
                            if current_state.stroke_width > 0.0 {
                                let scale = (current_state.transform[0]
                                    * current_state.transform[0]
                                    + current_state.transform[1] * current_state.transform[1])
                                    .sqrt();

                                let screen_width = current_state.stroke_width * scale;

                                process_path(
                                    d_bytes,
                                    &current_state.transform,
                                    &mut rasterizer,
                                    PathMode::Stroke(screen_width),
                                );

                                rasterizer.rasterize(stroke_color, FillRule::NonZero);
                            }
                        }
                    }
                }
                state_stack.pop(); // Pop state for EVERY tag
            }
            XmlEvent::Eof => {
                break;
            }
        }
    }
}

pub fn apply_view_box_attribute(
    val: &[u8],
    width: usize,
    height: usize,
    state_stack: &mut StateStack,
) {
    let mut parser = PathParser::new(val);
    let min_x = parser.next_number().unwrap_or(0.0);
    let min_y = parser.next_number().unwrap_or(0.0);
    let view_width = parser.next_number().unwrap_or(0.0);
    let view_height = parser.next_number().unwrap_or(0.0);

    if view_width <= f32::EPSILON || view_height <= f32::EPSILON {
        return;
    }

    let sx = width as f32 / view_width;
    let sy = height as f32 / view_height;
    let view_box_transform = [sx, 0.0, 0.0, sy, -min_x * sx, -min_y * sy];
    let parent_transform = state_stack.current().transform;
    state_stack.current_mut().transform =
        multiply_transforms(&parent_transform, &view_box_transform);
}

#[inline(always)]
pub fn parse_f32(val: &[u8]) -> Option<f32> {
    // Safely cast the ascii bytes to a string slice and parse using Rust's built-in float parser
    std::str::from_utf8(val).ok()?.parse::<f32>().ok()
}

pub fn apply_style_attribute(key: &[u8], val: &[u8], state_stack: &mut StateStack) {
    match key {
        b"fill" => state_stack.current_mut().fill = parse_color(val),
        b"fill-rule" => {
            state_stack.current_mut().fill_rule = match val {
                b"evenodd" => FillRule::EvenOdd,
                _ => FillRule::NonZero,
            };
        }
        b"stroke" => state_stack.current_mut().stroke = parse_color(val),
        b"stroke-width" => {
            if let Some(w) = parse_f32(val) {
                state_stack.current_mut().stroke_width = w;
            }
        }
        b"opacity" => {
            if let Some(o) = parse_f32(val) {
                state_stack.current_mut().opacity = o;
            }
        }
        b"transform" => {
            let local_transform = parse_transform(val);
            let parent_transform = state_stack.current().transform;
            state_stack.current_mut().transform =
                multiply_transforms(&parent_transform, &local_transform);
        }
        _ => {} // Ignore unhandled attributes (like 'd', 'id', etc.)
    }
}

pub fn parse_color(val: &[u8]) -> Option<u32> {
    if val == b"none" {
        return None; // The shape should not be filled
    }

    if val.is_empty() || val[0] != b'#' {
        // In a full engine, you'd handle named colors ("red", "blue") here.
        // For our fast renderer, we will default to black for unrecognised formats.
        return Some(0x000000FF);
    }

    let hex_bytes = &val[1..];

    // Helper closure to convert a single hex ASCII character to a number (0-15)
    let parse_hex_char = |c: u8| -> u32 {
        match c {
            b'0'..=b'9' => (c - b'0') as u32,
            b'a'..=b'f' => (c - b'a' + 10) as u32,
            b'A'..=b'F' => (c - b'A' + 10) as u32,
            _ => 0, // Fallback for invalid characters
        }
    };

    if hex_bytes.len() == 6 {
        // Format: #RRGGBB
        let r = (parse_hex_char(hex_bytes[0]) << 4) | parse_hex_char(hex_bytes[1]);
        let g = (parse_hex_char(hex_bytes[2]) << 4) | parse_hex_char(hex_bytes[3]);
        let b = (parse_hex_char(hex_bytes[4]) << 4) | parse_hex_char(hex_bytes[5]);

        // Return 0xRRGGBBFF (Fully opaque alpha)
        return Some((r << 24) | (g << 16) | (b << 8) | 0xFF);
    } else if hex_bytes.len() == 3 {
        // Format: #RGB (Shorthand, e.g., #F00 becomes #FF0000)
        let r = parse_hex_char(hex_bytes[0]);
        let g = parse_hex_char(hex_bytes[1]);
        let b = parse_hex_char(hex_bytes[2]);

        let r = (r << 4) | r;
        let g = (g << 4) | g;
        let b = (b << 4) | b;

        return Some((r << 24) | (g << 16) | (b << 8) | 0xFF);
    }

    Some(0x000000FF) // Default fallback
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiger_render_is_visible_and_not_black_dominated() {
        let width = 640;
        let height = 480;
        let mut pixels = vec![0xFFFFFFFF; width * height];

        parse_svg(include_bytes!("../svg/23.svg"), &mut pixels, width, height);

        let non_white = pixels.iter().filter(|&&pixel| pixel != 0xFFFFFFFF).count();
        let black = pixels.iter().filter(|&&pixel| pixel == 0x000000FF).count();
        let colored = pixels
            .iter()
            .filter(|&&pixel| pixel != 0xFFFFFFFF && pixel != 0x000000FF)
            .count();

        assert!(
            non_white > 20_000,
            "tiger render should draw visible pixels"
        );
        assert!(colored > 5_000, "tiger render should include filled colors");
        assert!(
            black * 2 < non_white,
            "black stroke pixels should not dominate the render"
        );
    }

    #[test]
    fn search_icon_renders_from_view_box_and_quadratic_path() {
        let width = 24;
        let height = 24;
        let mut pixels = vec![0xFFFFFFFF; width * height];

        parse_svg(
            include_bytes!("../svg/search_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg"),
            &mut pixels,
            width,
            height,
        );

        let icon_pixels = pixels.iter().filter(|&&pixel| pixel == 0xE3E3E3FF).count();

        assert!(
            icon_pixels > 40,
            "search icon should render visible inherited fill pixels"
        );
    }
}
