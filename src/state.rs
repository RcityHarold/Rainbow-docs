use std::sync::Arc;
use crate::{
    services::database::Database,
    config::Config,
    services::{
        auth::AuthService,
        spaces::SpaceService,
        space_member::SpaceMemberService,
        documents::DocumentService,
        comments::CommentService,
        publication::PublicationService,
        search::SearchService,
        versions::VersionService,
        tags::TagService,
        file_upload::FileUploadService,
        vector::VectorService,
        embedding::EmbeddingService,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub config: Config,
    pub auth_service: Arc<AuthService>,
    pub space_service: Arc<SpaceService>,
    pub space_member_service: Arc<SpaceMemberService>,
    pub file_upload_service: Arc<FileUploadService>,
    pub tag_service: Arc<TagService>,
    pub document_service: Arc<DocumentService>,
    pub comment_service: Arc<CommentService>,
    pub publication_service: Arc<PublicationService>,
    pub search_service: Arc<SearchService>,
    pub version_service: Arc<VersionService>,
    pub vector_service: Arc<VectorService>,
    pub embedding_service: Arc<EmbeddingService>,
}