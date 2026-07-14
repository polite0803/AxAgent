// SPDX-License-Identifier: AGPL-3.0-only
//
// AudioWorkletProcessor for PCM16 encoding (Float32 → Int16).
// Exported as a string constant so it can be loaded via Blob URL,
// avoiding any MIME/CORS issues in Tauri production builds.

const AUDIO_PROCESSOR_CODE = `
class AudioPcm16Processor extends AudioWorkletProcessor {
  process(inputs) {
    const input = inputs[0];
    if (!input || !input[0]) { return true; }

    const float32 = input[0];
    const int16 = new Int16Array(float32.length);
    for (let i = 0; i < float32.length; i++) {
      const s = Math.max(-1, Math.min(1, float32[i]));
      int16[i] = s < 0 ? s * 0x8000 : s * 0x7fff;
    }

    this.port.postMessage(int16.buffer, [int16.buffer]);
    return true;
  }
}

registerProcessor("audio-pcm16-processor", AudioPcm16Processor);
`.trim();

export function loadAudioWorklet(audioCtx: AudioContext): Promise<void> {
  const blob = new Blob([AUDIO_PROCESSOR_CODE], { type: "application/javascript" });
  const url = URL.createObjectURL(blob);
  return audioCtx.audioWorklet.addModule(url).finally(() => {
    URL.revokeObjectURL(url);
  });
}
