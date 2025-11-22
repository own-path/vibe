use chrono::{Duration, Utc};
use tempo_cli::db::queries::{ProjectQueries, SessionQueries};
use tempo_cli::models::{Project, Session, SessionContext};
use tempo_cli::test_utils::with_test_db_async;

#[tokio::test]
async fn test_project_crud_operations() {
    with_test_db_async(|ctx| async move {
        let project_path = ctx.create_temp_project_dir()?;

        // Test project creation
        let mut project = Project::new("Test Project".to_string(), project_path.clone());
        project = project.with_description(Some("A test project".to_string()));

        let project_id = ProjectQueries::create(&ctx.connection(), &project)?;
        assert!(project_id > 0);

        // Test project retrieval by ID
        let found_project = ProjectQueries::find_by_id(&ctx.connection(), project_id)?;
        assert!(found_project.is_some());
        let found = found_project.unwrap();
        assert_eq!(found.name, "Test Project");
        assert_eq!(found.description, Some("A test project".to_string()));

        // Test project retrieval by path
        let found_by_path = ProjectQueries::find_by_path(&ctx.connection(), &project_path)?;
        assert!(found_by_path.is_some());
        assert_eq!(found_by_path.unwrap().id, Some(project_id));

        // Test project listing
        let all_projects = ProjectQueries::list_all(&ctx.connection(), false)?;
        assert_eq!(all_projects.len(), 1);
        assert_eq!(all_projects[0].id, Some(project_id));

        // Test project name update
        let updated = ProjectQueries::update_name(
            &ctx.connection(),
            project_id,
            "Updated Project".to_string(),
        )?;
        assert!(updated);

        let updated_project = ProjectQueries::find_by_id(&ctx.connection(), project_id)?.unwrap();
        assert_eq!(updated_project.name, "Updated Project");

        // Test project archiving
        let archived = ProjectQueries::update_archived(&ctx.connection(), project_id, true)?;
        assert!(archived);

        // Test that archived projects are excluded by default
        let active_projects = ProjectQueries::list_all(&ctx.connection(), false)?;
        assert_eq!(active_projects.len(), 0);

        // Test that archived projects are included when requested
        let all_including_archived = ProjectQueries::list_all(&ctx.connection(), true)?;
        assert_eq!(all_including_archived.len(), 1);
        assert!(all_including_archived[0].is_archived);

        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_session_crud_operations() {
    with_test_db_async(|ctx| async move {
        // Create a project first
        let project_path = ctx.create_temp_project_dir()?;
        let project = Project::new("Session Test Project".to_string(), project_path);
        let project_id = ProjectQueries::create(&ctx.connection(), &project)?;

        // Test session creation
        let session = Session::new(project_id, SessionContext::Terminal);
        let session_id = SessionQueries::create(&ctx.connection(), &session)?;
        assert!(session_id > 0);

        // Test finding active session
        let active_session = SessionQueries::find_active_session(&ctx.connection())?;
        assert!(active_session.is_some());
        let active = active_session.unwrap();
        assert_eq!(active.id, Some(session_id));
        assert_eq!(active.project_id, project_id);
        assert_eq!(active.context, SessionContext::Terminal);
        assert!(active.end_time.is_none());

        // Test ending session
        SessionQueries::end_session(&ctx.connection(), session_id)?;

        // Verify session is ended
        let ended_session = SessionQueries::find_by_id(&ctx.connection(), session_id)?.unwrap();
        assert!(ended_session.end_time.is_some());

        // Verify no active session exists
        let no_active = SessionQueries::find_active_session(&ctx.connection())?;
        assert!(no_active.is_none());

        // Test recent sessions listing
        let recent = SessionQueries::list_recent(&ctx.connection(), 10)?;
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, Some(session_id));

        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_session_date_range_filtering() {
    with_test_db_async(|ctx| async move {
        // Create a project
        let project_path = ctx.create_temp_project_dir()?;
        let project = Project::new("Date Range Test".to_string(), project_path);
        let project_id = ProjectQueries::create(&ctx.connection(), &project)?;

        let now = Utc::now();
        let yesterday = now - Duration::days(1);
        let tomorrow = now + Duration::days(1);

        // Create session from yesterday
        let mut old_session = Session::new(project_id, SessionContext::Terminal);
        old_session.start_time = yesterday;
        old_session.end_time = Some(yesterday + Duration::hours(2));
        let _old_id = SessionQueries::create(&ctx.connection(), &old_session)?;

        // Create session from today
        let mut today_session = Session::new(project_id, SessionContext::IDE);
        today_session.start_time = now;
        today_session.end_time = Some(now + Duration::hours(1));
        let _today_id = SessionQueries::create(&ctx.connection(), &today_session)?;

        // Test date range filtering - today only
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let today_end = now.date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc();
        let today_sessions =
            SessionQueries::list_by_date_range(&ctx.connection(), today_start, today_end)?;
        assert_eq!(today_sessions.len(), 1);
        assert_eq!(today_sessions[0].context, SessionContext::IDE);

        // Test date range filtering - yesterday only
        let yesterday_start = yesterday
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let yesterday_end = yesterday
            .date_naive()
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_utc();
        let yesterday_sessions =
            SessionQueries::list_by_date_range(&ctx.connection(), yesterday_start, yesterday_end)?;
        assert_eq!(yesterday_sessions.len(), 1);
        assert_eq!(yesterday_sessions[0].context, SessionContext::Terminal);

        // Test wide date range - should get both
        let wide_sessions = SessionQueries::list_by_date_range(
            &ctx.connection(),
            yesterday - Duration::hours(1),
            tomorrow,
        )?;
        assert_eq!(wide_sessions.len(), 2);

        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_multiple_projects_and_sessions() {
    with_test_db_async(|ctx| async move {
        // Create multiple projects
        let project1_path = ctx.create_temp_git_repo()?;
        let project1 = Project::new("Git Project".to_string(), project1_path)
            .with_description(Some("Git repository".to_string()));
        let project1_id = ProjectQueries::create(&ctx.connection(), &project1)?;

        let project2_path = ctx.create_temp_tempo_project()?;
        let project2 = Project::new("Tempo Project".to_string(), project2_path)
            .with_description(Some("Tempo tracked project".to_string()));
        let project2_id = ProjectQueries::create(&ctx.connection(), &project2)?;

        // Create sessions for each project
        let session1 = Session::new(project1_id, SessionContext::Terminal);
        let session1_id = SessionQueries::create(&ctx.connection(), &session1)?;

        let session2 = Session::new(project2_id, SessionContext::IDE);
        let session2_id = SessionQueries::create(&ctx.connection(), &session2)?;

        // End first session
        SessionQueries::end_session(&ctx.connection(), session1_id)?;

        // Verify only second session is active
        let active = SessionQueries::find_active_session(&ctx.connection())?.unwrap();
        assert_eq!(active.id, Some(session2_id));
        assert_eq!(active.project_id, project2_id);

        // Verify project listing
        let all_projects = ProjectQueries::list_all(&ctx.connection(), false)?;
        assert_eq!(all_projects.len(), 2);

        // Find projects by path
        let found1 = ProjectQueries::find_by_path(&ctx.connection(), &project1.path)?.unwrap();
        let found2 = ProjectQueries::find_by_path(&ctx.connection(), &project2.path)?.unwrap();
        assert_eq!(found1.description, Some("Git repository".to_string()));
        assert_eq!(
            found2.description,
            Some("Tempo tracked project".to_string())
        );

        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_edge_cases_and_error_conditions() {
    with_test_db_async(|ctx| async move {
        // Test finding non-existent project
        let not_found = ProjectQueries::find_by_id(&ctx.connection(), 99999)?;
        assert!(not_found.is_none());

        // Test finding non-existent session
        let no_session = SessionQueries::find_by_id(&ctx.connection(), 99999)?;
        assert!(no_session.is_none());

        // Test updating non-existent project
        let not_updated =
            ProjectQueries::update_name(&ctx.connection(), 99999, "New Name".to_string())?;
        assert!(!not_updated);

        // Test empty project list
        let empty_list = ProjectQueries::list_all(&ctx.connection(), false)?;
        assert_eq!(empty_list.len(), 0);

        // Test no active session
        let no_active = SessionQueries::find_active_session(&ctx.connection())?;
        assert!(no_active.is_none());

        // Test date range with no sessions
        let now = Utc::now();
        let empty_range =
            SessionQueries::list_by_date_range(&ctx.connection(), now, now + Duration::hours(1))?;
        assert_eq!(empty_range.len(), 0);

        Ok(())
    })
    .await;
}
