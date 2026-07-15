// SPDX-License-Identifier: AGPL-3.0-only

import { forwardRef, useCallback, useImperativeHandle, useRef } from "react";
import { useTranslation } from "react-i18next";

export interface InputAreaHiddenHandle {
  selectFile: () => void;
  selectPhoto: () => void;
  selectAudio: () => void;
  selectVideo: () => void;
}

export const InputAreaHidden = forwardRef<
  InputAreaHiddenHandle,
  { onFilesSelected: (files: File[]) => void }
>(({ onFilesSelected }, ref) => {
  const { t } = useTranslation();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const photoInputRef = useRef<HTMLInputElement>(null);
  const audioInputRef = useRef<HTMLInputElement>(null);
  const videoInputRef = useRef<HTMLInputElement>(null);

  useImperativeHandle(ref, () => ({
    selectFile: () => fileInputRef.current?.click(),
    selectPhoto: () => photoInputRef.current?.click(),
    selectAudio: () => audioInputRef.current?.click(),
    selectVideo: () => videoInputRef.current?.click(),
  }));

  const handleFileChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files) {
        onFilesSelected(Array.from(files));
      }
      if (fileInputRef.current) {
        fileInputRef.current.value = "";
      }
    },
    [onFilesSelected],
  );

  const handlePhotoChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files) {
        onFilesSelected(Array.from(files));
      }
      if (photoInputRef.current) {
        photoInputRef.current.value = "";
      }
    },
    [onFilesSelected],
  );

  const handleAudioChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files) {
        onFilesSelected(Array.from(files));
      }
      if (audioInputRef.current) {
        audioInputRef.current.value = "";
      }
    },
    [onFilesSelected],
  );

  const handleVideoChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files) {
        onFilesSelected(Array.from(files));
      }
      if (videoInputRef.current) {
        videoInputRef.current.value = "";
      }
    },
    [onFilesSelected],
  );

  return (
    <>
      <input
        ref={fileInputRef}
        type="file"
        multiple
        style={{ display: "none" }}
        onChange={handleFileChange}
        aria-label={t("inputArea.uploadFile")}
      />
      <input
        ref={photoInputRef}
        type="file"
        accept="image/*"
        capture="environment"
        style={{ display: "none" }}
        onChange={handlePhotoChange}
        aria-label="Take photo"
      />
      <input
        ref={audioInputRef}
        type="file"
        accept="audio/*"
        capture
        style={{ display: "none" }}
        onChange={handleAudioChange}
        aria-label="Record audio"
      />
      <input
        ref={videoInputRef}
        type="file"
        accept="video/*"
        capture
        style={{ display: "none" }}
        onChange={handleVideoChange}
      />
    </>
  );
});

InputAreaHidden.displayName = "InputAreaHidden";
