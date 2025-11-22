use crate::{io, risk};
use anyhow::Result;

pub fn format_results_to_string(
    results: &crate::core::AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
    format: io::output::OutputFormat,
) -> Result<String> {
    match format {
        io::output::OutputFormat::Json => {
            let output = create_json_output(results, risk_insights);
            Ok(serde_json::to_string_pretty(&output)?)
        }
        _ => {
            let mut buffer = Vec::new();
            write_formatted_output(&mut buffer, results, risk_insights, format)?;
            Ok(String::from_utf8_lossy(&buffer).into_owned())
        }
    }
}

fn create_json_output(
    results: &crate::core::AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
) -> serde_json::Value {
    serde_json::json!({
        "analysis": results,
        "risk_insights": risk_insights,
    })
}

fn write_formatted_output(
    buffer: &mut Vec<u8>,
    results: &crate::core::AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
    format: io::output::OutputFormat,
) -> Result<()> {
    let mut writer = create_file_writer(buffer, format);
    writer.write_results(results)?;
    if let Some(insights) = risk_insights {
        writer.write_risk_insights(insights)?;
    }
    Ok(())
}

fn create_file_writer<'a>(
    buffer: &'a mut Vec<u8>,
    format: io::output::OutputFormat,
) -> Box<dyn io::output::OutputWriter + 'a> {
    match format {
        io::output::OutputFormat::Html => Box::new(io::writers::HtmlWriter::new(buffer)),
        io::output::OutputFormat::Markdown | io::output::OutputFormat::Terminal => {
            Box::new(io::writers::MarkdownWriter::new(buffer))
        }
        _ => Box::new(io::writers::MarkdownWriter::new(buffer)),
    }
}

pub fn determine_priority_output_format(
    top: Option<usize>,
    tail: Option<usize>,
) -> crate::priority::formatter::OutputFormat {
    use crate::priority::formatter::OutputFormat;

    if let Some(n) = tail {
        OutputFormat::Tail(n)
    } else if let Some(n) = top {
        OutputFormat::Top(n)
    } else {
        OutputFormat::Default
    }
}
