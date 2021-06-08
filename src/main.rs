use anyhow::{bail, Result};
use gst::{prelude::Cast, GstBinExt};
use gst::{prelude::*, ElementExtManual};
use structopt::StructOpt;

const VIDEO_TEST_CAPS: &str = "video/x-raw, width=1280, height=720, format=I420, framerate=30/1";

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(short, long)]
    file: Option<String>,
    #[structopt(short, long, default_value = "rtmp://127.0.0.1/live/stream1")]
    url: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::from_args();

    gst::init()?;

    // Pretty verbose pipeline - too lazy to make every element individually

    let pipeline = if let Some(file_path) = args.file {
        gst::parse_launch(&format!(
                            "filesrc location={file} ! decodebin3 name=dbin ! videoconvert ! x264enc tune=zerolatency bitrate=1000 ! video/x-h264 ! h264parse ! \
                            video/x-h264 ! queue ! flvmux name=mux ! tee name=tee ! queue ! fakesink \
                            dbin. ! audioconvert ! audioresample ! audio/x-raw, rate=48000 ! queue ! \
                            voaacenc bitrate=48000 ! audio/mpeg ! aacparse ! audio/mpeg, mpegversion=4 ! queue ! mux.",
            file = file_path
        ))?
    } else {
        println!("Video file was not supplied. Sending test stream");
        gst::parse_launch(
                            &format!("videotestsrc is-live=true ! {video_caps} ! videoconvert ! x264enc tune=zerolatency bitrate=1000 ! video/x-h264 ! h264parse ! \
                            video/x-h264 ! queue ! flvmux name=mux ! tee name=tee ! queue ! fakesink \
                            audiotestsrc is-live=true wave=ticks ! audioconvert ! audioresample ! audio/x-raw, rate=48000 ! \
                            voaacenc bitrate=48000 ! audio/mpeg ! aacparse ! audio/mpeg, mpegversion=4 ! queue ! mux.", video_caps = VIDEO_TEST_CAPS),
    )?
    };

    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .expect("You're not downcasting a pipeline");

    // To demultiplex the the muxed stream to multiple sources, tee is needed
    // to produce multiple outs from a single in
    let tee = pipeline.get_by_name("tee").unwrap();

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
