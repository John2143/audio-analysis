use std::{f32::consts::PI, fmt::Display, path::PathBuf};

use rustfft::{num_complex::Complex, FftPlanner};
use wav::Header;

pub type RawWav = Vec<f32>;

pub fn read_wav(filename: &str) -> (Header, RawWav) {
    let mut file = std::fs::File::open(filename).unwrap();

    let (header, data) = wav::read(&mut file).expect("invalid wav file");

    dbg!(&header);

    let data = match data {
        wav::BitDepth::Sixteen(v) => v
            .into_iter()
            .map(|i| {
                let i = i as f32;
                i / i16::MAX as f32
            })
            .collect(),
        wav::BitDepth::ThirtyTwoFloat(floats) => floats,
        _ => panic!("only supports bit depth of s16 or f32"),
    };

    dbg!(data.len());

    //let mut f = std::fs::File::create("./generated.wav").expect("cant open output wav");
    //wav::write(header, &wav::BitDepth::ThirtyTwoFloat(data.clone()), &mut f)
    //.expect("cant write output wav");

    (header, data)
}

pub fn gen_fake_wav() -> (Header, RawWav) {
    let length = 30;
    let rate = 48000;

    let header = Header::new(wav::header::WAV_FORMAT_PCM, 1, rate as u32, 16);

    let v: Vec<_> = (0..)
        .map(|i| {
            let seconds = i as f32 / rate as f32;
            let hz = 1050.0;
            let f = f32::sin(seconds * hz * PI * 2.0); //-1 - 1
            f
        })
        .take(length * rate)
        .collect();

    let mut f = std::fs::File::create("./generated.wav").expect("cant open output wav");
    wav::write(header, &wav::BitDepth::ThirtyTwoFloat(v.clone()), &mut f)
        .expect("cant write output wav");

    (header, v)
}

pub struct Seconds(f32);
impl Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:06.3}", (self.0 as usize) / 60, self.0 % 60.0)
    }
}

pub fn condense_channels(channel_count: u16, v: RawWav) -> RawWav {
    match channel_count {
        1 => return v,
        2 => {}
        _ => panic!("cannot support wavs with more than 2 channels"),
    };

    v.chunks_exact(2)
        .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
        .collect()
}

pub struct AnalysisInfo {
    pub sample_size: usize,
    pub fft_size: usize,
    pub sample_rate: usize,
    pub eas: Vec<EasEvent>,
}

pub struct EasEvent {
    pub start_sample: usize,
    pub end_sample: usize,
    pub start_seconds: f32,
    pub end_seconds: f32,

    pub has_weather: bool,
    pub has_eas: bool,
}

pub fn process_wav(_filename: &str, header: Header, data: RawWav) -> AnalysisInfo {
    println!("done reading wav");

    let sampling_rate = header.sampling_rate as usize;
    dbg!(header.channel_count);
    dbg!(data.len());
    let data = condense_channels(header.channel_count, data);
    dbg!(data.len());

    dbg!(sampling_rate);
    let fft_size = (8000usize).next_power_of_two();
    dbg!(fft_size);
    let sample_size = ((sampling_rate / 2) / fft_size + 1) * fft_size; //sliding window of ~.5 second
    dbg!(sample_size);

    let mut planner = FftPlanner::new();

    let fft = planner.plan_fft_inverse(fft_size);

    //TODO: Read bars&tone for calibration

    let mut flags = Vec::new();

    //let mut file_num = 0;
    for start in (0..(data.len() - sample_size)).step_by(sample_size / 8) {
        fn window(value: f32, i: usize, sample_size: usize) -> Complex<f32> {
            let x = i as f32 / sample_size as f32;
            let x = x - 0.5;

            let window_val = (1.0 / 2.0) * (1.0 + (2.0 * PI * x).cos());
            //dbg!(window_val);

            Complex::new(window_val * value, 0.0)
        }

        //Take some window of the input data:
        //  map to complex numbers with real part = input (i16) as f32
        let mut fft_input: Vec<_> = data
            .iter()
            .skip(start)
            .take(sample_size)
            .enumerate()
            .map(|(k, i)| window(f32::from(*i), k, sample_size))
            .collect();

        //TODO investigate windowing types
        //https://en.wikipedia.org/wiki/Window_function

        //let tc_offset = 58 * 60 + 40;
        let tc_offset = 0;
        let tc_offset = tc_offset as f32;

        let start_secs = start as f32 / sampling_rate as f32 + tc_offset;
        let end_secs = (start + sample_size) as f32 / sampling_rate as f32 + tc_offset;

        //TODO: needs 0 padding?
        //fft_input.extend(std::iter::repeat(Complex::new(0.0, 0.0)).take(sample_size));

        fft.process(&mut fft_input);

        //this FFT is symmetrical across x = fft_size / 2, so only care about first half
        //remove this when using 0-padding
        fft_input.truncate(fft_size / 2);

        for (i, c) in fft_input.iter_mut().enumerate() {
            //normalize imaginary part
            //c.im = (c.re * c.re * c.im * c.im).sqrt();
            c.re /= (fft_size as f32).sqrt();
            c.im = c.re;

            //transform real part to frequency domain
            let hz = (i as f32 * sampling_rate as f32) / fft_size as f32;
            c.re = hz;
        }

        //Now, find the peaks in the audio
        fft_input.sort_unstable_by(|a, b| b.im.partial_cmp(&a.im).unwrap());

        //let median_amplitude75 = fft_input[fft_input.len() / 4].im;

        //let mut has_1000 = false;
        let mut has_853 = false;
        let mut has_960 = false;
        let mut has_1050 = false;

        //look at the top 5 to find any important waveforms
        for c in fft_input.iter().take(5) {
            if (c.re - 853.0).abs() < 10.0 {
                has_853 = true;
            }
            if (c.re - 960.0).abs() < 10.0 {
                has_960 = true;
            }
            if (c.re - 1050.0).abs() < 4.0 {
                has_1050 = true;
            }
        }

        let has_eas_low = has_960 && has_853;
        if has_eas_low || has_1050 {
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
                "EAS: (850hz): {}, Weather(1050hz): {}",
                has_eas_low, has_1050,
            );

            flags.push(EasEvent {
                start_seconds: start_secs,
                end_seconds: end_secs,
                start_sample: start,
                end_sample: start + sample_size,
                has_weather: has_1050,
                has_eas: has_eas_low,
            });

            //let (fnum, fname) = (file_num, filename.to_string());
            //std::thread::spawn(move || {
            //plot(
            //fft_input.to_vec(),
            //&format!("{:.03} - {}", &fnum, &fname),
            //start_secs,
            //end_secs,
            //)
            //.unwrap();
            //});
            //file_num += 1;
        }
    }

    AnalysisInfo {
        sample_size,
        fft_size,
        sample_rate: sampling_rate,
        eas: flags,
    }
}

pub fn plot(
    mut fft_input: Vec<Complex<f32>>,
    filename: &str,
    start_secs: f32,
    end_secs: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    use plotters::prelude::*;
    //let mut fft_input = fft_input.to_vec();
    let max = fft_input[0].im as i32;
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
        .build_cartesian_2d(0..5000 as i32, 0..max)?;

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
