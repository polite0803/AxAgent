// SPDX-License-Identifier: AGPL-3.0-only

//! 基于 candle 的 LoRA 训练实现。
//!
//! 从 agent crate 的 candle_trainer.rs 迁移而来，实现 InferenceEngine trait 的
//! train_lora_with_embeddings 方法。

use std::path::Path;

use candle_core::{DType, Device, Tensor};
use tracing::info;

use axagent_harness::{LoRATrainConfig, LoRATrainResult};

// ── LoRA 层定义 ───────────────────────────────────────────────────────────

/// LoRA 线性层：`y = xW^T + x * (A * B)^T`，其中 W 冻结，A/B 为可训练低秩矩阵。
struct CandleLoraLayer {
    weight: Tensor,
    lora_a: Tensor,
    lora_b: Tensor,
    scaling: f64,
    bias: Option<Tensor>,
    device: Device,
}

impl CandleLoraLayer {
    fn new(
        weight: Tensor,
        bias: Option<Tensor>,
        rank: usize,
        alpha: f32,
        in_features: usize,
        out_features: usize,
        device: &Device,
    ) -> Self {
        let lora_a =
            Tensor::randn(0.0, 0.02, (rank, in_features), device).expect("lora_a init failed");
        let lora_b =
            Tensor::zeros((out_features, rank), DType::F32, device).expect("lora_b init failed");
        let scaling = alpha as f64 / rank as f64;

        Self { weight, lora_a, lora_b, scaling, bias, device: device.clone() }
    }

    fn forward(&self, input: &Tensor) -> candle_core::Result<Tensor> {
        let base = input.matmul(&self.weight.t()?)?;
        let lora_delta = input.matmul(&self.lora_a.t()?)?.matmul(&self.lora_b.t()?)?;
        let lora_delta =
            lora_delta.broadcast_mul(&Tensor::new(&[self.scaling as f32], &self.device)?)?;

        let mut output = base.add(&lora_delta)?;
        if let Some(ref b) = self.bias {
            output = output.add(b)?;
        }
        Ok(output)
    }
}

// ── 简易训练引擎 ──────────────────────────────────────────────────────────

struct SimpleLoraEngine {
    layer: CandleLoraLayer,
    lr: f64,
    grad_a: Option<Tensor>,
    grad_b: Option<Tensor>,
}

impl SimpleLoraEngine {
    fn new(layer: CandleLoraLayer, lr: f64) -> Self {
        Self { layer, lr, grad_a: None, grad_b: None }
    }

    fn train_step(&mut self, input: &Tensor, target: &Tensor) -> candle_core::Result<f64> {
        let output = self.layer.forward(input)?;
        let loss = output.sub(target)?.sqr()?.mean_all()?;
        let loss_val = loss.to_scalar::<f64>()?;

        let n = output.elem_count() as f64;
        let d_output =
            output.sub(target)?.broadcast_mul(&Tensor::new(&[2.0 / n], output.device())?)?;

        let xa = input.matmul(&self.layer.lora_a.t()?)?;
        let grad_b = xa.t()?.matmul(&d_output)?;
        self.grad_b = Some(grad_b.t()?);

        let db = d_output.matmul(&self.layer.lora_b)?;
        let grad_a = input.t()?.matmul(&db)?;
        self.grad_a = Some(grad_a.t()?);

        Ok(loss_val)
    }

    fn apply_gradients(&mut self) -> candle_core::Result<()> {
        if let (Some(ga), Some(gb)) = (&self.grad_a, &self.grad_b) {
            let ga_clipped = clip_gradient(ga, 1.0)?;
            let gb_clipped = clip_gradient(gb, 1.0)?;

            let lr_t = Tensor::new(&[self.lr as f32], &self.layer.device)?;
            let a_update = ga_clipped.broadcast_mul(&lr_t)?;
            self.layer.lora_a = self.layer.lora_a.sub(&a_update)?;

            let b_update = gb_clipped.broadcast_mul(&lr_t)?;
            self.layer.lora_b = self.layer.lora_b.sub(&b_update)?;

            self.grad_a = None;
            self.grad_b = None;
        }
        Ok(())
    }
}

