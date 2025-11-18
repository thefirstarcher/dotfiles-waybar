use anyhow::{Context, Result};
use std::collections::HashMap;
use waybar_common::WaybarOutput;
use zbus::{Connection, fdo::DBusProxy, zvariant::{OwnedValue, Value}};

fn escape_markup(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[derive(Debug)]
struct PlayerMetadata {
    title: String,
    artist: String,
    status: String,
}

async fn get_mpris_players(conn: &Connection) -> Result<Vec<String>> {
    let dbus_proxy = DBusProxy::new(conn).await?;
    let names = dbus_proxy.list_names().await?;

    Ok(names
        .into_iter()
        .map(|n| n.to_string())
        .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
        .collect())
}

async fn get_player_metadata(conn: &Connection, player: &str) -> Result<PlayerMetadata> {
    let proxy = zbus::Proxy::new(
        conn,
        player,
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    ).await?;

    // Get playback status
    let status: String = proxy
        .get_property("PlaybackStatus")
        .await
        .unwrap_or_else(|_| "Stopped".to_string());

    // Get metadata
    let metadata: HashMap<String, OwnedValue> = proxy
        .get_property("Metadata")
        .await
        .unwrap_or_default();

    let title = metadata
        .get("xesam:title")
        .and_then(|v| v.downcast_ref::<str>())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let artist = metadata
        .get("xesam:artist")
        .and_then(|v| v.downcast_ref::<zbus::zvariant::Array>())
        .and_then(|arr| arr.get().first())
        .and_then(|v| v.downcast_ref::<str>())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    
    Ok(PlayerMetadata {
        title,
        artist,
        status,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let conn = Connection::session()
        .await
        .context("Failed to connect to D-Bus")?;
    
    let players = get_mpris_players(&conn)
        .await
        .unwrap_or_default();
    
    if players.is_empty() {
        WaybarOutput::builder()
            .text("")
            .tooltip("No media playing")
            .class("idle")
            .build()
            .print();
        return Ok(());
    }
    
    // Use first available player
    if let Ok(metadata) = get_player_metadata(&conn, &players[0]).await {
        let icon = match metadata.status.as_str() {
            "Playing" => "",
            "Paused" => "",
            _ => "",
        };
        
        let class = match metadata.status.as_str() {
            "Playing" => "playing",
            "Paused" => "paused",
            _ => "stopped",
        };
        
        // Truncate long text
        let title = if metadata.title.len() > 30 {
            format!("{}...", &metadata.title[..27])
        } else {
            metadata.title.clone()
        };

        let artist_preview = if metadata.artist.len() > 25 {
            format!("{}...", &metadata.artist[..22])
        } else {
            metadata.artist.clone()
        };

        WaybarOutput::builder()
            .text(format!("{} {}", icon, title))
            .tooltip(escape_markup(&format!(
                "{}\nArtist: {}\nStatus: {}\n\n⏯ Click: Play/Pause\n⏭ Scroll Up: Next\n⏮ Scroll Down: Previous\n⏹ Right-click: Stop",
                metadata.title, metadata.artist, metadata.status
            )))
            .class(class)
            .alt(format!("{} - {}", artist_preview, title))
            .build()
            .print();
    } else {
        WaybarOutput::builder()
            .text("")
            .tooltip("No media info available")
            .class("idle")
            .build()
            .print();
    }
    
    Ok(())
}
