# analwave

Crude tool to detect underruns and silence in WAV files.
The primary use-case is for identifying problems and waste in large multi-track recording sessions.

```
Usage: analwave [OPTIONS] --input <INPUT>

Options:
  -i, --input <INPUT>
          The file to analyse
  -u, --underrun
          Detect underruns
      --samples <SAMPLES>
          Underrun detection minimum samples [default: 16]
  -s, --silence
          Detect silence
      --lufs <LUFS>
          Silence threshold (LUFS-S) [default: -70]
      --silence-percentage <SILENCE_PERCENTAGE>
          Silence percentage (returns error code if total silence is above this threshold) [default: 99]
      --no-progress
          No fancy progress-bar
      --debug
          Debug output
      --silent
          Silent (no output)
      --json <JSON>
          Output results as JSON to file
      --window-size <WINDOW_SIZE>
          Window size for silence / loudness in seconds [default: 1]
  -l, --loudness
          Track loudness to JSON (does nothing if JSON output is not enabled)
  -f, --fft
          Track FFT to file (does nothing if JSON output is not enabled)
      --fft-bins <FFT_BINS>
          Number of FFT bins [default: 2048]
      --fft-file <FFT_FILE>
          FFT output file (defaults to <json_file>_fft.png)
  -h, --help
          Print help
  -V, --version
          Print version
```


## Return codes

- If underruns are detected then `exit_code & 0b0001` will be true.
- If total silence amount exceeds --silence-percentage then `exit_code & 0b0010` will be true.
