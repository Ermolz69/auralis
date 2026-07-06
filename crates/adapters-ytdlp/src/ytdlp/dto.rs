#![allow(dead_code)]

#[derive(Debug, serde::Deserialize)]
pub struct YtDlpOutput {
    pub id: Option<String>,
    pub title: Option<String>,
    pub fulltitle: Option<String>,
    pub webpage_url: Option<String>,
    pub original_url: Option<String>,
    pub extractor: Option<String>,

    pub duration: Option<f64>,
    pub duration_string: Option<String>,

    pub thumbnail: Option<String>,
    #[serde(default)]
    pub thumbnails: Vec<YtDlpThumbnail>,

    pub uploader: Option<String>,
    pub channel: Option<String>,
    pub channel_id: Option<String>,
    pub upload_date: Option<String>,

    pub ext: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,

    pub vcodec: Option<String>,
    pub acodec: Option<String>,

    #[serde(default)]
    pub formats: Vec<YtDlpFormat>,
}

#[derive(Debug, serde::Deserialize)]
pub struct YtDlpThumbnail {
    pub url: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
pub struct YtDlpFormat {
    pub format_id: Option<String>,
    pub ext: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub tbr: Option<f32>,
    pub abr: Option<f32>,
    pub vbr: Option<f32>,
}
