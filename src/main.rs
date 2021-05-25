use rustfft::{num_complex::Complex, FftPlanner};

fn main() {
    let filename = "EAS_test_tone.wav";
    let mut file = std::fs::File::open("/Users/jschmidt/Downloads/".to_string() + filename)
        .unwrap_or_else(|_| {
            std::fs::File::open("/mnt/c/Users/John/Downloads/".to_string() + filename).unwrap()
        });

    let (header, data) = wav::read(&mut file).unwrap();

    let data = match data {
        wav::BitDepth::Sixteen(v) => v,
        _ => panic!("only supports bit depth of 16"),
    };

    let sampling_rate = header.sampling_rate as usize;
    dbg!(sampling_rate);
    let fft_size = sampling_rate.next_power_of_two(); //todo find nearest point of 2^a * 3^b
    dbg!(fft_size);
    let sample_size = fft_size * 6;
    dbg!(sample_size);

    let start = sampling_rate / 4; //start .25 seconds in for this clip

    let mut planner = FftPlanner::new();

    let fft = planner.plan_fft_inverse(fft_size);

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
        .map(|(_, i)| Complex::new(f32::from(*i), 0.0))
        .collect();

    //dbg!(&fft_input[0..20]);
    fft.process(&mut fft_input);
    fft_input.truncate(fft_size / 2);
    dbg!(fft_input[0]);
    for (i, c) in fft_input.iter_mut().enumerate() {
        c.re /= (fft_size as f32).sqrt();
        c.im /= (fft_size as f32).sqrt();

        if c.im > 30000.0 {
            println!("{}: {}\t{}", i, c.re, c.im);
            let hz = (i * sampling_rate) / fft_size;
            println!("{}", hz);
        }
    }
    //dbg!(&fft_input[0..20]);

    use plotters::prelude::*;

    let drawing_area = BitMapBackend::new("images/0.1.png", (600, 400)).into_drawing_area();

    drawing_area.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&drawing_area)
        .margin(5)
        .caption(filename, ("sans-serif", 30).into_font())
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0..(sampling_rate / 2) as i32, 0..100_000)
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    chart
        .draw_series(LineSeries::new(
            fft_input
                .iter()
                .enumerate()
                .map(|(i, c)| ((i * sampling_rate / fft_size) as i32, c.im as i32)),
            &BLACK,
        ))
        .unwrap()
        .label("frequency");

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()
        .unwrap();
}
