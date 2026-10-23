//! Offline codec comparison: no device input and no image output files.
use andromeda::capture::{self, Format, Options};
use clap::Parser;
use serde_json::json;
use std::{io::Read, path::PathBuf, time::Instant};

#[derive(Parser)]
struct Args {
    /// Operator-provided PNG. Image pixels are never written to the report.
    input: PathBuf,
    #[arg(long, default_value_t = 720)]
    max_width: u32,
    #[arg(long, default_value_t = 80)]
    quality: u8,
    #[arg(long, default_value_t = 3)]
    samples: usize,
    #[arg(long)]
    output: PathBuf,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if !(1..=20).contains(&args.samples) {
        return Err("samples must be 1..=20".into());
    }
    let mut png = Vec::new();
    std::fs::File::open(&args.input)?
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut png)?;
    if png.len() > 32 * 1024 * 1024 {
        return Err("input exceeds 32 MiB".into());
    }
    let source = capture::decode(&png)?;
    let mut workloads = Vec::new();
    let mut formats = vec![
        ("png", Format::Png),
        ("jpeg", Format::Jpeg),
        ("webp", Format::Webp),
        ("webp_lossless", Format::WebpLossless),
        ("auto_compatible", Format::Auto),
    ];
    if cfg!(feature = "avif") {
        formats.push(("avif_speed_10", Format::Avif));
    }
    for (name, format) in formats {
        let options = Options {
            max_width: Some(args.max_width),
            max_height: None,
            format,
            quality: Some(args.quality),
        };
        capture::encode(png.clone(), options.clone())?; // warm cache/encoder once
        let mut samples = Vec::new();
        let mut byte_count = 0;
        let mut mime_type = "";
        for _ in 0..args.samples {
            let started = Instant::now();
            let image = capture::encode(png.clone(), options.clone())?;
            samples.push(started.elapsed().as_secs_f64() * 1000.0);
            byte_count = image.bytes.len();
            mime_type = image.mime_type;
        }
        samples.sort_by(f64::total_cmp);
        workloads.push(json!({"format":name,"mime_type":mime_type,"bytes":byte_count,"processing_ms":samples,"p50_ms":samples[(samples.len()-1)/2],"p95_ms":samples[(samples.len()*95).div_ceil(100)-1]}));
    }
    let report = json!({"schema_version":1,"host_os":std::env::consts::OS,"host_arch":std::env::consts::ARCH,"profile":if cfg!(debug_assertions){"debug"}else{"release"},"source_width":source.width(),"source_height":source.height(),"max_width":args.max_width,"quality":args.quality,"samples_per_format":args.samples,"workloads":workloads,"limitations":["Fixed-source offline decode/resize/encode only; excludes Android capture, MCP upload and model thinking.","Quality numbers are codec-specific; these are not matched-perceptual-quality comparisons. webp_lossless preserves pixels; JPEG, webp and AVIF are lossy.","Auto compares PNG/JPEG/WebP bytes, including both lossy and lossless WebP. AVIF is opt-in and excluded from auto because ordinary vision clients do not list it as supported.","Smaller encoded files reduce transport bytes, not necessarily image token counts; image dimensions still matter."]});
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(args.output)?;
    use std::io::Write;
    serde_json::to_writer_pretty(&mut output, &report)?;
    output.write_all(b"\n")?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
