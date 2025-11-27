use std::fs::File;
use std::io::{BufReader, BufWriter};

use clap::Parser;

#[derive(Parser, Debug)]
struct Cli {
    /// The raw FFT file (PNG)
    #[arg(short, long, required(true))]
    input: String,

    /// The output visualization file (PNG)
    #[arg(short, long, required(true))]
    output: String,
}

fn main() {
    let args = Cli::parse();

    let decoder = png::Decoder::new(BufReader::new(
        File::open(args.input).expect("Could not open input PNG file"),
    ));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..info.buffer_size()];

    // Map to f64, storing min and max
    let mut min = None;
    let mut max = None;
    let mut float_data = vec![];

    for byte in (0..bytes.len()).step_by(8) {
        let mut value = f64::from_le_bytes([
            bytes[byte],
            bytes[byte + 1],
            bytes[byte + 2],
            bytes[byte + 3],
            bytes[byte + 4],
            bytes[byte + 5],
            bytes[byte + 6],
            bytes[byte + 7],
        ]);

        if value <= -1000000000.0 {
            float_data.push(value);

            continue;
        }

        value = (value / 10.0).powi(10);

        if min.is_none() || value < min.unwrap() {
            min = Some(value);
        }

        if max.is_none() || value > max.unwrap() {
            max = Some(value);
        }

        float_data.push(value);
    }

    let min = min.expect("No valid data found");
    let max = max.expect("No valid data found");
    let range = max - min;

    // Convert to RGB
    let rotated_width = info.height;
    let rotated_height = info.width;

    let mut rgb_data = vec![0u8; (rotated_width * rotated_height * 3) as usize];

    for (i, value) in float_data
        .iter()
        .map(|&v| ((v - min) / range).clamp(0.0, 1.0))
        .enumerate()
    {
        let blue = (value * 3.0).min(1.0);
        let green = ((value - 0.33) * 3.0).max(0.0).min(1.0);
        let red = ((value - 0.66) * 3.0).max(0.0).min(1.0);

        // Rotate coordinates 90 degrees counter-clockwise
        let x = i % info.width as usize;
        let y = i / info.width as usize;
        let new_x = y;
        let new_y = (info.width as usize - 1) - x;
        let new_index = (new_y * rotated_width as usize + new_x) * 3;

        rgb_data[new_index] = (red * 255.0) as u8;
        rgb_data[new_index + 1] = (green * 255.0) as u8;
        rgb_data[new_index + 2] = (blue * 255.0) as u8;
    }

    let file = File::create(args.output).expect("Could not create output PNG file");
    let mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(&mut w, rotated_width, rotated_height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().expect("Could not write PNG header");

    writer
        .write_image_data(&rgb_data)
        .expect("Could not write FFT visualization data");
}
