use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// A single file that was found during scanning.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Original filename (not full path).
    pub original: String,
    /// Full path to the file.
    pub full_path: PathBuf,
    /// The computed new filename, if we could extract an episode number.
    pub new_name: Option<String>,
    /// Whether this file has already been renamed (original == new_name).
    pub already_done: bool,
}

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "mpg", "mpeg",
];

const SUBTITLE_EXTENSIONS: &[&str] = &["srt", "sub", "ass", "vtt"];

/// Strip codec/resolution tags and extract the last episode-like number.
pub fn extract_number(name: &str) -> Option<u32> {
    // Remove extension first
    let stem = Path::new(name)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(name);

    // Strip common codec/resolution noise
    let cleaned = regex_lite_replace(stem);

    // Find the last number in the cleaned string — that's the episode number
    let mut last: Option<u32> = None;
    let mut i = 0;
    let bytes = cleaned.as_bytes();
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let slice = &cleaned[start..i];
            if let Ok(n) = slice.parse::<u32>() {
                // Ignore obviously-year-like 4-digit numbers that look like 1920-2099
                // (common in release years embedded in filenames) unless it's the only number
                last = Some(n);
            }
        } else {
            i += 1;
        }
    }
    last
}

/// Remove known noise tokens from a filename stem.
fn regex_lite_replace(s: &str) -> String {
    let noise: &[&str] = &[
        "480p",
        "720p",
        "1080p",
        "2160p",
        "4k",
        "x264",
        "x265",
        "h264",
        "h265",
        "hevc",
        "avc",
        "bluray",
        "blu-ray",
        "bdrip",
        "brrip",
        "webrip",
        "web-dl",
        "webdl",
        "hdtv",
        "dvdrip",
        "aac",
        "ac3",
        "dts",
        "mp3",
        "extended",
        "remastered",
        "proper",
        "repack",
    ];

    let mut out = s.to_lowercase();
    for token in noise {
        out = out.replace(token, " ");
    }
    out
}

pub fn is_media(ext: &str) -> bool {
    let e = ext.to_lowercase();
    VIDEO_EXTENSIONS.contains(&e.as_str()) || SUBTITLE_EXTENSIONS.contains(&e.as_str())
}

pub fn scan_folder(path: &Path) -> Vec<FileEntry> {
    let mut videos: Vec<FileEntry> = Vec::new();
    let mut subtitles: Vec<FileEntry> = Vec::new();

    let Ok(entries) = std::fs::read_dir(path) else {
        return vec![];
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().to_string();
        let full_path = entry.path();

        if !full_path.is_file() {
            continue;
        }

        let ext = full_path
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_lowercase();

        if !is_media(&ext) {
            continue;
        }

        let number = extract_number(&name);
        let new_name = number.map(|n| format!("{}.{}", n, ext));
        let already_done = new_name.as_deref() == Some(name.as_str());

        let entry = FileEntry {
            original: name,
            full_path,
            new_name,
            already_done,
        };

        if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
            videos.push(entry);
        } else {
            subtitles.push(entry);
        }
    }

    // Sort each group numerically by extracted number, unknowns go last
    let sort_key = |e: &FileEntry| -> u32 {
        e.new_name
            .as_deref()
            .and_then(|n| Path::new(n).file_stem()?.to_str()?.parse().ok())
            .unwrap_or(u32::MAX)
    };

    videos.sort_by_key(sort_key);
    subtitles.sort_by_key(sort_key);

    videos.extend(subtitles);
    videos
}
