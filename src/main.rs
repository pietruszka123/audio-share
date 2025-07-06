pub mod app;
pub mod connection;
pub mod resampler;
pub mod ui;
use std::{
    fs::OpenOptions,
    io::Write,
    sync::{Arc, mpsc::channel},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use base64::{
    Engine,
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE},
};
use cpal::{
    Device, InputCallbackInfo,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossterm::event::{KeyCode, KeyEvent};
use hound;
use ratatui::{layout::Rect, widgets::Widget};
use tokio::sync::Notify;
use webrtc::{
    api::{
        APIBuilder,
        interceptor_registry::register_default_interceptors,
        media_engine::{MIME_TYPE_OPUS, MIME_TYPE_PCMA, MediaEngine},
    },
    ice_transport::ice_connection_state::RTCIceConnectionState,
    interceptor::registry::Registry,
    media::Sample,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
    },
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::track_local_static_sample::TrackLocalStaticSample,
};

use crate::app::App;

pub fn create_stream(
    device: &Device,
) -> anyhow::Result<(
    cpal::Stream,
    tokio::sync::broadcast::Receiver<Vec<f32>>,
    tokio::sync::broadcast::Sender<Vec<f32>>,
)> {
    let (send, recv) = tokio::sync::broadcast::channel(30);

    let sender = send.clone();

    let config = device.default_output_config()?;
    let stream = device.build_input_stream(
        &config.config(),
        move |data: &[f32], _: &InputCallbackInfo| {
            // react to stream events and read or write stream data here.
            // dbg!(send.receiver_count());
            send.send(data.to_vec()).unwrap();
        },
        move |err| {
            // react to errors here.
            panic!("{}", err);
        },
        None,
    )?;

    return Ok((stream, recv, sender));
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut app_result = App::default();
    app_result.scan_devices()?;
    let mut terminal = ratatui::init();
    app_result.run(&mut terminal).await?;
    ratatui::restore();

    // return result;
    // let host = cpal::default_host();
    // let device = host
    //     .default_output_device()
    //     .ok_or(anyhow!("No default device"))?;

    // dbg!(&device.name());
    // let config = device.default_output_config()?;

    // let (stream, mut recv, sender) = create_stream(&device)?;

    // let mut m = MediaEngine::default();
    // m.register_default_codecs();

    // let mut registry = Registry::new();

    // // Use the default set of Interceptors
    // registry = register_default_interceptors(registry, &mut m)?;

    // let api = APIBuilder::new()
    //     .with_media_engine(m)
    //     .with_interceptor_registry(registry)
    //     .build();
    // let rtc_config = RTCConfiguration {
    //     ice_servers: vec![
    //         // RTCIceServer {
    //         // urls: vec!["stun:stun.l.google.com:19302".to_owned()],
    //     //     ..Default::default()
    //     // }
    //     ],
    //     ..Default::default()
    // };
    // // Create a new RTCPeerConnection
    // let peer_connection = Arc::new(api.new_peer_connection(rtc_config).await?);

    // let n = Arc::new(Notify::new());
    // let c = n.clone();

    // peer_connection.on_ice_connection_state_change(Box::new(
    //     move |connection_state: RTCIceConnectionState| {
    //         if connection_state == RTCIceConnectionState::Connected {
    //             c.notify_waiters();
    //             stream.play().unwrap();
    //         }
    //         Box::pin(async {})
    //     },
    // ));
    // peer_connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
    //     println!("Peer Connection State has changed: {s}");

    //     Box::pin(async {})
    // }));

    // let track = Arc::new(TrackLocalStaticSample::new(
    //     RTCRtpCodecCapability {
    //         mime_type: MIME_TYPE_OPUS.to_owned(),
    //         ..Default::default()
    //     },
    //     "audio".to_owned(),
    //     "test".to_owned(),
    // ));
    // let rtp_sender = peer_connection.add_track(track.clone()).await?;

    // let mut r = sender.subscribe();
    // let mut encoder = opus::Encoder::new(
    //     config.sample_rate().0,
    //     opus::Channels::Stereo,
    //     opus::Application::Audio,
    // )?;
    // let c = config.clone();
    // tokio::spawn(async move {
    //     n.notified().await;
    //     dbg!("play");

    //     // let buf = Vec::new();
    //     let mut i = 0;
    //     while let Ok(v) = r.recv().await {
    //         let l = (v.len() * 1000) / c.sample_rate().0 as usize / 2;
    //         let frame = encoder.encode_vec_float(&v, v.len()).unwrap();
    //         println!("Processed frame {}", l);

    //         // tokio::time::sleep(Duration::from_millis(10)).await;
    //         // dbg!(i);
    //         // i += 1;

    //         track
    //             .write_sample(&Sample {
    //                 data: frame.into(),
    //                 duration: Duration::from_millis(l as u64),
    //                 ..Default::default()
    //             })
    //             .await
    //             .unwrap();
    //     }
    // });

    // tokio::spawn(async move {
    //     let mut rtcp_buf = vec![0u8; 1500];
    //     while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
    //     anyhow::Result::<()>::Ok(())
    // });

    // let mut b = String::new();
    // std::io::stdin().read_line(&mut b)?;
    // dbg!(&b);
    // let d = BASE64_URL_SAFE.decode(b.trim())?;
    // dbg!(&d);

    // let offer = serde_json::from_slice::<RTCSessionDescription>(&d)?;

    // peer_connection.set_remote_description(offer).await?;

    // let answer = peer_connection.create_answer(None).await?;

    // let mut gather_complete = peer_connection.gathering_complete_promise().await;

    // peer_connection.set_local_description(answer).await?;

    // let _ = gather_complete.recv().await;

    // if let Some(local_desc) = peer_connection.local_description().await {
    //     let json_str = serde_json::to_string(&local_desc)?;

    //     let mut b64 = String::new();
    //     base64::prelude::BASE64_STANDARD.encode_string(&json_str, &mut b64);
    //     println!("{b64}");
    // } else {
    //     println!("generate local_description failed!");
    // }

    // let spec = hound::WavSpec {
    //     channels: config.channels(),
    //     sample_rate: config.sample_rate().0,
    //     bits_per_sample: 32,
    //     sample_format: hound::SampleFormat::Float,
    // };
    // dbg!(&config);
    // let mut writer = hound::WavWriter::create("test.wav", spec).unwrap();

    // // let start = Instant::now();

    // let stop = Arc::new(Notify::new());

    // let s = stop.clone();
    // let record = async move {
    //     loop {
    //         tokio::select! {
    //             v = recv.recv() =>{
    //                 if let Ok(v) = v{
    //                      for a in v.iter() {
    //                 use cpal::Sample;
    //                 let sample = f32::from_sample(*a);
    //                 writer.write_sample(sample).unwrap();
    //             }
    //             // let t = Instant::now().duration_since(start);
    //             // dbg!(&t);
    //                 }
    //             },
    //             _ = s.notified() =>{
    //                 break;
    //             }
    //         }
    //     }
    //     writer.finalize().unwrap();
    // };

    // tokio::select! {
    //     _ = record => {
    //         println!("received done signal!");
    //     }
    //     _ = tokio::signal::ctrl_c() => {
    //         println!();
    //     }
    // };
    // stop.notify_waiters();
    // peer_connection.close().await?;

    // println!("Hello, world!");
    Ok(())
}
