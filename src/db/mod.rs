//! Database module for Buildbot
//!
//! This module provides SeaORM entities for all Buildbot database tables.

pub mod entities;
pub mod migrations;


use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait,
    Database as SeaDatabase, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Statement,
};
use sea_orm_migration::{SchemaManager, MigrationTrait};
use chrono::Utc;
use std::time::{SystemTime, UNIX_EPOCH};

pub use entities::{
    builds, builders, build_requests, build_sets, source_stamps, changes,
    steps, logs, log_chunks,
};
#[derive(Clone)]
pub struct Database {
    conn: DatabaseConnection,
}

impl Database {
    /// Create a new database connection
    pub async fn new(database_url: &str) -> Result<Self, DbErr> {
        let conn = SeaDatabase::connect(database_url).await?;
        Ok(Self { conn })
    }

    /// Get the underlying database connection
    pub fn connection(&self) -> DatabaseConnection {
        self.conn.clone()
    }

    /// Create a connection pool with custom settings
    pub async fn with_pool(database_url: &str, _max_connections: u32) -> Result<Self, DbErr> {
        let conn = SeaDatabase::connect(database_url).await?;
        Ok(Self { conn })
    }

    /// Check if the database connection is alive
    pub async fn ping(&self) -> Result<bool, DbErr> {
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT 1",
            vec![],
        );
        let _result = self.conn.query_one(stmt).await?;
        Ok(true)
    }

    /// Run database migrations using SeaORM migration framework
    pub async fn run_migrations(&self) -> Result<(), DbErr> {
        tracing::info!("Running SeaORM database migrations...");

        let schema = SchemaManager::new(&self.conn);

        // Run each migration individually
        let migrations: Vec<(&str, Box<dyn MigrationTrait>)> = vec![
            ("m20250101_init", Box::new(migrations::m20250101_init::Migration) as Box<dyn MigrationTrait>),
            ("m20250102_secondary", Box::new(migrations::m20250102_secondary::Migration)),
            ("m20250103_dispatcher", Box::new(migrations::m20250103_dispatcher::Migration)),
        ];

        for (name, migration) in migrations {
            tracing::info!("Applying migration {}...", name);
            if let Err(e) = migration.up(&schema).await {
                tracing::error!("Migration {} failed: {}", name, e);
                return Err(e);
            }
            tracing::info!("Migration {} applied successfully", name);
        }

        tracing::info!("All SeaORM migrations completed successfully");
        Ok(())
    }
}


pub use entities::*;

// ─────────────────────────────────────────────────────────────────
// CRUD: Builds
// ─────────────────────────────────────────────────────────────────

impl Database {
    /// Create a new build record and return its assigned ID
    pub async fn create_build(
        &self,
        builder_id: i32,
        build_request_id: i32,
        master_id: i32,
        worker_id: Option<i32>,
    ) -> Result<i32, DbErr> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let active = builds::ActiveModel {
            number: ActiveValue::Set(1), // TODO: query max number per builder
            builderid: ActiveValue::Set(builder_id),
            buildrequestid: ActiveValue::Set(build_request_id),
            workerid: ActiveValue::Set(worker_id),
            masterid: ActiveValue::Set(master_id),
            started_at: ActiveValue::Set(now),
            complete_at: ActiveValue::Set(None),
            locks_duration_s: ActiveValue::Set(0),
            state_string: ActiveValue::Set("".to_string()),
            results: ActiveValue::Set(None),
            ..Default::default()
        };

