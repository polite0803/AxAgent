// SPDX-License-Identifier: AGPL-3.0-only

import { theme } from "antd";
import { useEffect, useRef } from "react";

interface VoiceWaveformProps {
  /** 是否正在聆听（用户侧） */
  isListening: boolean;
  /** 是否正在说话（AI 侧） */
  isSpeaking: boolean;
  /** 音频分析器（来自 AudioWorklet） */
  analyser: AnalyserNode | null;
  /** 高度（像素） */
  height?: number;
}

/**
 * SVG 三层波形可视化组件
 *
 * 三层设计：
 * - 外层包络线（渐变色，显示整体能量）
 * - 中层波形（显示频率特征）
 * - 内层中线（状态指示）
 */
export function VoiceWaveform({
  isListening,
  isSpeaking,
  analyser,
  height = 80,
}: VoiceWaveformProps) {
  const { token } = theme.useToken();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rafRef = useRef<number | null>(null);
  const dataRef = useRef<Float32Array | null>(null);

  useEffect(() => {
    if (!analyser || !canvasRef.current) {
      return;
    }

    const bufferLength = analyser.frequencyBinCount;
    dataRef.current = new Float32Array(bufferLength);
    const canvas = canvasRef.current;
    const ctx = canvas.getContext("2d");
    if (!ctx) { return; }

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);

    const draw = () => {
      if (!analyser || !dataRef.current || !canvasRef.current) { return; }

      const w = rect.width;
      const h = height;
      ctx.clearRect(0, 0, w, h);

      // 背景网格
      ctx.strokeStyle = token.colorBorderSecondary;
      ctx.lineWidth = 0.5;
      ctx.beginPath();
      ctx.moveTo(0, h / 2);
      ctx.lineTo(w, h / 2);
      ctx.stroke();

      if (isListening || isSpeaking) {
        analyser.getFloatTimeDomainData(dataRef.current as Float32Array<ArrayBuffer>);

        // 外层包络：渐变色填充
        const gradient = ctx.createLinearGradient(0, 0, 0, h);
        if (isSpeaking) {
          gradient.addColorStop(0, token.colorError);
          gradient.addColorStop(1, token.colorWarning);
        } else {
          gradient.addColorStop(0, token.colorPrimaryActive);
          gradient.addColorStop(1, token.colorPrimary);
        }

        ctx.fillStyle = gradient;
        ctx.beginPath();
        const sliceWidth = w / bufferLength;
        let x = 0;

        for (let i = 0; i < bufferLength; i++) {
          const v = dataRef.current[i];
          const y = (v + 1) / 2 * h;
          if (i === 0) {
            ctx.moveTo(x, y);
          } else {
            ctx.lineTo(x, y);
          }
          x += sliceWidth;
        }
        ctx.lineTo(w, h);
        ctx.lineTo(0, h);
        ctx.closePath();
        ctx.fill();

        // 中层波形：线条
        ctx.strokeStyle = isSpeaking ? token.colorError : token.colorPrimary;
        ctx.lineWidth = 2;
        ctx.beginPath();
        x = 0;
        for (let i = 0; i < bufferLength; i++) {
          const v = dataRef.current[i];
          const y = (v + 1) / 2 * h;
          if (i === 0) {
            ctx.moveTo(x, y);
          } else {
            ctx.lineTo(x, y);
          }
          x += sliceWidth;
        }
        ctx.stroke();
      } else {
        // 空闲状态：显示静态中线
        ctx.strokeStyle = token.colorBorder;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(0, h / 2);
        ctx.lineTo(w, h / 2);
        ctx.stroke();
      }

      rafRef.current = requestAnimationFrame(draw);
    };

    rafRef.current = requestAnimationFrame(draw);

    return () => {
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
      }
    };
  }, [analyser, isListening, isSpeaking, height, token]);

  return (
    <canvas
      ref={canvasRef}
      className="w-full rounded-lg"
      style={{ height, background: token.colorBgLayout }}
    />
  );
}
