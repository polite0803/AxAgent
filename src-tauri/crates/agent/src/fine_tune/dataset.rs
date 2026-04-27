use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneDataset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub samples: Vec<FineTuneSample>,
    pub format: DataFormat,
    pub metadata: DatasetMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneSample {
    pub id: String,
    pub input: String,
    pub output: String,
    pub system_prompt: Option<String>,
    pub metadata: SampleMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleMetadata {
    pub source: String,
    pub category: Option<String>,
    pub difficulty: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataFormat {
    Jsonl,
    Alpaca,
    ChatML,
    OpenAI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub source: String,
    pub license: String,
    pub tags: Vec<String>,
    pub num_samples: usize,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSpec {
    pub name: String,
    pub description: String,
    pub source: DatasetSource,
    pub preprocessing: Vec<PreprocessingStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatasetSource {
    ConversationHistory,
    ManualUpload,
    Synthetic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreprocessingStep {
    FilterLength { min: usize, max: usize },
    FilterPattern { pattern: String },
    Deduplicate,
    NormalizeWhitespace,
    Truncate { max_length: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    pub stats: DatasetStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    pub total_samples: usize,
    pub avg_input_length: usize,
    pub avg_output_length: usize,
    pub format_compliant: bool,
}

impl FineTuneDataset {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            samples: Vec::new(),
            format: DataFormat::Jsonl,
            metadata: DatasetMetadata {
                source: String::new(),
                license: "unknown".to_string(),
                tags: Vec::new(),
                num_samples: 0,
                created_at: Utc::now(),
            },
        }
    }

    pub fn add_sample(&mut self, sample: FineTuneSample) {
        self.metadata.num_samples += 1;
        self.samples.push(sample);
    }

    pub fn remove_sample(&mut self, sample_id: &str) -> Option<FineTuneSample> {
        if let Some(pos) = self.samples.iter().position(|s| s.id == sample_id) {
            self.metadata.num_samples -= 1;
            Some(self.samples.remove(pos))
        } else {
            None
        }
    }

    pub fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();
        let warnings = Vec::new();
        let mut total_input_len = 0;
        let mut total_output_len = 0;

        for (i, sample) in self.samples.iter().enumerate() {
            total_input_len += sample.input.len();
            total_output_len += sample.output.len();

            if sample.input.is_empty() {
                errors.push(ValidationError {
                    line: i,
                    message: "Empty input".to_string(),
                });
            }

            if sample.output.is_empty() {
                errors.push(ValidationError {
                    line: i,
                    message: "Empty output".to_string(),
                });
            }
        }

        let avg_input_len = if self.samples.is_empty() {
            0
        } else {
            total_input_len / self.samples.len()
        };

        let avg_output_len = if self.samples.is_empty() {
            0
        } else {
            total_output_len / self.samples.len()
        };

        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
            stats: DatasetStats {
                total_samples: self.samples.len(),
                avg_input_length: avg_input_len,
                avg_output_length: avg_output_len,
                format_compliant: true,
            },
        }
    }

    pub fn export(&self, path: &PathBuf, format: DataFormat) -> Result<(), FineTuneError> {
        match format {
            DataFormat::Jsonl => self.export_jsonl(path),
            DataFormat::Alpaca => self.export_alpaca(path),
            DataFormat::ChatML => self.export_chatml(path),
            DataFormat::OpenAI => self.export_openai(path),
        }
    }

    fn export_jsonl(&self, path: &PathBuf) -> Result<(), FineTuneError> {
        use std::fs::File;
        use std::io::Write;

        let file = File::create(path).map_err(|e| FineTuneError::IoError(e.to_string()))?;
        let mut writer = std::io::BufWriter::new(file);

        for sample in &self.samples {
            let json = serde_json::to_string(sample)
                .map_err(|e| FineTuneError::SerializationError(e.to_string()))?;
            writeln!(writer, "{}", json).map_err(|e| FineTuneError::IoError(e.to_string()))?;
        }

        Ok(())
    }

    fn export_alpaca(&self, path: &PathBuf) -> Result<(), FineTuneError> {
        use std::fs::File;
        use std::io::Write;

        let file = File::create(path).map_err(|e| FineTuneError::IoError(e.to_string()))?;
        let mut writer = std::io::BufWriter::new(file);

        for sample in &self.samples {
            let json = serde_json::to_string(&serde_json::json!({
                "instruction": sample.input,
                "output": sample.output,
                "system": sample.system_prompt,
            }))
            .map_err(|e| FineTuneError::SerializationError(e.to_string()))?;
            writeln!(writer, "{}", json).map_err(|e| FineTuneError::IoError(e.to_string()))?;
        }

        Ok(())
    }

    fn export_chatml(&self, path: &PathBuf) -> Result<(), FineTuneError> {
        use std::fs::File;
        use std::io::Write;

        let file = File::create(path).map_err(|e| FineTuneError::IoError(e.to_string()))?;
        let mut writer = std::io::BufWriter::new(file);

        for sample in &self.samples {
            let messages = serde_json::json!([
                {"role": "system", "content": sample.system_prompt.as_deref().unwrap_or("")},
                {"role": "user", "content": sample.input},
                {"role": "assistant", "content": sample.output}
            ]);
            let json = serde_json::to_string(&messages)
                .map_err(|e| FineTuneError::SerializationError(e.to_string()))?;
            writeln!(writer, "{}", json).map_err(|e| FineTuneError::IoError(e.to_string()))?;
        }

        Ok(())
    }

    fn export_openai(&self, path: &PathBuf) -> Result<(), FineTuneError> {
        self.export_jsonl(path)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FineTuneError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Dataset not found: {0}")]
    NotFound(String),
}
