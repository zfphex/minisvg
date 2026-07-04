#![allow(unused)]
use minisvg::*;
use std::fs::File;
use std::io::Write;

fn main() {
    let width = 720;
    let height = 720;
    let mut pixels = vec![0; width * height];
    let svg_data = std::fs::read("../svg/search_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg").unwrap();
    parse_svg(svg_data.as_slice(), &mut pixels, width, height);
    write_ppm(&pixels, width, height, "../target/search.ppm");
}

fn tiger() {
    let width = 640;
    let height = 480;
    let mut pixels = vec![0xFFFFFFFF; width * height];
    let svg_data = std::fs::read("../svg/23.svg").unwrap();

    println!("Parsing and rasterizing SVG...");
    parse_svg(svg_data.as_slice(), &mut pixels, width, height);

    println!("Writing output to test_render.ppm...");
    write_ppm(&pixels, width, height, "../target/test_render.ppm");

    println!("Done! Open test_render.ppm in an image viewer.");
}

/// A zero-dependency way to dump a pixel buffer into an image file.
fn write_ppm(pixels: &[u32], width: usize, height: usize, filename: &str) {
    let mut file = File::create(filename).unwrap();
    // P3 specifies an ASCII RGB image
    writeln!(file, "P3\n{} {}\n255", width, height).unwrap();

    for &pixel in pixels {
        let r = (pixel >> 24) & 0xFF;
        let g = (pixel >> 16) & 0xFF;
        let b = (pixel >> 8) & 0xFF;
        writeln!(file, "{} {} {}", r, g, b).unwrap();
    }
}
