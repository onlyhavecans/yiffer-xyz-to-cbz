use crate::yiffer::YifferComic;
use async_zip::write::{EntryOptions, ZipFileWriter};
use async_zip::Compression;
use log::info;
use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use tokio::fs::File;
use url::Url;

pub struct Cbz {
    name: String,
    artist: String,
    urls: Vec<Url>,
}

impl Cbz {
    pub fn from(comic: YifferComic) -> Self {
        let name = sanitize_name(&comic.name);
        let artist = comic.artist;
        let urls = comic.pages;
        Self { name, artist, urls }
    }

    pub async fn write(self, directory: Option<String>) -> anyhow::Result<()> {
        let base_dir = match directory {
            Some(d) => d,
            None => "comics".into(),
        };
        let file = comic_file(&base_dir, &self.name, &self.artist);

        let client = Client::new();

        if let Err(e) = write_cbz(&file, self.urls, &client).await {
            // Ignore removal error
            let _ = fs::remove_file(file);
            return Err(e.context("Failed to write file"));
        }

        Ok(())
    }
}

fn sanitize_name(s: &str) -> String {
    s.replace(':', "")
        .replace('/', "")
        .replace('\\', "")
        // Keep this last to remove duplicate spaces
        .replace("  ", " ")
}

fn comic_file(base_dir: &str, name: &str, artist: &str) -> PathBuf {
    let base = PathBuf::from(base_dir);
    let comic_folder = name.to_string();
    let cbz = format!("{} by {}.cbz", name, artist);
    base.join(comic_folder).join(cbz)
}

fn filename_from_url(url: &Url) -> String {
    let segs = url.path_segments().unwrap();
    let name = segs.last().unwrap();
    name.into()
}

async fn write_cbz(file: &PathBuf, urls: Vec<Url>, client: &Client) -> anyhow::Result<()> {
    // Make the dir
    let parent = file.parent().unwrap();
    info!("creating directories: {}", parent.display());
    fs::create_dir_all(parent)?;

    //Set up the zipfile
    info!("creating file: {}", file.display());
    let mut file = File::create(&file).await?;
    let mut zip = ZipFileWriter::new(&mut file);

    // Write files in turn
    for url in urls {
        write_url_to_zip(&mut zip, client, url).await?;
    }

    zip.close().await?;
    Ok(())
}

async fn write_url_to_zip(
    zip: &mut ZipFileWriter<&mut File>,
    client: &Client,
    url: Url,
) -> Result<(), anyhow::Error> {
    let filename = filename_from_url(&url);
    let res = client.get(url).send().await?;
    let bytes = res.bytes().await?;

    info!("writing to zip: {}", filename);
    let options = EntryOptions::new(filename, Compression::Deflate);
    zip.write_entry_whole(options, &bytes).await?;
    Ok(())
}
