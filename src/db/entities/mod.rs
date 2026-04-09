//! Database entities for Buildbot
//!
//! All entities correspond to tables in the Buildbot database schema.

pub mod build_data;
pub mod build_properties;
pub mod build_request_claims;
pub mod build_requests;
pub mod build_set_properties;
pub mod build_set_sourcestamps;
pub mod build_sets;
pub mod builder_masters;
pub mod builder_tags;
pub mod builders;
pub mod builds;
pub mod change_files;
pub mod change_properties;
pub mod change_sources;
pub mod change_users;
pub mod changes;
pub mod changesource_masters;
pub mod codebase;
pub mod codebase_branches;
pub mod codebase_commits;
pub mod configured_workers;
pub mod connected_workers;
pub mod dispatcher_jobs;
pub mod dispatcher_runners;
pub mod log_chunks;
pub mod logs;
pub mod masters;
pub mod object_state;
pub mod objects;
pub mod patches;
pub mod projects;
pub mod scheduler_changes;
pub mod scheduler_masters;
pub mod schedulers;
pub mod source_stamps;
pub mod steps;
pub mod tags;
pub mod test_code_path;
pub mod test_name;
pub mod test_result;
pub mod test_results;
pub mod user_info;
pub mod users;
pub mod workers;

// Re-export all entities
