use rustfft::{num_complex::Complex, FftPlanner};

fn main() {
    let mut file = std::fs::File::open("/Users/jschmidt/Downloads/EAS_test_tone.wav").unwrap();

    let (header, data) = wav::read(&mut file).unwrap();

    let data = match data {
        wav::BitDepth::Sixteen(v) => v,
        _ => panic!("only supports bit depth of 16"),
    };

    let found_sampling_rate = header.sampling_rate as usize;
    dbg!(found_sampling_rate);
    let sample_size = found_sampling_rate.next_power_of_two(); //todo find nearest point of 2^a * 3^b
    dbg!(sample_size);

    let start = found_sampling_rate / 4; //start .25 seconds in for this clip

    let mut planner = FftPlanner::new();

    let fft = planner.plan_fft_inverse(sample_size);

    // // Real number input
    //let mut fft_input: Vec<_> = data
    //.iter()
    //.skip(start)
    //.take(sample_size)
    //.map(|i| Complex::new(f32::from(*i), 0.0))
    //.collect();

    let mut fft_input: Vec<_> = data
        .iter()
        .skip(start)
        .take(sample_size)
        .enumerate()
        .map(|(sample, &i)| {
            Complex::new(
                (f64::from(sample as u32) / f64::from(found_sampling_rate as u32)) as f32,
                f32::from(i),
            )
        })
        .collect();

    //dbg!(&fft_input[0..20]);
    fft.process(&mut fft_input);
    for c in fft_input.iter_mut() {
        c.re /= (sample_size as f32).sqrt();
        c.im /= (sample_size as f32).sqrt();
    }
    //dbg!(&fft_input[0..20]);
    println!("plotting");

    use plotters::prelude::*;

    let drawing_area = BitMapBackend::new("images/0.1.png", (600, 400)).into_drawing_area();

    drawing_area.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&drawing_area)
        .build_cartesian_2d(0..100, 0..10000)
        .unwrap();

    chart
        .draw_series(LineSeries::new(
            fft_input.iter().map(|i| (i.re as i32, i.im as i32)),
            &BLACK,
        ))
        .unwrap();
}
