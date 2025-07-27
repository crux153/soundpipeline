use crate::config::{FormatsConfig, SelectedFormat};
use anyhow::Result;

pub fn parse_format_string(format_str: &str, formats_config: &FormatsConfig) -> Result<SelectedFormat> {
    let parts: Vec<&str> = format_str.split(':').collect();
    let format_name = parts[0];
    let second_param = parts.get(1).map(|s| s.to_string());
    
    // Find the format in available formats
    let format_option = formats_config.available.iter()
        .find(|f| f.format == format_name)
        .ok_or_else(|| anyhow::anyhow!("Format '{}' is not available. Available formats: {}", 
                                       format_name, 
                                       formats_config.available.iter()
                                           .map(|f| f.format.as_str())
                                           .collect::<Vec<_>>()
                                           .join(", ")))?;
    
    let mut bitrate = None;
    let mut bit_depth = None;
    
    if let Some(param) = second_param {
        // Check if it's a bit depth parameter (ends with "bit")
        if param.ends_with("bit") {
            // Parse bit depth for FLAC/ALAC only
            if format_name == "flac" || format_name == "alac" {
                let depth_str = param.trim_end_matches("bit");
                let depth_value = depth_str.parse::<u8>()
                    .map_err(|_| anyhow::anyhow!("Invalid bit depth format: {}", param))?;
                
                // Check against configured bit depths if available
                if let Some(available_depths) = &format_option.bit_depths {
                    if !available_depths.contains(&depth_value) {
                        anyhow::bail!("Bit depth '{}' is not available for format '{}'. Available bit depths: {}bit", 
                                     param, format_name, 
                                     available_depths.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("bit, ") + "bit");
                    }
                    bit_depth = Some(depth_value);
                } else {
                    // If no bit depths configured, allow 16 and 24
                    match depth_value {
                        16 | 24 => bit_depth = Some(depth_value),
                        _ => anyhow::bail!("Invalid bit depth '{}' for format '{}'. Supported: 16bit, 24bit", param, format_name),
                    }
                }
            } else {
                anyhow::bail!("Format '{}' does not support bit depth specification", format_name);
            }
        } else {
            // It's a bitrate parameter
            if let Some(available_bitrates) = &format_option.bitrates {
                if !available_bitrates.contains(&param) {
                    anyhow::bail!("Bitrate '{}' is not available for format '{}'. Available bitrates: {}", 
                                 param, format_name, available_bitrates.join(", "));
                }
                bitrate = Some(param);
            } else {
                anyhow::bail!("Format '{}' does not support bitrate specification", format_name);
            }
        }
    } else {
        // Use default bitrate or None for lossless formats
        bitrate = format_option.default_bitrate.clone();
        // Use default bit depth for FLAC/ALAC
        if format_name == "flac" || format_name == "alac" {
            bit_depth = format_option.default_bit_depth.or(Some(24));
        }
    }
    
    Ok(SelectedFormat {
        format: format_name.to_string(),
        bitrate,
        bit_depth,
    })
}