        let saved = active.insert(&self.conn).await?;
        Ok(saved.id)
    }

    /// Update a build with its final result and completion time
    pub async fn finish_build(
        &self,
        build_id: i32,
        results: i32,
        state_string: &str,
    ) -> Result<(), DbErr> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let build: builds::ActiveModel = builds::Entity::find_by_id(build_id)
            .one(&self.conn)
            .await?
            .map(|m| m.into())
            .unwrap_or_else(|| builds::ActiveModel {
                id: ActiveValue::Unchanged(build_id),
                ..Default::default()
            });

        let mut updated: builds::ActiveModel = build.into();
        updated.results = ActiveValue::Set(Some(results));
        updated.complete_at = ActiveValue::Set(Some(now));
        updated.state_string = ActiveValue::Set(state_string.to_string());
        updated.update(&self.conn).await?;
        Ok(())
    }

    /// Find builds for a builder, newest first
    pub async fn find_builds(
        &self,
        builder_id: Option<i32>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<builds::Model>, DbErr> {
        let mut query = builds::Entity::find();
        if let Some(bid) = builder_id {
            query = query.filter(builds::Column::Builderid.eq(bid));
        }
        query
            .order_by_desc(builds::Column::Id)
            .limit(limit as u64)
            .offset(offset as u64)
            .all(&self.conn)
            .await
    }

    /// Find a build by ID
    pub async fn find_build_by_id(&self, id: i32) -> Result<Option<builds::Model>, DbErr> {
        builds::Entity::find_by_id(id).one(&self.conn).await
    }

    /// Find a builder by name
    pub async fn find_builder_by_name(&self, name: &str) -> Result<Option<builders::Model>, DbErr> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        name.hash(&mut h);
        let name_hash = format!("{:x}", h.finish());
        builders::Entity::find()
            .filter(builders::Column::Name.eq(name))
            .filter(builders::Column::NameHash.eq(name_hash))
            .one(&self.conn)
            .await
    }

    /// Create or get builder by name, returns builder id
    pub async fn upsert_builder(&self, name: &str) -> Result<i32, DbErr> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        name.hash(&mut h);
        let name_hash = format!("{:x}", h.finish());

        if let Some(existing) = builders::Entity::find()
            .filter(builders::Column::Name.eq(name))
            .one(&self.conn)
            .await?
        {
            return Ok(existing.id);
        }

        let active = builders::ActiveModel {
            name: ActiveValue::Set(name.to_string()),
            name_hash: ActiveValue::Set(name_hash),
            description: ActiveValue::Set(None),
            description_format: ActiveValue::Set(None),
            description_html: ActiveValue::Set(None),
            projectid: ActiveValue::Set(None),
            ..Default::default()
        };
        let saved = active.insert(&self.conn).await?;
        Ok(saved.id)
    }

    // ─────────────────────────────────────────────────────────────
    // CRUD: Steps
    // ─────────────────────────────────────────────────────────────

    /// Create a step record and return its assigned ID
    pub async fn create_step(
        &self,
        build_id: i32,
        number: i32,
        name: &str,
    ) -> Result<i32, DbErr> {
        let active = steps::ActiveModel {
            buildid: ActiveValue::Set(build_id),
            number: ActiveValue::Set(number),
            name: ActiveValue::Set(name.to_string()),
            started_at: ActiveValue::Set(None),
            locks_acquired_at: ActiveValue::Set(None),
            complete_at: ActiveValue::Set(None),
            state_string: ActiveValue::Set("".to_string()),
            results: ActiveValue::Set(None),
            urls_json: ActiveValue::Set("{}".to_string()),
            hidden: ActiveValue::Set(0),
            ..Default::default()
        };
        let saved = active.insert(&self.conn).await?;
        Ok(saved.id)
    }

    /// Update step with result and completion time
    pub async fn finish_step(
        &self,
        step_id: i32,
        results: i32,
        state_string: &str,
    ) -> Result<(), DbErr> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let step: steps::ActiveModel = steps::Entity::find_by_id(step_id)
            .one(&self.conn)
            .await?
            .map(|m| m.into())
            .unwrap_or_else(|| steps::ActiveModel {
                id: ActiveValue::Unchanged(step_id),
                ..Default::default()
            });

        let mut updated: steps::ActiveModel = step.into();
        updated.results = ActiveValue::Set(Some(results));
        updated.complete_at = ActiveValue::Set(Some(now));
        updated.state_string = ActiveValue::Set(state_string.to_string());
        updated.update(&self.conn).await?;
        Ok(())
    }

    /// Find steps for a build
    pub async fn find_steps(&self, build_id: i32) -> Result<Vec<steps::Model>, DbErr> {
        steps::Entity::find()
            .filter(steps::Column::Buildid.eq(build_id))
            .order_by_asc(steps::Column::Number)
            .all(&self.conn)
            .await
    }

    // ─────────────────────────────────────────────────────────────
    // CRUD: Logs
    // ─────────────────────────────────────────────────────────────

    /// Create a log record and return its assigned ID
    pub async fn create_log(
        &self,
        step_id: i32,
        name: &str,
        log_type: &str,
    ) -> Result<i32, DbErr> {
        let slug = name.replace([' ', '/', '\\', '.'], "_").to_lowercase();
        let active = logs::ActiveModel {
            stepid: ActiveValue::Set(step_id),
            name: ActiveValue::Set(name.to_string()),
            slug: ActiveValue::Set(slug),
            complete: ActiveValue::Set(0),
            num_lines: ActiveValue::Set(0),
            log_type: ActiveValue::Set(log_type.to_string()),
            ..Default::default()
        };
        let saved = active.insert(&self.conn).await?;
        Ok(saved.id)
    }

    /// Append a log chunk and update num_lines
    pub async fn append_log_chunk(
        &self,
        log_id: i32,
        content: &str,
    ) -> Result<(), DbErr> {
        let lines: Vec<&str> = content.lines().collect();
        let num_lines = lines.len() as i32;

        // Get current max line number
        let last_line = log_chunks::Entity::find()
            .filter(log_chunks::Column::Logid.eq(log_id))
            .order_by_desc(log_chunks::Column::LastLine)
            .one(&self.conn)
            .await?
            .map(|c| c.last_line)
            .unwrap_or(-1);

        let first_line = last_line + 1;
        let last_line = first_line + num_lines as i32 - 1;

        let chunk = log_chunks::ActiveModel {
            logid: ActiveValue::Set(log_id),
            first_line: ActiveValue::Set(first_line),
            last_line: ActiveValue::Set(last_line),
            content: ActiveValue::Set(Some(content.as_bytes().to_vec())),
            compressed: ActiveValue::Set(0),
            ..Default::default()
        };
        chunk.insert(&self.conn).await?;

        // Update log num_lines
        let current_log = logs::Entity::find_by_id(log_id)
            .one(&self.conn)
            .await?
            .unwrap();
        let current_lines = current_log.num_lines;
        let mut updated: logs::ActiveModel = current_log.into();
        updated.num_lines = ActiveValue::Set(current_lines + num_lines);
        updated.update(&self.conn).await?;
        Ok(())
    }

    /// Mark a log as complete
    pub async fn finish_log(&self, log_id: i32) -> Result<(), DbErr> {
        let log: logs::ActiveModel = logs::Entity::find_by_id(log_id)
            .one(&self.conn)
            .await?
            .map(|m| m.into())
            .unwrap_or_else(|| logs::ActiveModel {
                id: ActiveValue::Unchanged(log_id),
                ..Default::default()
            });
        let mut updated: logs::ActiveModel = log.into();
        updated.complete = ActiveValue::Set(1);
        updated.update(&self.conn).await?;
        Ok(())
    }

    /// Get all log chunks for a log, ordered by first_line
    pub async fn get_log_content(&self, log_id: i32) -> Result<String, DbErr> {
        let chunks: Vec<log_chunks::Model> = log_chunks::Entity::find()
            .filter(log_chunks::Column::Logid.eq(log_id))
            .order_by_asc(log_chunks::Column::FirstLine)
            .all(&self.conn)
            .await?;

        let mut content = String::new();
        for chunk in chunks {
            if let Some(data) = chunk.content {
                if let Ok(s) = String::from_utf8(data) {
                    content.push_str(&s);
                }
            }
        }
        Ok(content)
    }

    /// Get all logs for a step
    pub async fn find_logs(&self, step_id: i32) -> Result<Vec<logs::Model>, DbErr> {
        logs::Entity::find()
            .filter(logs::Column::Stepid.eq(step_id))
            .all(&self.conn)
            .await
    }

    // ─────────────────────────────────────────────────────────────
    // CRUD: BuildRequests
    // ─────────────────────────────────────────────────────────────

    /// Create a build request and return its ID
    pub async fn create_build_request(
        &self,
        buildset_id: i32,
        builder_id: i32,
        priority: i32,
    ) -> Result<i32, DbErr> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let active = build_requests::ActiveModel {
            buildsetid: ActiveValue::Set(buildset_id),
            builderid: ActiveValue::Set(builder_id),
            priority: ActiveValue::Set(priority),
            complete: ActiveValue::Set(0),
            results: ActiveValue::Set(None),
            submitted_at: ActiveValue::Set(now),
            complete_at: ActiveValue::Set(None),
            waited_for: ActiveValue::Set(0),
            ..Default::default()
        };
        let saved = active.insert(&self.conn).await?;
        Ok(saved.id)
    }

    /// Find pending (incomplete) build requests for a builder
    pub async fn find_pending_build_requests(
        &self,
        builder_id: i32,
    ) -> Result<Vec<build_requests::Model>, DbErr> {
        build_requests::Entity::find()
            .filter(build_requests::Column::Builderid.eq(builder_id))
            .filter(build_requests::Column::Complete.eq(0))
            .order_by_asc(build_requests::Column::Priority)
            .order_by_asc(build_requests::Column::SubmittedAt)
            .all(&self.conn)
            .await
    }

    /// Update build request result
    pub async fn finish_build_request(
        &self,
        br_id: i32,
        results: i32,
    ) -> Result<(), DbErr> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let br: build_requests::ActiveModel = build_requests::Entity::find_by_id(br_id)
            .one(&self.conn)
            .await?
            .map(|m| m.into())
            .unwrap_or_else(|| build_requests::ActiveModel {
                id: ActiveValue::Unchanged(br_id),
                ..Default::default()
            });
        let mut updated: build_requests::ActiveModel = br.into();
        updated.complete = ActiveValue::Set(1);
        updated.results = ActiveValue::Set(Some(results));
        updated.complete_at = ActiveValue::Set(Some(now));
        updated.update(&self.conn).await?;
        Ok(())
    }

    /// Find build requests
    pub async fn find_build_requests(
        &self,
        builder_id: Option<i32>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<build_requests::Model>, DbErr> {
        let mut query = build_requests::Entity::find();
        if let Some(bid) = builder_id {
            query = query.filter(build_requests::Column::Builderid.eq(bid));
        }
        query
            .order_by_desc(build_requests::Column::SubmittedAt)
            .limit(limit as u64)
            .offset(offset as u64)
            .all(&self.conn)
            .await
    }

    /// Find build request by ID
    pub async fn find_build_request_by_id(
        &self,
        id: i32,
    ) -> Result<Option<build_requests::Model>, DbErr> {
        build_requests::Entity::find_by_id(id).one(&self.conn).await
    }

    // ─────────────────────────────────────────────────────────────
    // CRUD: BuildSets
    // ─────────────────────────────────────────────────────────────

    /// Create a buildset and return its ID
    pub async fn create_buildset(
        &self,
        reason: &str,
        external_id: Option<&str>,
    ) -> Result<i32, DbErr> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let active = build_sets::ActiveModel {
            reason: ActiveValue::Set(reason.to_string()),
            external_idstring: ActiveValue::Set(external_id.map(String::from)),
            submitted_at: ActiveValue::Set(now),
            complete: ActiveValue::Set(0),
            complete_at: ActiveValue::Set(None),
            results: ActiveValue::Set(None),
            parent_buildid: ActiveValue::Set(None),
            parent_relationship: ActiveValue::Set(None),
            rebuilt_buildid: ActiveValue::Set(None),
            ..Default::default()
        };
        let saved = active.insert(&self.conn).await?;
        Ok(saved.id)
    }

    /// Find buildsets
    pub async fn find_buildsets(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<build_sets::Model>, DbErr> {
        build_sets::Entity::find()
            .order_by_desc(build_sets::Column::SubmittedAt)
            .limit(limit as u64)
            .offset(offset as u64)
            .all(&self.conn)
            .await
    }

    /// Find buildset by ID
    pub async fn find_buildset_by_id(
        &self,
        id: i32,
    ) -> Result<Option<build_sets::Model>, DbErr> {
        build_sets::Entity::find_by_id(id).one(&self.conn).await
    }

    // ─────────────────────────────────────────────────────────────
    // CRUD: SourceStamps
    // ─────────────────────────────────────────────────────────────

    /// Create a sourcestamp and return its ID
    pub async fn create_sourcestamp(
        &self,
        repository: &str,
        branch: Option<&str>,
        revision: Option<&str>,
    ) -> Result<i32, DbErr> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let ss_hash = format!(
            "{:x}",
            {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut h = DefaultHasher::new();
                (repository, &branch, &revision).hash(&mut h);
                h.finish()
            }
        );

        let active = source_stamps::ActiveModel {
            ss_hash: ActiveValue::Set(ss_hash),
            branch: ActiveValue::Set(branch.map(String::from)),
            revision: ActiveValue::Set(revision.map(String::from)),
            patchid: ActiveValue::Set(None),
            repository: ActiveValue::Set(repository.to_string()),
            codebase: ActiveValue::Set("".to_string()),
            project: ActiveValue::Set("".to_string()),
            created_at: ActiveValue::Set(now),
            ..Default::default()
        };
        let saved = active.insert(&self.conn).await?;
        Ok(saved.id)
    }

    /// Find sourcestamps
    pub async fn find_sourcestamps(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<source_stamps::Model>, DbErr> {
        source_stamps::Entity::find()
            .order_by_desc(source_stamps::Column::CreatedAt)
            .limit(limit as u64)
            .offset(offset as u64)
            .all(&self.conn)
            .await
    }

    /// Find sourcestamp by ID
    pub async fn find_sourcestamp_by_id(
        &self,
        id: i32,
    ) -> Result<Option<source_stamps::Model>, DbErr> {
        source_stamps::Entity::find_by_id(id).one(&self.conn).await
    }
}

