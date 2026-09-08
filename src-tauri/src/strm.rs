use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use serde::Serialize;
use std::fs;
use std::io::Cursor;
use std::io::Read;
use std::path::PathBuf;

const HEADER_SIZE: usize = 44;
const FRAME_ENTRY_SIZE: usize = 8;

#[derive(Clone)]
struct FrameEntry {
    offset: usize,
    size: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameRange {
    id: u32,
    generation: u32,
    name: String,
    begin: i32,
    end: i32,
    end_action: i32,
    end_action_label: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamInfo {
    path: String,
    file_name: String,
    file_stem: String,
    file_size: usize,
    version: u32,
    width: u32,
    height: u32,
    frame_count: u32,
    fps: u32,
    gpu_compression: u32,
    gpu_compression_label: String,
    alpha_type: u32,
    block_compression: u32,
    compressed_frame_bytes: u32,
    frame_rate_override: Option<i32>,
    ranges: Vec<FrameRange>,
}

pub struct LoadedStream {
    path: PathBuf,
    file: fs::File,
    file_size: usize,
    version: u32,
    width: u32,
    height: u32,
    frame_count: u32,
    fps: u32,
    gpu_compression: u32,
    alpha_type: u32,
    block_compression: u32,
    compressed_frame_bytes: u32,
    frame_rate_override: Option<i32>,
    frames: Vec<FrameEntry>,
    ranges: Vec<FrameRange>,
}

impl LoadedStream {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let mut file = fs::File::open(&path).map_err(|error| format!("Could not open file: {error}"))?;
        let file_size = file.metadata().map_err(|error| format!("Could not inspect file: {error}"))?.len() as usize;
        if file_size < HEADER_SIZE {
            return Err("This is not a supported STRM file".to_string());
        }
        let mut header = vec![0u8; HEADER_SIZE];
        file.read_exact(&mut header).map_err(|error| format!("Could not read STRM header: {error}"))?;
        if &header[0..4] != b"STRM" {
            return Err("This is not a supported STRM file".to_string());
        }
        let version = read_u32(&header, 4)?;
        if version != 2 {
            return Err(format!("Unsupported STRM version {version}; expected version 2"));
        }
        let width = read_u32(&header, 8)?;
        let height = read_u32(&header, 12)?;
        let frame_count = read_u32(&header, 16)?;
        let gpu_compression = read_u32(&header, 20)?;
        let alpha_type = read_u32(&header, 24)?;
        let block_compression = read_u32(&header, 28)?;
        let fps = read_u32(&header, 32)?;
        let compressed_frame_bytes = read_u32(&header, 36)?;
        if width == 0 || height == 0 || frame_count == 0 || fps == 0 {
            return Err("The STRM header contains invalid dimensions, frame count, or frame rate".to_string());
        }
        if gpu_compression > 5 {
            return Err(format!("GPU compression format {gpu_compression} is not a known BC7 variant"));
        }
        if block_compression != 2 {
            return Err(format!("Frame compression {block_compression} is unsupported; expected LZ4 block compression"));
        }
        let table_end = HEADER_SIZE.checked_add(frame_count as usize * FRAME_ENTRY_SIZE)
            .ok_or_else(|| "Frame table size overflow".to_string())?;
        if table_end > file_size {
            return Err("The frame table extends beyond the file".to_string());
        }
        let mut frame_table = vec![0u8; frame_count as usize * FRAME_ENTRY_SIZE];
        read_at(&file, HEADER_SIZE as u64, &mut frame_table)?;
        let mut frames = Vec::with_capacity(frame_count as usize);
        let mut frame_data_end = 0usize;
        for index in 0..frame_count as usize {
            let table_offset = index * FRAME_ENTRY_SIZE;
            let offset = read_u32(&frame_table, table_offset)? as usize;
            let size = read_u32(&frame_table, table_offset + 4)? as usize;
            let end = offset.checked_add(size).ok_or_else(|| format!("Frame {index} offset overflow"))?;
            if offset < table_end || end > file_size {
                return Err(format!("Frame {index} points outside the file"));
            }
            frame_data_end = frame_data_end.max(end);
            frames.push(FrameEntry { offset, size });
        }
        let mut metadata_header = [0u8; 8];
        let (frame_rate_override, mut ranges) = if frame_data_end.checked_add(8).is_some_and(|end| end <= file_size) {
            read_at(&file, frame_data_end as u64, &mut metadata_header)?;
            let blob_size = read_u32(&metadata_header, 0)? as usize;
            if blob_size == 0 || frame_data_end.checked_add(8).and_then(|start| start.checked_add(blob_size)).is_none_or(|end| end > file_size) {
                (None, Vec::new())
            } else {
                let mut blob = vec![0u8; blob_size];
                read_at(&file, (frame_data_end + 8) as u64, &mut blob)?;
                parse_user_blob(&blob)?
            }
        } else {
            (None, Vec::new())
        };
        if ranges.is_empty() {
            ranges.push(FrameRange {
                id: 0,
                generation: 0,
                name: "All Frames".to_string(),
                begin: -1,
                end: -1,
                end_action: 0,
                end_action_label: "Stop".to_string(),
            });
        }
        Ok(Self { path, file, file_size, version, width, height, frame_count, fps, gpu_compression, alpha_type, block_compression, compressed_frame_bytes, frame_rate_override, frames, ranges })
    }

    pub fn info(&self) -> StreamInfo {
        StreamInfo {
            path: self.path.to_string_lossy().into_owned(),
            file_name: self.path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            file_stem: self.path.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
            file_size: self.file_size,
            version: self.version,
            width: self.width,
            height: self.height,
            frame_count: self.frame_count,
            fps: self.fps,
            gpu_compression: self.gpu_compression,
            gpu_compression_label: gpu_compression_label(self.gpu_compression).to_string(),
            alpha_type: self.alpha_type,
            block_compression: self.block_compression,
            compressed_frame_bytes: self.compressed_frame_bytes,
            frame_rate_override: self.frame_rate_override,
            ranges: self.ranges.clone(),
        }
    }

    fn decode_frame_rgba(&self, index: usize) -> Result<Vec<u8>, String> {
        let entry = self.frames.get(index).ok_or_else(|| format!("Frame {index} is outside the stream"))?;
        let mut compressed = vec![0u8; entry.size];
        read_at(&self.file, entry.offset as u64, &mut compressed)?;
        let bc7_size = ((self.width as usize + 3) / 4) * ((self.height as usize + 3) / 4) * 16;
        let mut bc7 = vec![0u8; bc7_size];
        let written = lz4_flex::block::decompress_into(&compressed, &mut bc7)
            .map_err(|error| format!("Frame {index} LZ4 decode failed: {error}"))?;
        if written != bc7_size {
            return Err(format!("Frame {index} decoded to {written} bytes; expected {bc7_size}"));
        }
        let mut bgra = vec![0u32; self.width as usize * self.height as usize];
        let decode_result = match self.gpu_compression {
            2 => texture2ddecoder::decode_bc3(&bc7, self.width as usize, self.height as usize, &mut bgra),
            3..=5 => texture2ddecoder::decode_bc7(&bc7, self.width as usize, self.height as usize, &mut bgra),
            value => return Err(format!("Frame {index} uses unsupported GPU compression format {value}")),
        };
        decode_result.map_err(|error| format!("Frame {index} texture decode failed: {error}"))?;
        let width = self.width as usize;
        let mut rgba = Vec::with_capacity(bgra.len() * 4);
        for row in (0..self.height as usize).rev() {
            for pixel in &bgra[row * width..(row + 1) * width] {
                let [blue, green, red, alpha] = pixel.to_le_bytes();
                rgba.extend_from_slice(&[red, green, blue, alpha]);
            }
        }
        Ok(rgba)
    }

    pub fn decode_frame_png(&self, index: usize) -> Result<Vec<u8>, String> {
        let rgba = self.decode_frame_rgba(index)?;
        let mut png = Vec::new();
        PngEncoder::new(Cursor::new(&mut png))
            .write_image(&rgba, self.width, self.height, ExtendedColorType::Rgba8)
            .map_err(|error| format!("Could not encode frame {index} as PNG: {error}"))?;
        Ok(png)
    }

    pub fn export_frame(&self, directory: PathBuf, index: usize, file_name: &str, separator: &str, start_number: usize) -> Result<PathBuf, String> {
        fs::create_dir_all(&directory).map_err(|error| format!("Could not create export folder: {error}"))?;
        let number = start_number.checked_add(index).ok_or_else(|| "Export number overflowed".to_string())?;
        let path = directory.join(build_export_name(file_name, separator, number)?);
        fs::write(&path, self.decode_frame_png(index)?).map_err(|error| format!("Could not write {}: {error}", path.display()))?;
        Ok(path)
    }

    pub fn export_ranges(&self, directory: PathBuf, range_indexes: &[usize], file_name: &str, separator: &str, start_number: usize) -> Result<usize, String> {
        fs::create_dir_all(&directory).map_err(|error| format!("Could not create export folder: {error}"))?;
        let stem = safe_name(self.path.file_stem().unwrap_or_default().to_string_lossy().as_ref());
        let root = directory.join(format!("{stem}_export"));
        fs::create_dir_all(&root).map_err(|error| format!("Could not create export folder: {error}"))?;
        let mut count = 0usize;
        for &range_index in range_indexes {
            let range = self.ranges.get(range_index).ok_or_else(|| format!("Section index {range_index} does not exist"))?;
            let begin = if range.begin < 0 { 0 } else { range.begin as usize };
            let end = if range.end < 0 { self.frame_count as usize - 1 } else { range.end.min(self.frame_count as i32 - 1) as usize };
            if begin > end || begin >= self.frame_count as usize {
                return Err(format!("Section '{}' has an invalid frame range", range.name));
            }
            let folder = root.join(format!("{:02}_{}", range_index, safe_name(&range.name)));
            fs::create_dir_all(&folder).map_err(|error| format!("Could not create {}: {error}", folder.display()))?;
            for (local_index, frame_index) in (begin..=end).enumerate() {
                let number = start_number.checked_add(local_index).ok_or_else(|| "Export number overflowed".to_string())?;
                let path = folder.join(build_export_name(file_name, separator, number)?);
                fs::write(&path, self.decode_frame_png(frame_index)?)
                    .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
                count += 1;
            }
        }
        Ok(count)
    }
}

fn build_export_name(file_name: &str, separator: &str, number: usize) -> Result<PathBuf, String> {
    let stem = safe_name(file_name);
    let separator = if separator.is_empty() { "_" } else { separator };
    Ok(PathBuf::from(format!("{stem}{separator}{number}.png")))
}

fn parse_user_blob(bytes: &[u8]) -> Result<(Option<i32>, Vec<FrameRange>), String> {
    if bytes.is_empty() { return Ok((None, Vec::new())); }
    let mut reader = BinaryReader::new(bytes);
    let version = reader.i32()?;
    if version != 1 {
        return Err(format!("Unsupported section metadata version {version}"));
    }
    let override_value = reader.i32()?;
    reader.strong_id()?;
    let id_count = reader.u32()? as usize;
    if id_count > 100_000 {
        return Err("Section ID count is unreasonable".to_string());
    }
    for _ in 0..id_count { reader.strong_id()?; }
    let range_count = reader.u32()? as usize;
    if range_count > 100_000 {
        return Err("Section count is unreasonable".to_string());
    }
    let mut ranges = Vec::with_capacity(range_count);
    for _ in 0..range_count {
        let range_version = reader.i32()?;
        if range_version != 1 {
            return Err(format!("Unsupported frame range version {range_version}"));
        }
        let (id, generation) = reader.strong_id()?;
        let name = reader.dotnet_string()?;
        let begin = reader.i32()?;
        let end = reader.i32()?;
        let end_action = reader.i32()?;
        reader.strong_id()?;
        ranges.push(FrameRange { id, generation, name, begin, end, end_action, end_action_label: end_action_label(end_action).to_string() });
    }
    Ok(((override_value > 0).then_some(override_value), ranges))
}

struct BinaryReader<'a> { bytes: &'a [u8], position: usize }

