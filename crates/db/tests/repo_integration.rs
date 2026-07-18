#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration tests"
)]
//! Postgres integration tests for the eight `SqlxRepository` trait impls.
//!
//! Each test gets its own isolated database via `#[sqlx::test]`, which
//! creates a fresh database from a template with migrations applied,
//! then drops it after the test.

use chrono::{DateTime, Utc};
use health_core::{
    CoreSession, ExerciseKind, HeartrateSample, NewApiToken, NewExerciseSession,
    NewHeartrateSamples, NewOidcState, RunningSession, WeightSession,
};
use health_db::{
    ApiTokenRepository, CoreRepository, DbError, HeartrateRepository, OidcStateRepository,
    RunningRepository, SessionsRepository, SqlxRepository, UsersRepository, WeightRepository,
};
use sqlx::PgPool;
use uuid::Uuid;

const fn repo(pool: PgPool) -> SqlxRepository {
    SqlxRepository::new(pool)
}

async fn make_user(r: &SqlxRepository) -> Uuid {
    let u = UsersRepository::upsert_by_external_id(r, "test-sub", Some("Test User"))
        .await
        .expect("upsert user");
    u.id
}

#[allow(clippy::min_ident_chars)]
fn new_session(kind: ExerciseKind) -> NewExerciseSession {
    NewExerciseSession {
        kind,
        started_at: DateTime::parse_from_rfc3339("2026-07-16T08:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        duration: std::time::Duration::from_mins(30),
        notes: Some("test session".into()),
        quality: None,
    }
}

// ---------------------------------------------------------------------------
// SessionsRepository
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn sessions_insert_get_list_delete(pool: PgPool) {
    let r = repo(pool);
    let uid = make_user(&r).await;
    let session = SessionsRepository::insert(&r, uid, &new_session(ExerciseKind::Weight))
        .await
        .unwrap();
    assert_eq!(session.user_id, uid);
    assert_eq!(session.kind, ExerciseKind::Weight);

    let fetched = SessionsRepository::get(&r, session.id).await.unwrap();
    assert_eq!(fetched.id, session.id);

    let listed = SessionsRepository::list(&r, uid, None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, session.id);

    assert!(SessionsRepository::delete(&r, session.id).await.unwrap());
    assert!(SessionsRepository::get(&r, session.id).await.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
async fn sessions_list_filters_by_kind_and_range(pool: PgPool) {
    let r = repo(pool);
    let uid = make_user(&r).await;

    let w = SessionsRepository::insert(&r, uid, &new_session(ExerciseKind::Weight))
        .await
        .unwrap();
    let c = SessionsRepository::insert(&r, uid, &new_session(ExerciseKind::Core))
        .await
        .unwrap();
    let mut run = new_session(ExerciseKind::Running);
    run.started_at = DateTime::parse_from_rfc3339("2026-07-20T08:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let run = SessionsRepository::insert(&r, uid, &run).await.unwrap();

    let weight_only =
        SessionsRepository::list(&r, uid, Some(ExerciseKind::Weight), None, None, None, None)
            .await
            .unwrap();
    assert_eq!(weight_only.len(), 1);
    assert_eq!(weight_only[0].id, w.id);

    let from = DateTime::parse_from_rfc3339("2026-07-19T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let recent = SessionsRepository::list(&r, uid, None, Some(from), None, None, None)
        .await
        .unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].id, run.id);

    let to = DateTime::parse_from_rfc3339("2026-07-17T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let early = SessionsRepository::list(&r, uid, None, None, Some(to), None, None)
        .await
        .unwrap();
    assert_eq!(early.len(), 2);

    let range = SessionsRepository::list(
        &r,
        uid,
        Some(ExerciseKind::Core),
        Some(from),
        Some(from),
        None,
        None,
    )
    .await
    .unwrap();
    assert!(range.is_empty());
    let _ = (c, run);
}

#[sqlx::test(migrations = "../../migrations")]
async fn sessions_get_not_found(pool: PgPool) {
    let r = repo(pool);
    let err = SessionsRepository::get(&r, Uuid::new_v4())
        .await
        .unwrap_err();
    assert!(matches!(err, DbError::NotFound));
}

// ---------------------------------------------------------------------------
// WeightRepository
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn weight_insert_and_get(pool: PgPool) {
    let r = repo(pool);
    let uid = make_user(&r).await;
    let s = SessionsRepository::insert(&r, uid, &new_session(ExerciseKind::Weight))
        .await
        .unwrap();

    let row = WeightSession {
        session_id: s.id,
        weight_kg: 80.0,
        sets: 3,
        quality: Some(4),
    };
    WeightRepository::insert(&r, s.id, &row).await.unwrap();
    let back = WeightRepository::get_by_session(&r, s.id).await.unwrap();
    assert!((back.weight_kg - 80.0).abs() < f64::EPSILON);
    assert_eq!(back.sets, 3);
    assert_eq!(back.quality, Some(4));
}

#[sqlx::test(migrations = "../../migrations")]
async fn weight_insert_kind_mismatch(pool: PgPool) {
    let r = repo(pool);
    let uid = make_user(&r).await;
    let s = SessionsRepository::insert(&r, uid, &new_session(ExerciseKind::Running))
        .await
        .unwrap();

    let row = WeightSession {
        session_id: s.id,
        weight_kg: 80.0,
        sets: 3,
        quality: None,
    };
    let err = WeightRepository::insert(&r, s.id, &row).await.unwrap_err();
    assert!(matches!(err, DbError::KindMismatch { .. }));
}

// ---------------------------------------------------------------------------
// CoreRepository
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn core_insert_and_get(pool: PgPool) {
    let r = repo(pool);
    let uid = make_user(&r).await;
    let s = SessionsRepository::insert(&r, uid, &new_session(ExerciseKind::Core))
        .await
        .unwrap();

    let row = CoreSession {
        session_id: s.id,
        quality: Some(3),
    };
    CoreRepository::insert(&r, s.id, &row).await.unwrap();
    let back = CoreRepository::get_by_session(&r, s.id).await.unwrap();
    assert_eq!(back.quality, Some(3));
}

// ---------------------------------------------------------------------------
// RunningRepository
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn running_insert_get_and_gpx_blob(pool: PgPool) {
    let r = repo(pool);
    let uid = make_user(&r).await;
    let s = SessionsRepository::insert(&r, uid, &new_session(ExerciseKind::Running))
        .await
        .unwrap();

    let blob = b"<gpx></gpx>".to_vec();
    let row = RunningSession {
        session_id: s.id,
        distance_m: 5_000.0,
        quality: None,
        moving_distance_m: Some(4_800.0),
        moving_time: Some(1800.0),
        gpx_data: Some(blob.clone()),
    };
    RunningRepository::insert(&r, s.id, &row).await.unwrap();

    let back = RunningRepository::get_by_session(&r, s.id).await.unwrap();
    assert!((back.distance_m - 5_000.0).abs() < f64::EPSILON);
    assert!(back.gpx_data.is_none());

    let gpx = RunningRepository::get_gpx(&r, s.id).await.unwrap();
    assert_eq!(gpx.as_deref(), Some(blob.as_slice()));

    let s2 = SessionsRepository::insert(&r, uid, &new_session(ExerciseKind::Running))
        .await
        .unwrap();
    let row2 = RunningSession {
        session_id: s2.id,
        distance_m: 100.0,
        quality: None,
        moving_distance_m: None,
        moving_time: None,
        gpx_data: None,
    };
    RunningRepository::insert(&r, s2.id, &row2).await.unwrap();
    assert_eq!(RunningRepository::get_gpx(&r, s2.id).await.unwrap(), None);
}

// ---------------------------------------------------------------------------
// HeartrateRepository
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn heartrate_bulk_insert_idempotent_and_list(pool: PgPool) {
    let r = repo(pool);
    let uid = make_user(&r).await;
    let s = SessionsRepository::insert(&r, uid, &new_session(ExerciseKind::Running))
        .await
        .unwrap();

    let samples = NewHeartrateSamples {
        session_id: s.id,
        samples: vec![
            HeartrateSample {
                session_id: s.id,
                offset_secs: 0,
                bpm: 100,
            },
            HeartrateSample {
                session_id: s.id,
                offset_secs: 10,
                bpm: 120,
            },
            HeartrateSample {
                session_id: s.id,
                offset_secs: 20,
                bpm: 140,
            },
        ],
    };
    let n = HeartrateRepository::insert_bulk(&r, &samples)
        .await
        .unwrap();
    assert_eq!(n, 3);

    let n2 = HeartrateRepository::insert_bulk(&r, &samples)
        .await
        .unwrap();
    assert_eq!(n2, 0);

    let listed = HeartrateRepository::list_for_session(&r, s.id)
        .await
        .unwrap();
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].offset_secs, 0);
    assert_eq!(listed[2].bpm, 140);
}

#[sqlx::test(migrations = "../../migrations")]
async fn heartrate_insert_bulk_empty_is_zero(pool: PgPool) {
    let r = repo(pool);
    let uid = make_user(&r).await;
    let s = SessionsRepository::insert(&r, uid, &new_session(ExerciseKind::Running))
        .await
        .unwrap();
    let empty = NewHeartrateSamples {
        session_id: s.id,
        samples: vec![],
    };
    assert_eq!(
        HeartrateRepository::insert_bulk(&r, &empty).await.unwrap(),
        0
    );
}

// ---------------------------------------------------------------------------
// UsersRepository
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn users_upsert_inserts_then_updates(pool: PgPool) {
    let r = repo(pool);

    let u1 = UsersRepository::upsert_by_external_id(&r, "sub-1", Some("Alice"))
        .await
        .unwrap();
    assert_eq!(u1.external_id, "sub-1");
    assert_eq!(u1.display_name.as_deref(), Some("Alice"));

    let u2 = UsersRepository::upsert_by_external_id(&r, "sub-1", Some("Alice Smith"))
        .await
        .unwrap();
    assert_eq!(u1.id, u2.id);
    assert_eq!(u2.display_name.as_deref(), Some("Alice Smith"));

    let fetched = UsersRepository::get(&r, u1.id).await.unwrap();
    assert_eq!(fetched.id, u1.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn users_get_not_found(pool: PgPool) {
    let r = repo(pool);
    assert!(UsersRepository::get(&r, Uuid::new_v4()).await.is_err());
}

// ---------------------------------------------------------------------------
// OidcStateRepository
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn oidc_state_insert_fetch_delete(pool: PgPool) {
    let r = repo(pool);

    let state = NewOidcState {
        csrf: "csrf-1".into(),
        nonce: "nonce-1".into(),
        code_verifier: "verifier-1".into(),
        resume_token: Some("resume-1".into()),
    };
    OidcStateRepository::insert(&r, &state).await.unwrap();

    let fetched = OidcStateRepository::fetch(&r, "csrf-1").await.unwrap();
    assert_eq!(fetched.csrf, "csrf-1");
    assert_eq!(fetched.resume_token.as_deref(), Some("resume-1"));

    OidcStateRepository::delete(&r, "csrf-1").await.unwrap();
    assert!(OidcStateRepository::fetch(&r, "csrf-1").await.is_err());

    OidcStateRepository::delete(&r, "csrf-1").await.unwrap();
}

// ---------------------------------------------------------------------------
// ApiTokenRepository
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn api_token_issue_verify_revoke_list(pool: PgPool) {
    let r = repo(pool);
    let uid = make_user(&r).await;

    let tok: NewApiToken = ApiTokenRepository::issue(&r, uid, "matrix-bot")
        .await
        .unwrap();
    assert_eq!(tok.user_id, uid);
    assert_eq!(tok.label, "matrix-bot");
    assert_eq!(tok.token.len(), 64);

    let uid2 = ApiTokenRepository::verify(&r, &tok.token).await.unwrap();
    assert_eq!(uid2, Some(uid));

    let listed = ApiTokenRepository::list_for_user(&r, uid).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].label, "matrix-bot");
    assert!(listed[0].last_used_at.is_some());

    let bad = ApiTokenRepository::verify(&r, "00deadbeef").await.unwrap();
    assert!(bad.is_none());

    assert!(ApiTokenRepository::revoke(&r, tok.id).await.unwrap());
    assert!(!ApiTokenRepository::revoke(&r, tok.id).await.unwrap());
    let listed2 = ApiTokenRepository::list_for_user(&r, uid).await.unwrap();
    assert!(listed2.is_empty());
}
