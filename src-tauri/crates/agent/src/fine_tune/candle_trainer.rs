// SPDX-License-Identifier: AGPL-3.0-only

//! 基于 candle 的真实 LoRA 训练后端。
//!
//! 替换 `FineTuneTrainer` 中的模拟训练和占位符导出，
//! 使用 candle-core + candle-nn 执行实际的梯度下降训练，
//! 产出标准 safetensors 格式的 LoRA 权重文件。
//!
//! ## 架构
//!
//! - `CandleLoraLayer`：可训练的 LoRA 线性层（A/B 分解）
//! - `CandleLoraTrainer`：训练器，管理前向/反向/优化
//! - `train_lora_model()`：高层入口（接收训练任务，返回 safetensors 路径）

use std::path::{Path, PathBuf};

use candle_core::{DType, Device, Tensor};
use tracing::info;

use crate::fine_tune::dataset::FineTuneDataset;
use crate::fine_tune::lora::LoRAConfig;

// ── LoRA 层定义 ───────────────────────────────────────────────────────────

/// LoRA 线性层：`y = xW^T + x * (A * B)^T`，其中 W 冻结，A/B 为可训练低秩矩阵。
///
/// - `W`：基础权重（shape `[out_features, in_features]`），冻结不更新
/// - `A`：低秩矩阵（shape `[rank, in_features]`），可训练
/// - `B`：低秩矩阵（shape `[out_features, rank]`），可训练
/// - `scaling = alpha / rank`
struct CandleLoraLayer {
    /// 基础线性层权重（冻结）
    weight: Tensor,
    /// LoRA A 矩阵（可训练）— shape [rank, in_features]
    lora_a: Tensor,
    /// LoRA B 矩阵（可训练）— shape [out_features, rank]
    lora_b: Tensor,
    /// 缩放因子 = alpha / rank
    scaling: f64,
    /// 偏置（可选）
    bias: Option<Tensor>,
    /// 设备
    device: Device,
    _in_features: usize,
    _out_features: usize,
    _rank: usize,
}

impl CandleLoraLayer {
    /// 创建 LoRA 层，从正态分布初始化 A/B。
    fn new(
        weight: Tensor,
        bias: Option<Tensor>,
        rank: usize,
        alpha: f32,
        in_features: usize,
        out_features: usize,
        device: &Device,
    ) -> Self {
        // LoRA A: kaiming uniform init (常用于 LoRA A)
        let lora_a = Tensor::randn(0.0, 0.02, (rank, in_features), device)
            .expect("lora_a init failed");
        // LoRA B: 零初始化（训练开始时 LoRA 不改变输出）
        let lora_b = Tensor::zeros((out_features, rank), DType::F32, device)
            .expect("lora_b init failed");
        let scaling = alpha as f64 / rank as f64;

        Self { weight, lora_a, lora_b, scaling, bias, device: device.clone(), _in_features: in_features, _out_features: out_features, _rank: rank }
    }

    /// 前向传播：`y = x @ W^T + x @ (A @ B)^T * scaling + bias`
    fn forward(&self, input: &Tensor) -> candle_core::Result<Tensor> {
        // base = x @ W^T → shape [batch, out_features]
        let base = input.matmul(&self.weight.t()?)?;

        // lora_delta = x @ A^T → [batch, rank] → @ B^T → [batch, out_features]
        let lora_delta = input.matmul(&self.lora_a.t()?)?.matmul(&self.lora_b.t()?)?;
        let lora_delta = lora_delta.broadcast_mul(&Tensor::new(&[self.scaling as f32], &self.device)?)?;

        let mut output = base.add(&lora_delta)?;
        if let Some(ref b) = self.bias {
            output = output.add(b)?;
        }
        Ok(output)
    }

    /// 获取当前损失
    fn _evaluate(&self, input: &Tensor, target: &Tensor) -> candle_core::Result<f64> {
        let output = self.forward(input)?;
        let loss = output.sub(target)?.sqr()?.mean_all()?;
        loss.to_scalar::<f64>()
    }
}

// ── 简易训练引擎 ──────────────────────────────────────────────────────────

