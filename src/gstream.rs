use std::sync::atomic::{AtomicBool, Ordering};

use gstreamer::{prelude::*, ClockTime, ElementFactory, MessageType, MessageView, Pipeline, State};

//can't use path because ToValue path isnt implemented
pub fn read_audio_to_wav(path: &str) -> Result<Vec<(wav::Header, crate::wav_analyze::RawWav)>, ()> {
    gstreamer::init().unwrap();

    //let msg = bus.timed_pop_filtered(ClockTime::none(), &[MessageType::Error, MessageType::Eos]);

    //println!("Message from gstreamer: {:?}", msg);

    let infile = ElementFactory::make("filesrc", None).unwrap();
    let source = ElementFactory::make("decodebin", None).unwrap();

    let convert = ElementFactory::make("audioconvert", None).unwrap();
    let resample = ElementFactory::make("audioresample", None).unwrap();
    let wavenc = ElementFactory::make("wavenc", None).unwrap();
    let out = ElementFactory::make("filesink", None).unwrap();

    infile.set_property("location", &path).unwrap();

    let pipeline = Pipeline::new(Some("test pipeline"));

    pipeline
        .add_many(&[&infile, &source, &resample, &convert, &wavenc, &out])
        .unwrap();

    infile.link(&source).unwrap();
    //parse.link(&source).unwrap();

    convert.link(&resample).unwrap();
    resample.link(&wavenc).unwrap();
    wavenc.link(&out).unwrap();

    out.set_property("location", &"./test.wav").unwrap();

    source.connect_pad_added(move |_, pad| {
        let sink_pads = convert.get_sink_pads();

        let first_pad = sink_pads
            .iter()
            .next()
            .expect("convert has no sync pads ????");

        if first_pad.is_linked() {
            println!("already linked");
            return;
        }

        let new_pad_caps = pad.get_current_caps().unwrap();
        let new_pad_struct = new_pad_caps.get_structure(0).unwrap();
        let new_pad_type = new_pad_struct.get_name();
        if !new_pad_type.starts_with("audio/x-raw") {
            return;
        }

        println!("Found output channel:");
        for cap in new_pad_caps.iter() {
            dbg!(cap);
        }

        pad.link(first_pad).unwrap();
    });

    match pipeline.set_state(State::Playing) {
        Ok(_) => {}
        Err(_) => {
            println!("Unable to set the pipeline to the playing state.");
            return Err(());
        }
    }

    let bus = pipeline.get_bus().unwrap();

    let is_transcoding = AtomicBool::new(true);
    crossbeam::thread::scope(|s| {
        s.spawn(|_| {
            let time = std::time::Duration::from_secs(2);
            while is_transcoding.load(Ordering::Relaxed) {
                let d: Option<ClockTime> = pipeline.query_position();

                match d {
                    Some(time) => {
                        let dur: ClockTime = pipeline.query_duration().unwrap();
                        println!("Current time {} / {}     \r", time, dur);
                    }
                    None => {}
                };
                std::thread::sleep(time);
            }
        });

        for msg in bus.iter_timed_filtered(
            ClockTime::none(),
            &[
                MessageType::StateChanged,
                MessageType::Error,
                MessageType::Eos,
            ],
        ) {
            match msg.view() {
                MessageView::Eos(eos) => {
                    println!("done reading {:?}", eos);
                    is_transcoding.store(false, Ordering::Relaxed);
                    break;
                }
                MessageView::Error(e) => {
                    println!("Some kind of error {:?}", e);
                    is_transcoding.store(false, Ordering::Relaxed);
                    return Err(());
                }
                MessageView::StateChanged(_) => {
                    //println!("State change {:?}", sc);
                }

                _ => {}
            }
        }

        Ok(())
    })
    .unwrap()?;

    pipeline.set_state(State::Null).unwrap();

    let data = crate::wav_analyze::read_wav("./test.wav");

    Ok(vec![data])
}
