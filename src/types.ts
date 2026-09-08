export type FrameRange = {
  id: number;
  generation: number;
  name: string;
  begin: number;
  end: number;
  endAction: number;
  endActionLabel: string;
};

export type StreamInfo = {
  path: string;
  fileName: string;
  fileStem: string;
  fileSize: number;
  version: number;
  width: number;
  height: number;
  frameCount: number;
  fps: number;
  gpuCompression: number;
  gpuCompressionLabel: string;
  alphaType: number;
  blockCompression: number;
  compressedFrameBytes: number;
  frameRateOverride: number | null;
  ranges: FrameRange[];
};