fn clip_gradient(t: &Tensor, max_norm: f64) -> candle_core::Result<Tensor> {
    let norm = t.sqr()?.sum_all()?.sqrt()?;
    let norm_val = norm.to_scalar::<f64>()?;
    if norm_val > max_norm {
        let scale = Tensor::new(&[max_norm as f32 / norm_val as f32], t.device())?;
        t.broadcast_mul(&scale)
    } else {
        Ok(t.clone())
    }
}

// ── 训练入口 ──────────────────────────────────────────────────────────────

/// 使用预计算的真实 embedding 向量执行 LoRA 训练。
pub fn train_with_embeddings(
    input_embeddings: Vec<Vec<f32>>,
    target_embeddings: Vec<Vec<f32>>,
    config: &LoRATrainConfig,
    output_dir: &Path,
    embedding_model_dim: usize,
) -> Result<LoRATrainResult, String> {
    let device = Device::Cpu;

    if input_embeddings.is_empty() {
        return Err("Dataset has no samples".to_string());
    }
    if input_embeddings.len() != target_embeddings.len() {
        return Err(format!(
            "Input/target count mismatch: {} vs {}",
            input_embeddings.len(),
            target_embeddings.len()
        ));
    }
    for v in &input_embeddings {
        if v.len() != embedding_model_dim {
            return Err(format!(
                "Input embedding dimension mismatch: expected {}, got {}",
                embedding_model_dim,
                v.len()
            ));
        }
    }
    for v in &target_embeddings {
        if v.len() != embedding_model_dim {
            return Err(format!(
                "Target embedding dimension mismatch: expected {}, got {}",
                embedding_model_dim,
                v.len()
            ));
        }
    }

    let input_dim = embedding_model_dim;
    let hidden_dim = input_dim;

    let samples: Vec<(Tensor, Tensor)> = input_embeddings
        .into_iter()
        .zip(target_embeddings)
        .map(|(inp, tgt)| {
            let inp_t = Tensor::from_vec(inp, (1, input_dim), &device).unwrap_or_else(|_| {
                Tensor::zeros(input_dim, DType::F32, &device)
                    .expect("创建零值 input Tensor 兜底失败")
            });
            let tgt_t = Tensor::from_vec(tgt, (1, hidden_dim), &device).unwrap_or_else(|_| {
                Tensor::zeros(hidden_dim, DType::F32, &device)
                    .expect("创建零值 target Tensor 兜底失败")
            });
            (inp_t, tgt_t)
        })
        .collect();

    let base_weight = Tensor::randn(0.0, 0.1, (hidden_dim, input_dim), &device)
        .map_err(|e| format!("base weight init: {e}"))?;
    let lora_layer = CandleLoraLayer::new(
        base_weight,
        None,
        config.rank as usize,
        config.alpha as f32,
        input_dim,
        hidden_dim,
        &device,
    );

    let lr = config.learning_rate as f64;
    let mut engine = SimpleLoraEngine::new(lora_layer, lr);
    let batch_size = config.batch_size as usize;
    let epochs = config.epochs as usize;

    info!(
        "[CandleLoRA] Training with real embeddings: {} samples, {} epochs, lr={}, rank={}, dim={}",
        samples.len(),
        epochs,
        lr,
        config.rank,
        embedding_model_dim
    );

    for epoch in 0..epochs {
        let mut epoch_loss = 0.0_f64;
        let mut batch_count = 0_usize;

        for batch_start in (0..samples.len()).step_by(batch_size) {
            let batch_end = (batch_start + batch_size).min(samples.len());
            let batch = &samples[batch_start..batch_end];

            let inputs =
                stack_tensors(&batch.iter().map(|(i, _)| i).cloned().collect::<Vec<_>>(), &device)
                    .map_err(|e| format!("stack inputs: {e}"))?;
            let targets =
                stack_tensors(&batch.iter().map(|(_, t)| t).cloned().collect::<Vec<_>>(), &device)
                    .map_err(|e| format!("stack targets: {e}"))?;

            let loss =
                engine.train_step(&inputs, &targets).map_err(|e| format!("train step: {e}"))?;
            engine.apply_gradients().map_err(|e| format!("apply grads: {e}"))?;

            epoch_loss += loss;
            batch_count += 1;
        }

        let avg_loss = if batch_count > 0 {
            epoch_loss / batch_count as f64
        } else {
            0.0
        };

        info!("[CandleLoRA] Epoch {}/{}: loss={:.6}", epoch + 1, epochs, avg_loss);
    }

    let adapter_id = uuid::Uuid::new_v4().to_string();
    let adapter_dir = output_dir.join(&adapter_id);
    std::fs::create_dir_all(&adapter_dir).map_err(|e| format!("create output dir: {e}"))?;

    let safetensors_path = adapter_dir.join("adapter_model.safetensors");
    export_lora_safetensors(&engine.layer.lora_a, &engine.layer.lora_b, config, &safetensors_path)?;

    let config_json = serde_json::json!({
        "adapter_type": "lora",
        "base_model": "candle-builtin",
        "rank": config.rank,
        "alpha": config.alpha,
        "target_modules": config.target_modules,
        "training_job_id": adapter_id,
        "format": "safetensors",
        "embedding_model_dim": embedding_model_dim,
        "lora_a_shape": [config.rank, embedding_model_dim as u32],
        "lora_b_shape": [embedding_model_dim as u32, config.rank],
    });
    let config_path = adapter_dir.join("adapter_config.json");
    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&config_json)
            .expect("Candle 训练：序列化 config JSON 不应失败"),
    )
    .map_err(|e| format!("write config: {e}"))?;

    info!(
        "[CandleLoRA] Training complete: adapter={}, safetensors={} (dim={})",
        adapter_id,
        safetensors_path.display(),
        embedding_model_dim
    );

    Ok(LoRATrainResult {
        safetensors_path: safetensors_path.to_string_lossy().to_string(),
        adapter_id,
    })
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────