// ──────────────────────────────────────────────────────────────────────────────────
// Dispatcher database methods
// ──────────────────────────────────────────────────────────────────────────────────

pub use entities::dispatcher_jobs;
use crate::dispatcher::{Job, JobStatus};

/// Get current Unix timestamp
fn current_timestamp() -> i64 {
    Utc::now().timestamp()
}

impl Database {
    /// Insert a new dispatcher job into database
    pub async fn create_dispatcher_job(
        &self,
        job: &Job,
    ) -> Result<i32, DbErr> {
        let job_active_model = dispatcher_jobs::ActiveModel {
            id: ActiveValue::NotSet,
            job_id: ActiveValue::set(job.id.clone()),
            name: ActiveValue::set(job.name.clone()),
            sort_key: ActiveValue::set(job.sort_key as i32),
            status: ActiveValue::set(job.status.to_string()),
            labels: ActiveValue::set(serde_json::to_string(&job.labels).unwrap()),
            source_type: ActiveValue::set(format!("{:?}", job.source)),
            source_json: ActiveValue::set(serde_json::to_string(&job.source).unwrap()),
            repository_url: ActiveValue::set(job.repository_url.clone()),
            branch: ActiveValue::set(job.branch.clone()),
            revision: ActiveValue::set(job.revision.clone()),
            runner_name: ActiveValue::set(job.runner_name.clone()),
            env_json: ActiveValue::set(serde_json::to_string(&job.env).unwrap()),
            exit_code: ActiveValue::set(job.exit_code),
            error_message: ActiveValue::set(job.error_message.clone()),
            script_path: ActiveValue::set(job.script_path.clone()),
            workdir: ActiveValue::set(job.workdir.clone()),
            created_at: ActiveValue::set(current_timestamp()),
            updated_at: ActiveValue::set(current_timestamp()),
            started_at: ActiveValue::set(job.started_at.map(|dt| dt.timestamp())),
            finished_at: ActiveValue::set(job.finished_at.map(|dt| dt.timestamp())),
        };

        let inserted = dispatcher_jobs::Entity::insert(job_active_model)
            .exec(&self.conn)
            .await?;
        Ok(inserted.last_insert_id)
    }

