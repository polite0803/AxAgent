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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample(id: &str, input: &str, output: &str) -> FineTuneSample {
        FineTuneSample {
            id: id.to_string(),
            input: input.to_string(),
            output: output.to_string(),
            system_prompt: None,
            metadata: SampleMetadata {
                source: "test".to_string(),
                category: None,
                difficulty: None,
                tags: vec![],
            },
        }
    }

    #[test]
    fn test_fine_tune_dataset_new() {
        let ds = FineTuneDataset::new("ds1".to_string(), "Test Dataset".to_string());
        assert_eq!(ds.id, "ds1");
        assert_eq!(ds.name, "Test Dataset");
        assert!(ds.samples.is_empty());
        assert_eq!(ds.format, DataFormat::Jsonl);
    }

    #[test]
    fn test_fine_tune_dataset_add_sample() {
        let mut ds = FineTuneDataset::new("ds1".to_string(), "Test".to_string());
        let sample = make_sample("s1", "input", "output");
        ds.add_sample(sample);
        assert_eq!(ds.samples.len(), 1);
        assert_eq!(ds.metadata.num_samples, 1);
    }

    #[test]
    fn test_fine_tune_dataset_remove_sample() {
        let mut ds = FineTuneDataset::new("ds1".to_string(), "Test".to_string());
        ds.add_sample(make_sample("s1", "input", "output"));
        ds.add_sample(make_sample("s2", "input2", "output2"));
        let removed = ds.remove_sample("s1");
        assert!(removed.is_some());
        assert_eq!(ds.samples.len(), 1);
        assert_eq!(ds.metadata.num_samples, 1);
    }

    #[test]
    fn test_fine_tune_dataset_remove_nonexistent() {
        let mut ds = FineTuneDataset::new("ds1".to_string(), "Test".to_string());
        ds.add_sample(make_sample("s1", "input", "output"));
        let removed = ds.remove_sample("nonexistent");
        assert!(removed.is_none());
        assert_eq!(ds.samples.len(), 1);
    }

    #[test]
    fn test_fine_tune_dataset_validate_valid() {
        let mut ds = FineTuneDataset::new("ds1".to_string(), "Test".to_string());
        ds.add_sample(make_sample("s1", "input", "output"));
        let result = ds.validate();
        assert!(result.valid);
        assert!(result.errors.is_empty());
        assert_eq!(result.stats.total_samples, 1);
    }

    #[test]
    fn test_fine_tune_dataset_validate_empty_input() {
        let mut ds = FineTuneDataset::new("ds1".to_string(), "Test".to_string());
        ds.add_sample(FineTuneSample {
            id: "s1".to_string(),
            input: "".to_string(),
            output: "output".to_string(),
            system_prompt: None,
            metadata: SampleMetadata {
                source: "test".to_string(),
                category: None,
                difficulty: None,
                tags: vec![],
            },
        });
        let result = ds.validate();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_fine_tune_dataset_validate_empty_output() {
        let mut ds = FineTuneDataset::new("ds1".to_string(), "Test".to_string());
        ds.add_sample(FineTuneSample {
            id: "s1".to_string(),
            input: "input".to_string(),
            output: "".to_string(),
            system_prompt: None,
            metadata: SampleMetadata {
                source: "test".to_string(),
                category: None,
                difficulty: None,
                tags: vec![],
            },
        });
        let result = ds.validate();
        assert!(!result.valid);
    }

    #[test]
    fn test_fine_tune_dataset_validate_empty_dataset() {
        let ds = FineTuneDataset::new("ds1".to_string(), "Test".to_string());
        let result = ds.validate();
        assert!(result.valid);
        assert_eq!(result.stats.total_samples, 0);
        assert_eq!(result.stats.avg_input_length, 0);
    }

    #[test]
    fn test_fine_tune_dataset_validate_avg_lengths() {
        let mut ds = FineTuneDataset::new("ds1".to_string(), "Test".to_string());
        ds.add_sample(make_sample("s1", "hello", "world"));
        ds.add_sample(make_sample("s2", "hi", "earth"));
        let result = ds.validate();
        assert_eq!(result.stats.avg_input_length, 3);
        assert_eq!(result.stats.avg_output_length, 5);
    }

    #[test]
    fn test_data_format_variants() {
        let formats = vec![DataFormat::Jsonl, DataFormat::Alpaca, DataFormat::ChatML, DataFormat::OpenAI];
        for fmt in formats {
            let json = serde_json::to_string(&fmt).unwrap();
            let de: DataFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(de, fmt);
        }
    }

    #[test]
    fn test_preprocessing_step_variants() {
        let steps = vec![
            PreprocessingStep::FilterLength { min: 1, max: 100 },
            PreprocessingStep::FilterPattern { pattern: "test".to_string() },
            PreprocessingStep::Deduplicate,
            PreprocessingStep::NormalizeWhitespace,
            PreprocessingStep::Truncate { max_length: 512 },
        ];
        for step in steps {
            let json = serde_json::to_string(&step).unwrap();
            let _: PreprocessingStep = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_dataset_source_variants() {
        let sources = vec![DatasetSource::ConversationHistory, DatasetSource::ManualUpload, DatasetSource::Synthetic];
        for source in sources {
            let json = serde_json::to_string(&source).unwrap();
            let _: DatasetSource = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_fine_tune_sample_serialization() {
        let sample = make_sample("s1", "input text", "output text");
        let json = serde_json::to_string(&sample).unwrap();
        let de: FineTuneSample = serde_json::from_str(&json).unwrap();
        assert_eq!(de.id, "s1");
        assert_eq!(de.input, "input text");
    }

    #[test]
    fn test_validation_result_serialization() {
        let result = ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec!["test warning".to_string()],
            stats: DatasetStats {
                total_samples: 5,
                avg_input_length: 10,
                avg_output_length: 20,
                format_compliant: true,
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        let de: ValidationResult = serde_json::from_str(&json).unwrap();
        assert!(de.valid);
        assert_eq!(de.stats.total_samples, 5);
    }

    #[test]
    fn test_fine_tune_error_display() {
        let err = FineTuneError::IoError("file not found".to_string());
        assert!(err.to_string().contains("file not found"));
        let err2 = FineTuneError::NotFound("ds1".to_string());
        assert!(err2.to_string().contains("ds1"));
    }
}
