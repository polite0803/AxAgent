use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// ===========================================================================
// Iden enums – one per table, all columns included
// ===========================================================================

#[derive(DeriveIden)]
enum Providers {
    Table,
    Id,
    Name,
    ProviderType,
    ApiHost,
    ApiPath,
    Enabled,
    ProxyConfig,
    SortOrder,
    CreatedAt,
    UpdatedAt,
    CustomHeaders,
    Icon,
    BuiltinId,
}

#[derive(DeriveIden)]
enum ProviderKeys {
    Table,
    Id,
    ProviderId,
    KeyEncrypted,
    KeyPrefix,
    Enabled,
    LastValidatedAt,
    LastError,
    RotationIndex,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Models {
    Table,
    ProviderId,
    ModelId,
    Name,
    Capabilities,
    MaxTokens,
    Enabled,
    ParamOverrides,
    ModelType,
    GroupName,
}

#[derive(DeriveIden)]
enum Conversations {
    Table,
    Id,
    Title,
    ModelId,
    ProviderId,
    AppId,
    SystemPrompt,
    Temperature,
    MaxTokens,
    TopP,
    FrequencyPenalty,
    MessageCount,
    IsPinned,
    IsArchived,
    WorkspaceSnapshotJson,
    ActiveBranchId,
    ActiveArtifactId,
    ResearchMode,
    SearchEnabled,
    SearchProviderId,
    ThinkingBudget,
    EnabledMcpServerIds,
    EnabledKnowledgeBaseIds,
    EnabledMemoryNamespaceIds,
    CreatedAt,
    UpdatedAt,
    ContextCompression,
    CategoryId,
    ParentConversationId,
    Mode,
    WorkStrategy,
    Scenario,
    EnabledSkillIds,
    ExpertRoleId,
    WorkflowTemplateId,
    SessionType,
    WorkflowStatus,
}

#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
    ConversationId,
    Role,
    Content,
    ProviderId,
    ModelId,
    TokenCount,
    Attachments,
    Thinking,
    ParentMessageId,
    VersionIndex,
    IsActive,
    BranchId,
    ToolCallsJson,
    ToolCallId,
    CreatedAt,
    Parts,
    PromptTokens,
    CompletionTokens,
    Status,
    TokensPerSecond,
    FirstTokenLatencyMs,
}

#[derive(DeriveIden)]
enum Categories {
    Table,
    Id,
    Name,
    SortOrder,
}