    /// Update a dispatcher job in database
    pub async fn update_dispatcher_job(
        &self,
        _db_id: i32,
        job: &Job,
    ) -> Result<(), DbErr> {
        let job_search = dispatcher_jobs::Entity::find()
            .filter(dispatcher_jobs::Column::JobId.eq(&job.id))
            .one(&self.conn)
            .await?;

        if let Some(existing) = job_search {
            let mut active: dispatcher_jobs::ActiveModel = existing.into();
            active.status = ActiveValue::set(job.status.to_string());
            active.runner_name = ActiveValue::set(job.runner_name.clone());
            active.exit_code = ActiveValue::set(job.exit_code);
            active.error_message = ActiveValue::set(job.error_message.clone());
            active.updated_at = ActiveValue::set(current_timestamp());
            active.started_at = ActiveValue::set(job.started_at.map(|dt| dt.timestamp()));
            active.finished_at = ActiveValue::set(job.finished_at.map(|dt| dt.timestamp()));
            active.update(&self.conn).await?;
        }

        Ok(())
    }

    /// Update job status (for completion events)
    pub async fn update_job_status(
        &self,
        job_id: &str,
        status: &str,
        exit_code: Option<i32>,
        error_message: Option<String>,
    ) -> Result<(), DbErr> {
        let job_search = dispatcher_jobs::Entity::find()
            .filter(dispatcher_jobs::Column::JobId.eq(job_id))
            .one(&self.conn)
            .await?;

        if let Some(existing) = job_search {
            let mut active: dispatcher_jobs::ActiveModel = existing.into();
            active.status = ActiveValue::set(status.to_string());
            active.exit_code = ActiveValue::set(exit_code);
            active.error_message = ActiveValue::set(error_message);
            active.updated_at = ActiveValue::set(current_timestamp());
            active.finished_at = ActiveValue::set(Some(current_timestamp()));
            active.update(&self.conn).await?;
        }

        Ok(())
    }

