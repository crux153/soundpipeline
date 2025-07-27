use crate::config::{FormatsConfig, SelectedFormat};
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};

pub fn select_format(formats_config: &FormatsConfig) -> Result<SelectedFormat> {
    let format_names: Vec<String> = formats_config
        .available
        .iter()
        .map(|f| {
            let display_name = match &f.format[..] {
                "mp3" => "MP3".to_string(),
                "aac" => "AAC (M4A)".to_string(),
                "flac" => "FLAC (Lossless)".to_string(),
                "alac" => "ALAC (Apple Lossless)".to_string(),
                _ => f.format.to_uppercase(),
            };
            
            // Add (Default) suffix if this is the default format
            if let Some(default) = &formats_config.default {
                if f.format == *default {
                    format!("{} (Default)", display_name)
                } else {
                    display_name
                }
            } else {
                display_name
            }
        })
        .collect();

    let default_index = formats_config.default.as_ref()
        .and_then(|default| {
            formats_config.available.iter()
                .position(|f| f.format == *default)
        })
        .unwrap_or(0);

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select output format")
        .items(&format_names)
        .default(default_index)
        .interact()?;

    let format_option = &formats_config.available[selection];
    
    let bitrate = if let Some(bitrates) = &format_option.bitrates {
        if bitrates.len() == 1 {
            Some(bitrates[0].clone())
        } else {
            let default_index = format_option.default_bitrate.as_ref()
                .and_then(|default| bitrates.iter().position(|b| b == default))
                .unwrap_or(0);

            // Create display names for bitrates with (Default) suffix
            let bitrate_display_names: Vec<String> = bitrates
                .iter()
                .map(|bitrate| {
                    if let Some(default_bitrate) = &format_option.default_bitrate {
                        if bitrate == default_bitrate {
                            format!("{} (Default)", bitrate)
                        } else {
                            bitrate.clone()
                        }
                    } else {
                        bitrate.clone()
                    }
                })
                .collect();

            let bitrate_index = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(&format!("Select bitrate for {}", format_names[selection]))
                .items(&bitrate_display_names)
                .default(default_index)
                .interact()?;

            Some(bitrates[bitrate_index].clone())
        }
    } else {
        None
    };

    Ok(SelectedFormat {
        format: format_option.format.clone(),
        bitrate,
    })
}