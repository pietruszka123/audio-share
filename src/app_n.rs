use crate::connection::Connection;

pub trait Test {
    fn render(&mut self);
}

pub struct App {
    running: bool,
    devices: Vec<cpal::Device>,
    selected_device: usize,
    connection: Option<Connection>,
}
impl App {}
impl Test for App {
    fn render(&mut self) {
        //
    }
}
