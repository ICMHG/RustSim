use clap::ArgMatches;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub input_file: String,
    pub output_file: Option<String>,
    pub analysis_type: AnalysisType,
    pub output_format: OutputFormat,
    pub verbose_level: u8,
}

#[derive(Debug, Clone)]
pub enum AnalysisType {
    Operating,
    Transient { tstep: f64, tstop: f64 },
    DcSweep { source: String, start: f64, stop: f64, step: f64 },
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Csv,
    Json,
}

impl CliArgs {
    pub fn from_matches(matches: &ArgMatches) -> Result<Self> {
        let input_file = matches
            .get_one::<String>("input")
            .ok_or_else(|| anyhow!("Input file is required"))?
            .clone();

        let output_file = matches.get_one::<String>("output").cloned();

        let verbose_level = matches.get_count("verbose");

        let output_format = match matches.get_one::<String>("format").unwrap().as_str() {
            "csv" => OutputFormat::Csv,
            "json" => OutputFormat::Json,
            _ => return Err(anyhow!("Invalid output format")),
        };

        let analysis_type = if let Some(tran_values) = matches.get_many::<String>("tran") {
            let values: Vec<&String> = tran_values.collect();
            if values.len() != 2 {
                return Err(anyhow!("Transient analysis requires exactly 2 parameters: tstep and tstop"));
            }
            
            let tstep = parse_time_value(values[0])?;
            let tstop = parse_time_value(values[1])?;
            
            if tstep <= 0.0 || tstop <= 0.0 || tstep >= tstop {
                return Err(anyhow!("Invalid time parameters: tstep must be positive and less than tstop"));
            }
            
            AnalysisType::Transient { tstep, tstop }
        } else if let Some(dc_values) = matches.get_many::<String>("dc") {
            let values: Vec<&String> = dc_values.collect();
            if values.len() != 4 {
                return Err(anyhow!("DC sweep requires exactly 4 parameters: source, start, stop, step"));
            }
            
            let source = values[0].clone();
            let start = parse_voltage_value(values[1])?;
            let stop = parse_voltage_value(values[2])?;
            let step = parse_voltage_value(values[3])?;
            
            if step <= 0.0 {
                return Err(anyhow!("Step size must be positive"));
            }
            
            AnalysisType::DcSweep { source, start, stop, step }
        } else {
            // Default to operating point analysis
            AnalysisType::Operating
        };

        Ok(CliArgs {
            input_file,
            output_file,
            analysis_type,
            output_format,
            verbose_level,
        })
    }
}

/// Parse time value with unit (e.g., "1ns", "1.5ms", "10us")
fn parse_time_value(value: &str) -> Result<f64> {
    let value = value.trim().to_lowercase();
    
    if let Some(num_str) = value.strip_suffix("fs") {
        Ok(num_str.parse::<f64>()? * 1e-15)
    } else if let Some(num_str) = value.strip_suffix("ps") {
        Ok(num_str.parse::<f64>()? * 1e-12)
    } else if let Some(num_str) = value.strip_suffix("ns") {
        Ok(num_str.parse::<f64>()? * 1e-9)
    } else if let Some(num_str) = value.strip_suffix("us") {
        Ok(num_str.parse::<f64>()? * 1e-6)
    } else if let Some(num_str) = value.strip_suffix("ms") {
        Ok(num_str.parse::<f64>()? * 1e-3)
    } else if let Some(num_str) = value.strip_suffix("s") {
        Ok(num_str.parse::<f64>()?)
    } else {
        // Assume seconds if no unit specified
        Ok(value.parse::<f64>()?)
    }
}

/// Parse voltage/current value with unit (e.g., "1V", "1.5mA", "10uA")
fn parse_voltage_value(value: &str) -> Result<f64> {
    let value = value.trim().to_lowercase();
    
    if let Some(num_str) = value.strip_suffix("mv") {
        Ok(num_str.parse::<f64>()? * 1e-3)
    } else if let Some(num_str) = value.strip_suffix("uv") {
        Ok(num_str.parse::<f64>()? * 1e-6)
    } else if let Some(num_str) = value.strip_suffix("v") {
        Ok(num_str.parse::<f64>()?)
    } else if let Some(num_str) = value.strip_suffix("ma") {
        Ok(num_str.parse::<f64>()? * 1e-3)
    } else if let Some(num_str) = value.strip_suffix("ua") {
        Ok(num_str.parse::<f64>()? * 1e-6)
    } else if let Some(num_str) = value.strip_suffix("na") {
        Ok(num_str.parse::<f64>()? * 1e-9)
    } else if let Some(num_str) = value.strip_suffix("a") {
        Ok(num_str.parse::<f64>()?)
    } else {
        // Assume base unit if no unit specified
        Ok(value.parse::<f64>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_value() {
        assert_eq!(parse_time_value("1ns").unwrap(), 1e-9);
        assert_eq!(parse_time_value("1.5us").unwrap(), 1.5e-6);
        assert_eq!(parse_time_value("10ms").unwrap(), 10e-3);
        assert_eq!(parse_time_value("1").unwrap(), 1.0);
    }

    #[test]
    fn test_parse_voltage_value() {
        assert_eq!(parse_voltage_value("1V").unwrap(), 1.0);
        assert_eq!(parse_voltage_value("1.5mV").unwrap(), 1.5e-3);
        assert_eq!(parse_voltage_value("10mA").unwrap(), 10e-3);
    }
} 