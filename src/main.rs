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

fn main() {
    //let filename = "EAS_test_tone.wav";
    let filename = "eas_file.wav";
    //let filename = "/mnt/c/Users/John/Downloads/Required Monthly Test.mp3";

    let (header, data) = read_wav(filename);
    println!("done reading wav");

    let sampling_rate = header.sampling_rate as usize;
    dbg!(sampling_rate);
    //we are looking for frequencies of approx 2.5khz, so use 2.5khz * 2
    let fft_size = (2500usize * 2).next_power_of_two();
    dbg!(fft_size);
    let sample_size = ((sampling_rate) / fft_size) * fft_size; //sliding window of ~1 second
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

    for start in (0..(data.len() - sample_size)).step_by(sample_size / 3) {
        let mut fft_input: Vec<_> = data
            .iter()
            .skip(start)
            .take(sample_size)
            .enumerate()
            .map(|(_, i)| Complex::new(f32::from(*i), 0.0))
            .collect();

        let start_secs = start as f32 / sampling_rate as f32 / 2.0;
        let end_secs = (start + sample_size) as f32 / sampling_rate as f32 / 2.0;

        // 0 padding ?
        //fft_input.extend(std::iter::repeat(Complex::new(0.0, 0.0)).take(sample_size));

        fft.process(&mut fft_input);
        //this FFT is symmetrical across x = fft_size / 2, so only care about first half
        fft_input.truncate(fft_size / 2);

        for (i, c) in fft_input.iter_mut().enumerate() {
            //normalize imaginary part
            c.im /= (fft_size as f32).sqrt();

            let hz = 2.0 * (i as f32 * sampling_rate as f32) / fft_size as f32;
            //transform real part to frequency domain
            c.re = hz;
        }

        //plot(&fft_input, sampling_rate, filename).expect("failed to plot");

        fft_input.sort_by(|a, b| b.im.partial_cmp(&a.im).unwrap());
        let median_amplitude75 = fft_input[fft_input.len() / 4].im;

        let mut has_2083 = false;
        let mut has_1562 = false;
        let mut has_1000 = false;
        let mut has_853 = false;
        let mut has_960 = false;

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
            if (c.re - 1000.0).abs() < 5.0 {
                has_1000 = true;
            }
        }
        let has_eas_low = has_960 && has_853;
        let has_eas_high = has_2083 && has_1562;
        if has_eas_low || has_eas_high {
            println!(
                "#==============#\n{} - {}\n~{}:{}",
                start_secs,
                end_secs,
                //median_amplitude75,
                start_secs as usize / 60,
                start_secs as usize % 60,
            );
            //for (i, c) in fft_input.iter().enumerate().take(5) {
            //println!("#{}: {}hz ({})", i + 1, c.re, c.im);
            //}
            println!(
                "EAS (2khz): {}, EAS: (850hz): {}",
                has_eas_high, has_eas_low,
            );
        }
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
