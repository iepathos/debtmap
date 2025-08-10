use crate::core::AnalysisResults;
use crate::io::output::OutputWriter;
use crate::risk::RiskInsight;
use serde_json;
use std::io::Write;

pub struct JsonWriter<W: Write> {
    writer: W,
}

impl<W: Write> JsonWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: Write> OutputWriter for JsonWriter<W> {
    fn write_results(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(results)?;
        self.writer.write_all(json.as_bytes())?;
        Ok(())
    }

    fn write_risk_insights(&mut self, insights: &RiskInsight) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(insights)?;
        self.writer.write_all(json.as_bytes())?;
        Ok(())
    }
}
