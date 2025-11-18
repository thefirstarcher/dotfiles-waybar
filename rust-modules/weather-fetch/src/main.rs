use anyhow::Result;
use serde::Deserialize;
use waybar_common::WaybarOutput;

#[derive(Debug, Deserialize)]
struct WeatherResponse {
    #[serde(rename = "current_condition")]
    current: Vec<CurrentCondition>,
}

#[derive(Debug, Deserialize)]
struct CurrentCondition {
    #[serde(rename = "temp_C")]
    temp_c: String,
    #[serde(rename = "weatherDesc")]
    weather_desc: Vec<WeatherDesc>,
    #[serde(rename = "weatherCode")]
    weather_code: String,
}

#[derive(Debug, Deserialize)]
struct WeatherDesc {
    value: String,
}

fn get_weather_icon(code: &str) -> &str {
    match code {
        "113" => "☀", // Clear/Sunny
        "116" => "⛅", // Partly cloudy
        "119" | "122" => "☁", // Cloudy/Overcast
        "143" | "248" | "260" => "🌫", // Fog/Mist
        "176" | "263" | "266" | "293" | "296" => "🌦", // Light rain
        "179" | "227" | "230" | "323" | "326" => "🌨", // Snow
        "182" | "185" | "281" | "284" => "🌧", // Sleet/Freezing rain
        "200" | "386" | "389" => "⛈", // Thunder
        "299" | "302" | "305" | "308" | "356" => "🌧", // Heavy rain
        "314" | "317" | "320" | "350" | "362" | "365" | "374" | "377" => "🌨", // Heavy snow
        _ => "🌡", // Default
    }
}

fn get_weather_class(code: &str) -> &str {
    match code {
        "113" => "clear",
        "119" | "122" | "143" => "cloudy",
        "176" | "263" | "266" | "293" | "296" | "299" | "302" | "305" | "308" => "rain",
        "179" | "227" | "230" | "323" | "326" | "350" | "362" | "365" => "snow",
        _ => "normal",
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Try to fetch weather from wttr.in
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    match client
        .get("https://wttr.in/?format=j1")
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(weather) = response.json::<WeatherResponse>().await {
                if let Some(current) = weather.current.first() {
                    let icon = get_weather_icon(&current.weather_code);
                    let desc = current.weather_desc.first()
                        .map(|d| d.value.as_str())
                        .unwrap_or("Unknown");
                    let class = get_weather_class(&current.weather_code);

                    WaybarOutput::builder()
                        .text(format!("{} {}°C", icon, current.temp_c))
                        .tooltip(format!("{}\n{}", desc, current.temp_c))
                        .class(class)
                        .build()
                        .print();
                    return Ok(());
                }
            }
        }
        Err(_) => {
            // Fallback if network is unavailable
            WaybarOutput::builder()
                .text("󰼯")
                .tooltip("Weather unavailable\nCheck network connection")
                .class("error")
                .build()
                .print();
            return Ok(());
        }
    }

    // Default fallback
    WaybarOutput::builder()
        .text("󰼯")
        .tooltip("Weather data unavailable")
        .class("error")
        .build()
        .print();

    Ok(())
}
