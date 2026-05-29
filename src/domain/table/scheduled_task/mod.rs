//! Scheduled task table module.
//! Exports task definition and execution record table models.

use log::LevelFilter;
use rbatis::RBatis;
use rbatis::dark_std::defer;
use rbatis::intercept_log::LogInterceptor;
use rbatis::table_sync::ColumnMapper;

/// Scheduled task table model.
pub mod scheduled_task;
pub use scheduled_task::ScheduledTask;

/// Scheduled task execution record table model.
pub mod task_execution_record;
pub use task_execution_record::TaskExecutionRecord;

pub async fn sync_scheduled_task_tables(rb: &RBatis, mapper: &dyn ColumnMapper) {
    let log_intercept = rb
        .get_intercept::<LogInterceptor>()
        .expect("not find log interceptor");
    let level = log_intercept.get_level_filter();
    log_intercept.set_level_filter(LevelFilter::Off);
    defer!(|| {
        log_intercept.set_level_filter(level);
    });

    let conn = rb.acquire().await.expect("connection database fail");

    let table = ScheduledTask::default();
    let _ = RBatis::sync(&conn, mapper, &table, "scheduled_task").await;

    let table = TaskExecutionRecord::default();
    let _ = RBatis::sync(&conn, mapper, &table, "task_execution_record").await;
}