fn stack_tensors(tensors: &[Tensor], device: &Device) -> candle_core::Result<Tensor> {
    if tensors.is_empty() {
        return Err(candle_core::Error::Msg("empty tensor list".to_string()));
    }
    let dim1 = tensors[0].dims()[1];
    let data: Vec<f32> = tensors
        .iter()
        .flat_map(|t| {
            let d: Vec<f32> = t
                .flatten_all()
                .expect("Candle 训练：flatten_all 不应失败")
                .to_vec1()
                .unwrap_or_default();
            d
        })
        .collect();
    let batch = tensors.len();
    Tensor::from_vec(data, (batch, dim1), device)
}

/// 以 safetensors 格式导出 LoRA A/B 权重。
fn export_lora_safetensors(
    lora_a: &Tensor,
    lora_b: &Tensor,
    _config: &LoRATrainConfig,
    path: &Path,
) -> Result<(), String> {
    let a_shape: Vec<usize> = lora_a.dims().to_vec();
    let b_shape: Vec<usize> = lora_b.dims().to_vec();

    let a_raw = lora_a.flatten_all().map_err(|e| format!("lora_a flatten: {e}"))?;
    let b_raw = lora_b.flatten_all().map_err(|e| format!("lora_b flatten: {e}"))?;

    let header = serde_json::json!({
        "lora_a.weight": {
            "dtype": "F32",
            "shape": a_shape,
            "data_offsets": [0, a_raw.elem_count() * 4]
        },
        "lora_b.weight": {
            "dtype": "F32",
            "shape": b_shape,
            "data_offsets": [a_raw.elem_count() * 4, (a_raw.elem_count() + b_raw.elem_count()) * 4]
        }
    });

    let header_bytes = serde_json::to_vec(&header).map_err(|e| format!("header json: {e}"))?;
    let header_len = header_bytes.len() as u64;

    let mut output: Vec<u8> = Vec::new();
    output.extend_from_slice(&header_len.to_le_bytes());
    output.extend_from_slice(&header_bytes);

    let a_storage: Vec<u8> = a_raw
        .to_dtype(DType::F32)
        .map_err(|e| format!("a dtype: {e}"))?
        .to_vec1::<f32>()
        .map_err(|e| format!("a vec: {e}"))?
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    output.extend_from_slice(&a_storage);

    let b_storage: Vec<u8> = b_raw
        .to_dtype(DType::F32)
        .map_err(|e| format!("b dtype: {e}"))?
        .to_vec1::<f32>()
        .map_err(|e| format!("b vec: {e}"))?
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    output.extend_from_slice(&b_storage);

    std::fs::write(path, &output).map_err(|e| format!("write safetensors: {e}"))?;

    info!("[CandleLoRA] Exported safetensors: {} ({} bytes)", path.display(), output.len());
    Ok(())
}
