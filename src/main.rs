use rustfft::{num_complex::Complex, FftPlanner};

fn read_wav(filename: &str) -> (wav::Header, Vec<i16>) {
    let mut file = std::fs::File::open("/Users/jschmidt/Downloads/".to_string() + filename)
        .unwrap_or_else(|_| {
            std::fs::File::open("/mnt/c/Users/John/Downloads/".to_string() + filename).unwrap()
        });

    let (header, data) = wav::read(&mut file).unwrap();

    let data = match data {
        wav::BitDepth::Sixteen(v) => v,
        _ => panic!("only supports bit depth of 16"),
    };

    (header, data)
}

fn main() {
    let filename = "EAS_test_tone.wav";

    let (header, data) = read_wav(filename);

    let sampling_rate = header.sampling_rate as usize;
    dbg!(sampling_rate);
    //we are looking for frequencies of approx 2.5khz, so use 2.5khz * 4
    let fft_size = (2500usize * 1).next_power_of_two();
    dbg!(fft_size);
    let sample_size = ((2 * sampling_rate) / fft_size) * fft_size; //sliding window of ~ 2 seconds
    dbg!(sample_size);

    let mut planner = FftPlanner::new();

    let fft = planner.plan_fft_inverse(fft_size);

    // // Real number input
    //let mut fft_input: Vec<_> = data
    //.iter()
    //.skip(start)
    //.take(sample_size)
    //.map(|i| Complex::new(f32::from(*i), 0.0))
    //.collect();
    let start = 0;

    let mut fft_input: Vec<_> = data
        .iter()
        .skip(start)
        .take(sample_size)
        .enumerate()
        .map(|(_, i)| Complex::new(f32::from(*i), 0.0))
        .collect();

    // 0 padding ?
    //fft_input.extend(std::iter::repeat(Complex::new(0.0, 0.0)).take(sample_size));

    fft.process(&mut fft_input);
    //this FFT is symmetrical across x = fft_size / 2, so only care about first half
    fft_input.truncate(fft_size / 2);

    for (i, c) in fft_input.iter_mut().enumerate() {
        //normalize imaginary part
        c.im /= (fft_size as f32).sqrt();

        let hz = (i as f32 * sampling_rate as f32) / fft_size as f32;
        //transform real part to frequency domain
        c.re = hz;
    }

    plot(&fft_input, sampling_rate, filename).expect("failed to plot");

    fft_input.sort_by(|a, b| b.im.partial_cmp(&a.im).unwrap());
    for (i, c) in fft_input.iter().enumerate().take(5) {
        println!("#{}: {}hz ({})", i + 1, c.re, c.im);
    }
}

fn plot(
    fft_input: &[Complex<f32>],
    sampling_rate: usize,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use plotters::prelude::*;

    dbg!("start plot");

    let scale = 2;

    let drawing_area =
        BitMapBackend::new("images/0.1.png", (600 * scale, 400 * scale)).into_drawing_area();

    drawing_area.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&drawing_area)
        .margin(5 * scale)
        .caption(filename, ("sans-serif", 30 * scale).into_font())
        .x_label_area_size(30 * scale)
        .y_label_area_size(30 * scale)
        .build_cartesian_2d(0..(sampling_rate / 2) as i32, 0..100_000)?;

    chart.configure_mesh().draw()?;

    chart.draw_series(LineSeries::new(
        fft_input.iter().enumerate().map(|(i, c)| {
            (
                (i * sampling_rate / fft_input.len() / 2) as i32,
                c.im as i32,
            )
        }),
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
