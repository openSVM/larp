use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_xml_rs::from_str;
use reqwest::Client;

use crate::agentic::tool::file::types::FileImportantError;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "overwrite")]
pub struct OverwriteFileXMLRequest {
    fs_file_path: String,
    updated_content: String,
}

impl OverwriteFileXMLRequest {
    pub fn parse_response(response: &str) -> Result<Self, FileImportantError> {
        if response.is_empty() {
            return Err(FileImportantError::EmptyResponse);
        }

        let lines = response
            .lines()
            .skip_while(|line| !line.contains("<reply>"))
            .skip(1)
            .take_while(|line| !line.contains("</reply>"))
            .map(|line| line.to_owned())
            .collect::<Vec<String>>();

        let line_string = lines.join("\n");

        match from_str::<OverwriteFileXMLRequest>(&line_string) {
            Ok(parsed) => Ok(parsed),
            Err(e) => {
                eprintln!("parsing error: {:?}", e);
                Err(FileImportantError::SerdeError(crate::agentic::tool::file::types::SerdeError::new(
                    e,
                    line_string.to_owned(),
                )))
            }
        }
    }
}

pub struct OverwriteFile {
    client: Arc<Client>,
    base_url: String,
}

impl OverwriteFile {
    pub fn new(client: Arc<Client>, base_url: String) -> Self {
        Self { client, base_url }
    }

    pub async fn overwrite_file(&self, fs_file_path: String, updated_content: String) -> Result<(), anyhow::Error> {
        let url = format!("{}/api/file/overwrite_file", self.base_url);
        
        let response = self.client
            .post(&url)
            .json(&serde_json::json!({
                "file_path": fs_file_path,
                "updated_content": updated_content,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to overwrite file: {}",
                response.status()
            ));
        }

        Ok(())
    }
}