use std::rc::Rc;

use cpal::traits::DeviceTrait;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::{Line, ToSpan},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, StatefulWidget, Widget},
};
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;

use crate::app::{App, Selected};

pub fn draw_right_panel(app: &mut App, frame: &mut Frame, layout: &Rc<[Rect]>) {
    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[1]);
    let title = Line::from(" Counter App Tutorial ".bold());
    let instructions = Line::from(vec![
        " Decrement ".into(),
        "<Left>".blue().bold(),
        " Increment ".into(),
        "<Right>".blue().bold(),
        " Quit ".into(),
        "<Q> ".blue().bold(),
    ]);
    let mut block = Block::bordered()
        .title(title.centered())
        .border_set(border::PLAIN);
    if *app.state() == Selected::Right {
        block = block
            .border_set(border::THICK)
            .title_bottom(instructions.centered());
    }

    Paragraph::new(Line::from(format!("desc: {}", app.local_desc)))
        .centered()
        .block(block)
        .render(layout[1], frame.buffer_mut());

    let mut block = Block::bordered()
        .title((" Status: ".to_span() + "Unknown ".gray().bold()).centered())
        .border_set(border::PLAIN);
    if let Some(state) = app.connection_state() {
        let t = state.to_string();
        let (style, t) = match state {
            webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState::Unspecified => {
                (
                    Style::new().black().on_gray().bold().italic(),
                    "Unspecified".gray(),
                )
            }

            webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState::Connecting => (
                Style::new().white().on_light_green().bold().italic(),
                "Connecting".green(),
            ),
            webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState::Connected => (
                Style::new().white().on_green().bold().italic(),
                "Connected".green(),
            ),
            RTCPeerConnectionState::Disconnected
            | RTCPeerConnectionState::Failed
            | RTCPeerConnectionState::Closed
            | RTCPeerConnectionState::New => (
                Style::new().white().on_red().bold().italic(),
                t.to_string().red(),
            ),
        };
        block = block.title((" Status: ".to_span() + t.bold() + " ".to_span()).centered());
    }

    // ratatui::widgets::
    Paragraph::new(Line::from(""))
        .block(block)
        .render(layout[0], frame.buffer_mut());
}
pub fn draw_left_panel(app: &mut App, frame: &mut Frame, layout: &Rc<[Rect]>) {
    let mut block = Block::bordered()
        .title(Line::from(" Available devices ".bold()).centered())
        .border_set(border::PLAIN);
    if *app.state() == Selected::Left {
        let i = vec![
            KeyInfo::new("Move up", KeyCode::Up),
            KeyInfo::new("Select", KeyCode::Enter),
            KeyInfo::new("Move down", KeyCode::Down),
        ];
        block = block
            .border_set(border::THICK)
            .title_bottom(instructions(&i).centered());
    }
    let l = List::new(
        app.devices()
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let name = if i == app.selected_device {
                    v.name.clone().unwrap_or("Error".to_string()).green()
                } else {
                    v.name.clone().unwrap_or("Error".to_string()).into()
                };
                return ListItem::from(vec![name.into()]);
            })
            .collect::<Vec<ListItem>>(),
    )
    .block(block)
    .highlight_symbol(">")
    .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);
    StatefulWidget::render(l, layout[0], frame.buffer_mut(), &mut app.list_state);
}
pub fn draw_popup(app: &mut App, frame: &mut Frame) {
    let block = Block::bordered().title("Popup");
    let area = popup_area(frame.area(), 60, 20);
    frame.render_widget(Clear, area); //this clears out the background
    frame.render_widget(block, area);
}
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

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
