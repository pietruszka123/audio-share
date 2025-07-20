pub mod app;
pub mod app_n;
pub mod connection;
pub mod net;
pub mod resampler;
pub mod ui;

use cpal::{Device, InputCallbackInfo, traits::DeviceTrait};

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

    Ok((stream, recv, sender))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut app_result = App::new().await?;
    app_result.scan_devices()?;
    let mut terminal = ratatui::init();
    app_result.run(&mut terminal).await?;
    ratatui::restore();
    Ok(())
}