/// 基于 SGD 的 LoRA 训练器（冻结基础权重，仅训练 A/B）。
struct SimpleLoraEngine {
    layer: CandleLoraLayer,
    lr: f64,
    /// A 梯度缓存
    grad_a: Option<Tensor>,
    /// B 梯度缓存
    grad_b: Option<Tensor>,
    _step_count: usize,
}

impl SimpleLoraEngine {
    fn new(layer: CandleLoraLayer, lr: f64) -> Self {
        Self { layer, lr, grad_a: None, grad_b: None, _step_count: 0 }
    }

    /// 前向 + 反向一步
    fn train_step(&mut self, input: &Tensor, target: &Tensor) -> candle_core::Result<f64> {
        // 前向
        let output = self.layer.forward(input)?;

        // MSE 损失
        let loss = output.sub(target)?.sqr()?.mean_all()?;
        let loss_val = loss.to_scalar::<f64>()?;

        // 反向：对 MSE 损失手工求导
        // dL/doutput = 2 * (output - target) / n
        let n = output.elem_count() as f64;
        let d_output = output.sub(target)?.broadcast_mul(&Tensor::new(&[2.0 / n], output.device())?)?;

        // dL/dB = (x @ A^T)^T @ d_output （简化梯度估计）
        // 注：真正的 auto-grad 需要完整计算图。
        // 这里用简化的梯度近似，适用于演示级 LoRA 训练。
        // shape 映射：
        //   d_output: [batch, out_features]
        //   x @ A^T:  [batch, rank]
        //   dL/dB^T = (x @ A^T)^T @ d_output → [rank, out_features]
        //   → dL/dB = [out_features, rank]  ✓
        let xa = input.matmul(&self.layer.lora_a.t()?)?; // [batch, rank]
        let grad_b = xa.t()?.matmul(&d_output)?; // [rank, out_features]
        self.grad_b = Some(grad_b.t()?); // [out_features, rank]

        // dL/dA = (x)^T @ (d_output @ B)
        //   x: [batch, in_features]
        //   d_output @ B: [batch, rank]
        //   dL/dA^T = x^T @ (d_output @ B) → [in_features, rank]
        //   → dL/dA = [rank, in_features]  ✓
        let db = d_output.matmul(&self.layer.lora_b)?;
        let grad_a = input.t()?.matmul(&db)?; // [in_features, rank]
        self.grad_a = Some(grad_a.t()?); // [rank, in_features]

        self._step_count += 1;
        Ok(loss_val)
    }

    /// 应用梯度更新（SGD）
    fn apply_gradients(&mut self) -> candle_core::Result<()> {
        if let (Some(ga), Some(gb)) = (&self.grad_a, &self.grad_b) {
            // 梯度裁剪（防止爆炸）
            let ga_clipped = clip_gradient(ga, 1.0)?;
            let gb_clipped = clip_gradient(gb, 1.0)?;

            // 梯度平均
            let _ga_mean = ga_clipped.mean_all()?;
            let _gb_mean = gb_clipped.mean_all()?;

            // 更新 A: A -= lr * grad
            let lr_t = Tensor::new(&[self.lr as f32], &self.layer.device)?;
            let a_update = ga_clipped.broadcast_mul(&lr_t)?;
            self.layer.lora_a = self.layer.lora_a.sub(&a_update)?;

            // 更新 B: B -= lr * grad
            let b_update = gb_clipped.broadcast_mul(&lr_t)?;
            self.layer.lora_b = self.layer.lora_b.sub(&b_update)?;

            self.grad_a = None;
            self.grad_b = None;
        }
        Ok(())
    }

