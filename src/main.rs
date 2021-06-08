mod gstream;
mod wav_analyze;

fn main() {
    let filename = match std::env::args().nth(1) {
        Some(name) => name,
        None => {
            println!("must call with filename to analyze");
            return;
        }
    };

    if !filename.ends_with("wav") {
        let output_audios = gstream::read_audio_to_wav(&filename).unwrap();

        for data in output_audios {
            wav_analyze::process_wav(&filename, data.0, data.1);
        }
    } else {
        let data = wav_analyze::read_wav(&filename);
        wav_analyze::process_wav(&filename, data.0, data.1);
    }
}

pub fn test_fft() {
    //let filename = "EAS_test_tone.wav";
    let filename = "eas_file.wav";
    //let filename = "/mnt/c/Users/John/Downloads/Required Monthly Test.mp3";

    let (header, data) = if true {
        wav_analyze::read_wav(filename)
    } else {
        wav_analyze::gen_fake_wav()
    };

    wav_analyze::process_wav(&filename, header, data);
}
