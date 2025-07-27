use crate::config::{FormatsConfig, SelectedFormat};
use anyhow::Result;

pub fn parse_format_string(format_str: &str, formats_config: &FormatsConfig) -> Result<SelectedFormat> {
    let parts: Vec<&str> = format_str.split(':').collect();
    let format_name = parts[0];
    let specified_bitrate = parts.get(1).map(|s| s.to_string());
    
    // Find the format in available formats
    let format_option = formats_config.available.iter()
        .find(|f| f.format == format_name)
        .ok_or_else(|| anyhow::anyhow!("Format '{}' is not available. Available formats: {}", 
                                       format_name, 
                                       formats_config.available.iter()
                                           .map(|f| f.format.as_str())
                                           .collect::<Vec<_>>()
                                           .join(", ")))?;
    
    let bitrate = if let Some(specified) = specified_bitrate {
        // Check if the specified bitrate is valid for this format
        if let Some(available_bitrates) = &format_option.bitrates {
            if !available_bitrates.contains(&specified) {
                anyhow::bail!("Bitrate '{}' is not available for format '{}'. Available bitrates: {}", 
                             specified, format_name, available_bitrates.join(", "));
            }
            Some(specified)
        } else {
            anyhow::bail!("Format '{}' does not support bitrate specification", format_name);
        }
    } else {
        // Use default bitrate or None for lossless formats
        format_option.default_bitrate.clone()
    };
    
    Ok(SelectedFormat {
        format: format_name.to_string(),
        bitrate,
    })
}