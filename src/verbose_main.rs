use anyhow::{bail, Result};
use gst::{prelude::Cast, GstBinExt};
use gst::{prelude::*, ElementExtManual};
use structopt::StructOpt;

const VIDEO_TEST_CAPS: &str = "video/x-raw, width=1280, height=720, format=I420, framerate=30/1";

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(short, long)]
    file: Option<String>,
    #[structopt(short, long, default_value = "rtmp://127.0.0.1/live1/stream1")]
    url: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::from_args();

    gst::init()?;

    // Pretty verbose pipeline - too lazy to make every element individually
    // let pipeline = if let Some(file_path) = args.file {
    //     gst::parse_launch(&format!(
    //                         "filesrc location={file} ! decodebin name=dbin ! videoconvert ! x264enc tune=zerolatency bitrate=1000 ! \
    //                         queue ! flvmux name=mux ! tee name=tee ! queue ! fakesink \
    //                         dbin. ! audioconvert ! audioresample ! voaacenc bitrate=48000 ! queue ! mux.",
    //         file = file_path
    //     ))?
    // } else {
    //     println!("Video file was not supplied. Sending test stream");
    //     gst::parse_launch(
    //                         &format!("videotestsrc is-live=true ! {video_caps} ! videoconvert ! x264enc tune=zerolatency bitrate=1000 ! video/x-h264 ! h264parse ! \
    //                         video/x-h264 ! queue ! flvmux name=mux ! tee name=tee ! queue ! fakesink \
    //                         audiotestsrc is-live=true wave=ticks ! audioconvert ! audioresample ! audio/x-raw, rate=48000 ! \
    //                         voaacenc bitrate=48000 ! audio/mpeg ! aacparse ! audio/mpeg, mpegversion=4 ! queue ! mux.", video_caps = VIDEO_TEST_CAPS),
    // )?
    // };

    // let pipeline = make_pipeline(args.file.unwrap().as_str())?;

    let pipeline = gst::Pipeline::new(None);

    let filesrc = gst::ElementFactory::make("filesrc", Some("video_src"))?;
    filesrc.set_property_from_str("location", args.file.unwrap().as_str());

    let decodebin = gst::ElementFactory::make("decodebin", Some("dbin"))?;

    pipeline.add_many(&[&filesrc, &decodebin])?;
    gst::Element::link_many(&[&filesrc, &decodebin])?;

    // Elements to handle video
    let video_conv = gst::ElementFactory::make("videoconvert", None)?;
    let x264 = gst::ElementFactory::make("x264enc", None)?;
    x264.set_property_from_str("tune", "zerolatency");
    x264.set_property_from_str("bitrate", "1000");
    let queue = gst::ElementFactory::make("queue", None)?;
    let flvmux = gst::ElementFactory::make("flvmux", Some("mux"))?;

    let tee = gst::ElementFactory::make("tee", Some("tee"))?;
    let queue1 = gst::ElementFactory::make("queue", None)?;
    let fakesink = gst::ElementFactory::make("fakesink", None)?;

    // Elements to handle audio
    let audio_conv = gst::ElementFactory::make("audioconvert", None)?;
    let audio_res = gst::ElementFactory::make("audioresample", None)?;
    let voaac = gst::ElementFactory::make("voaacenc", None)?;
    voaac.set_property_from_str("bitrate", "48000");
    let queue2 = gst::ElementFactory::make("queue", None)?;

    pipeline.add_many(&[
        &video_conv,
        &x264,
        &queue,
        &flvmux,
        &tee,
        &queue1,
        &fakesink,
        &audio_conv,
        &audio_res,
        &voaac,
        &queue2,
    ])?;

    gst::Element::link_many(&[
        &video_conv,
        &x264,
        &queue,
        &flvmux,
        &tee,
        &queue1,
        &fakesink,
    ])?;
    gst::Element::link_many(&[&audio_conv, &audio_res, &voaac, &queue2])?;

    let audio_mux_snk = flvmux.get_request_pad("audio").unwrap();
    let audio_src = queue2.get_static_pad("src").unwrap();
    audio_src.link(&audio_mux_snk).unwrap();

    let dbin = pipeline.get_by_name("dbin").expect("Cannot find decodebin");

    dbin.connect_pad_added(move |src, src_pad| {
        println!(
            "Received new pad {} from {}",
            src_pad.get_name(),
            src.get_name()
        );

        let new_pad_caps = src_pad
            .get_current_caps()
            .expect("Failed to get caps of new pad.");
        let new_pad_struct = new_pad_caps
            .get_structure(0)
            .expect("Failed to get first structure of caps.");
        let new_pad_type = new_pad_struct.get_name();

        if new_pad_type.starts_with("audio/x-raw") {
            println!("Received audio pad from dbin");
            let audio_conv_sink = audio_conv.get_static_pad("sink").unwrap();

            src_pad.link(&audio_conv_sink).unwrap();
        } else {
            println!("Received video pad from dbin");
            let video_conv_sink = video_conv.get_static_pad("sink").unwrap();

            src_pad.link(&video_conv_sink).unwrap();
        }
    });

    for rtmp_url in args.url {
        println!("\nPublishing video to : {}\n", rtmp_url);

        // For every rtmp endpoint supplied, create a copy of the muxed stream
        let tee_src = tee.get_request_pad("src_%u").unwrap();

        // queue - to keep the pipeline from jamming up
        let queue = gst::ElementFactory::make("queue", None)?;
        let rtmpsink = gst::ElementFactory::make("rtmp2sink", None)?;

        // set the supplied rtmp url as the publishing point for the rtmpsink
        rtmpsink.set_property_from_str("location", &rtmp_url.as_str());
        let queue_sink = queue.get_static_pad("sink").unwrap();

        // Add and link the newly created elements
        pipeline.add_many(&[&queue, &rtmpsink])?;
        gst::Element::link_many(&[&queue, &rtmpsink])?;

        // Link copy of the muxed stream to the rtmp sink elements
        tee_src.link(&queue_sink)?;
    }

    // Very basic setup

    pipeline
        .set_state(gst::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");

    let bus = pipeline.get_bus().expect("Unable to get bus from pipeline");

    for msg in bus.iter_timed(gst::CLOCK_TIME_NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Error(err) => bail!(
                "Error from element {}: {} ({})",
                err.get_src()
                    .map(|s| String::from(s.get_path_string()))
                    .unwrap_or_else(|| String::from("None")),
                err.get_error(),
                err.get_debug().unwrap_or_else(|| String::from("None"))
            ),
            MessageView::Warning(warning) => {
                println!("Warning: \"{}\"", warning.get_debug().unwrap());
            }
            _ => (),
        }
    }

    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");

    Ok(())
}
