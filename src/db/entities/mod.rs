//! Database entities for Buildbot
//!
//! All entities correspond to tables in the Buildbot database schema.

pub mod masters;
pub mod builders;
pub mod builder_masters;
pub mod workers;
pub mod configured_workers;
pub mod connected_workers;
pub mod builds;
pub mod build_properties;
pub mod build_data;
pub mod build_requests;
pub mod build_request_claims;
pub mod build_sets;
pub mod build_set_properties;
pub mod build_set_sourcestamps;
pub mod source_stamps;
pub mod patches;
pub mod changes;
pub mod change_files;
pub mod change_properties;
pub mod change_users;
pub mod schedulers;
pub mod scheduler_masters;
pub mod scheduler_changes;
pub mod change_sources;
pub mod changesource_masters;
pub mod projects;
pub mod codebase;
pub mod codebase_commits;
pub mod codebase_branches;
pub mod objects;
pub mod object_state;
pub mod tags;
pub mod builder_tags;
pub mod users;
pub mod user_info;
pub mod steps;
pub mod logs;
pub mod log_chunks;
pub mod test_results;
pub mod test_result;
pub mod test_name;
pub mod test_code_path;
pub mod dispatcher_jobs;
pub mod dispatcher_runners;

// Re-export all entities
