use std::{fs::File, io::BufWriter, path::PathBuf};

use aus::analysis::dbfs;
use png::{BitDepth, ColorType, Encoder};
use wavers::{Samples, Wav};

use crate::{analysers::Analyser, cli::Cli};

pub struct PeaksAnalyzer {
    channels: usize,
    path: PathBuf,
    peaks: Vec<Vec<f64>>,
}

/** Writes peaks to a .png file as little-endian raw f64s.
Each channel is written as a square with dimensions ⌈√(sample count)⌉² and padded with f64::NEG_INFINITY. */
impl PeaksAnalyzer {
    pub fn new(_args: &Cli, wav: &Wav<i32>, path: PathBuf) -> Self {
        let channels = wav.n_channels() as usize;

        Self {
            channels,
            path,
            peaks: vec![vec![]; channels],
        }
    }
}

impl Analyser for PeaksAnalyzer {
    fn analyse(&mut self, _label: &str, _frame_counter: usize, frame: &Samples<i32>) {
        for (channel, sample) in frame.iter().enumerate() {
            self.peaks[channel].push(dbfs(*sample as f64, 1e-20));
        }
    }

    fn finish(&mut self, _label: &str) -> u8 {
        if self.peaks.is_empty() {
            return 0;
        }

        let Ok(file) = File::create(&self.path) else {
            println!(
                "Peaks: Could not create output file at {}",
                self.path.display()
            );

            return 0;
        };

        let mut results = vec![];

        for channel in &self.peaks {
            for peak in channel {
                results.extend(peak.to_le_bytes());
            }

            let num_peaks = channel.len();
            let sqrt = (num_peaks as f64).sqrt();
            let width = sqrt.ceil() as u32;
            let height = sqrt.ceil() as u32;

            // Pad the image to a square shape
            for _ in 0..(width * height - num_peaks as u32) {
                results.extend(f64::NEG_INFINITY.to_le_bytes());
            }
        }

        let sqrt = (self.peaks[0].len() as f64).sqrt();
        let width = sqrt.ceil() as u32;
        let height = sqrt.ceil() as u32 * self.channels as u32;

        let mut w = BufWriter::new(file);
        let mut encoder = Encoder::new(&mut w, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Sixteen);

        let Ok(mut writer) = encoder.write_header() else {
            println!("Peaks: Could not write PNG header");

            return 0;
        };

        let Ok(_) = writer.write_image_data(&results) else {
            println!("Peaks: Could not write image data");

            return 0;
        };

        0
    }

    fn json(&self) -> Vec<(String, serde_json::Value)> {
        let mut results = vec![];

        if let Ok(path) = self.path.canonicalize()
            && self.peaks.len() > 0
        {
            let path = path.to_string_lossy().to_string();
            let channel_size = self.peaks[0].len();
            let w = (channel_size as f64).sqrt().ceil() as u32;
            let squared_size = w * w;
            let padding = squared_size - channel_size as u32;

            let json = serde_json::json!({
                "output": path,
                "channelSize": channel_size,
                "squareSize": squared_size,
                "padding": padding,
            });
            results.push(("peaks".to_string(), json));
        }

        results
    }
}