#[derive(DeriveIden)]
enum Apps {
    Table,
    Id,
    Name,
    Description,
    Icon,
    IconColor,
    SystemPrompt,
    DefaultModelId,
    DefaultProviderId,
    Temperature,
    MaxTokens,
    TopP,
    CategoryId,
    IsFavorite,
    Variables,
    SearchPolicyJson,
    ToolBindingJson,
    KnowledgeBindingJson,
    MemoryPolicyJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum GatewayKeys {
    Table,
    Id,
    Name,
    KeyHash,
    KeyPrefix,
    EncryptedKey,
    Enabled,
    CreatedAt,
    LastUsedAt,
}

#[derive(DeriveIden)]
enum GatewayUsage {
    Table,
    Id,
    KeyId,
    ProviderId,
    ModelId,
    RequestTokens,
    ResponseTokens,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Settings {
    Table,
    Key,
    Value,
}

#[derive(DeriveIden)]
enum SearchProviders {
    Table,
    Id,
    Name,
    ProviderType,
    Endpoint,
    ApiKeyRef,
    Enabled,
    Region,
    Language,
    SafeSearch,
    ResultLimit,
    TimeoutMs,
}

#[derive(DeriveIden)]
enum SearchCitations {
    Table,
    Id,
    ConversationId,
    MessageId,
    Title,
    Url,
    Snippet,
    ProviderId,
    Rank,
}

#[derive(DeriveIden)]
enum McpServers {
    Table,
    Id,
    Name,
    Transport,
    Command,
    ArgsJson,
    Endpoint,
    EnvJson,
    Enabled,
    PermissionPolicy,
    Source,
    DiscoverTimeoutSecs,
    ExecuteTimeoutSecs,
    HeadersJson,
    IconType,
    IconValue,
}

#[derive(DeriveIden)]
enum ToolDescriptors {
    Table,
    Id,
    ServerId,
    Name,
    Description,
    InputSchemaJson,
}

#[derive(DeriveIden)]
enum ToolExecutions {
    Table,
    Id,
    ConversationId,
    MessageId,
    ServerId,
    ToolName,
    Status,
    InputPreview,
    OutputPreview,
    ErrorMessage,
    DurationMs,
    ApprovalStatus,
    SkillStepsJson,
    DependsOn,
    CreatedAt,
}

#[derive(DeriveIden)]
enum KnowledgeBases {
    Table,
    Id,
    Name,
    Description,
    EmbeddingProvider,
    Enabled,
    IconType,
    IconValue,
    SortOrder,
    EmbeddingDimensions,
    RetrievalThreshold,
    RetrievalTopK,
    ChunkSize,
    ChunkOverlap,
    Separator,
}

#[derive(DeriveIden)]
enum KnowledgeDocuments {
    Table,
    Id,
    KnowledgeBaseId,
    Title,
    SourcePath,
    MimeType,
    SizeBytes,
    IndexingStatus,
    DocType,
    IndexError,
    SourceConversationId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum RetrievalHits {
    Table,
    Id,
    ConversationId,
    MessageId,
    KnowledgeBaseId,
    DocumentId,
    ChunkRef,
    Score,
    Preview,
}

#[derive(DeriveIden)]
enum MemoryNamespaces {
    Table,
    Id,
    Name,
    Scope,
    AppId,
    EmbeddingProvider,
    EmbeddingDimensions,
    RetrievalThreshold,
    RetrievalTopK,
    IconType,
    IconValue,
    SortOrder,
}

#[derive(DeriveIden)]
enum MemoryItems {
    Table,
    Id,
    NamespaceId,
    Title,
    Content,
    Source,
    IndexStatus,
    IndexError,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Artifacts {
    Table,
    Id,
    ConversationId,
    Kind,
    Title,
    Content,
    Format,
    Pinned,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ContextPacks {
    Table,
    Id,
    AppId,
    Name,
    Content,
    EnabledByDefault,
}

#[derive(DeriveIden)]
enum ContextSources {
    Table,
    Id,
    ConversationId,
    MessageId,
    #[sea_orm(iden = "type")]
    SourceType,
    RefId,
    Title,
    Enabled,
    Summary,
}

#[derive(DeriveIden)]
enum ConversationBranches {
    Table,
    Id,
    ConversationId,
    ParentMessageId,
    BranchLabel,
    BranchIndex,
    ComparedMessageIdsJson,
    CreatedAt,
}

#[derive(DeriveIden)]
enum BackupManifests {
    Table,
    Id,
    Version,
    CreatedAt,
    Encrypted,
    Checksum,
    ObjectCountsJson,
    SourceAppVersion,
    FilePath,
    FileSize,
}

#[derive(DeriveIden)]
enum BackupTargets {
    Table,
    Id,
    Kind,
    ConfigJson,
}

#[derive(DeriveIden)]
enum ImportJobs {
    Table,
    Id,
    SourceType,
    Status,
    SummaryJson,
    ConflictCount,
    CreatedAt,
}

#[derive(DeriveIden)]
enum ProgramPolicies {
    Table,
    Id,
    ProgramName,
    AllowedProviderIdsJson,
    AllowedModelIdsJson,
    DefaultProviderId,
    DefaultModelId,
    RateLimitPerMinute,
}

#[derive(DeriveIden)]
enum GatewayDiagnostics {
    Table,
    Id,
    Category,
    Status,
    Message,
    CreatedAt,
}

#[derive(DeriveIden)]
enum DesktopState {
    Table,
    WindowKey,
    Width,
    Height,
    X,
    Y,
    Maximized,
    Visible,
}

#[derive(DeriveIden)]
enum StoredFiles {
    Table,
    Id,
    Hash,
    OriginalName,
    MimeType,
    SizeBytes,
    StoragePath,
    ConversationId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum GatewayRequestLogs {
    Table,
    Id,
    KeyId,
    KeyName,
    Method,
    Path,
    Model,
    ProviderId,
    StatusCode,
    DurationMs,
    RequestTokens,
    ResponseTokens,
    ErrorMessage,
    CreatedAt,
}

#[derive(DeriveIden)]
enum ConversationSummaries {
    Table,
    Id,
    ConversationId,
    SummaryText,
    CompressedUntilMessageId,
    TokenCount,
    ModelUsed,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ConversationCategories {
    Table,
    Id,
    Name,
    IconType,
    IconValue,
    SystemPrompt,
    DefaultProviderId,
    DefaultModelId,
    DefaultTemperature,
    DefaultMaxTokens,
    DefaultTopP,
    DefaultFrequencyPenalty,
    SortOrder,
    IsCollapsed,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum SkillStates {
    Table,
    Name,
    Enabled,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AgentSessions {
    Table,
    Id,
    ConversationId,
    Cwd,
    WorkspaceLocked,
    PermissionMode,
    RuntimeStatus,
    SdkContextJson,
    SdkContextBackupJson,
    TotalTokens,
    TotalCostUsd,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Wikis {
    Table,
    Id,
    Name,
    RootPath,
    SchemaVersion,
    Description,
    NoteCount,
    SourceCount,
    EmbeddingProvider,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum WikiSources {
    Table,
    Id,
    WikiId,
    SourceType,
    SourcePath,
    Title,
    MimeType,
    SizeBytes,
    ContentHash,
    MetadataJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum WikiPages {
    Table,
    Id,
    WikiId,
    NoteId,
    PageType,
    Title,
    SourceIds,
    QualityScore,
    LastLintedAt,
    LastCompiledAt,
    CompiledSourceHash,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum WikiOperations {
    Table,
    Id,
    WikiId,
    OperationType,
    TargetType,
    TargetId,
    Status,
    DetailsJson,
    ErrorMessage,
    CreatedAt,
    CompletedAt,
}

#[derive(Iden)]
enum WikiSyncQueue {
    Table,
    Id,
    WikiId,
    EventType,
    TargetType,
    TargetId,
    Payload,
    Status,
    RetryCount,
    ErrorMessage,
    CreatedAt,
    ProcessedAt,
}

#[derive(Iden)]
enum NoteLinks {
    Table,
    Id,
    VaultId,
    SourceNoteId,
    TargetNoteId,
    LinkText,
    LinkType,
    CreatedAt,
}

#[derive(Iden)]
enum NoteBacklinks {
    Table,
    Id,
    VaultId,
    SourceNoteId,
    TargetNoteId,
    LinkText,
    LinkType,
    CreatedAt,
}

#[derive(Iden)]
enum Plans {
    Table,
    Id,
    ConversationId,
    UserMessageId,
    Title,
    StepsJson,
    Status,
    IsActive,
    CreatedUnderStrategy,
    Reason,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AgencyExperts {
    Table,
    Id,
    Name,
    Description,
    Category,
    SystemPrompt,
    Color,
    SourceDir,
    IsEnabled,
    ImportedAt,
    RecommendedWorkflows,
    RecommendedTools,
}

#[derive(DeriveIden)]
enum AgentProfiles {
    Table,
    Id,
    Name,
    Description,
    Category,
    Icon,
    SystemPrompt,
    AgentRole,
    Source,
    Tags,
    SuggestedProviderId,
    SuggestedModelId,
    SuggestedTemperature,
    SuggestedMaxTokens,
    SearchEnabled,
    RecommendPermissionMode,
    RecommendedTools,
    DisallowedTools,
    RecommendedWorkflows,
    SortOrder,
    IsEnabled,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AgentRoles {
    Table,
    Id,
    Name,
    Description,
    SystemPrompt,
    DefaultTools,
    MaxConcurrent,
    TimeoutSeconds,
    Source,
    SortOrder,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum SemanticCache {
    Table,
    Id,
    PromptHash,
    Response,
    ModelId,
    TokenCount,
    TaskType,
    TtlSecs,
    CreatedAt,
    HitCount,
}

// ===========================================================================
// Migration implementation
// ===========================================================================

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // =================================================================
        // 1. Providers
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(Providers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Providers::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Providers::Name).string().not_null())
                    .col(ColumnDef::new(Providers::ProviderType).string().not_null())
                    .col(ColumnDef::new(Providers::ApiHost).string().not_null())
                    .col(ColumnDef::new(Providers::ApiPath).string().null())
                    .col(
                        ColumnDef::new(Providers::Enabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(Providers::ProxyConfig).string().null())
                    .col(
                        ColumnDef::new(Providers::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Providers::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(Providers::UpdatedAt).integer().not_null())
                    .col(ColumnDef::new(Providers::CustomHeaders).string().null())
                    .col(ColumnDef::new(Providers::Icon).string().null())
                    .col(ColumnDef::new(Providers::BuiltinId).string().null())
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 2. Provider Keys
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(ProviderKeys::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProviderKeys::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ProviderKeys::ProviderId).string().not_null())
                    .col(
                        ColumnDef::new(ProviderKeys::KeyEncrypted)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProviderKeys::KeyPrefix)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(ProviderKeys::Enabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(ProviderKeys::LastValidatedAt)
                            .integer()
                            .null(),
                    )
                    .col(ColumnDef::new(ProviderKeys::LastError).string().null())
                    .col(
                        ColumnDef::new(ProviderKeys::RotationIndex)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(ProviderKeys::CreatedAt).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(ProviderKeys::Table, ProviderKeys::ProviderId)
                            .to(Providers::Table, Providers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 3. Models (composite primary key)
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(Models::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Models::ProviderId).string().not_null())
                    .col(ColumnDef::new(Models::ModelId).string().not_null())
                    .col(ColumnDef::new(Models::Name).string().not_null())
                    .col(
                        ColumnDef::new(Models::Capabilities)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(ColumnDef::new(Models::MaxTokens).integer().null())
                    .col(
                        ColumnDef::new(Models::Enabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(Models::ParamOverrides).string().null())
                    .col(
                        ColumnDef::new(Models::ModelType)
                            .string()
                            .not_null()
                            .default("chat"),
                    )
                    .col(ColumnDef::new(Models::GroupName).string().null())
                    .primary_key(Index::create().col(Models::ProviderId).col(Models::ModelId))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Models::Table, Models::ProviderId)
                            .to(Providers::Table, Providers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 4. Conversations
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(Conversations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Conversations::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Conversations::Title).string().not_null())
                    .col(ColumnDef::new(Conversations::ModelId).string().not_null())
                    .col(
                        ColumnDef::new(Conversations::ProviderId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Conversations::AppId).string().null())
                    .col(ColumnDef::new(Conversations::SystemPrompt).string().null())
                    .col(
                        ColumnDef::new(Conversations::Temperature)
                            .float()
                            .null()
                            .to_owned(),
                    )
                    .col(ColumnDef::new(Conversations::MaxTokens).integer().null())
                    .col(
                        ColumnDef::new(Conversations::TopP)
                            .float()
                            .null()
                            .to_owned(),
                    )
                    .col(
                        ColumnDef::new(Conversations::FrequencyPenalty)
                            .float()
                            .null()
                            .to_owned(),
                    )
                    .col(
                        ColumnDef::new(Conversations::MessageCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Conversations::IsPinned)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Conversations::IsArchived)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Conversations::WorkspaceSnapshotJson)
                            .string()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(Conversations::ActiveBranchId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Conversations::ActiveArtifactId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Conversations::ResearchMode)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Conversations::SearchEnabled)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Conversations::SearchProviderId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Conversations::ThinkingBudget)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Conversations::EnabledMcpServerIds)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(Conversations::EnabledKnowledgeBaseIds)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(Conversations::EnabledMemoryNamespaceIds)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(Conversations::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Conversations::UpdatedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Conversations::ContextCompression)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Conversations::CategoryId).string().null())
                    .col(
                        ColumnDef::new(Conversations::ParentConversationId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Conversations::Mode)
                            .string()
                            .not_null()
                            .default("chat"),
                    )
                    .col(ColumnDef::new(Conversations::WorkStrategy).string().null())
                    .col(ColumnDef::new(Conversations::Scenario).string().null())
                    .col(
                        ColumnDef::new(Conversations::EnabledSkillIds)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(ColumnDef::new(Conversations::ExpertRoleId).string().null())
                    .col(
                        ColumnDef::new(Conversations::WorkflowTemplateId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Conversations::SessionType)
                            .string()
                            .not_null()
                            .default("conversation"),
                    )
                    .col(
                        ColumnDef::new(Conversations::WorkflowStatus)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 5. Messages
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(Messages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Messages::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Messages::ConversationId).string().not_null())
                    .col(ColumnDef::new(Messages::Role).string().not_null())
                    .col(ColumnDef::new(Messages::Content).string().not_null())
                    .col(ColumnDef::new(Messages::ProviderId).string().null())
                    .col(ColumnDef::new(Messages::ModelId).string().null())
                    .col(ColumnDef::new(Messages::TokenCount).integer().null())
                    .col(
                        ColumnDef::new(Messages::Attachments)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(ColumnDef::new(Messages::Thinking).string().null())
                    .col(ColumnDef::new(Messages::ParentMessageId).string().null())
                    .col(
                        ColumnDef::new(Messages::VersionIndex)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Messages::IsActive)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(Messages::BranchId).string().null())
                    .col(ColumnDef::new(Messages::ToolCallsJson).string().null())
                    .col(ColumnDef::new(Messages::ToolCallId).string().null())
                    .col(ColumnDef::new(Messages::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(Messages::Parts).string().null())
                    .col(ColumnDef::new(Messages::PromptTokens).big_integer().null())
                    .col(
                        ColumnDef::new(Messages::CompletionTokens)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Messages::Status)
                            .string()
                            .not_null()
                            .default("complete"),
                    )
                    .col(ColumnDef::new(Messages::TokensPerSecond).float().null())
                    .col(
                        ColumnDef::new(Messages::FirstTokenLatencyMs)
                            .big_integer()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Messages::Table, Messages::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 6. Categories
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(Categories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Categories::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Categories::Name).string().not_null())
                    .col(
                        ColumnDef::new(Categories::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 7. Apps
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(Apps::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Apps::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Apps::Name).string().not_null())
                    .col(
                        ColumnDef::new(Apps::Description)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Apps::Icon).string().not_null().default("🤖"))
                    .col(
                        ColumnDef::new(Apps::IconColor)
                            .string()
                            .not_null()
                            .default("#22c55e"),
                    )
                    .col(ColumnDef::new(Apps::SystemPrompt).string().not_null())
                    .col(ColumnDef::new(Apps::DefaultModelId).string().null())
                    .col(ColumnDef::new(Apps::DefaultProviderId).string().null())
                    .col(ColumnDef::new(Apps::Temperature).float().null().to_owned())
                    .col(ColumnDef::new(Apps::MaxTokens).integer().null())
                    .col(ColumnDef::new(Apps::TopP).float().null().to_owned())
                    .col(ColumnDef::new(Apps::CategoryId).string().null())
                    .col(
                        ColumnDef::new(Apps::IsFavorite)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Apps::Variables)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(ColumnDef::new(Apps::SearchPolicyJson).string().null())
                    .col(ColumnDef::new(Apps::ToolBindingJson).string().null())
                    .col(ColumnDef::new(Apps::KnowledgeBindingJson).string().null())
                    .col(ColumnDef::new(Apps::MemoryPolicyJson).string().null())
                    .col(ColumnDef::new(Apps::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(Apps::UpdatedAt).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Apps::Table, Apps::CategoryId)
                            .to(Categories::Table, Categories::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 8. Gateway Keys
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(GatewayKeys::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GatewayKeys::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GatewayKeys::Name).string().not_null())
                    .col(
                        ColumnDef::new(GatewayKeys::KeyHash)
                            .string()
                            .not_null()
                            .unique_key()
                            .to_owned(),
                    )
                    .col(ColumnDef::new(GatewayKeys::KeyPrefix).string().not_null())
                    .col(ColumnDef::new(GatewayKeys::EncryptedKey).string().null())
                    .col(
                        ColumnDef::new(GatewayKeys::Enabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(GatewayKeys::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(GatewayKeys::LastUsedAt).integer().null())
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 9. Gateway Usage
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(GatewayUsage::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GatewayUsage::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key()
                            .to_owned(),
                    )
                    .col(ColumnDef::new(GatewayUsage::KeyId).string().not_null())
                    .col(ColumnDef::new(GatewayUsage::ProviderId).string().not_null())
                    .col(ColumnDef::new(GatewayUsage::ModelId).string().null())
                    .col(
                        ColumnDef::new(GatewayUsage::RequestTokens)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(GatewayUsage::ResponseTokens)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(GatewayUsage::CreatedAt).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(GatewayUsage::Table, GatewayUsage::KeyId)
                            .to(GatewayKeys::Table, GatewayKeys::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 10. Settings
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(Settings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Settings::Key)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Settings::Value).string().not_null())
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 11. Search Providers
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(SearchProviders::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SearchProviders::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SearchProviders::Name).string().not_null())
                    .col(
                        ColumnDef::new(SearchProviders::ProviderType)
                            .string()
                            .not_null()
                            .default("tavily"),
                    )
                    .col(ColumnDef::new(SearchProviders::Endpoint).string().null())
                    .col(ColumnDef::new(SearchProviders::ApiKeyRef).string().null())
                    .col(
                        ColumnDef::new(SearchProviders::Enabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(SearchProviders::Region).string().null())
                    .col(ColumnDef::new(SearchProviders::Language).string().null())
                    .col(ColumnDef::new(SearchProviders::SafeSearch).integer().null())
                    .col(
                        ColumnDef::new(SearchProviders::ResultLimit)
                            .integer()
                            .not_null()
                            .default(10),
                    )
                    .col(
                        ColumnDef::new(SearchProviders::TimeoutMs)
                            .integer()
                            .not_null()
                            .default(5000),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_search_providers_enabled")
                    .table(SearchProviders::Table)
                    .col(SearchProviders::Enabled)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 12. Search Citations
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(SearchCitations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SearchCitations::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SearchCitations::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SearchCitations::MessageId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(SearchCitations::Title).string().not_null())
                    .col(ColumnDef::new(SearchCitations::Url).string().not_null())
                    .col(ColumnDef::new(SearchCitations::Snippet).string().null())
                    .col(
                        ColumnDef::new(SearchCitations::ProviderId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SearchCitations::Rank)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(SearchCitations::Table, SearchCitations::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_search_citations_conv")
                    .table(SearchCitations::Table)
                    .col(SearchCitations::ConversationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_search_citations_msg")
                    .table(SearchCitations::Table)
                    .col(SearchCitations::MessageId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 13. MCP Servers
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(McpServers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(McpServers::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(McpServers::Name).string().not_null())
                    .col(
                        ColumnDef::new(McpServers::Transport)
                            .string()
                            .not_null()
                            .default("stdio"),
                    )
                    .col(ColumnDef::new(McpServers::Command).string().null())
                    .col(ColumnDef::new(McpServers::ArgsJson).string().null())
                    .col(ColumnDef::new(McpServers::Endpoint).string().null())
                    .col(ColumnDef::new(McpServers::EnvJson).string().null())
                    .col(
                        ColumnDef::new(McpServers::Enabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(McpServers::PermissionPolicy)
                            .string()
                            .not_null()
                            .default("ask"),
                    )
                    .col(
                        ColumnDef::new(McpServers::Source)
                            .string()
                            .not_null()
                            .default("custom"),
                    )
                    .col(
                        ColumnDef::new(McpServers::DiscoverTimeoutSecs)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(McpServers::ExecuteTimeoutSecs)
                            .integer()
                            .null(),
                    )
                    .col(ColumnDef::new(McpServers::HeadersJson).string().null())
                    .col(ColumnDef::new(McpServers::IconType).string().null())
                    .col(ColumnDef::new(McpServers::IconValue).string().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_mcp_servers_enabled")
                    .table(McpServers::Table)
                    .col(McpServers::Enabled)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 14. Tool Descriptors
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(ToolDescriptors::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ToolDescriptors::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ToolDescriptors::ServerId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ToolDescriptors::Name).string().not_null())
                    .col(ColumnDef::new(ToolDescriptors::Description).string().null())
                    .col(
                        ColumnDef::new(ToolDescriptors::InputSchemaJson)
                            .string()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ToolDescriptors::Table, ToolDescriptors::ServerId)
                            .to(McpServers::Table, McpServers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_tool_descriptors_server")
                    .table(ToolDescriptors::Table)
                    .col(ToolDescriptors::ServerId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 15. Tool Executions
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(ToolExecutions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ToolExecutions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ToolExecutions::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ToolExecutions::MessageId).string().null())
                    .col(ColumnDef::new(ToolExecutions::ServerId).string().not_null())
                    .col(ColumnDef::new(ToolExecutions::ToolName).string().not_null())
                    .col(
                        ColumnDef::new(ToolExecutions::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(ToolExecutions::InputPreview).string().null())
                    .col(
                        ColumnDef::new(ToolExecutions::OutputPreview)
                            .string()
                            .null(),
                    )
                    .col(ColumnDef::new(ToolExecutions::ErrorMessage).string().null())
                    .col(ColumnDef::new(ToolExecutions::DurationMs).integer().null())
                    .col(
                        ColumnDef::new(ToolExecutions::ApprovalStatus)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ToolExecutions::SkillStepsJson)
                            .string()
                            .null(),
                    )
                    .col(ColumnDef::new(ToolExecutions::DependsOn).string().null())
                    .col(
                        ColumnDef::new(ToolExecutions::CreatedAt)
                            .string()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ToolExecutions::Table, ToolExecutions::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_tool_executions_conv")
                    .table(ToolExecutions::Table)
                    .col(ToolExecutions::ConversationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_tool_executions_msg")
                    .table(ToolExecutions::Table)
                    .col(ToolExecutions::MessageId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_tool_executions_server")
                    .table(ToolExecutions::Table)
                    .col(ToolExecutions::ServerId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 16. Knowledge Bases
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(KnowledgeBases::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(KnowledgeBases::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(KnowledgeBases::Name).string().not_null())
                    .col(ColumnDef::new(KnowledgeBases::Description).string().null())
                    .col(
                        ColumnDef::new(KnowledgeBases::EmbeddingProvider)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeBases::Enabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(KnowledgeBases::IconType).string().null())
                    .col(ColumnDef::new(KnowledgeBases::IconValue).string().null())
                    .col(
                        ColumnDef::new(KnowledgeBases::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(KnowledgeBases::EmbeddingDimensions)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeBases::RetrievalThreshold)
                            .float()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeBases::RetrievalTopK)
                            .integer()
                            .null(),
                    )
                    .col(ColumnDef::new(KnowledgeBases::ChunkSize).integer().null())
                    .col(
                        ColumnDef::new(KnowledgeBases::ChunkOverlap)
                            .integer()
                            .null(),
                    )
                    .col(ColumnDef::new(KnowledgeBases::Separator).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_knowledge_bases_enabled")
                    .table(KnowledgeBases::Table)
                    .col(KnowledgeBases::Enabled)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 17. Knowledge Documents
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(KnowledgeDocuments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(KnowledgeDocuments::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::KnowledgeBaseId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::Title)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::SourcePath)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::MimeType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::SizeBytes)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::IndexingStatus)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::DocType)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::IndexError)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::SourceConversationId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::CreatedAt)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(KnowledgeDocuments::UpdatedAt)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                KnowledgeDocuments::Table,
                                KnowledgeDocuments::KnowledgeBaseId,
                            )
                            .to(KnowledgeBases::Table, KnowledgeBases::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_knowledge_documents_kb")
                    .table(KnowledgeDocuments::Table)
                    .col(KnowledgeDocuments::KnowledgeBaseId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 18. Retrieval Hits
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(RetrievalHits::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RetrievalHits::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(RetrievalHits::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RetrievalHits::MessageId).string().not_null())
                    .col(
                        ColumnDef::new(RetrievalHits::KnowledgeBaseId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RetrievalHits::DocumentId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RetrievalHits::ChunkRef).string().not_null())
                    .col(
                        ColumnDef::new(RetrievalHits::Score)
                            .float()
                            .not_null()
                            .default(0.0),
                    )
                    .col(ColumnDef::new(RetrievalHits::Preview).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetrievalHits::Table, RetrievalHits::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RetrievalHits::Table, RetrievalHits::KnowledgeBaseId)
                            .to(KnowledgeBases::Table, KnowledgeBases::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_retrieval_hits_conv")
                    .table(RetrievalHits::Table)
                    .col(RetrievalHits::ConversationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_retrieval_hits_msg")
                    .table(RetrievalHits::Table)
                    .col(RetrievalHits::MessageId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_retrieval_hits_kb")
                    .table(RetrievalHits::Table)
                    .col(RetrievalHits::KnowledgeBaseId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 19. Memory Namespaces
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(MemoryNamespaces::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MemoryNamespaces::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MemoryNamespaces::Name).string().not_null())
                    .col(
                        ColumnDef::new(MemoryNamespaces::Scope)
                            .string()
                            .not_null()
                            .default("global"),
                    )
                    .col(ColumnDef::new(MemoryNamespaces::AppId).string().null())
                    .col(
                        ColumnDef::new(MemoryNamespaces::EmbeddingProvider)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(MemoryNamespaces::EmbeddingDimensions)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(MemoryNamespaces::RetrievalThreshold)
                            .float()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(MemoryNamespaces::RetrievalTopK)
                            .integer()
                            .null(),
                    )
                    .col(ColumnDef::new(MemoryNamespaces::IconType).string().null())
                    .col(ColumnDef::new(MemoryNamespaces::IconValue).string().null())
                    .col(
                        ColumnDef::new(MemoryNamespaces::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_memory_namespaces_scope")
                    .table(MemoryNamespaces::Table)
                    .col(MemoryNamespaces::Scope)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 20. Memory Items
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(MemoryItems::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MemoryItems::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MemoryItems::NamespaceId).string().not_null())
                    .col(ColumnDef::new(MemoryItems::Title).string().not_null())
                    .col(ColumnDef::new(MemoryItems::Content).string().not_null())
                    .col(
                        ColumnDef::new(MemoryItems::Source)
                            .string()
                            .not_null()
                            .default("manual"),
                    )
                    .col(
                        ColumnDef::new(MemoryItems::IndexStatus)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(MemoryItems::IndexError).string().null())
                    .col(
                        ColumnDef::new(MemoryItems::UpdatedAt)
                            .string()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(MemoryItems::Table, MemoryItems::NamespaceId)
                            .to(MemoryNamespaces::Table, MemoryNamespaces::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_memory_items_ns")
                    .table(MemoryItems::Table)
                    .col(MemoryItems::NamespaceId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 21. Artifacts
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(Artifacts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Artifacts::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Artifacts::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Artifacts::Kind)
                            .string()
                            .not_null()
                            .default("draft"),
                    )
                    .col(ColumnDef::new(Artifacts::Title).string().not_null())
                    .col(
                        ColumnDef::new(Artifacts::Content)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(Artifacts::Format)
                            .string()
                            .not_null()
                            .default("markdown"),
                    )
                    .col(
                        ColumnDef::new(Artifacts::Pinned)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Artifacts::UpdatedAt)
                            .string()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Artifacts::Table, Artifacts::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_artifacts_conv")
                    .table(Artifacts::Table)
                    .col(Artifacts::ConversationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_artifacts_pinned")
                    .table(Artifacts::Table)
                    .col(Artifacts::Pinned)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 22. Context Packs
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(ContextPacks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ContextPacks::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ContextPacks::AppId).string().not_null())
                    .col(ColumnDef::new(ContextPacks::Name).string().not_null())
                    .col(
                        ColumnDef::new(ContextPacks::Content)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(ContextPacks::EnabledByDefault)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ContextPacks::Table, ContextPacks::AppId)
                            .to(Apps::Table, Apps::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_context_packs_app")
                    .table(ContextPacks::Table)
                    .col(ContextPacks::AppId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 23. Context Sources
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(ContextSources::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ContextSources::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ContextSources::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ContextSources::MessageId).string().null())
                    .col(
                        ColumnDef::new(ContextSources::SourceType)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ContextSources::RefId).string().not_null())
                    .col(ColumnDef::new(ContextSources::Title).string().not_null())
                    .col(
                        ColumnDef::new(ContextSources::Enabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(ContextSources::Summary).string().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(ContextSources::Table, ContextSources::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_context_sources_conv")
                    .table(ContextSources::Table)
                    .col(ContextSources::ConversationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_context_sources_msg")
                    .table(ContextSources::Table)
                    .col(ContextSources::MessageId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 24. Conversation Branches
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(ConversationBranches::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ConversationBranches::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ConversationBranches::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationBranches::ParentMessageId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationBranches::BranchLabel)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationBranches::BranchIndex)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ConversationBranches::ComparedMessageIdsJson)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationBranches::CreatedAt)
                            .string()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                ConversationBranches::Table,
                                ConversationBranches::ConversationId,
                            )
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_conv_branches_parent")
                    .table(ConversationBranches::Table)
                    .col(ConversationBranches::ParentMessageId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 25. Backup Manifests
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(BackupManifests::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BackupManifests::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(BackupManifests::Version).string().not_null())
                    .col(
                        ColumnDef::new(BackupManifests::CreatedAt)
                            .string()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(BackupManifests::Encrypted)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(BackupManifests::Checksum)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BackupManifests::ObjectCountsJson)
                            .string()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(BackupManifests::SourceAppVersion)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(BackupManifests::FilePath).string().null())
                    .col(
                        ColumnDef::new(BackupManifests::FileSize)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 26. Backup Targets
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(BackupTargets::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BackupTargets::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(BackupTargets::Kind)
                            .string()
                            .not_null()
                            .default("local"),
                    )
                    .col(
                        ColumnDef::new(BackupTargets::ConfigJson)
                            .string()
                            .not_null()
                            .default("{}"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_backup_targets_kind")
                    .table(BackupTargets::Table)
                    .col(BackupTargets::Kind)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 27. Import Jobs
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(ImportJobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ImportJobs::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ImportJobs::SourceType).string().not_null())
                    .col(
                        ColumnDef::new(ImportJobs::Status)
                            .string()
                            .not_null()
                            .default("scanning"),
                    )
                    .col(ColumnDef::new(ImportJobs::SummaryJson).string().null())
                    .col(
                        ColumnDef::new(ImportJobs::ConflictCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ImportJobs::CreatedAt)
                            .string()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_import_jobs_status")
                    .table(ImportJobs::Table)
                    .col(ImportJobs::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_import_jobs_created")
                    .table(ImportJobs::Table)
                    .col(ImportJobs::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 28. Program Policies
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(ProgramPolicies::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProgramPolicies::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ProgramPolicies::ProgramName)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(ProgramPolicies::AllowedProviderIdsJson)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(ProgramPolicies::AllowedModelIdsJson)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(ProgramPolicies::DefaultProviderId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProgramPolicies::DefaultModelId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProgramPolicies::RateLimitPerMinute)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_program_policies_name")
                    .table(ProgramPolicies::Table)
                    .col(ProgramPolicies::ProgramName)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 29. Gateway Diagnostics
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(GatewayDiagnostics::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GatewayDiagnostics::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GatewayDiagnostics::Category)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GatewayDiagnostics::Status)
                            .string()
                            .not_null()
                            .default("ok"),
                    )
                    .col(
                        ColumnDef::new(GatewayDiagnostics::Message)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GatewayDiagnostics::CreatedAt)
                            .string()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_gateway_diagnostics_cat")
                    .table(GatewayDiagnostics::Table)
                    .col(GatewayDiagnostics::Category)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_gateway_diagnostics_created")
                    .table(GatewayDiagnostics::Table)
                    .col(GatewayDiagnostics::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 30. Desktop State
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(DesktopState::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DesktopState::WindowKey)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DesktopState::Width)
                            .integer()
                            .not_null()
                            .default(1200),
                    )
                    .col(
                        ColumnDef::new(DesktopState::Height)
                            .integer()
                            .not_null()
                            .default(800),
                    )
                    .col(ColumnDef::new(DesktopState::X).integer().null())
                    .col(ColumnDef::new(DesktopState::Y).integer().null())
                    .col(
                        ColumnDef::new(DesktopState::Maximized)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(DesktopState::Visible)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 31. Stored Files
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(StoredFiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StoredFiles::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(StoredFiles::Hash).string().not_null())
                    .col(
                        ColumnDef::new(StoredFiles::OriginalName)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StoredFiles::MimeType)
                            .string()
                            .not_null()
                            .default("application/octet-stream"),
                    )
                    .col(ColumnDef::new(StoredFiles::SizeBytes).integer().not_null())
                    .col(ColumnDef::new(StoredFiles::StoragePath).string().not_null())
                    .col(ColumnDef::new(StoredFiles::ConversationId).string().null())
                    .col(
                        ColumnDef::new(StoredFiles::CreatedAt)
                            .string()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(StoredFiles::Table, StoredFiles::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_stored_files_hash")
                    .table(StoredFiles::Table)
                    .col(StoredFiles::Hash)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_stored_files_conversation")
                    .table(StoredFiles::Table)
                    .col(StoredFiles::ConversationId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 32. Gateway Request Logs
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(GatewayRequestLogs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GatewayRequestLogs::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GatewayRequestLogs::KeyId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GatewayRequestLogs::KeyName)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GatewayRequestLogs::Method)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GatewayRequestLogs::Path).string().not_null())
                    .col(ColumnDef::new(GatewayRequestLogs::Model).string().null())
                    .col(
                        ColumnDef::new(GatewayRequestLogs::ProviderId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(GatewayRequestLogs::StatusCode)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GatewayRequestLogs::DurationMs)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GatewayRequestLogs::RequestTokens)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(GatewayRequestLogs::ResponseTokens)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(GatewayRequestLogs::ErrorMessage)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(GatewayRequestLogs::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // FTS5 virtual table and triggers (raw SQL for SQLite-specific features)
        // =================================================================
        let db = manager.get_connection();

        db.execute_unprepared(
            "CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(\
                content, \
                content=messages, \
                content_rowid=rowid, \
                tokenize='unicode61'\
            )",
        )
        .await?;

        // =================================================================
        // 33. Conversation Summaries
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(ConversationSummaries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ConversationSummaries::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::SummaryText)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::CompressedUntilMessageId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::TokenCount)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::ModelUsed)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::UpdatedAt)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_conversation_summaries_conversation")
                    .table(ConversationSummaries::Table)
                    .col(ConversationSummaries::ConversationId)
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 34. Conversation Categories
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(ConversationCategories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ConversationCategories::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::Name)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::IconType)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::IconValue)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::SystemPrompt)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::DefaultProviderId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::DefaultModelId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::DefaultTemperature)
                            .double()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::DefaultMaxTokens)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::DefaultTopP)
                            .double()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::DefaultFrequencyPenalty)
                            .double()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::IsCollapsed)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationCategories::UpdatedAt)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 35. Skill States
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(SkillStates::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SkillStates::Name)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SkillStates::Enabled)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(SkillStates::UpdatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // 36. Agent Sessions
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(AgentSessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AgentSessions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AgentSessions::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AgentSessions::Cwd).string().null())
                    .col(
                        ColumnDef::new(AgentSessions::WorkspaceLocked)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(AgentSessions::PermissionMode)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AgentSessions::RuntimeStatus)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AgentSessions::SdkContextJson).text().null())
                    .col(
                        ColumnDef::new(AgentSessions::SdkContextBackupJson)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgentSessions::TotalTokens)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(AgentSessions::TotalCostUsd)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(ColumnDef::new(AgentSessions::CreatedAt).string().not_null())
                    .col(ColumnDef::new(AgentSessions::UpdatedAt).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_agent_sessions_conversation")
                    .table(AgentSessions::Table)
                    .col(AgentSessions::ConversationId)
                    .to_owned(),
            )
            .await?;

        db.execute_unprepared(
            "CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN \
                INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content); \
            END",
        )
        .await?;

        db.execute_unprepared(
            "CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN \
                INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content); \
            END",
        )
        .await?;

        db.execute_unprepared(
            "CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE OF content ON messages BEGIN \
                INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content); \
                INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content); \
            END",
        )
        .await?;

        // =================================================================
        // Wiki Tables
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(Wikis::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Wikis::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Wikis::Name).string().not_null())
                    .col(ColumnDef::new(Wikis::RootPath).string().not_null())
                    .col(
                        ColumnDef::new(Wikis::SchemaVersion)
                            .string()
                            .not_null()
                            .default("1.0"),
                    )
                    .col(ColumnDef::new(Wikis::Description).string().null())
                    .col(
                        ColumnDef::new(Wikis::NoteCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Wikis::SourceCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Wikis::EmbeddingProvider).string().null())
                    .col(ColumnDef::new(Wikis::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(Wikis::UpdatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WikiSources::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WikiSources::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WikiSources::WikiId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(WikiSources::Table, WikiSources::WikiId)
                            .to(Wikis::Table, Wikis::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(WikiSources::SourceType).string().not_null())
                    .col(ColumnDef::new(WikiSources::SourcePath).string().not_null())
                    .col(ColumnDef::new(WikiSources::Title).string().not_null())
                    .col(ColumnDef::new(WikiSources::MimeType).string().not_null())
                    .col(
                        ColumnDef::new(WikiSources::SizeBytes)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WikiSources::ContentHash).string().not_null())
                    .col(ColumnDef::new(WikiSources::MetadataJson).json().null())
                    .col(ColumnDef::new(WikiSources::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(WikiSources::UpdatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WikiPages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WikiPages::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WikiPages::WikiId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(WikiPages::Table, WikiPages::WikiId)
                            .to(Wikis::Table, Wikis::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(WikiPages::NoteId).string().not_null())
                    .col(ColumnDef::new(WikiPages::PageType).string().not_null())
                    .col(ColumnDef::new(WikiPages::Title).string().not_null())
                    .col(ColumnDef::new(WikiPages::SourceIds).json().null())
                    .col(ColumnDef::new(WikiPages::QualityScore).decimal().null())
                    .col(ColumnDef::new(WikiPages::LastLintedAt).integer().null())
                    .col(ColumnDef::new(WikiPages::LastCompiledAt).integer().null())
                    .col(
                        ColumnDef::new(WikiPages::CompiledSourceHash)
                            .string()
                            .null(),
                    )
                    .col(ColumnDef::new(WikiPages::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(WikiPages::UpdatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WikiOperations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WikiOperations::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(WikiOperations::WikiId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(WikiOperations::Table, WikiOperations::WikiId)
                            .to(Wikis::Table, Wikis::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(WikiOperations::OperationType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WikiOperations::TargetType)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WikiOperations::TargetId).string().not_null())
                    .col(ColumnDef::new(WikiOperations::Status).string().not_null())
                    .col(ColumnDef::new(WikiOperations::DetailsJson).json().null())
                    .col(ColumnDef::new(WikiOperations::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(WikiOperations::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WikiOperations::CompletedAt).integer().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WikiSyncQueue::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WikiSyncQueue::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(WikiSyncQueue::WikiId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(WikiSyncQueue::Table, WikiSyncQueue::WikiId)
                            .to(Wikis::Table, Wikis::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(WikiSyncQueue::EventType).string().not_null())
                    .col(
                        ColumnDef::new(WikiSyncQueue::TargetType)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WikiSyncQueue::TargetId).string().not_null())
                    .col(ColumnDef::new(WikiSyncQueue::Payload).json().null())
                    .col(
                        ColumnDef::new(WikiSyncQueue::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(WikiSyncQueue::RetryCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(WikiSyncQueue::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(WikiSyncQueue::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WikiSyncQueue::ProcessedAt).integer().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(NoteLinks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NoteLinks::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(NoteLinks::VaultId).string().not_null())
                    .col(ColumnDef::new(NoteLinks::SourceNoteId).string().not_null())
                    .col(ColumnDef::new(NoteLinks::TargetNoteId).string().not_null())
                    .col(ColumnDef::new(NoteLinks::LinkText).string().null())
                    .col(ColumnDef::new(NoteLinks::LinkType).string().not_null())
                    .col(ColumnDef::new(NoteLinks::CreatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(NoteBacklinks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NoteBacklinks::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(NoteBacklinks::VaultId).string().not_null())
                    .col(
                        ColumnDef::new(NoteBacklinks::SourceNoteId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NoteBacklinks::TargetNoteId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(NoteBacklinks::LinkText).string().null())
                    .col(ColumnDef::new(NoteBacklinks::LinkType).string().not_null())
                    .col(
                        ColumnDef::new(NoteBacklinks::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // =================================================================
        // Plans table — stores execution plans for the plan work strategy
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(Plans::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Plans::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Plans::ConversationId).string().not_null())
                    .col(ColumnDef::new(Plans::UserMessageId).string().not_null())
                    .col(ColumnDef::new(Plans::Title).string().not_null())
                    .col(
                        ColumnDef::new(Plans::StepsJson)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(Plans::Status)
                            .string()
                            .not_null()
                            .default("draft"),
                    )
                    .col(
                        ColumnDef::new(Plans::IsActive)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(Plans::CreatedUnderStrategy).string().null())
                    .col(ColumnDef::new(Plans::Reason).string().null())
                    .col(ColumnDef::new(Plans::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(Plans::UpdatedAt).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Plans::Table, Plans::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Add work_strategy column for existing databases
        // Use raw SQL to safely add the column if it doesn't exist
        let db = manager.get_connection();
        db.execute_unprepared("ALTER TABLE conversations ADD COLUMN work_strategy TEXT")
            .await
            .ok(); // Silently ignore if column already exists

        // Agency experts table — imported from
        manager
            .create_table(
                Table::create()
                    .table(AgencyExperts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AgencyExperts::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AgencyExperts::Name).string().not_null())
                    .col(ColumnDef::new(AgencyExperts::Description).string().null())
                    .col(ColumnDef::new(AgencyExperts::Category).string().not_null())
                    .col(
                        ColumnDef::new(AgencyExperts::SystemPrompt)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AgencyExperts::Color).string().null())
                    .col(ColumnDef::new(AgencyExperts::SourceDir).string().not_null())
                    .col(
                        ColumnDef::new(AgencyExperts::IsEnabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(AgencyExperts::ImportedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AgencyExperts::RecommendedWorkflows)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgencyExperts::RecommendedTools)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add agency expert columns for existing databases
        db.execute_unprepared("ALTER TABLE agency_experts ADD COLUMN recommended_workflows TEXT")
            .await
            .ok();
        db.execute_unprepared("ALTER TABLE agency_experts ADD COLUMN recommended_tools TEXT")
            .await
            .ok();

        // =================================================================
        // 37. Agent Profiles (融合 ExpertRole + AgentRole 的智能体能力集)
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(AgentProfiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AgentProfiles::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AgentProfiles::Name).string().not_null())
                    .col(ColumnDef::new(AgentProfiles::Description).string().null())
                    .col(
                        ColumnDef::new(AgentProfiles::Category)
                            .string()
                            .not_null()
                            .default("general"),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::Icon)
                            .string()
                            .not_null()
                            .default("🤖"),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::SystemPrompt)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(AgentProfiles::AgentRole).string().null())
                    .col(
                        ColumnDef::new(AgentProfiles::Source)
                            .string()
                            .not_null()
                            .default("builtin"),
                    )
                    .col(ColumnDef::new(AgentProfiles::Tags).string().null())
                    .col(
                        ColumnDef::new(AgentProfiles::SuggestedProviderId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::SuggestedModelId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::SuggestedTemperature)
                            .double()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::SuggestedMaxTokens)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::SearchEnabled)
                            .boolean()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::RecommendPermissionMode)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::RecommendedTools)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::DisallowedTools)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::RecommendedWorkflows)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::IsEnabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AgentProfiles::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Seed built-in agent profiles (12 presets: General, CodeReviewer, Developer, etc.)
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            let builtin_profiles: Vec<(
                &str, &str, &str, &str, &str, &str, Option<&str>, &str,
            )> = vec![
                ("general-assistant", "通用助手", "全能型 AI 助手", "general", "🤖", "", Some("coordinator"), "builtin"),
                ("code-reviewer", "代码审查专家", "专注代码审查和安全", "development", "🔍", "你是一位代码审查专家。仔细审查代码的正确性、安全性、性能和可维护性。检查潜在的 bug、安全漏洞、不良实践和代码异味。提供具体、可操作的改进建议。", Some("reviewer"), "builtin"),
                ("senior-developer", "高级开发工程师", "精通多语言，负责核心功能开发", "development", "⚡", "你是一位高级开发工程师。精通多种编程语言和框架。编写清晰、高效、可维护的代码。遵循最佳实践和设计模式。在实现新功能时考虑边界情况和错误处理。", Some("developer"), "builtin"),
                ("security-auditor", "安全审计专家", "负责安全检查与审计", "security", "🛡️", "你是一位安全审计专家。审查代码和系统配置的安全漏洞。关注 OWASP Top 10、注入攻击、认证缺陷、敏感数据泄露等问题。提供安全加固建议。", Some("reviewer"), "builtin"),
                ("data-analyst", "数据分析师", "负责数据分析和洞察", "data", "📊", "你是一位数据分析师。擅长使用统计方法和数据可视化工具分析数据。从数据中提取有意义的模式、趋势和洞察。清晰地呈现分析结果和结论。", Some("researcher"), "builtin"),
                ("sql-expert", "SQL 专家", "精通 SQL 查询和数据库设计", "data", "🗄️", "你是一位 SQL 专家。精通 SQL 查询优化、数据库设计和数据建模。编写高效、安全的 SQL 查询。考虑索引策略、查询计划和并发控制。", Some("researcher"), "builtin"),
                ("devops-engineer", "DevOps 工程师", "负责 CI/CD 和运维自动化", "devops", "🚀", "你是一位 DevOps 工程师。负责 CI/CD 流水线、基础设施即代码、容器化和监控。编写可靠的部署脚本和配置管理。重视自动化和可重复性。", Some("executor"), "builtin"),
                ("tech-writer", "技术文档专家", "撰写技术文档和使用手册", "writing", "📝", "你是一位技术文档专家。撰写清晰、准确、易于理解的技术文档。将复杂的技术概念转化为通俗易懂的描述。组织文档结构使其易于导航和搜索。", Some("synthesizer"), "builtin"),
                ("product-manager", "产品经理", "负责产品规划和需求分析", "business", "🎯", "你是一位产品经理。负责产品规划和需求分析。从用户角度思考问题，定义产品功能和优先级。平衡技术可行性和业务需求。", Some("coordinator"), "builtin"),
                ("architect", "系统架构师", "负责系统架构设计和技术选型", "development", "🏗️", "你是一位系统架构师。负责系统架构设计、技术选型和架构评审。考虑可扩展性、可靠性、性能和成本。设计清晰的模块边界和接口契约。", Some("planner"), "builtin"),
                ("debug-expert", "调试专家", "专业的 bug 排查和问题分析", "development", "🐛", "你是一位调试专家。系统性地分析 bug 报告和错误日志。使用科学方法定位问题根因。考虑多层依赖关系。验证修复方案并确保不引入新问题。", Some("developer"), "builtin"),
                ("translator", "翻译专家", "专业的多语言翻译服务", "writing", "🌐", "你是一位翻译专家。精通多种语言的翻译。准确传达原文的含义和语气。注意文化差异和上下文。保持翻译的自然流畅。", Some("synthesizer"), "builtin"),
            ];

            for (id, name, desc, cat, icon, prompt, role, src) in &builtin_profiles {
                let exists = db
                    .query_one(sea_orm::Statement::from_sql_and_values(
                        sea_orm::DatabaseBackend::Sqlite,
                        "SELECT id FROM agent_profiles WHERE id = ?",
                        vec![(*id).into()],
                    ))
                    .await
                    .map(|r| r.is_some())
                    .unwrap_or(false);

                if !exists {
                    db.execute(sea_orm::Statement::from_sql_and_values(
                        sea_orm::DatabaseBackend::Sqlite,
                        "INSERT INTO agent_profiles (id, name, description, category, icon, system_prompt, agent_role, source, sort_order, is_enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, 1, ?, ?)",
                        vec![
                            (*id).into(),
                            (*name).into(),
                            (*desc).into(),
                            (*cat).into(),
                            (*icon).into(),
                            (*prompt).into(),
                            (*role).into(),
                            (*src).into(),
                            now.into(),
                            now.into(),
                        ],
                    )).await.ok();
                }
            }
        }

        // =================================================================
        // 38. Agent Roles (数据库驱动的角色定义，可从外部导入)
        manager
            .create_table(
                Table::create()
                    .table(AgentRoles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AgentRoles::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AgentRoles::Name).string().not_null())
                    .col(ColumnDef::new(AgentRoles::Description).string().null())
                    .col(
                        ColumnDef::new(AgentRoles::SystemPrompt)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(AgentRoles::DefaultTools).string().null())
                    .col(
                        ColumnDef::new(AgentRoles::MaxConcurrent)
                            .integer()
                            .not_null()
                            .default(3),
                    )
                    .col(
                        ColumnDef::new(AgentRoles::TimeoutSeconds)
                            .big_integer()
                            .not_null()
                            .default(600),
                    )
                    .col(
                        ColumnDef::new(AgentRoles::Source)
                            .string()
                            .not_null()
                            .default("builtin"),
                    )
                    .col(
                        ColumnDef::new(AgentRoles::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(AgentRoles::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AgentRoles::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Seed 8 built-in roles
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let roles: Vec<(&str, &str, &str, &[&str], usize, u64)> = vec![
                (
                    "coordinator",
                    "Coordinator",
                    "任务分解、工作者分配和结果综合",
                    &[
                        "web_search",
                        "read_file",
                        "list_directory",
                        "search_files",
                        "grep_content",
                        "skill_manage",
                        "session_search",
                        "memory_flush",
                        "get_system_info",
                        "get_storage_info",
                        "list_storage_files",
                    ],
                    1,
                    300,
                ),
                (
                    "researcher",
                    "Researcher",
                    "信息收集、数据分析和综合研究",
                    &[
                        "web_search",
                        "fetch_url",
                        "fetch_markdown",
                        "read_file",
                        "list_directory",
                        "search_files",
                        "grep_content",
                        "search_knowledge",
                        "list_knowledge_bases",
                        "session_search",
                        "list_storage_files",
                        "download_storage_file",
                    ],
                    4,
                    600,
                ),
                (
                    "developer",
                    "Developer",
                    "编写、编辑和重构代码",
                    &[
                        "write_file",
                        "edit_file",
                        "search_replace",
                        "read_file",
                        "list_directory",
                        "search_files",
                        "grep_content",
                        "run_command",
                        "file_exists",
                        "get_file_info",
                        "create_directory",
                        "delete_file",
                        "move_file",
                        "get_system_info",
                        "list_processes",
                        "get_storage_info",
                        "list_storage_files",
                        "upload_storage_file",
                        "download_storage_file",
                        "delete_storage_file",
                        "git_status",
                        "git_diff",
                        "git_commit",
                        "git_log",
                        "git_branch",
                        "git_review",
                    ],
                    3,
                    900,
                ),
                (
                    "reviewer",
                    "Reviewer",
                    "评估工作质量、提供建设性反馈",
                    &[
                        "read_file",
                        "list_directory",
                        "search_files",
                        "grep_content",
                        "run_command",
                        "file_exists",
                        "get_file_info",
                        "get_system_info",
                        "list_processes",
                        "git_status",
                        "git_diff",
                        "git_log",
                        "git_review",
                    ],
                    2,
                    600,
                ),
                (
                    "browser",
                    "Browser",
                    "与网页交互、填充表单和验证视觉内容",
                    &["fetch_url", "fetch_markdown", "web_search"],
                    3,
                    300,
                ),
                (
                    "synthesizer",
                    "Synthesizer",
                    "聚合多个 Agent 的结果为统一输出",
                    &[
                        "write_file",
                        "read_file",
                        "list_directory",
                        "search_files",
                        "grep_content",
                    ],
                    1,
                    180,
                ),
                (
                    "planner",
                    "Planner",
                    "战略思维、风险评估和计划制定",
                    &[
                        "read_file",
                        "list_directory",
                        "search_files",
                        "grep_content",
                        "web_search",
                        "session_search",
                        "memory_flush",
                        "get_system_info",
                        "get_storage_info",
                        "list_storage_files",
                    ],
                    2,
                    300,
                ),
                (
                    "executor",
                    "Executor",
                    "精确执行离散任务",
                    &[
                        "run_command",
                        "write_file",
                        "edit_file",
                        "read_file",
                        "list_directory",
                        "search_files",
                        "grep_content",
                        "create_directory",
                        "delete_file",
                        "move_file",
                        "file_exists",
                        "get_system_info",
                        "list_processes",
                        "upload_storage_file",
                        "download_storage_file",
                        "delete_storage_file",
                    ],
                    5,
                    600,
                ),
            ];
            for (id, name, desc, tools, mc, to) in &roles {
                let exists = db
                    .query_one(sea_orm::Statement::from_sql_and_values(
                        sea_orm::DatabaseBackend::Sqlite,
                        "SELECT id FROM agent_roles WHERE id = ?",
                        vec![(*id).into()],
                    ))
                    .await
                    .map(|r| r.is_some())
                    .unwrap_or(false);
                if !exists {
                    db.execute(sea_orm::Statement::from_sql_and_values(
                        sea_orm::DatabaseBackend::Sqlite,
                        "INSERT INTO agent_roles (id, name, description, system_prompt, default_tools, max_concurrent, timeout_seconds, source, sort_order, created_at, updated_at) VALUES (?, ?, ?, '', ?, ?, ?, 'builtin', 0, ?, ?)",
                        vec![
                            (*id).into(), (*name).into(), (*desc).into(),
                            serde_json::to_string(tools).unwrap_or_default().into(),
                            (*mc as i32).into(), (*to as i64).into(),
                            now.into(), now.into(),
                        ],
                    )).await.ok();
                }
            }
        }

        // 39. Semantic Cache
        // =================================================================
        manager
            .create_table(
                Table::create()
                    .table(SemanticCache::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SemanticCache::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SemanticCache::PromptHash)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(SemanticCache::Response).string().not_null())
                    .col(ColumnDef::new(SemanticCache::ModelId).string().null())
                    .col(
                        ColumnDef::new(SemanticCache::TokenCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SemanticCache::TaskType)
                            .string()
                            .not_null()
                            .default("moderate"),
                    )
                    .col(ColumnDef::new(SemanticCache::TtlSecs).integer().not_null())
                    .col(
                        ColumnDef::new(SemanticCache::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SemanticCache::HitCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_semantic_cache_hash")
                    .table(SemanticCache::Table)
                    .col(SemanticCache::PromptHash)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_semantic_cache_created")
                    .table(SemanticCache::Table)
                    .col(SemanticCache::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop triggers and FTS5 table first
        let db = manager.get_connection();
        db.execute_unprepared("DROP TRIGGER IF EXISTS messages_au")
            .await?;
        db.execute_unprepared("DROP TRIGGER IF EXISTS messages_ad")
            .await?;
        db.execute_unprepared("DROP TRIGGER IF EXISTS messages_ai")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS messages_fts")
            .await?;

        // Drop tables in reverse creation order
        macro_rules! drop_tbl {
            ($t:expr) => {
                manager
                    .drop_table(Table::drop().table($t).if_exists().to_owned())
                    .await?;
            };
        }
        drop_tbl!(SemanticCache::Table);
        drop_tbl!(GatewayRequestLogs::Table);
        drop_tbl!(AgentSessions::Table);
        drop_tbl!(SkillStates::Table);
        drop_tbl!(ConversationCategories::Table);
        drop_tbl!(ConversationSummaries::Table);
        drop_tbl!(StoredFiles::Table);
        drop_tbl!(DesktopState::Table);
        drop_tbl!(GatewayDiagnostics::Table);
        drop_tbl!(ProgramPolicies::Table);
        drop_tbl!(ImportJobs::Table);
        drop_tbl!(BackupTargets::Table);
        drop_tbl!(BackupManifests::Table);
        drop_tbl!(ConversationBranches::Table);
        drop_tbl!(ContextSources::Table);
        drop_tbl!(ContextPacks::Table);
        drop_tbl!(Artifacts::Table);
        drop_tbl!(MemoryItems::Table);
        drop_tbl!(MemoryNamespaces::Table);
        drop_tbl!(RetrievalHits::Table);
        drop_tbl!(KnowledgeDocuments::Table);
        drop_tbl!(KnowledgeBases::Table);
        drop_tbl!(ToolExecutions::Table);
        drop_tbl!(ToolDescriptors::Table);
        drop_tbl!(McpServers::Table);
        drop_tbl!(SearchCitations::Table);
        drop_tbl!(SearchProviders::Table);
        drop_tbl!(Settings::Table);
        drop_tbl!(GatewayUsage::Table);
        drop_tbl!(GatewayKeys::Table);
        drop_tbl!(Apps::Table);
        drop_tbl!(Categories::Table);
        drop_tbl!(Messages::Table);
        drop_tbl!(Plans::Table);
        drop_tbl!(AgencyExperts::Table);
        drop_tbl!(AgentProfiles::Table);
        drop_tbl!(AgentRoles::Table);
        drop_tbl!(Conversations::Table);
        drop_tbl!(Models::Table);
        drop_tbl!(ProviderKeys::Table);
        drop_tbl!(Providers::Table);

        // Drop wiki tables in reverse creation order
        drop_tbl!(NoteBacklinks::Table);
        drop_tbl!(NoteLinks::Table);
        drop_tbl!(WikiSyncQueue::Table);
        drop_tbl!(WikiOperations::Table);
        drop_tbl!(WikiPages::Table);
        drop_tbl!(WikiSources::Table);
        drop_tbl!(Wikis::Table);

        Ok(())
    }
}
