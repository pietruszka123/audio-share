use std::{
    fs::OpenOptions,
    io::Write,
    sync::{Arc, mpsc::channel},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use cpal::{
    Device, InputCallbackInfo,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use hound;
use webrtc::{
    api::{
        APIBuilder,
        interceptor_registry::register_default_interceptors,
        media_engine::{MIME_TYPE_OPUS, MIME_TYPE_PCMA, MediaEngine},
    },
    interceptor::registry::Registry,
    peer_connection::configuration::RTCConfiguration,
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::track_local_static_sample::TrackLocalStaticSample,
};

fn create_stream(
    device: &Device,
) -> anyhow::Result<(cpal::Stream, std::sync::mpsc::Receiver<Vec<f32>>)> {
    let (send, recv) = channel();

    let config = device.default_output_config()?;
    let stream = device.build_input_stream(
        &config.config(),
        move |data: &[f32], _: &InputCallbackInfo| {
            // react to stream events and read or write stream data here.
            send.send(data.to_vec()).unwrap();
        },
        move |err| {
            // react to errors here.
            panic!("{}", err);
        },
        None,
    )?;
    stream.play()?;
    return Ok((stream, recv));
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(anyhow!("No default device"))?;

    dbg!(&device.name());
    let config = device.default_output_config()?;

    let (stream, recv) = create_stream(&device)?;

    let m = MediaEngine::default();
    m.register_default_codecs();

    let mut registry = Registry::new();

    // Use the default set of Interceptors
    registry = register_default_interceptors(registry, &mut m)?;

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();
    let rtc_config = RTCConfiguration {
        ice_servers: vec![
            // RTCIceServer {
            // urls: vec!["stun:stun.l.google.com:19302".to_owned()],
        //     ..Default::default()
        // }
        ],
        ..Default::default()
    };

    // Create a new RTCPeerConnection
    let peer_connection = Arc::new(api.new_peer_connection(rtc_config).await?);

    let track = TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_OPUS.to_owned(),
            ..Default::default()
        },
        "audio".to_owned(),
        "test".to_owned(),
    );

    let rtp_send = peer_connection.add_track(track).await?;
    tokio::spawn(async move {
        let mut rtcp_buf = vec![0u8; 1500];
        while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
        Result::<()>::Ok(())
    });

    let spec = hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    dbg!(&config);
    let mut writer = hound::WavWriter::create("test.wav", spec).unwrap();

    let start = Instant::now();
    while let Ok(v) = recv.recv() {
        for a in v.iter() {
            use cpal::Sample;
            let sample = f32::from_sample(*a);
            writer.write_sample(sample).unwrap();
        }
        let t = Instant::now().duration_since(start);
        dbg!(&t);
        if t >= Duration::from_secs(10) {
            break;
        }
    }
    writer.finalize().unwrap();

    println!("Hello, world!");
    Ok(())
}