impl<'a> BinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self { Self { bytes, position: 0 } }
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self.position.checked_add(count).ok_or_else(|| "Metadata offset overflow".to_string())?;
        if end > self.bytes.len() { return Err("Section metadata ended unexpectedly".to_string()); }
        let result = &self.bytes[self.position..end];
        self.position = end;
        Ok(result)
    }
    fn u32(&mut self) -> Result<u32, String> { let b = self.take(4)?; Ok(u32::from_le_bytes(b.try_into().unwrap())) }
    fn i32(&mut self) -> Result<i32, String> { let b = self.take(4)?; Ok(i32::from_le_bytes(b.try_into().unwrap())) }
    fn strong_id(&mut self) -> Result<(u32, u32), String> { Ok((self.u32()?, self.u32()?)) }
    fn dotnet_string(&mut self) -> Result<String, String> {
        let mut length = 0usize;
        let mut shift = 0usize;
        loop {
            if shift >= 35 { return Err("Invalid section name length".to_string()); }
            let byte = self.take(1)?[0];
            length |= ((byte & 0x7f) as usize) << shift;
            if byte & 0x80 == 0 { break; }
            shift += 7;
        }
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| "Section name is not valid UTF-8".to_string())
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let end = offset.checked_add(4).ok_or_else(|| "File offset overflow".to_string())?;
    let slice = bytes.get(offset..end).ok_or_else(|| "The STRM file ended unexpectedly".to_string())?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

