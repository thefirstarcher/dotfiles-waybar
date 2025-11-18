#!/bin/bash

# Weather script for Waybar
# Requires: curl, jq

# Configuration
CITY="Kyiv"  # Change to your city
UNITS="metric"  # metric or imperial
API_KEY=""  # Optional: Use OpenWeatherMap API for more stability

# Cache settings
CACHE_DIR="$HOME/.cache/waybar"
CACHE_FILE="$CACHE_DIR/weather.json"
CACHE_TIME=1800  # 30 minutes in seconds

# Create cache directory if it doesn't exist
mkdir -p "$CACHE_DIR"

# Check if cache is still valid
if [ -f "$CACHE_FILE" ]; then
    CACHE_AGE=$(($(date +%s) - $(stat -c %Y "$CACHE_FILE" 2>/dev/null || stat -f %m "$CACHE_FILE")))
    if [ "$CACHE_AGE" -lt "$CACHE_TIME" ]; then
        cat "$CACHE_FILE"
        exit 0
    fi
fi

# Function to save and output result
save_and_output() {
    echo "$1" > "$CACHE_FILE"
    echo "$1"
}

# Fetch weather data
if [ -n "$API_KEY" ]; then
    # Use OpenWeatherMap API
    WEATHER_URL="https://api.openweathermap.org/data/2.5/weather?q=${CITY}&units=${UNITS}&appid=${API_KEY}"
    WEATHER_DATA=$(curl -sf "$WEATHER_URL")

    if [ $? -eq 0 ] && [ -n "$WEATHER_DATA" ]; then
        TEMP=$(echo "$WEATHER_DATA" | jq -r '.main.temp' | cut -d'.' -f1)
        CONDITION=$(echo "$WEATHER_DATA" | jq -r '.weather[0].main')
        DESCRIPTION=$(echo "$WEATHER_DATA" | jq -r '.weather[0].description')

        # Icon mapping
        case "$CONDITION" in
            "Clear") ICON="󰖙" ;;
            "Clouds") ICON="󰖐" ;;
            "Rain") ICON="󰖗" ;;
            "Drizzle") ICON="󰖗" ;;
            "Thunderstorm") ICON="󰙾" ;;
            "Snow") ICON="󰖘" ;;
            "Mist"|"Fog") ICON="󰖑" ;;
            *) ICON="󰖐" ;;
        esac

        OUTPUT=$(cat <<EOF
{
    "text": "${ICON} ${TEMP}°C",
    "tooltip": "${DESCRIPTION^}\nTemperature: ${TEMP}°C",
    "class": "weather"
}
EOF
)
        save_and_output "$OUTPUT"
        exit 0
    fi
fi

# Fallback to wttr.in if no API key or OWM failed
WEATHER_DATA=$(curl -sf "https://wttr.in/${CITY}?format=j1")

if [ $? -eq 0 ] && [ -n "$WEATHER_DATA" ]; then
    TEMP=$(echo "$WEATHER_DATA" | jq -r '.current_condition[0].temp_C')
    FEELS_LIKE=$(echo "$WEATHER_DATA" | jq -r '.current_condition[0].FeelsLikeC')
    CONDITION=$(echo "$WEATHER_DATA" | jq -r '.current_condition[0].weatherDesc[0].value' | tr '[:upper:]' '[:lower:]')

    # Icon mapping (lowercased)
    case "$CONDITION" in
        *"clear"*|*"sunny"*) ICON="󰖙" ;;
        *"cloud"*|*"cloudy"*) ICON="󰖐" ;;
        *"rain"*|*"shower"*) ICON="󰖗" ;;
        *"storm"*|*"thunder"*) ICON="󰙾" ;;
        *"snow"*) ICON="󰖘" ;;
        *"mist"*|*"fog"*) ICON="󰖑" ;;
        *) ICON="󰖐" ;;
    esac

    OUTPUT=$(cat <<EOF
{
    "text": "${ICON} ${TEMP}°C",
    "tooltip": "${CONDITION^}\nTemperature: ${TEMP}°C (feels like ${FEELS_LIKE}°C)",
    "class": "weather"
}
EOF
)
    save_and_output "$OUTPUT"
    exit 0
fi

# If all else fails
ERROR_OUTPUT='{"text": "󰖐 N/A", "tooltip": "Weather data unavailable", "class": "weather-error"}'
save_and_output "$ERROR_OUTPUT"
