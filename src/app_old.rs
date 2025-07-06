use std::fs::OpenOptions;
use std::io::{Read, Write};

use base64::Engine;
use base64::prelude::BASE64_URL_SAFE;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, Stream};
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind};
use futures::{FutureExt, StreamExt};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    text::Line,
    widgets::ListState,
};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
pub struct KeyInfo {
    info: String,
    key: KeyCode,
}
impl KeyInfo {
    pub fn new(info: &str, key: KeyCode) -> Self {
        Self {
            info: info.to_string(),
            key,
        }
    }
}

pub struct Device {
    pub cpal_device: cpal::Device,
    pub name: Option<String>,
}
impl From<cpal::Device> for Device {
    fn from(value: cpal::Device) -> Self {
        let name = if let Ok(name) = value.name() {
            Some(name)
        } else {
            None
        };
        Self {
            cpal_device: value,
            name: name,
        }
    }
}

pub fn instructions<'a>(keys: &Vec<KeyInfo>) -> Line<'a> {
    let mut res = Vec::new();
    for (i, key) in keys.iter().enumerate() {
        res.push((" ".to_string() + key.info.as_str() + " ").into());
        let mut key = "<".to_string() + &key.key.to_string() + ">";
        if i == keys.len() - 1 {
            key += " ";
        }
        let key = key.blue().bold();
        res.push(key);
    }
    Line::from(res)
}

#[derive(Debug, PartialEq)]
pub enum Selected {
    None,
    Left,
    Right,
    Popup,
}
impl Default for Selected {
    fn default() -> Self {
        Selected::None
    }
}

use crate::connection::Connection;
use crate::ui::{draw_left_panel, draw_popup, draw_right_panel};
#[derive(Default)]
pub struct AppOld {
    pub exit: bool,
    devices: Vec<Device>,
    pub selected_device: usize,
    connection: Option<Connection>,
    pub list_state: ListState,
    state: Selected,

    pub local_desc: String,

    event_stream: EventStream,
    connection_status: String,

    stream: Option<Stream>,
}
impl AppOld {
    pub fn scan_devices(&mut self) -> anyhow::Result<()> {
        let host = cpal::Host::default();
        let default_device = host.default_output_device();
        for (i, device) in host.devices()?.enumerate() {
            if let Some(d) = &default_device {
                if d.name() == device.name() {
                    self.selected_device = i;
                }
            }
            self.devices.push(device.clone().into());
        }
        self.list_state.select(Some(self.selected_device));
        Ok(())
    }
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_crossterm_events().await?;
        }
        Ok(())
    }
    pub fn is_connected(&self) -> bool {
        if let Some(c) = &self.connection {
            return c.is_connected();
        }
        false
    }
    pub fn connection_state(&self) -> Option<RTCPeerConnectionState> {
        if let Some(c) = &self.connection {
            return Some(c.connection_state());
        }
        None
    }
    pub fn state(&self) -> &Selected {
        &self.state
    }
    pub fn devices(&self) -> &Vec<Device> {
        &self.devices
    }

    fn draw(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(frame.area());
        draw_left_panel(self, frame, &layout);
        draw_right_panel(self, frame, &layout);
        if self.state == Selected::Popup {
            draw_popup(self, frame);
        }
    }

    async fn handle_crossterm_events(&mut self) -> anyhow::Result<()> {
        tokio::select! {
               event = self.event_stream.next().fuse() =>{
                if let Some(Ok(event)) = event{
                    match event {
                        // it's important to check that the event is a key press event as
                        // crossterm also emits key release and repeat events on Windows.
                        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                            self.handle_key_event(key_event).await
                        }
                        Event::Paste(content) =>{
                            self.handle_paste_event(content).await
                        }
                        _ => {}
                    };
                }
            },
             _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
            // Sleep for a short duration to avoid busy waiting.
        }
        }
        Ok(())
    }
    async fn handle_paste_event(&mut self, content: String) {
        self.connection_status = "Paste".to_string();
        if let Some(conn) = &self.connection {
            let d = BASE64_URL_SAFE.decode(content.trim()).unwrap();

            let offer = serde_json::from_slice::<RTCSessionDescription>(&d).unwrap();
            conn.set_remote_description(offer).await.unwrap();
            conn.create_answer().await.unwrap();
            let a = conn.get_local_desc().await.unwrap();
            let json_str = serde_json::to_string(&a).unwrap();

            let mut b64 = String::new();
            base64::prelude::BASE64_STANDARD.encode_string(&json_str, &mut b64);
            self.local_desc = b64;
        }
    }
    async fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Up | KeyCode::Char('w') => match self.state {
                Selected::None => todo!(),
                Selected::Left => {
                    self.list_state.select_previous();
                }
                Selected::Right => {}
                Selected::Popup => todo!(),
            },
            KeyCode::Down | KeyCode::Char('s') => match self.state {
                Selected::None => todo!(),
                Selected::Left => {
                    self.list_state.select_next();
                }
                Selected::Right => {}
                Selected::Popup => todo!(),
            },
            KeyCode::Right | KeyCode::Char('d') => match self.state {
                _ => self.state = Selected::Right,
            },
            KeyCode::Left | KeyCode::Char('a') => match self.state {
                _ => self.state = Selected::Left,
            },
            KeyCode::Enter => match self.state {
                Selected::Left => self.selected_device = self.list_state.selected().unwrap(),
                Selected::Right => {
                    self.connection = Some(
                        Connection::new(RTCConfiguration {
                            ice_servers: vec![RTCIceServer {
                                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                                ..Default::default()
                            }],
                            ..Default::default()
                        })
                        .await
                        .unwrap(),
                    );
                    self.connection_status = "Started".to_string();
                }
                _ => {}
            },
            KeyCode::Char('e') => match self.state {
                Selected::Right => {
                    self.connection_status = "Paste".to_string();
                    if let Some(conn) = &self.connection {
                        let mut content = String::new();
                        OpenOptions::new()
                            .read(true)
                            .open("./desc.txt")
                            .unwrap()
                            .read_to_string(&mut content)
                            .unwrap();
                        let d = BASE64_URL_SAFE.decode(content.trim()).unwrap();

                        let offer = serde_json::from_slice::<RTCSessionDescription>(&d).unwrap();
                        conn.set_remote_description(offer).await.unwrap();
                        conn.create_answer().await.unwrap();
                        let a = conn.get_local_desc().await.unwrap();
                        let json_str = serde_json::to_string(&a).unwrap();

                        let mut b64 = String::new();
                        base64::prelude::BASE64_STANDARD.encode_string(&json_str, &mut b64);
                        self.local_desc = b64.clone();
                        let mut f = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .open("./desc1.txt")
                            .unwrap();
                        f.write_all(b64.as_bytes()).unwrap();

                        let device = &self.devices[self.selected_device].cpal_device;
                        let (stream, receiver, _sender) = crate::create_stream(&device).unwrap();
                        self.stream = Some(stream);
                        self.stream.as_ref().unwrap().play().unwrap();
                        let config = device.default_output_config().unwrap();
                        // let config =
                        //     device
                        //         .supported_output_configs()
                        //         .unwrap()
                        //         .into_iter()
                        //         .find(|v| {
                        //             v.try_with_sample_rate(SampleRate(48000)).is_some()
                        //                 && v.channels() == 2
                        //                 && v.sample_format() == SampleFormat::F32
                        //         });
                        // let config = config.unwrap().with_sample_rate(SampleRate(48000));
                        conn.start(receiver, config).unwrap();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
