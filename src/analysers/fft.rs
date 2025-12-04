use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use aus::{
    WindowType,
    analysis::{make_log_spectrogram, make_power_spectrogram},
    spectrum::{complex_to_polar_rstft, rstft},
};
use png::{BitDepth, ColorType, Encoder};
use serde_json::Map;
use wavers::{Samples, Wav};

use crate::cli::Cli;

use super::Analyser;

pub struct FftVisualizer {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub path: PathBuf,
    pub data: Vec<f64>,
}

impl FftVisualizer {
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            min: None,
            max: None,
            data: vec![],
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn find_min_max(&mut self) {
        let (min, max) = self.data.iter().fold((None, None), |(min, max), &value| {
            let min = match min {
                Some(m) if value < m => Some(value),
                Some(m) => Some(m),
                None => Some(value),
            };

            let max = match max {
                Some(m) if value > m => Some(value),
                Some(m) => Some(m),
                None => Some(value),
            };

            (min, max)
        });

        self.min = min;
        self.max = max;
    }

    pub fn extend<I>(&mut self, data: I)
    where
        I: IntoIterator<Item = f64>,
    {
        self.data.extend(data.into_iter().map(|v| {
            // Update min and max while mapping to avoid another iteration
            if self.min.is_none() || (self.min.is_some() && v < self.min.unwrap()) {
                self.min = Some(v);
            }
            if self.max.is_none() || (self.max.is_some() && v > self.max.unwrap()) {
                self.max = Some(v);
            }

            v
        }));
    }

    pub fn visualize(&self, width: usize, height: usize) {
        if self.min.is_none() || self.max.is_none() {
            println!("FFT Visualization: No valid data to visualize.");

            return;
        }

        let min = self.min.unwrap();
        let max = self.max.unwrap();
        let range = max - min;

        // Convert to RGB
        let rotated_width = height;
        let rotated_height = width;

        let mut rgb_data = vec![0u8; (rotated_width * rotated_height * 3) as usize];

        for (i, value) in self
            .data
            .iter()
            .map(|v| ((v - min) / range).clamp(0.0, 1.0).powi(2)) // Squaring for better contrast
            .enumerate()
        {
            let blue = (value * 3.0).min(1.0);
            let green = ((value - 0.33) * 3.0).max(0.0).min(1.0);
            let red = ((value - 0.66) * 3.0).max(0.0).min(1.0);

            // Rotate coordinates 90 degrees counter-clockwise
            let x = i % width;
            let y = i / width;
            let new_x = y;
            let new_y = (width - 1) - x;
            let new_index = (new_y * rotated_width + new_x) * 3;

            rgb_data[new_index] = (red * 255.0) as u8;
            rgb_data[new_index + 1] = (green * 255.0) as u8;
            rgb_data[new_index + 2] = (blue * 255.0) as u8;
        }

        let Ok(file) = File::create(&self.path) else {
            println!("Could not create output PNG file");

            return;
        };

        let mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(&mut w, rotated_width as u32, rotated_height as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);

        let Ok(mut writer) = encoder.write_header() else {
            println!("Could not write PNG header");

            return;
        };

        let Ok(_) = writer.write_image_data(&rgb_data) else {
            println!("Could not write FFT visualization data");

            return;
        };
    }
}

struct FftOutput {
    results: Vec<u8>,
    path: PathBuf,
}

pub struct FftAnalyser {
    fft_size: usize,
    channels: usize,
    bins: Vec<Vec<f64>>, // [channel][bin]
    raw: Option<FftOutput>,
    vis: Option<FftVisualizer>,
}

/** Writes FFT results to a .png file as little-endian raw bytes. */
impl FftAnalyser {
    pub fn new(args: &Cli, wav: &Wav<i32>, path: Option<PathBuf>) -> Self {
        let channels = wav.n_channels() as usize;

        Self {
            fft_size: args.fft_bins,
            channels,
            bins: vec![Vec::new(); channels],
            raw: path.map(|path| FftOutput {
                results: vec![],
                path,
            }),
            vis: args.fft_vis.as_ref().map(|p| FftVisualizer::new(p)),
        }
    }
}

impl Analyser for FftAnalyser {
    fn analyse(&mut self, _label: &str, _frame_counter: usize, frame: &Samples<i32>) {
        for (channel_index, sample) in frame.iter().enumerate() {
            let bin = *sample as f64;
            self.bins[channel_index].push(bin);
        }
    }

    fn finish(&mut self, _label: &str) -> u8 {
        let mut spectra = vec![];

        for i in 0..self.channels {
            let data = &self.bins[i];
            let imaginary = rstft(data, self.fft_size, self.fft_size / 2, WindowType::Hanning);

            let (magnitude, _) = complex_to_polar_rstft(&imaginary);
            let power = make_power_spectrogram(&magnitude);
            let log_spectrogram = make_log_spectrogram(&power, 10.0, 10e-8, None);

            spectra.push(log_spectrogram);
        }

        // Interleave spectra data
        let num_slices = spectra[0].len();
        for slice_index in 0..num_slices {
            for channel_index in 0..self.channels {
                if let Some(raw) = &mut self.raw {
                    raw.results.extend(
                        spectra[channel_index][slice_index]
                            .iter()
                            .flat_map(|&v| v.to_le_bytes()),
                    );
                }

                if let Some(vis) = &mut self.vis {
                    vis.extend(spectra[channel_index][slice_index].iter().cloned());
                }
            }
        }

        // Create an image where each row is a single time slice with each channel concatenated
        let width = self.channels * (self.fft_size / 2 + 1);

        if let Some(vis) = &self.vis {
            let height = vis.data.len() / width;
            vis.visualize(width, height);
        }

        if self.raw.is_none() {
            return 0;
        }

        let raw = self.raw.as_ref().unwrap();
        let height = raw.results.len() / (width * 8);
        let Ok(file) = File::create(&raw.path) else {
            println!(
                "FFT: Could not create output file at {}",
                raw.path.display()
            );

            return 0;
        };
        let mut w = BufWriter::new(file);

        let mut encoder = Encoder::new(&mut w, width as u32, height as u32);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Sixteen);

        let Ok(mut writer) = encoder.write_header() else {
            println!("FFT: Could not write PNG header");

            return 0;
        };

        let Ok(_) = writer.write_image_data(&raw.results) else {
            println!("Could not write FFT image data");

            return 0;
        };

        0
    }

    fn json(&self) -> Vec<(String, serde_json::Value)> {
        let mut map = Map::new();

        if let Some(raw) = &self.raw {
            // We can canonicalize here because the file has already been written in finish()
            if let Ok(path) = raw.path.canonicalize() {
                map.insert(
                    "output".to_string(),
                    serde_json::Value::from(path.to_string_lossy()),
                );
            };
        }

        if let Some(vis) = &self.vis {
            if let Ok(vis_path) = vis.path.canonicalize() {
                map.insert(
                    "visualization".to_string(),
                    serde_json::Value::from(vis_path.to_string_lossy()),
                );
            };
        }

        vec![(
            "fft".to_string(),
            serde_json::json!({ "size": self.fft_size, "results": map }),
        )]
    }
}
