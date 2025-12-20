use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use wavers::{Wav, WaversResult};

use analwave::analysers::{
    Analyser, fft::FftAnalyser, loudness::LoudnessAnalyser, peaks::PeaksAnalyzer,
    underruns::UnderrunAnalyser,
};
use analwave::cli::Cli;
use analwave::output;
use analwave::output::{fmt_frame, init_output};

use analwave::json::write_json;

/// Set png output path to either the provided PNG file path,
/// or derive it from the JSON output path.
fn calculate_png_path(
    json: &Option<String>,
    file: &Option<String>,
    suffix: &str,
) -> Option<PathBuf> {
    if let Some(file) = file {
        Some(PathBuf::from(file))
    } else if let Some(json) = json {
        let mut path = PathBuf::from(json);
        let name = path.file_stem().unwrap().to_string_lossy();
        path.set_file_name(format!("{name}_{suffix}.png"));

        Some(path)
    } else {
        None
    }
}

fn analyse(args: &Cli, wav: &mut Wav<i32>) -> Result<u8, ()> {
    let mut return_code = 0;

    let mut analysers: Vec<Box<dyn Analyser>> = vec![];

    if args.silence || args.loudness {
        analysers.push(Box::new(
            LoudnessAnalyser::new(args, wav).expect("Could not initialize EbuR128"),
        ));
    }

    if args.underrun {
        analysers.push(Box::new(UnderrunAnalyser::new(args, wav)));
    }

    if args.fft || args.fft_vis.is_some() {
        let mut path = None;
        if args.fft {
            path = calculate_png_path(&args.json, &args.fft_file, "fft");
        }

        if args.fft && path.is_none() {
            println!(
                "FFT output was enabled but no path could be determined, please provide --fft-file or --json"
            );
            return Err(());
        } else {
            analysers.push(Box::new(FftAnalyser::new(args, wav, path)));
        }
    }

    if args.peaks {
        let mut path = None;
        if args.peaks {
            path = calculate_png_path(&args.json, &args.peaks_file, "peaks");
        }

        if let Some(path) = path {
            analysers.push(Box::new(PeaksAnalyzer::new(args, wav, path)));
        } else {
            println!(
                "Peaks output was enabled but no path could be determined, please provide --peaks-file or --json"
            );
            return Err(());
        }
    }

    if analysers.is_empty() {
        println!("No detection is active, exiting.");
        return Err(());
    }

    let (_, spec) = wav.wav_spec();
    init_output(&args, wav.n_samples() as u64);

    output!("[+] sample rate:        {}", &spec.fmt_chunk.sample_rate);
    output!("[+] channels:           {}", wav.n_channels());
    output!("[+] total samples:      {}", wav.n_samples());

    if args.silence {
        output!("[+] silence threshold:  {} LUFS-S", &args.lufs);
        output!("[+] silence window:     {} seconds", &args.window_size);
    }

    if args.underrun {
        output!("[+] underrun threshold: {} samples", &args.samples);
    }

    if args.fft || args.fft_vis.is_some() {
        output!("[+] FFT bins:           {}", &args.fft_bins);
    }

    let digits = wav.n_samples().to_string().len();
    let num_frames = wav.n_samples();
    let frames = wav.frames();

    for (frame_counter, frame) in frames.enumerate() {
        let frame_label = fmt_frame(frame_counter, digits);
        output::inc();

        for analyser in analysers.iter_mut() {
            analyser.analyse(&frame_label, frame_counter, &frame);
        }
    }

    let frame_label = fmt_frame(num_frames, digits);

    for analyser in analysers.iter_mut() {
        return_code |= analyser.finish(&frame_label);
    }

    output::finish();

    write_json(args, wav, &analysers);

    Ok(return_code)
}

fn main() -> ExitCode {
    let args = Cli::parse();
    let Ok(mut wav): WaversResult<Wav<i32>> = Wav::from_path(&args.input) else {
        println!("Could not open file: {}", args.input);
        return ExitCode::from(1);
    };

    let Ok(code) = analyse(&args, &mut wav) else {
        return ExitCode::from(1);
    };

    ExitCode::from(code)
}
