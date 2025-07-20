use tokio::{net::TcpStream, select};

use crate::app::App;

#[repr(u8)]
enum PacketType {
    Offer,
    Answer,
}
impl TryFrom<u8> for PacketType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PacketType::Offer),
            1 => Ok(PacketType::Answer),
            _ => Err(anyhow::anyhow!("TODO")),
        }
    }
}

struct Packet {
    r#type: PacketType,
    size: u32,
    data: Vec<u8>,
}

pub struct NetHandler {
    listener: tokio::net::TcpListener,
    stream: Option<TcpStream>,
}
impl NetHandler {
    pub async fn process_events(&mut self, app: &mut App) -> anyhow::Result<()> {
        select! {
            Ok((stream, addr)) = self.listener.accept() => {
                if self.stream.is_some() {
                    todo!()
                }
                self.stream = Some(stream);
            }
            r = self.stream.as_ref().unwrap().readable() =>{
                self.handle_new_data(app).await?;
            }
        };
        Ok(())
    }
    async fn handle_new_data(&self, app: &mut App) -> anyhow::Result<()> {
        let stream = self.stream.as_ref().unwrap();
        let mut buf = Vec::<u8>::new();
        let read = stream.try_read(&mut buf)?;
        if read < 5 {
            todo!()
        }
        let t = u8::from_le_bytes(buf[0..1].try_into().unwrap());
        buf.drain(0..1);
        let s = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        buf.drain(0..4);
        if s as usize > buf.len() {
            todo!()
        }
        let data = buf.drain(0..s as usize).collect();
        let packet = Packet {
            r#type: t.try_into()?,
            size: s,
            data,
        };
        match packet.r#type {
            PacketType::Offer => todo!(),
            PacketType::Answer => todo!(),
        }

        Ok(())
    }
}
