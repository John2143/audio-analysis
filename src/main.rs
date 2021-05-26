use std::{fmt::Display, path::PathBuf};

use rustfft::{num_complex::Complex, FftPlanner};

fn read_wav(filename: &str) -> (wav::Header, Vec<i16>) {
    let mut file = std::fs::File::open("/Users/jschmidt/Downloads/".to_string() + filename)
        .unwrap_or_else(|_| {
            std::fs::File::open("/mnt/c/Users/John/Downloads/".to_string() + filename).unwrap()
        });

    let (header, data) = wav::read(&mut file).expect("invalid wav file");

    let data = match data {
        wav::BitDepth::Sixteen(v) => v,
        _ => panic!("only supports bit depth of 16"),
    };

    (header, data)
}

struct Seconds(f32);
impl Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02.3}", (self.0 as usize) / 60, self.0 % 60.0)
    }
}

fn main() {
    //let filename = "EAS_test_tone.wav";
    let filename = "eas_file.wav";
    //let filename = "/mnt/c/Users/John/Downloads/Required Monthly Test.mp3";

    let (header, data) = read_wav(filename);
    println!("done reading wav");

    let sampling_rate = header.sampling_rate as usize;
    dbg!(sampling_rate);
    //we are looking for frequencies of approx 2.5khz, so use ~2000
    let fft_size = (8000usize).next_power_of_two();
    dbg!(fft_size);
    let sample_size = ((sampling_rate / 2) / fft_size + 1) * fft_size; //sliding window of ~1 second
    dbg!(sample_size);

    let mut planner = FftPlanner::new();

    let fft = planner.plan_fft_inverse(fft_size);

    //TODO: Read bars&tone for calibration

    let mut file_num = 0;
    for start in (0..(data.len() - sample_size)).step_by(sample_size / 3) {
        //Take some window of the input data:
        //  map to complex numbers with real part = input (i16) as f32
        let mut fft_input: Vec<_> = data
            .iter()
            .skip(start)
            .take(sample_size)
            .enumerate()
            .map(|(_, i)| Complex::new(f32::from(*i), 0.0))
            .collect();

        //TODO investigate windowing types
        //https://en.wikipedia.org/wiki/Window_function

        let start_secs = start as f32 / sampling_rate as f32 / 2.0; //TODO why divide 2?
        let end_secs = (start + sample_size) as f32 / sampling_rate as f32 / 2.0;

        //TODO: needs 0 padding?
        //fft_input.extend(std::iter::repeat(Complex::new(0.0, 0.0)).take(sample_size));

        fft.process(&mut fft_input);

        //this FFT is symmetrical across x = fft_size / 2, so only care about first half
        //remove this when using 0-padding
        fft_input.truncate(fft_size / 2);

        for (i, c) in fft_input.iter_mut().enumerate() {
            //normalize imaginary part
            c.im /= (fft_size as f32).sqrt();

            //transform real part to frequency domain
            //TODO why *2.0 for real pcm files?
            let hz = 2.0 * (i as f32 * sampling_rate as f32) / fft_size as f32;
            c.re = hz;
        }

        //Now, find the peaks in the audio
        fft_input.sort_unstable_by(|a, b| b.im.partial_cmp(&a.im).unwrap());

        //let median_amplitude75 = fft_input[fft_input.len() / 4].im;

        let mut has_2083 = false;
        let mut has_1562 = false;
        //let mut has_1000 = false;
        let mut has_853 = false;
        let mut has_960 = false;

        //look at the top 5 to find any important waveforms
        for c in fft_input.iter().take(5) {
            if (c.re - 2083.0).abs() < 10.0 {
                has_2083 = true;
            }
            if (c.re - 1562.0).abs() < 10.0 {
                has_1562 = true;
            }
            if (c.re - 853.0).abs() < 10.0 {
                has_853 = true;
            }
            if (c.re - 960.0).abs() < 10.0 {
                has_960 = true;
            }
        }

        let has_eas_low = has_960 && has_853;
        let has_eas_high = has_2083 && has_1562;
        if has_eas_low || has_eas_high {
            //TODO Start testing finer FFT boundaries.
            println!(
                "#==============#\n{} - {} ({})",
                Seconds(start_secs),
                Seconds(end_secs),
                //median_amplitude75,
                start_secs,
            );
            // //print top 5 peaks in audio
            //for (i, c) in fft_input.iter().enumerate().take(5) {
            //println!("#{}: {}hz ({})", i + 1, c.re, c.im);
            //}
            println!(
                "EAS (2khz): {}, EAS: (850hz): {}",
                has_eas_high, has_eas_low,
            );

            let (fnum, fname) = (file_num, filename.to_string());
            std::thread::spawn(move || {
                plot(
                    fft_input.to_vec(),
                    &format!("{:.03} - {}", &fnum, &fname),
                    start_secs,
                    end_secs,
                )
                .unwrap();
            });
            file_num += 1;
        }
    }
}

fn plot(
    mut fft_input: Vec<Complex<f32>>,
    filename: &str,
    start_secs: f32,
    end_secs: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    use plotters::prelude::*;
    //let mut fft_input = fft_input.to_vec();
    fft_input.sort_unstable_by(|a, b| a.re.partial_cmp(&b.re).unwrap());

    let scale = 2;
    let mut path: PathBuf = ["images", filename].iter().collect();
    path.set_extension("png");
    let drawing_area = BitMapBackend::new(&path, (600 * scale, 400 * scale)).into_drawing_area();

    drawing_area.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&drawing_area)
        .margin(5 * scale)
        .caption(
            format!(
                "{}: {} - {}",
                filename,
                Seconds(start_secs),
                Seconds(end_secs)
            ),
            ("sans-serif", 30 * scale).into_font(),
        )
        .x_label_area_size(30 * scale)
        .y_label_area_size(30 * scale)
        .build_cartesian_2d(0..5000 as i32, 0..100_000)?;

    chart.configure_mesh().draw()?;

    chart.draw_series(LineSeries::new(
        fft_input
            .iter()
            .enumerate()
            .map(|(_, c)| (c.re as i32, c.im as i32)),
        &BLACK,
    ))?;
    //.label("frequency");

    //chart
    //.configure_series_labels()
    //.background_style(&WHITE.mix(0.8))
    //.border_style(&BLACK)
    //.draw()?;

    Ok(())
}