fn gpu_compression_label(value: u32) -> &'static str {
    match value { 1 => "Unknown / Standard", 2 => "BC3 / DXT5", 3 => "BC7 / Standard", 4 => "BC7 / Rough", 5 => "BC7 / Roughest", _ => "Unknown" }
}

fn end_action_label(value: i32) -> &'static str {
    match value { 0 => "Stop", 100 => "Loop", 200 => "Play section", _ => "Unknown" }
}

fn safe_name(value: &str) -> String {
    let name: String = value.chars().map(|character| if character.is_ascii_alphanumeric() || character == '-' || character == '_' { character } else { '_' }).collect();
    if name.is_empty() { "section".to_string() } else { name }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_export_names() {
        assert_eq!(safe_name("Loop / Win"), "Loop___Win");
    }

    #[test]
    fn maps_known_actions() {
        assert_eq!(end_action_label(0), "Stop");
        assert_eq!(end_action_label(100), "Loop");
    }

}

#[cfg(windows)]
fn read_at(file: &fs::File, offset: u64, buffer: &mut [u8]) -> Result<(), String> {
    use std::os::windows::fs::FileExt;
    let mut read = 0;
    while read < buffer.len() {
        let count = file.seek_read(&mut buffer[read..], offset + read as u64)
            .map_err(|error| format!("Could not read STRM data: {error}"))?;
        if count == 0 { return Err("The STRM file ended unexpectedly".to_string()); }
        read += count;
    }
    Ok(())
}

#[cfg(unix)]
fn read_at(file: &fs::File, offset: u64, buffer: &mut [u8]) -> Result<(), String> {
    use std::os::unix::fs::FileExt;
    let mut read = 0;
    while read < buffer.len() {
        let count = file.read_at(&mut buffer[read..], offset + read as u64)
            .map_err(|error| format!("Could not read STRM data: {error}"))?;
        if count == 0 { return Err("The STRM file ended unexpectedly".to_string()); }
        read += count;
    }
    Ok(())
}
