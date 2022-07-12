use reqwest::header;

/// A representation of a download task.
#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub file_url: String,
    pub percentage_completed: u8,
    pub meta: DownloadMeta,
}

/// Basic information on the file to download.
#[derive(Debug, Clone)]
pub struct DownloadMeta {
    pub content_length: u64,
    pub supports_resume: bool,
    pub content_type: String,
    pub file_name: String,
}

/// Representation of a part of the file to download.
#[derive(Debug)]
pub struct DownloadPart {
    pub id: u8,
    pub start_byte: u64,
    pub end_byte: u64,
    pub file_url: String,
}

impl DownloadTask {
    pub async fn new(url: String) -> Result<Self, String> {
        if let Ok(download_meta) = Self::get_file_info(&url).await {
            return Ok(DownloadTask {
                file_url: url,
                percentage_completed: 0,
                meta: download_meta,
            });
        }
        Err(String::from("Error getting file info"))
    }

    /// Gets some basic informations on the file to download.
    /// File size, file name, content type and check if we
    /// can make partial downloads.
    async fn get_file_info(url: &String) -> Result<DownloadMeta, ()> {
        let client = reqwest::Client::new();
        if let Ok(response) = client.head(url).send().await {
            let mut meta = DownloadMeta {
                content_length: 0,
                supports_resume: false,
                content_type: String::new(),
                file_name: String::new(),
            };

            // Get size (Bytes)
            if let Some(content_length) = response.headers().get(header::CONTENT_LENGTH) {
                if !content_length.is_empty() {
                    meta.content_length = String::from(content_length.to_str().unwrap())
                        .parse::<u64>()
                        .unwrap_or(meta.content_length);
                }
            }

            // Get content type
            if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
                if !content_type.is_empty() {
                    meta.content_type =
                        String::from(content_type.to_str().unwrap_or(&meta.content_type).trim())
                }
            }

            // Check if supports partial download
            if let Some(accept_ranges) = response.headers().get(header::ACCEPT_RANGES) {
                if !accept_ranges.is_empty() && meta.content_length > 0 {
                    meta.supports_resume = true;
                }
            }

            // Get file name (Might not be present)
            if let Some(content_disposition) = response.headers().get(header::CONTENT_DISPOSITION) {
                if !content_disposition.is_empty() {
                    let cd = String::from(content_disposition.to_str().unwrap_or("").trim());
                    if cd.to_lowercase().contains("filename") {
                        if let Some(index) = cd.rfind('=') {
                            if index != cd.len() - 1 {
                                meta.file_name = cd[index + 1..].to_string();
                            }
                        }
                    }
                }
            }

            if meta.file_name.is_empty() {
                meta.file_name = Self::get_file_name_from_url(url).unwrap();
            }

            meta.file_name = meta.file_name.replace('\"', "");

            return Ok(meta);
        }
        Err(())
    }

    /// Gets a file name from a dowload url
    fn get_file_name_from_url(url: &str) -> Result<String, ()> {
        if let Some(last_slash_index) = url.rfind('/') {
            if last_slash_index + 1 != url.len() {
                let mut file_name = url[last_slash_index + 1..].to_string();
                if let Some(query_start_index) = file_name.find('?') {
                    file_name = file_name[..query_start_index].to_string();
                }
                return Ok(file_name);
            }
        }

        Ok(String::from("Unknown_File"))
    }

    /// Returns the download parts to download
    pub fn get_download_parts(&self, mut thread_count: u64) -> Vec<DownloadPart> {
        let mut download_parts = Vec::new();

        if self.meta.content_length == 0 || !self.meta.supports_resume {
            download_parts.push(DownloadPart {
                id: 0,
                start_byte: 0,
                end_byte: self.meta.content_length,
                file_url: self.file_url.clone(),
            });
            return download_parts;
        }

        while self.meta.content_length / thread_count < 1000000 && thread_count > 1 {
            thread_count -= 1;
        }

        let part_size = self.meta.content_length / thread_count;

        for i in 0..thread_count {
            let start = i * part_size;
            download_parts.push(DownloadPart {
                id: i as u8,
                start_byte: start,
                end_byte: if i + 1 != thread_count {
                    start + part_size - 1
                } else {
                    self.meta.content_length
                },
                file_url: self.file_url.clone(),
            })
        }

        download_parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_name_from_url_1() {
        let result = DownloadTask::get_file_name_from_url(
            "https://github.com/lokaimoma/Bugza/archive/refs/heads/main.zip",
        )
        .unwrap();

        assert_eq!(result, String::from("main.zip"));
    }

    #[test]
    fn test_get_file_name_from_url_2() {
        let result = DownloadTask::get_file_name_from_url(
            "https://github.com/lokaimoma/Bugza/archive/refs/heads/main.zip?lifetime=100&expire=4000",
        )
        .unwrap();

        assert_eq!(result, String::from("main.zip"));
    }

    #[test]
    fn test_get_file_name_from_url_3() {
        let result = DownloadTask::get_file_name_from_url(
            "https://github.com/lokaimoma/Bugza/archive/refs/heads/",
        )
        .unwrap();

        assert_eq!(result, String::from("Unknown_File"));
    }

    #[test]
    fn test_get_download_parts_1() {
        let task = DownloadTask {
            file_url: String::from(
                "https://github.com/lokaimoma/Bugza/archive/refs/heads/main.zip",
            ),
            percentage_completed: 0,
            meta: DownloadMeta {
                content_length: 0,
                supports_resume: false,
                content_type: String::from("application/zip"),
                file_name: String::from("bugza.zip"),
            },
        };

        let result = task.get_download_parts(4);

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_get_download_parts_2() {
        let task = DownloadTask {
            file_url: String::from("https://go.dev/dl/go1.18.3.linux-amd64.tar.gz"),
            percentage_completed: 0,
            meta: DownloadMeta {
                content_length: 141748419,
                supports_resume: true,
                content_type: String::from("application/x-gzip"),
                file_name: String::from("go1.18.3.linux-amd64.tar.gz"),
            },
        };

        let result = task.get_download_parts(4);

        assert_eq!(result.len(), 4);
        assert_eq!(result[3].end_byte, task.meta.content_length);
    }

    #[test]
    fn test_get_download_parts_3() {
        let task = DownloadTask {
            file_url: String::from("https://hello.com/smallFile.zip"),
            percentage_completed: 0,
            meta: DownloadMeta {
                content_length: 141,
                supports_resume: true,
                content_type: String::from("application/zip"),
                file_name: String::from("smallFile.zip"),
            },
        };

        let result = task.get_download_parts(4);

        assert_eq!(result.len(), 1);
    }
}
