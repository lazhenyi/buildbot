//! Dispatcher tables - GitHub Actions-style job dispatch

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Dispatcher Jobs table
        manager
            .create_table(
                Table::create()
                    .table(DispatcherJobs::Table)
                    .col(
                        ColumnDef::new(DispatcherJobs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DispatcherJobs::JobId)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(DispatcherJobs::Name).string().not_null())
                    .col(ColumnDef::new(DispatcherJobs::SortKey).integer().not_null())
                    .col(ColumnDef::new(DispatcherJobs::Status).string().not_null())
                    .col(
                        ColumnDef::new(DispatcherJobs::Labels)
                            .text()
                            .not_null()
                            .default("[]".to_string()),
                    )
                    .col(ColumnDef::new(DispatcherJobs::SourceType).text().not_null())
                    .col(
                        ColumnDef::new(DispatcherJobs::SourceJson)
                            .text()
                            .not_null()
                            .default("{}".to_string()),
                    )
                    .col(
                        ColumnDef::new(DispatcherJobs::RepositoryUrl)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(DispatcherJobs::Branch).text().not_null())
                    .col(ColumnDef::new(DispatcherJobs::Revision).text().null())
                    .col(ColumnDef::new(DispatcherJobs::RunnerName).text().null())
                    .col(
                        ColumnDef::new(DispatcherJobs::EnvJson)
                            .text()
                            .not_null()
                            .default("{}".to_string()),
                    )
                    .col(ColumnDef::new(DispatcherJobs::ExitCode).integer().null())
                    .col(ColumnDef::new(DispatcherJobs::ErrorMessage).text().null())
                    .col(ColumnDef::new(DispatcherJobs::ScriptPath).text().not_null())
                    .col(ColumnDef::new(DispatcherJobs::Workdir).text().not_null())
                    .col(
                        ColumnDef::new(DispatcherJobs::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DispatcherJobs::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DispatcherJobs::StartedAt)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DispatcherJobs::FinishedAt)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Dispatcher Runners table
        manager
            .create_table(
                Table::create()
                    .table(DispatcherRunners::Table)
                    .col(
                        ColumnDef::new(DispatcherRunners::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DispatcherRunners::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(DispatcherRunners::RunnerType)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DispatcherRunners::Labels)
                            .text()
                            .not_null()
                            .default("[]".to_string()),
                    )
                    .col(
                        ColumnDef::new(DispatcherRunners::CapabilitiesJson)
                            .text()
                            .not_null()
                            .default("{}".to_string()),
                    )
                    .col(
                        ColumnDef::new(DispatcherRunners::LastHeartbeatAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DispatcherRunners::RegisteredAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DispatcherRunners::ActiveJobsJson)
                            .text()
                            .not_null()
                            .default("[]".to_string()),
                    )
                    .col(
                        ColumnDef::new(DispatcherRunners::MaxJobs)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(DispatcherRunners::Connected)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(DispatcherRunners::Status)
                            .text()
                            .not_null()
                            .default("idle".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        // Dispatcher table indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_dispatcher_jobs_job_id")
                    .table(DispatcherJobs::Table)
                    .col(DispatcherJobs::JobId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dispatcher_jobs_status")
                    .table(DispatcherJobs::Table)
                    .col(DispatcherJobs::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dispatcher_jobs_runner_name")
                    .table(DispatcherJobs::Table)
                    .col(DispatcherJobs::RunnerName)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dispatcher_runners_name")
                    .table(DispatcherRunners::Table)
                    .col(DispatcherRunners::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dispatcher_runners_connected")
                    .table(DispatcherRunners::Table)
                    .col(DispatcherRunners::Connected)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DispatcherRunners::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DispatcherJobs::Table).to_owned())
            .await?;
        Ok(())
    }
}

// ─── Dispatcher table definitions ──────────────────────────────────────────────

#[derive(Iden)]
enum DispatcherJobs {
    Table,
    Id,
    JobId,
    Name,
    SortKey,
    Status,
    Labels,
    SourceType,
    SourceJson,
    RepositoryUrl,
    Branch,
    Revision,
    RunnerName,
    EnvJson,
    ExitCode,
    ErrorMessage,
    ScriptPath,
    Workdir,
    CreatedAt,
    UpdatedAt,
    StartedAt,
    FinishedAt,
}

#[derive(Iden)]
enum DispatcherRunners {
    Table,
    Id,
    Name,
    RunnerType,
    Labels,
    CapabilitiesJson,
    LastHeartbeatAt,
    RegisteredAt,
    ActiveJobsJson,
    MaxJobs,
    Connected,
    Status,
}