    /// 获取当前损失
    fn _evaluate(&self, input: &Tensor, target: &Tensor) -> candle_core::Result<f64> {
        let output = self.layer.forward(input)?;
        let loss = output.sub(target)?.sqr()?.mean_all()?;
        loss.to_scalar::<f64>()
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

// ── 高层训练入口 ──────────────────────────────────────────────────────────

/// 使用 candle 执行一次实际的 LoRA 训练。
///
/// 返回生成的 safetensors 文件路径，或错误信息。
///
/// ## 参数
/// - `dataset`: 训练数据集（样本需包含输入和输出文本）
/// - `config`: LoRA 训练配置（rank, alpha, lr, epochs）
/// - `output_dir`: 输出目录（safetensors 文件写入此处）
///
/// ## 限制
/// 当前实现使用简化的字符级向量映射（非真实 LLM embedding），
/// 适用于验证训练管线端到端工作。未来可接入真实 tokenizer + embedding。
pub fn train_lora(
    dataset: &FineTuneDataset,
    config: &LoRAConfig,
    output_dir: &Path,
    progress_callback: Option<impl Fn(f32, f64)>,
) -> Result<PathBuf, String> {
    let device = Device::Cpu;

    // 1. 准备简易特征——将文本映射为固定长度向量
    // 基于字符频率的简单特征提取，而非真实 embedding
    let vocab_size: usize = 128;
    let hidden_size: usize = 64;

    let samples: Vec<(Tensor, Tensor)> = dataset
        .samples
        .iter()
        .map(|s| {
            let input_vec = text_to_features(&s.input, vocab_size, &device)
                .unwrap_or_else(|_| Tensor::zeros(vocab_size, DType::F32, &device).unwrap());
            let output_vec = text_to_features(&s.output, vocab_size, &device)
                .unwrap_or_else(|_| Tensor::zeros(vocab_size, DType::F32, &device).unwrap());
            (input_vec, output_vec)
        })
        .collect();

    if samples.is_empty() {
        return Err("Dataset has no samples".to_string());
    }

    // 2. 构造 LoRA 层（模拟基础权重）
    let base_weight = Tensor::randn(0.0, 0.1, (hidden_size, vocab_size), &device)
        .map_err(|e| format!("base weight init: {e}"))?;
    let lora_layer = CandleLoraLayer::new(
        base_weight,
        None,
        config.rank as usize,
        config.alpha as f32,
        vocab_size,
        hidden_size,
        &device,
    );

    // 3. 训练循环
    let lr = config.learning_rate as f64;
    let mut engine = SimpleLoraEngine::new(lora_layer, lr);
    let batch_size = config.batch_size as usize;
    let epochs = config.epochs as usize;

    info!("[CandleLoRA] Starting training: {} samples, {} epochs, lr={}, rank={}",
        samples.len(), epochs, lr, config.rank);

    for epoch in 0..epochs {
        let mut epoch_loss = 0.0_f64;
        let mut batch_count = 0_usize;

        for batch_start in (0..samples.len()).step_by(batch_size) {
            let batch_end = (batch_start + batch_size).min(samples.len());
            let batch = &samples[batch_start..batch_end];

            // 拼合 batch
            let inputs = stack_tensors(&batch.iter().map(|(i, _)| i).cloned().collect::<Vec<_>>(), &device)
                .map_err(|e| format!("stack inputs: {e}"))?;
            let targets = stack_tensors(&batch.iter().map(|(_, t)| t).cloned().collect::<Vec<_>>(), &device)
                .map_err(|e| format!("stack targets: {e}"))?;

            let loss = engine.train_step(&inputs, &targets)
                .map_err(|e| format!("train step: {e}"))?;
            engine.apply_gradients()
                .map_err(|e| format!("apply grads: {e}"))?;

            epoch_loss += loss;
            batch_count += 1;
        }

        let avg_loss = if batch_count > 0 { epoch_loss / batch_count as f64 } else { 0.0 };
        let progress = (epoch + 1) as f32 / epochs as f32;

        info!("[CandleLoRA] Epoch {}/{}: loss={:.6}, progress={:.1}%",
            epoch + 1, epochs, avg_loss, progress * 100.0);

        if let Some(ref cb) = progress_callback {
            cb(progress, avg_loss);
        }
    }

    // 4. 导出 safetensors
    let adapter_id = uuid::Uuid::new_v4().to_string();
    let adapter_dir = output_dir.join(&adapter_id);
    std::fs::create_dir_all(&adapter_dir)
        .map_err(|e| format!("create output dir: {e}"))?;

    // 用 safetensors 格式保存 LoRA 权重
    let safetensors_path = adapter_dir.join("adapter_model.safetensors");
    export_lora_safetensors(
        &engine.layer.lora_a,
        &engine.layer.lora_b,
        config,
        &safetensors_path,
    )?;

    // 同时写入配置 JSON 供加载时使用
    let config_json = serde_json::json!({
        "adapter_type": "lora",
        "base_model": "candle-builtin",
        "rank": config.rank,
        "alpha": config.alpha,
        "target_modules": config.target_modules,
        "training_job_id": dataset.id,
        "format": "safetensors",
        "lora_a_shape": [config.rank as u32, 128u32],
        "lora_b_shape": [64u32, config.rank as u32],
    });
    let config_path = adapter_dir.join("adapter_config.json");
    std::fs::write(&config_path, serde_json::to_string_pretty(&config_json).unwrap())
        .map_err(|e| format!("write config: {e}"))?;

    info!("[CandleLoRA] Training complete: adapter={}, safetensors={}",
        adapter_id, safetensors_path.display());

    Ok(safetensors_path)
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────

/// 将文本转换为固定长度 float 特征向量（基于字符频率）。
fn text_to_features(text: &str, size: usize, device: &Device) -> candle_core::Result<Tensor> {
    let mut features = vec![0.0_f32; size];
    for (i, ch) in text.chars().enumerate() {
        let idx = (ch as usize) % size;
        features[idx] += 1.0;
        if i >= 1000 { break; } // 截断过长的文本
    }
    // 归一化
    let sum: f32 = features.iter().sum();
    if sum > 0.0 {
        for v in &mut features {
            *v /= sum;
        }
    }
    Tensor::from_vec(features, (1, size), device)
}

/// 将多个 2D 张量堆叠成 Batch 维度。
fn stack_tensors(tensors: &[Tensor], device: &Device) -> candle_core::Result<Tensor> {
    if tensors.is_empty() {
        return Err(candle_core::Error::Msg("empty tensor list".to_string()));
    }
    let dim1 = tensors[0].dims()[1]; // 假设均为 [1, dim1]
    let data: Vec<f32> = tensors
        .iter()
        .flat_map(|t| {
            let d: Vec<f32> = t.flatten_all().unwrap().to_vec1().unwrap_or_default();
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
    _config: &LoRAConfig,
    path: &Path,
) -> Result<(), String> {

    let a_shape: Vec<usize> = lora_a.dims().to_vec();
    let b_shape: Vec<usize> = lora_b.dims().to_vec();

    // 使用标准 safetensors 格式
    let a_raw = lora_a.flatten_all()
        .map_err(|e| format!("lora_a flatten: {e}"))?;
    let b_raw = lora_b.flatten_all()
        .map_err(|e| format!("lora_b flatten: {e}"))?;

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

    let header_bytes = serde_json::to_vec(&header)
        .map_err(|e| format!("header json: {e}"))?;
    let header_len = header_bytes.len() as u64;

    let mut output: Vec<u8> = Vec::new();
    // 8-byte little-endian header length
    output.extend_from_slice(&header_len.to_le_bytes());
    output.extend_from_slice(&header_bytes);

    // 附加原始 float32 数据
    let a_storage: Vec<u8> = a_raw.to_dtype(DType::F32)
        .map_err(|e| format!("a dtype: {e}"))?
        .to_vec1::<f32>()
        .map_err(|e| format!("a vec: {e}"))?
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    output.extend_from_slice(&a_storage);

    let b_storage: Vec<u8> = b_raw.to_dtype(DType::F32)
        .map_err(|e| format!("b dtype: {e}"))?
        .to_vec1::<f32>()
        .map_err(|e| format!("b vec: {e}"))?
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    output.extend_from_slice(&b_storage);

    std::fs::write(path, &output)
        .map_err(|e| format!("write safetensors: {e}"))?;

    info!("[CandleLoRA] Exported safetensors: {} ({} bytes)",
        path.display(), output.len());
    Ok(())
}
