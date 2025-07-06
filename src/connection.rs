use std::{
    fs::OpenOptions,
    io::Write,
    sync::Arc,
    time::{Duration, Instant},
};

use rubato::{
    FastFixedIn, FftFixedInOut, Resampler, SincFixedIn, SincInterpolationParameters,
    SincInterpolationType,
};
use tokio::sync::Notify;
use webrtc::{
    api::{
        APIBuilder,
        interceptor_registry::register_default_interceptors,
        media_engine::{MIME_TYPE_OPUS, MediaEngine},
    },
    ice_transport::ice_connection_state::RTCIceConnectionState,
    interceptor::registry::Registry,
    peer_connection::{
        RTCPeerConnection, configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
    },
    rtp_transceiver::{rtp_codec::RTCRtpCodecCapability, rtp_sender::RTCRtpSender},
    track::track_local::track_local_static_sample::TrackLocalStaticSample,
};

pub struct Connection {
    peer_connection: Arc<RTCPeerConnection>,
    audio_track: Arc<TrackLocalStaticSample>,
    connected_notify: Arc<Notify>,
    rtc_sender: Arc<RTCRtpSender>,
}
impl Connection {
    pub async fn new(rtc_config: RTCConfiguration) -> anyhow::Result<Self> {
        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs()?;
        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media_engine)?;

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();

        let peer_connection = Arc::new(api.new_peer_connection(rtc_config).await?);

        let audio_track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                ..Default::default()
            },
            "audio".to_owned(),
            "test".to_owned(),
        ));
        let rtc_sender = peer_connection.add_track(audio_track.clone()).await?;
        let connected_notify = Arc::new(Notify::new());
        {
            let connected_notify = connected_notify.clone();
            peer_connection.on_ice_connection_state_change(Box::new(
                move |connection_state: RTCIceConnectionState| {
                    println!("ICE changed {}", &connection_state);
                    if connection_state == RTCIceConnectionState::Connected {
                        connected_notify.notify_waiters();
                    }
                    Box::pin(async {})
                },
            ));
            peer_connection.on_peer_connection_state_change(Box::new(
                move |s: RTCPeerConnectionState| {
                    println!("Peer Connection State has changed: {s}");

                    Box::pin(async {})
                },
            ));
        }
        Ok(Self {
            peer_connection,
            audio_track,
            connected_notify: Arc::new(Notify::new()),
            rtc_sender,
        })
    }

    pub async fn set_remote_description(
        &self,
        desc: RTCSessionDescription,
    ) -> Result<(), webrtc::Error> {
        self.peer_connection.set_remote_description(desc).await
    }
    pub async fn create_answer(&self) -> anyhow::Result<()> {
        let answer = self.peer_connection.create_answer(None).await?;

        let mut gather_complete = self.peer_connection.gathering_complete_promise().await;

        self.peer_connection.set_local_description(answer).await?;

        let _ = gather_complete.recv().await;
        Ok(())
    }
    pub async fn get_local_desc(&self) -> Option<RTCSessionDescription> {
        self.peer_connection.local_description().await
    }
    pub fn get_sender(&self) -> Arc<RTCRtpSender> {
        self.rtc_sender.clone()
    }
    pub async fn close(self) -> Result<(), webrtc::Error> {
        self.peer_connection.close().await
    }
    pub fn is_connected(&self) -> bool {
        self.peer_connection.connection_state() == RTCPeerConnectionState::Connected
    }
    pub fn connection_state(&self) -> RTCPeerConnectionState {
        self.peer_connection.connection_state()
    }
    pub fn start(
        &self,
        receiver: tokio::sync::broadcast::Receiver<Vec<f32>>,
        config: cpal::SupportedStreamConfig,
    ) -> anyhow::Result<()> {
        dbg!(&config, config.sample_rate());
        let mut encoder =
            opus::Encoder::new(48000, opus::Channels::Stereo, opus::Application::Audio)?;

        let track = self.audio_track.clone();
        let mut r = receiver;
        dbg!(&r);

        // let spec = hound::WavSpec {
        //     channels: config.channels(),
        //     sample_rate: 48000,
        //     bits_per_sample: 32,
        //     sample_format: hound::SampleFormat::Float,
        // };
        // dbg!(&config);
        // let mut writer_r = hound::WavWriter::create("resampled.wav", spec).unwrap();
        // let spec = hound::WavSpec {
        //     channels: config.channels(),
        //     sample_rate: config.sample_rate().0,
        //     bits_per_sample: 32,
        //     sample_format: hound::SampleFormat::Float,
        // };
        // let mut writer_o = hound::WavWriter::create("original.wav", spec).unwrap();

        let mut resampler = if config.sample_rate().0 == 48_000 {
            None
        } else {
            Some(crate::resampler::Resampler::new(
                config.sample_rate().0 as usize,
                48_000,
                config.channels() as usize,
            )?)
        };

        let i = Instant::now();

        let samples_per_ms = 48000 / 1000;
        let samples_per_segment = samples_per_ms * 10;
        let total_values = samples_per_segment * 2;

        let mut frame_size = None;

        tokio::spawn(async move {
            let mut left = Vec::new();
            while let Ok(v) = r.recv().await {
                // for a in v.iter() {
                //     use cpal::Sample;
                //     let sample = f32::from_sample(*a);
                //     writer_o.write_sample(sample).unwrap();
                // }
                let pcm = if let Some(resampler) = resampler.as_mut() {
                    resampler.process(&v)
                } else {
                    v
                };
                left.extend_from_slice(&pcm);

                if frame_size.is_none() {
                    let l = left.len();
                    let options = vec![960, 1920, 2880];
                    let full_chunks = options.iter().map(|v| l / v);
                    let m = full_chunks
                        .filter(|v| *v > 0)
                        .enumerate()
                        .min_by(|(_, v), (_, v2)| v.cmp(v2))
                        .unwrap_or((0, 0));
                    // let reminders = options.iter().map(|v| l % v);
                    let v = match m.0 {
                        0 => 960,
                        1 => 1920,
                        2 => 2880,
                        _ => 960,
                    };
                    frame_size = Some(v);
                }

                let frame_size = frame_size.unwrap();
                let full_chunks = left.len() / frame_size;
                let remainder = left.len() % frame_size;

                let mut frames = Vec::new();

                for chunk in 0..full_chunks {
                    let buffered_pcm = &left[chunk * frame_size..(chunk + 1) * frame_size];
                    let frame = encoder
                        .encode_vec_float(&buffered_pcm, buffered_pcm.len() * 2)
                        .unwrap();
                    frames.push(frame);
                }
                if remainder == 0 {
                    left.clear();
                } else {
                    left.copy_within(full_chunks * frame_size.., 0);
                    left.truncate(remainder);
                    println!("Leftover data {} mode: {}", left.len(), frame_size);
                }
                let l = frame_size / 2 / 48; // 48kHz
                for frame in frames.into_iter() {
                    track
                        .write_sample(&webrtc::media::Sample {
                            data: frame.into(),
                            duration: Duration::from_millis(l as u64),
                            ..Default::default()
                        })
                        .await
                        .unwrap();
                }
                // for a in resampled_pcm.iter() {
                //     use cpal::Sample;
                //     let sample = f32::from_sample(*a);
                //     writer_r.write_sample(sample).unwrap();
                // }
            }
        });
        Ok(())
    }
}