    /// Find dispatcher job by UUID
    pub async fn find_dispatcher_job_by_uuid(
        &self,
        job_id: &str,
    ) -> Result<Option<dispatcher_jobs::Model>, DbErr> {
        dispatcher_jobs::Entity::find()
            .filter(dispatcher_jobs::Column::JobId.eq(job_id))
            .one(&self.conn)
            .await
    }

    /// List dispatcher jobs filtered by status
    pub async fn list_dispatcher_jobs(
        &self,
        status_filter: Option<JobStatus>,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<dispatcher_jobs::Model>, DbErr> {
        let mut query = dispatcher_jobs::Entity::find();
        if let Some(status) = status_filter {
            query = query.filter(dispatcher_jobs::Column::Status.eq(status.to_string()));
        }
        query
            .order_by_desc(dispatcher_jobs::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(&self.conn)
            .await
    }

    /// Insert a new runner into database
    pub async fn create_dispatcher_runner(
        &self,
        runner: &crate::dispatcher::Runner,
    ) -> Result<i32, DbErr> {
        let runner_active_model = dispatcher_runners::ActiveModel {
            id: ActiveValue::NotSet,
            name: ActiveValue::set(runner.name.clone()),
            runner_type: ActiveValue::set(format!("{:?}", runner.runner_type)),
            labels: ActiveValue::set(serde_json::to_string(&runner.labels).unwrap()),
            capabilities_json: ActiveValue::set(serde_json::to_string(&runner.capabilities).unwrap()),
            last_heartbeat_at: ActiveValue::set(runner.last_heartbeat_at.timestamp()),
            registered_at: ActiveValue::set(runner.registered_at.timestamp()),
            active_jobs_json: ActiveValue::set(serde_json::to_string(&runner.active_jobs).unwrap()),
            max_jobs: ActiveValue::set(runner.max_jobs as i32),
            connected: ActiveValue::set(runner.connected),
            status: ActiveValue::set(runner.status.clone()),
        };

        let inserted = dispatcher_runners::Entity::insert(runner_active_model)
            .exec(&self.conn)
            .await?;
        Ok(inserted.last_insert_id)
    }

    /// Update runner heartbeat and state
    pub async fn update_dispatcher_runner_heartbeat(
        &self,
        name: &str,
    ) -> Result<(), DbErr> {
        let runner_search = dispatcher_runners::Entity::find()
            .filter(dispatcher_runners::Column::Name.eq(name))
            .one(&self.conn)
            .await?;

        if let Some(existing) = runner_search {
            let mut active: dispatcher_runners::ActiveModel = existing.into();
            active.last_heartbeat_at = ActiveValue::set(current_timestamp());
            active.connected = ActiveValue::set(true);
            active.update(&self.conn).await?;
        }

        Ok(())
    }

    /// Mark runner disconnected
    pub async fn mark_dispatcher_runner_disconnected(
        &self,
        name: &str,
    ) -> Result<(), DbErr> {
        let runner_search = dispatcher_runners::Entity::find()
            .filter(dispatcher_runners::Column::Name.eq(name))
            .one(&self.conn)
            .await?;

        if let Some(existing) = runner_search {
            let mut active: dispatcher_runners::ActiveModel = existing.into();
            active.connected = ActiveValue::set(false);
            active.status = ActiveValue::set("offline".to_string());
            active.update(&self.conn).await?;
        }

        Ok(())
    }

    /// List all dispatcher runners
    pub async fn list_dispatcher_runners(
        &self,
    ) -> Result<Vec<dispatcher_runners::Model>, DbErr> {
        dispatcher_runners::Entity::find()
            .order_by_desc(dispatcher_runners::Column::LastHeartbeatAt)
            .all(&self.conn)
            .await
    }
}

