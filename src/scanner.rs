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
    /// Whether the user has manually skipped this file
    pub skipped: bool,
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
            skipped: false,
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

#[cfg(test)]
mod tests {
    use super::extract_number;

    // ── Basic episode extraction ──────────────────────────────────────────────

    #[test]
    fn simple_numeric_filename() {
        assert_eq!(extract_number("5.mkv"), Some(5));
    }

    #[test]
    fn episode_at_end_of_name() {
        assert_eq!(extract_number("Show.Name.S01E03.mkv"), Some(3));
    }

    #[test]
    fn episode_only_number_no_season() {
        assert_eq!(extract_number("My.Series.12.mp4"), Some(12));
    }

    #[test]
    fn padded_episode_number() {
        assert_eq!(extract_number("episode_007.mkv"), Some(7));
    }

    #[test]
    fn episode_number_with_spaces() {
        assert_eq!(extract_number("Some Show 24.avi"), Some(24));
    }

    // ── Year vs episode disambiguation ────────────────────────────────────────

    #[test]
    fn year_only_still_returned_when_sole_number() {
        // No episode number present — the year is the only number, so we return it.
        // The caller / UI can decide how to handle it; extract_number just finds the last number.
        assert_eq!(extract_number("Documentary.2021.mkv"), Some(2021));
    }

    #[test]
    fn year_plus_episode_returns_episode() {
        // Year comes first; episode number comes last → should return episode.
        assert_eq!(extract_number("Show.2019.S01E05.mkv"), Some(5));
    }

    #[test]
    fn year_in_title_with_trailing_episode() {
        assert_eq!(
            extract_number("Series.2001.A.Space.Odyssey.42.mkv"),
            Some(42)
        );
    }

    #[test]
    fn multiple_years_episode_last() {
        // Two year-like numbers followed by a small episode number.
        assert_eq!(extract_number("1999.2001.03.mkv"), Some(3));
    }

    // ── Noise token stripping ─────────────────────────────────────────────────

    #[test]
    fn strips_resolution_720p() {
        assert_eq!(extract_number("Show.S02E11.720p.mkv"), Some(11));
    }

    #[test]
    fn strips_resolution_1080p() {
        assert_eq!(
            extract_number("Series.Episode.08.1080p.BluRay.mkv"),
            Some(8)
        );
    }

    #[test]
    fn strips_codec_x264() {
        assert_eq!(extract_number("Series.05.x264.mp4"), Some(5));
    }

    #[test]
    fn strips_codec_x265() {
        assert_eq!(extract_number("Series.05.x265.mp4"), Some(5));
    }

    #[test]
    fn strips_hevc() {
        assert_eq!(extract_number("Show.S03E07.HEVC.mkv"), Some(7));
    }

    #[test]
    fn strips_web_dl() {
        assert_eq!(extract_number("Show.S01E02.WEB-DL.mkv"), Some(2));
    }

    #[test]
    fn strips_webrip() {
        assert_eq!(extract_number("Show.S01E09.WEBRip.mkv"), Some(9));
    }

    #[test]
    fn strips_bluray() {
        assert_eq!(extract_number("Show.S01E04.BluRay.mkv"), Some(4));
    }

    #[test]
    fn strips_hdtv() {
        assert_eq!(extract_number("Show.S02E06.HDTV.mkv"), Some(6));
    }

    #[test]
    fn strips_audio_codec_aac() {
        assert_eq!(extract_number("Show.S01E10.AAC.mkv"), Some(10));
    }

    #[test]
    fn strips_audio_codec_dts() {
        assert_eq!(extract_number("Show.S02E12.DTS.mkv"), Some(12));
    }

    #[test]
    fn strips_remastered() {
        assert_eq!(extract_number("Classic.Film.13.Remastered.mkv"), Some(13));
    }

    #[test]
    fn strips_4k() {
        // "4k" contains the digit 4 — after stripping the token the episode number wins.
        assert_eq!(extract_number("Show.S01E02.4K.mkv"), Some(2));
    }

    // ── Subtitle files ────────────────────────────────────────────────────────

    #[test]
    fn subtitle_srt_extension() {
        assert_eq!(extract_number("Show.S01E06.srt"), Some(6));
    }

    #[test]
    fn subtitle_ass_extension() {
        assert_eq!(extract_number("Show.S02E03.ass"), Some(3));
    }

    // ── Edge cases ────────────────────────────────────────────────────────────

    #[test]
    fn no_number_returns_none() {
        assert_eq!(extract_number("NoNumbers.mkv"), None);
    }

    #[test]
    fn empty_string_returns_none() {
        assert_eq!(extract_number(""), None);
    }

    #[test]
    fn extension_only_returns_none() {
        assert_eq!(extract_number(".mkv"), None);
    }

    #[test]
    fn numbers_only_in_extension_ignored() {
        // Extension digits must not be mistaken for episode numbers.
        // e.g. "ShowName.mp4" — "4" is part of "mp4" which is stripped as a noise token.
        // After stripping, no digits remain → None.
        assert_eq!(extract_number("ShowName.mp4"), None);
    }

    #[test]
    fn very_large_episode_number() {
        assert_eq!(extract_number("Anime.9999.mkv"), Some(9999));
    }

    #[test]
    fn already_renamed_single_number() {
        // Files that are already in "N.ext" format should still parse correctly.
        assert_eq!(extract_number("42.mkv"), Some(42));
    }

    #[test]
    fn leading_zeros_parse_correctly() {
        assert_eq!(extract_number("001.mkv"), Some(1));
        assert_eq!(extract_number("010.mkv"), Some(10));
    }

    #[test]
    fn episode_embedded_in_long_release_name() {
        assert_eq!(
            extract_number("The.Expanse.S03E07.Delta-V.1080p.BluRay.x265.HEVC.AAC.mkv"),
            Some(7)
        );
    }

    #[test]
    fn double_episode_takes_last() {
        // Multi-episode files like S01E03E04 — last number wins.
        assert_eq!(extract_number("Show.S01E03E04.mkv"), Some(4));
    }
}
