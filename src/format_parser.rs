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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FormatOption, FormatsConfig};

    fn create_test_formats_config() -> FormatsConfig {
        FormatsConfig {
            available: vec![
                FormatOption {
                    format: "mp3".to_string(),
                    bitrates: Some(vec!["320k".to_string(), "256k".to_string(), "192k".to_string()]),
                    default_bitrate: Some("320k".to_string()),
                    bit_depths: None,
                    default_bit_depth: None,
                },
                FormatOption {
                    format: "flac".to_string(),
                    bitrates: None,
                    default_bitrate: None,
                    bit_depths: Some(vec![16, 24]),
                    default_bit_depth: Some(24),
                },
                FormatOption {
                    format: "alac".to_string(),
                    bitrates: None,
                    default_bitrate: None,
                    bit_depths: Some(vec![16, 24, 32]),
                    default_bit_depth: Some(24),
                },
                FormatOption {
                    format: "aac".to_string(),
                    bitrates: Some(vec!["128k".to_string(), "256k".to_string()]),
                    default_bitrate: Some("256k".to_string()),
                    bit_depths: None,
                    default_bit_depth: None,
                },
                FormatOption {
                    format: "wav".to_string(),
                    bitrates: None,
                    default_bitrate: None,
                    bit_depths: None,
                    default_bit_depth: None,
                },
            ],
            default: Some("mp3".to_string()),
        }
    }

    #[test]
    fn test_parse_format_string_mp3_with_bitrate() {
        let formats = create_test_formats_config();
        let result = parse_format_string("mp3:320k", &formats).unwrap();
        
        assert_eq!(result.format, "mp3");
        assert_eq!(result.bitrate, Some("320k".to_string()));
        assert_eq!(result.bit_depth, None);
    }

    #[test]
    fn test_parse_format_string_mp3_default() {
        let formats = create_test_formats_config();
        let result = parse_format_string("mp3", &formats).unwrap();
        
        assert_eq!(result.format, "mp3");
        assert_eq!(result.bitrate, Some("320k".to_string())); // default bitrate
        assert_eq!(result.bit_depth, None);
    }

    #[test]
    fn test_parse_format_string_flac_with_bit_depth() {
        let formats = create_test_formats_config();
        let result = parse_format_string("flac:16bit", &formats).unwrap();
        
        assert_eq!(result.format, "flac");
        assert_eq!(result.bitrate, None);
        assert_eq!(result.bit_depth, Some(16));
    }

    #[test]
    fn test_parse_format_string_flac_default() {
        let formats = create_test_formats_config();
        let result = parse_format_string("flac", &formats).unwrap();
        
        assert_eq!(result.format, "flac");
        assert_eq!(result.bitrate, None);
        assert_eq!(result.bit_depth, Some(24)); // default bit depth
    }

    #[test]
    fn test_parse_format_string_alac_with_bit_depth() {
        let formats = create_test_formats_config();
        let result = parse_format_string("alac:32bit", &formats).unwrap();
        
        assert_eq!(result.format, "alac");
        assert_eq!(result.bitrate, None);
        assert_eq!(result.bit_depth, Some(32));
    }

    #[test]
    fn test_parse_format_string_wav_no_params() {
        let formats = create_test_formats_config();
        let result = parse_format_string("wav", &formats).unwrap();
        
        assert_eq!(result.format, "wav");
        assert_eq!(result.bitrate, None);
        assert_eq!(result.bit_depth, None);
    }

    #[test]
    fn test_parse_format_string_invalid_format() {
        let formats = create_test_formats_config();
        let result = parse_format_string("ogg", &formats);
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Format 'ogg' is not available"));
        assert!(error.contains("Available formats: mp3, flac, alac, aac, wav"));
    }

    #[test]
    fn test_parse_format_string_invalid_bitrate() {
        let formats = create_test_formats_config();
        let result = parse_format_string("mp3:128k", &formats);
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Bitrate '128k' is not available for format 'mp3'"));
        assert!(error.contains("Available bitrates: 320k, 256k, 192k"));
    }

    #[test]
    fn test_parse_format_string_invalid_bit_depth() {
        let formats = create_test_formats_config();
        let result = parse_format_string("flac:32bit", &formats);
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Bit depth '32bit' is not available for format 'flac'"));
        assert!(error.contains("Available bit depths: 16bit, 24bit"));
    }

    #[test]
    fn test_parse_format_string_bit_depth_on_lossy_format() {
        let formats = create_test_formats_config();
        let result = parse_format_string("mp3:16bit", &formats);
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Format 'mp3' does not support bit depth specification"));
    }

    #[test]
    fn test_parse_format_string_bitrate_on_lossless_format() {
        let formats = create_test_formats_config();
        let result = parse_format_string("flac:320k", &formats);
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Format 'flac' does not support bitrate specification"));
    }

    #[test]
    fn test_parse_format_string_invalid_bit_depth_format() {
        let formats = create_test_formats_config();
        let result = parse_format_string("flac:notanumber", &formats);
        
        assert!(result.is_err());
        // Should be treated as invalid bitrate, not bit depth since it doesn't end with "bit"
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Format 'flac' does not support bitrate specification"));
    }

    #[test]
    fn test_parse_format_string_malformed_bit_depth() {
        let formats = create_test_formats_config();
        let result = parse_format_string("flac:abcbit", &formats);
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Invalid bit depth format: abcbit"));
    }

    #[test]
    fn test_parse_format_string_fallback_bit_depths() {
        // Test FLAC with no configured bit depths - should use fallback
        let formats = FormatsConfig {
            available: vec![
                FormatOption {
                    format: "flac".to_string(),
                    bitrates: None,
                    default_bitrate: None,
                    bit_depths: None, // No configured bit depths
                    default_bit_depth: None,
                },
            ],
            default: None,
        };
        
        // Should accept 16bit and 24bit as fallback for FLAC
        let result = parse_format_string("flac:16bit", &formats).unwrap();
        assert_eq!(result.format, "flac");
        assert_eq!(result.bit_depth, Some(16));
        
        let result = parse_format_string("flac:24bit", &formats).unwrap();
        assert_eq!(result.format, "flac");
        assert_eq!(result.bit_depth, Some(24));
        
        // Should reject other bit depths
        let result = parse_format_string("flac:32bit", &formats);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Invalid bit depth '32bit' for format 'flac'"));
        assert!(error.contains("Supported: 16bit, 24bit"));
    }

    #[test]
    fn test_parse_format_string_flac_default_bit_depth_fallback() {
        // Test FLAC with no default bit depth configured
        let formats = FormatsConfig {
            available: vec![
                FormatOption {
                    format: "flac".to_string(),
                    bitrates: None,
                    default_bitrate: None,
                    bit_depths: Some(vec![16, 24]),
                    default_bit_depth: None, // No default
                },
            ],
            default: None,
        };
        
        let result = parse_format_string("flac", &formats).unwrap();
        assert_eq!(result.format, "flac");
        assert_eq!(result.bit_depth, Some(24)); // Should fallback to 24
    }

    #[test]
    fn test_parse_format_string_alac_default_bit_depth_fallback() {
        // Test ALAC with no default bit depth configured
        let formats = FormatsConfig {
            available: vec![
                FormatOption {
                    format: "alac".to_string(),
                    bitrates: None,
                    default_bitrate: None,
                    bit_depths: Some(vec![16, 24, 32]),
                    default_bit_depth: None, // No default
                },
            ],
            default: None,
        };
        
        let result = parse_format_string("alac", &formats).unwrap();
        assert_eq!(result.format, "alac");
        assert_eq!(result.bit_depth, Some(24)); // Should fallback to 24
    }

    #[test]
    fn test_parse_format_string_multiple_colons() {
        let formats = create_test_formats_config();
        // This should just take the first parameter after the first colon
        let result = parse_format_string("mp3:320k:extra", &formats).unwrap();
        
        assert_eq!(result.format, "mp3");
        assert_eq!(result.bitrate, Some("320k".to_string()));
        assert_eq!(result.bit_depth, None);
    }

    #[test]
    fn test_parse_format_string_aac_with_bitrate() {
        let formats = create_test_formats_config();
        let result = parse_format_string("aac:256k", &formats).unwrap();
        
        assert_eq!(result.format, "aac");
        assert_eq!(result.bitrate, Some("256k".to_string()));
        assert_eq!(result.bit_depth, None);
    }

    #[test]
    fn test_parse_format_string_aac_default() {
        let formats = create_test_formats_config();
        let result = parse_format_string("aac", &formats).unwrap();
        
        assert_eq!(result.format, "aac");
        assert_eq!(result.bitrate, Some("256k".to_string())); // default bitrate
        assert_eq!(result.bit_depth, None);
    }

    #[test]
    fn test_parse_format_string_empty_formats_config() {
        let formats = FormatsConfig {
            available: vec![],
            default: None,
        };
        
        let result = parse_format_string("mp3", &formats);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Format 'mp3' is not available"));
    }

    #[test]
    fn test_parse_format_string_case_sensitivity() {
        let formats = create_test_formats_config();
        
        // Format names should be case sensitive
        let result = parse_format_string("MP3", &formats);
        assert!(result.is_err());
        
        let result = parse_format_string("FLAC", &formats);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_format_string_wav_with_invalid_param() {
        let formats = create_test_formats_config();
        // WAV doesn't support bitrates or bit depths in this config
        let result = parse_format_string("wav:320k", &formats);
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Format 'wav' does not support bitrate specification"));
    }
}