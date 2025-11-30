use std::fs::File;
use std::io::BufReader;

use clap::Parser;

use analwave::analysers::fft::FftVisualizer;

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

    let mut vis = FftVisualizer::new(args.output);
    for byte in (0..bytes.len()).step_by(8) {
        let mut v = f64::from_le_bytes([
            bytes[byte],
            bytes[byte + 1],
            bytes[byte + 2],
            bytes[byte + 3],
            bytes[byte + 4],
            bytes[byte + 5],
            bytes[byte + 6],
            bytes[byte + 7],
        ]);

        if v <= -1000000000.0 {
            vis.data.push(v);

            continue;
        }

        if vis.min.is_none() || v < vis.min.unwrap() {
            vis.min = Some(v);
        }
        if vis.max.is_none() || v > vis.max.unwrap() {
            vis.max = Some(v);
        }

        vis.data.push(v);
    }

    vis.visualize(info.width as usize, info.height as usize);
}
