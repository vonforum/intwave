use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use aus::{
    analysis::{make_log_spectrum, make_power_spectrum},
    spectrum::{complex_to_polar_rfft, rfft},
};
use png::{BitDepth, ColorType, Encoder};
use serde_json::Map;
use wavers::{Samples, Wav};

use crate::cli::Cli;

use super::Analyser;

pub struct FftAnalyser {
    fft_size: usize,
    channels: usize,
    counter: usize,
    bins: Vec<Vec<f64>>, // [channel][bin]
    results: Vec<u8>,
}

/** Writes FFT results to a .png file as little-endian raw bytes. */
impl FftAnalyser {
    pub fn new(args: &Cli, wav: &Wav<i32>) -> Self {
        let channels = wav.n_channels() as usize;

        Self {
            fft_size: args.fft_bins,
            channels,
            counter: 0,
            bins: vec![Vec::new(); channels],
            results: vec![],
        }
    }
}

fn analyse_bins(results: &mut Vec<u8>, fft_size: usize, bins: &[f64]) {
    let imaginary = rfft(bins, fft_size);
    let (magnitude, _) = complex_to_polar_rfft(&imaginary);
    let power_spectrum = make_power_spectrum(&magnitude);
    let log_spectrum = make_log_spectrum(&power_spectrum, 1.0, -10e8, None);

    results.extend(log_spectrum.iter().map(|f| f.to_le_bytes()).flatten());
}

impl Analyser for FftAnalyser {
    fn analyse(&mut self, _label: &str, _frame_counter: usize, frame: &Samples<i32>) {
        for (channel_index, sample) in frame.iter().enumerate() {
            let bin = *sample as f64;
            self.bins[channel_index].push(bin);
        }

        self.counter += 1;

        if self.counter >= self.fft_size {
            // Perform FFT for each channel
            for bins in self.bins.iter_mut() {
                analyse_bins(&mut self.results, self.fft_size, bins);
                bins.clear();
            }

            self.counter = 0;
        }
    }

    fn finish(&mut self, _label: &str) -> u8 {
        for bins in self.bins.iter_mut() {
            if !bins.is_empty() {
                bins.extend(vec![0.0; self.fft_size - bins.len()]); // Zero-pad to fft_size
                analyse_bins(&mut self.results, self.fft_size, bins);
            }
        }

        0
    }

    fn json(&self, args: &Cli) -> Vec<(String, serde_json::Value)> {
        // Set output path to either fft_file or next to json with _fft suffix
        let mut path;
        if let Some(file) = args.fft_file.as_ref() {
            path = PathBuf::from(file);
        } else {
            path = PathBuf::from(args.json.as_ref().unwrap());
            let name = path.file_stem().unwrap().to_string_lossy();
            path.set_file_name(format!("{name}_fft.png"));
        };

        let file = File::create(&path).expect("Could not create FFT output file");
        let mut w = BufWriter::new(file);

        // Create an image where each row is a single time slice with each channel concatenated
        let width = self.channels * (self.fft_size / 2 + 1);
        let height = self.results.len() / (width * 8);

        let mut encoder = Encoder::new(&mut w, width as u32, height as u32);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Sixteen);

        let mut writer = encoder.write_header().expect("Could not write PNG header");

        writer
            .write_image_data(&self.results)
            .expect("Could not write FFT image data");

        let mut map = Map::new();

        if let Some(json_dir) = Path::new(args.json.as_ref().unwrap()).parent() {
            path = path.strip_prefix(json_dir).unwrap_or(&path).to_path_buf();
        }

        map.insert(
            "output".to_string(),
            serde_json::Value::from(path.to_string_lossy().to_string()),
        );

        vec![(
            "fft".to_string(),
            serde_json::json!({ "size": self.fft_size, "results": map }),
        )]
    }
}
