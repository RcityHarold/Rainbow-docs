use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUpload {
    pub id: Option<Thing>,
    pub filename: String,
    pub original_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub file_type: String,
    pub mime_type: String,
    pub uploaded_by: String,
    pub space_id: Option<Thing>,
    pub document_id: Option<Thing>,
    pub is_deleted: bool,
    pub deleted_at: Option<Datetime>,
    pub deleted_by: Option<String>,
    pub created_at: Datetime,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UploadFileRequest {
    pub space_id: Option<String>,
    pub document_id: Option<String>,
    #[validate(length(max = 500))]
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileResponse {
    pub id: String,
    pub filename: String,
    pub original_name: String,
    pub file_size: i64,
    pub file_type: String,
    pub mime_type: String,
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub space_id: Option<String>,
    pub document_id: Option<String>,
    pub uploaded_by: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct FileListResponse {
    pub files: Vec<FileResponse>,
    pub total_count: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    pub space_id: Option<String>,
    pub document_id: Option<String>,
    pub file_type: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl FileUpload {
    pub fn new(
        filename: String,
        original_name: String,
        file_path: String,
        file_size: i64,
        file_type: String,
        mime_type: String,
        uploaded_by: String,
    ) -> Self {
        Self {
            id: None,
            filename,
            original_name,
            file_path,
            file_size,
            file_type,
            mime_type,
            uploaded_by,
            space_id: None,
            document_id: None,
            is_deleted: false,
            deleted_at: None,
            deleted_by: None,
            created_at: Datetime::default(),
        }
    }

    pub fn with_space(mut self, space_id: Thing) -> Self {
        self.space_id = Some(space_id);
        self
    }

    pub fn with_document(mut self, document_id: Thing) -> Self {
        self.document_id = Some(document_id);
        self
    }

    pub fn is_image(&self) -> bool {
        self.mime_type.starts_with("image/")
    }

    pub fn get_file_extension(&self) -> Option<&str> {
        self.filename.split('.').last()
    }

    pub fn mark_deleted(&mut self, deleted_by: String) {
        self.is_deleted = true;
        self.deleted_at = Some(Datetime::default());
        self.deleted_by = Some(deleted_by);
    }
}

impl From<FileUpload> for FileResponse {
    fn from(file: FileUpload) -> Self {
        let id = file.id.as_ref().map(|t| t.to_string()).unwrap_or_default();
        let space_id = file.space_id.as_ref().map(|t| t.to_string());
        let document_id = file.document_id.as_ref().map(|t| t.to_string());
        
        // 生成文件访问URL
        let url = format!("/api/files/{}/download", id);
        let thumbnail_url = if file.is_image() {
            Some(format!("/api/files/{}/thumbnail", id))
        } else {
            None
        };

        Self {
            id,
            filename: file.filename,
            original_name: file.original_name,
            file_size: file.file_size,
            file_type: file.file_type,
            mime_type: file.mime_type,
            url,
            thumbnail_url,
            space_id,
            document_id,
            uploaded_by: file.uploaded_by,
            created_at: file.created_at.to_string(),
        }
    }
}