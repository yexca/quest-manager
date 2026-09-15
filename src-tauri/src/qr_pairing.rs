//! In-memory Android wireless-debugging QR credentials and public session state.
use base64::{Engine, engine::general_purpose::STANDARD};
use qrcode::{Color, QrCode};
use serde::Serialize;
use std::fmt::Write;

pub const LIFETIME_SECS: u64 = 120;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub status: String,
    pub message: String,
    pub qr_data_url: Option<String>,
    pub expires_at: u64,
    pub serial: Option<String>,
}

pub struct Credentials {
    pub id: String,
    pub service: String,
    pub secret: String,
}

impl Credentials {
    pub fn generate() -> Result<Self, String> {
        let mut bytes = [0; 40];
        getrandom::fill(&mut bytes)
            .map_err(|_| "Could not generate secure pairing credentials.")?;
        let hex = |slice: &[u8]| slice.iter().map(|b| format!("{b:02x}")).collect::<String>();
        Ok(Self {
            id: hex(&bytes[..16]),
            service: format!("studio-{}", hex(&bytes[16..24])),
            secret: hex(&bytes[24..]),
        })
    }

    pub fn image(&self) -> Result<String, String> {
        // Hex credentials cannot introduce QR field separators or XML markup.
        let payload = format!("WIFI:T:ADB;S:{};P:{};;", self.service, self.secret);
        let qr =
            QrCode::new(payload.as_bytes()).map_err(|_| "Could not encode the pairing QR code.")?;
        let side = qr.width() + 8;
        let mut path = String::new();
        for y in 0..qr.width() {
            for x in 0..qr.width() {
                if qr[(x, y)] == Color::Dark {
                    let _ = write!(path, "M{},{}h1v1h-1z", x + 4, y + 4);
                }
            }
        }
        let svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {side} {side}\" shape-rendering=\"crispEdges\"><path fill=\"white\" d=\"M0,0h{side}v{side}H0z\"/><path fill=\"black\" d=\"{path}\"/></svg>"
        );
        Ok(format!(
            "data:image/svg+xml;base64,{}",
            STANDARD.encode(svg)
        ))
    }
}
