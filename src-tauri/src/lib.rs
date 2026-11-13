//! Tauri commands. Each one only locks the app state and forwards to `study_api::App`,
//! where all behaviour lives and is tested.

use std::sync::Mutex;

use chrono::{Local, NaiveDate};
use tauri::{Manager, State};

use study_api::{
    AnswerResult, ApiError, ApiResult, App, QuestionInput, QuizSetupInput, QuizView, ResultsView,
};
use study_core::bibliography::SourceFields;
use study_core::image::ImageId;
use study_core::progress::Overview;
use study_core::question::{QuestionId, SourceId, TopicId};
use study_core::results::GivenReason;
use study_api::cards::{CardsHome, NoteInput, NoteView, ReviewStep, Suggestion, TopicCards};
use study_core::cards::{CardSettings, Rating};
use study_store::cards::{CardId, NoteId};
use study_api::recall::{Candidate, CheckResult, ChecklistView, Item, PointInput, Preview, RecallHistory, RecallRow};
use study_core::recall::Schedule;
use study_api::backup::{BackupInfo, BackupSettings, BackupStatus, RestorePreview};
use study_api::calendar::{CalendarDay, DayView, DueToday};
use study_api::dates::{DatePanel, DatePreview, DateSave, DateView};
use study_store::dates::DateId;
use study_api::transfer::{ExportFile, ExportRequest, ExportSummary, ImportChoice, ImportPreview, ImportResult};
use study_api::practice::{DrillView, IssueView, ProjectRow, ProjectView, QuizQueue, ReviewChoice, StruggleInput, StruggleView, TopicQuizView};
use study_core::practice::DrillRating;
use study_store::practice::{DrillId, ProjectId, SessionId, StruggleId};
use study_store::recall::{ChecklistId, Point, Subject};
use study_store::search::{Hit, SearchQuery};
use study_store::{AttemptId, HistoryRow, ModuleId, ModuleRow, SourceRow, Store, TopicRow};

/// Converts PowerPoint to PDF for the preview. The file arrives as base64 with its name,
/// which works on every IPC path, and the PDF goes back as raw bytes. Nothing is stored.
#[tauri::command]
async fn slides_to_pdf(name: String, data: String) -> ApiResult<tauri::ipc::Response> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|_| ApiError { message: "the slides couldn't be read, try opening the file again".into() })?;
    let pdf = tauri::async_runtime::spawn_blocking(move || {
        study_api::convert::slides_to_pdf(&bytes, &name, None, std::time::Duration::from_secs(90))
    })
    .await
    .map_err(|_| ApiError { message: "the conversion stopped unexpectedly".into() })?
    .map_err(|error| ApiError { message: error.to_string() })?;
    Ok(tauri::ipc::Response::new(pdf))
}

type AppState<'a> = State<'a, Mutex<App>>;

/// Today as days since 1970-01-01 in local time, the day number the core uses.
fn today() -> i64 {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid date");
    (Local::now().date_naive() - epoch).num_days()
}

/// Now, for sessions and struggles: the day, minutes after midnight and unix seconds.
fn clock() -> study_api::practice::Clock {
    use chrono::Timelike;
    let now = Local::now();
    study_api::practice::Clock { day: today(), minute: now.hour() * 60 + now.minute(), unix: now.timestamp() }
}

fn with<T>(state: AppState, action: impl FnOnce(&mut App) -> ApiResult<T>) -> ApiResult<T> {
    // A panic in an earlier command doesn't lock the app for good (see study_api::locked).
    let mut app = study_api::locked(&state);
    action(&mut app)
}

#[tauri::command]
fn modules(state: AppState) -> ApiResult<Vec<ModuleRow>> {
    with(state, |app| app.modules())
}
#[tauri::command]
fn create_module(state: AppState, name: String) -> ApiResult<ModuleId> {
    with(state, |app| app.create_module(&name))
}
#[tauri::command]
fn topics(state: AppState) -> ApiResult<Vec<TopicRow>> {
    with(state, |app| app.topics())
}
#[tauri::command]
fn create_topic(state: AppState, name: String, module: Option<ModuleId>) -> ApiResult<TopicId> {
    with(state, |app| app.create_topic(&name, module))
}
#[tauri::command]
fn move_topic(state: AppState, topic: TopicId, module: Option<ModuleId>) -> ApiResult<()> {
    with(state, |app| app.move_topic(topic, module))
}
#[tauri::command]
fn set_topic_tags(state: AppState, topic: TopicId, tags: Vec<String>) -> ApiResult<()> {
    with(state, |app| app.set_topic_tags(topic, tags))
}
#[tauri::command]
fn delete_topic(state: AppState, topic: TopicId) -> ApiResult<()> {
    with(state, |app| app.delete_topic(topic))
}

#[tauri::command]
fn save_question(state: AppState, input: QuestionInput) -> ApiResult<QuestionId> {
    with(state, |app| app.save_question(input))
}
#[tauri::command]
fn update_question(state: AppState, id: QuestionId, input: QuestionInput) -> ApiResult<()> {
    with(state, |app| app.update_question(id, input))
}
#[tauri::command]
fn question(state: AppState, id: QuestionId) -> ApiResult<QuestionInput> {
    with(state, |app| app.question(id))
}
#[tauri::command]
fn questions_in(state: AppState, topic: TopicId) -> ApiResult<Vec<(QuestionId, String)>> {
    with(state, |app| app.questions_in(topic))
}
#[tauri::command]
fn delete_question(state: AppState, id: QuestionId) -> ApiResult<()> {
    with(state, |app| app.delete_question(id))
}

/// R7: a pasted or picked image, sent as base64 like the slides. Returns its id.
#[tauri::command]
fn add_image(state: AppState, data: String) -> ApiResult<ImageId> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|_| ApiError { message: "the image couldn't be read, try adding it again".into() })?;
    with(state, |app| app.add_image(&bytes))
}
/// The image's raw bytes, which the frontend shows through a blob: URL.
#[tauri::command]
fn image(state: AppState, id: ImageId) -> ApiResult<tauri::ipc::Response> {
    with(state, |app| app.image(id)).map(tauri::ipc::Response::new)
}

#[tauri::command]
fn add_source(state: AppState, fields: SourceFields) -> ApiResult<SourceId> {
    with(state, |app| app.add_source(fields))
}
#[tauri::command]
fn update_source(state: AppState, id: SourceId, fields: SourceFields) -> ApiResult<()> {
    with(state, |app| app.update_source(id, fields))
}
#[tauri::command]
fn delete_source(state: AppState, id: SourceId) -> ApiResult<usize> {
    with(state, |app| app.delete_source(id))
}
#[tauri::command]
fn delete_module(state: AppState, module: ModuleId) -> ApiResult<()> {
    with(state, |app| app.delete_module(module))
}
#[tauri::command]
fn sources(state: AppState) -> ApiResult<Vec<SourceRow>> {
    with(state, |app| app.sources())
}

#[tauri::command]
fn start_quiz(state: AppState, setup: QuizSetupInput) -> ApiResult<QuizView> {
    with(state, |app| app.start_quiz(setup))
}
#[tauri::command]
fn current_question(state: AppState) -> ApiResult<QuizView> {
    with(state, |app| app.current_question())
}
#[tauri::command]
fn toggle_flag(state: AppState) -> ApiResult<QuizView> {
    with(state, |app| app.toggle_flag())
}
#[tauri::command]
fn answer(state: AppState, picked: Vec<usize>) -> ApiResult<AnswerResult> {
    with(state, |app| app.answer(picked, today()))
}
/// R83: a typed answer to a short answer question.
#[tauri::command]
fn answer_text(state: AppState, typed: String) -> ApiResult<AnswerResult> {
    with(state, |app| app.answer_text(typed, today()))
}
/// R84: overrule a short answer during the quiz (attempt is set once it has finished).
#[tauri::command]
fn overrule_answer(state: AppState, attempt: Option<AttemptId>, index: usize) -> ApiResult<study_api::Feedback> {
    with(state, |app| app.overrule_answer(attempt, index))
}
#[tauri::command]
fn overrule_result(state: AppState, attempt: AttemptId, index: usize) -> ApiResult<ResultsView> {
    with(state, |app| app.overrule_result(attempt, index))
}
/// R82: how an answer would be marked, for "Try an answer". Nothing is saved.
#[tauri::command]
fn try_short_answer(check: study_core::short::Check, typed: String) -> study_core::short::Marking {
    study_api::try_short_answer(&check, &typed)
}
#[tauri::command]
fn question_timed_out(state: AppState) -> ApiResult<AnswerResult> {
    with(state, |app| app.question_timed_out(today()))
}
#[tauri::command]
fn total_timed_out(state: AppState) -> ApiResult<AttemptId> {
    with(state, |app| app.total_timed_out(today()))
}
#[tauri::command]
fn quit_quiz(state: AppState) -> ApiResult<AttemptId> {
    with(state, |app| app.quit_quiz(today()))
}
#[tauri::command]
fn retry(state: AppState, attempt: AttemptId) -> ApiResult<QuizView> {
    with(state, |app| app.retry(attempt))
}

#[tauri::command]
fn results(state: AppState, attempt: AttemptId) -> ApiResult<ResultsView> {
    with(state, |app| app.results(attempt))
}
#[tauri::command]
fn give_reason(
    state: AppState,
    attempt: AttemptId,
    index: usize,
    reason: GivenReason,
) -> ApiResult<ResultsView> {
    with(state, |app| app.give_reason(attempt, index, reason))
}
#[tauri::command]
fn history(state: AppState) -> ApiResult<Vec<HistoryRow>> {
    with(state, |app| app.history())
}
#[tauri::command]
fn overview(state: AppState) -> ApiResult<Overview> {
    with(state, |app| app.overview(today()))
}
#[tauri::command]
fn today_day() -> i64 {
    today()
}
#[tauri::command]
fn search(state: AppState, query: SearchQuery) -> ApiResult<Vec<Hit>> {
    with(state, |app| app.search(query))
}


// ---------- Flashcards (R62 to R77) ----------

#[tauri::command]
fn cards_home(state: AppState) -> ApiResult<CardsHome> {
    with(state, |app| app.cards_home(today()))
}
#[tauri::command]
fn cards_due_count(state: AppState) -> ApiResult<usize> {
    with(state, |app| app.cards_due_count(today()))
}
#[tauri::command]
fn topic_cards(state: AppState, topic: TopicId) -> ApiResult<TopicCards> {
    with(state, |app| app.topic_cards(topic, today()))
}
#[tauri::command]
fn note_view(state: AppState, id: NoteId) -> ApiResult<NoteView> {
    with(state, |app| app.note_view(id, today()))
}
#[tauri::command]
fn save_note(state: AppState, id: Option<NoteId>, input: NoteInput) -> ApiResult<NoteId> {
    with(state, |app| app.save_note(id, input, today()))
}
#[tauri::command]
fn delete_note(state: AppState, id: NoteId) -> ApiResult<()> {
    with(state, |app| app.delete_note(id))
}
#[tauri::command]
fn archive_card(state: AppState, id: CardId, archived: bool) -> ApiResult<()> {
    with(state, |app| app.archive_card(id, archived))
}
#[tauri::command]
fn start_review(state: AppState, scope: Option<Subject>) -> ApiResult<ReviewStep> {
    with(state, |app| app.start_review(scope, today()))
}
#[tauri::command]
fn current_card(state: AppState) -> ApiResult<ReviewStep> {
    with(state, |app| app.current_card(today()))
}
#[tauri::command]
fn grade_card(state: AppState, rating: Rating, seconds: u32) -> ApiResult<ReviewStep> {
    with(state, |app| app.grade_card(rating, seconds, today()))
}
#[tauri::command]
fn undo_card(state: AppState) -> ApiResult<ReviewStep> {
    with(state, |app| app.undo_card(today()))
}
#[tauri::command]
fn leave_review(state: AppState) -> ApiResult<()> {
    with(state, |app| { app.leave_review(); Ok(()) })
}
#[tauri::command]
fn card_suggestions(state: AppState) -> ApiResult<Vec<Suggestion>> {
    with(state, |app| app.card_suggestions(today()))
}
#[tauri::command]
fn dismiss_suggestion(state: AppState, key: String) -> ApiResult<()> {
    with(state, |app| app.dismiss_suggestion(&key))
}
#[tauri::command]
fn card_settings(state: AppState) -> ApiResult<CardSettings> {
    with(state, |app| app.card_settings())
}
#[tauri::command]
fn set_card_settings(state: AppState, settings: CardSettings) -> ApiResult<()> {
    with(state, |app| app.set_card_settings(settings))
}
#[tauri::command]
fn reset_card_settings(state: AppState) -> ApiResult<CardSettings> {
    with(state, |app| app.reset_card_settings())
}
#[tauri::command]
fn fit_scheduler(state: AppState) -> ApiResult<usize> {
    with(state, |app| app.fit_scheduler())
}

// ---------- Free recall (R52 to R61) ----------

#[tauri::command]
fn recall_list(state: AppState) -> ApiResult<Vec<RecallRow>> {
    with(state, |app| app.recall_list(today()))
}
#[tauri::command]
fn recall_due_count(state: AppState) -> ApiResult<usize> {
    with(state, |app| app.recall_due_count(today()))
}
#[tauri::command]
fn create_checklist(state: AppState, subject: Subject) -> ApiResult<ChecklistId> {
    with(state, |app| app.create_checklist(subject, today()))
}
#[tauri::command]
fn delete_checklist(state: AppState, id: ChecklistId) -> ApiResult<()> {
    with(state, |app| app.delete_checklist(id))
}
#[tauri::command]
fn checklist(state: AppState, id: ChecklistId) -> ApiResult<ChecklistView> {
    with(state, |app| app.checklist(id, today()))
}
#[tauri::command]
fn save_points(state: AppState, id: ChecklistId, points: Vec<PointInput>) -> ApiResult<Vec<Point>> {
    with(state, |app| app.save_points(id, points))
}
#[tauri::command]
fn recall_candidates(state: AppState, id: ChecklistId) -> ApiResult<Vec<Candidate>> {
    with(state, |app| app.recall_candidates(id))
}
#[tauri::command]
fn add_from_items(state: AppState, id: ChecklistId, add: Vec<Item>, remove: Vec<Item>) -> ApiResult<Vec<Point>> {
    with(state, |app| app.add_from_items(id, add, remove))
}
#[tauri::command]
fn preview_check(state: AppState, id: ChecklistId, recalled: usize) -> ApiResult<Preview> {
    with(state, |app| app.preview_check(id, recalled, today()))
}
#[tauri::command]
fn finish_check(state: AppState, id: ChecklistId, dump: Option<String>, recalled: Vec<usize>) -> ApiResult<CheckResult> {
    with(state, |app| app.finish_check(id, dump, recalled, today()))
}
#[tauri::command]
fn recall_history(state: AppState, id: ChecklistId) -> ApiResult<RecallHistory> {
    with(state, |app| app.recall_history(id))
}
#[tauri::command]
fn recall_schedule(state: AppState) -> ApiResult<Schedule> {
    with(state, |app| app.schedule())
}
#[tauri::command]
fn set_recall_schedule(state: AppState, schedule: Schedule) -> ApiResult<()> {
    with(state, |app| app.set_schedule(schedule))
}
#[tauri::command]
fn reset_recall_schedule(state: AppState) -> ApiResult<Schedule> {
    with(state, |app| app.reset_schedule())
}

// ---------- Projects, drills, quiz schedules and the calendar (R87 to R100) ----------

#[tauri::command]
fn projects(state: AppState) -> ApiResult<Vec<ProjectRow>> {
    with(state, |app| app.projects(today()))
}
#[tauri::command]
fn create_project(state: AppState, name: String, kind: String) -> ApiResult<ProjectId> {
    with(state, |app| app.create_project(&name, &kind, today()))
}
#[tauri::command]
fn delete_project(state: AppState, id: ProjectId) -> ApiResult<()> {
    with(state, |app| app.delete_project(id))
}
#[tauri::command]
fn project(state: AppState, id: ProjectId) -> ApiResult<ProjectView> {
    with(state, |app| app.project_view(id, today()))
}
#[tauri::command]
fn start_project_session(state: AppState, project: ProjectId) -> ApiResult<ProjectView> {
    with(state, |app| app.start_session(project, clock()))
}
#[tauri::command]
fn end_project_session(state: AppState, project: ProjectId) -> ApiResult<SessionId> {
    with(state, |app| app.end_session(project, clock()))
}
#[tauri::command]
fn log_struggle(state: AppState, project: ProjectId, input: StruggleInput) -> ApiResult<StruggleView> {
    with(state, |app| app.log_struggle(project, input, clock()))
}
#[tauri::command]
fn delete_struggle(state: AppState, id: StruggleId) -> ApiResult<()> {
    with(state, |app| app.delete_struggle(id))
}
#[tauri::command]
fn struggles_to_review(state: AppState, project: ProjectId) -> ApiResult<Vec<StruggleView>> {
    with(state, |app| app.struggles_to_review(project))
}
#[tauri::command]
fn review_struggle(state: AppState, id: StruggleId, choice: ReviewChoice) -> ApiResult<StruggleView> {
    with(state, |app| app.review_struggle(id, choice, today()))
}
#[tauri::command]
fn issues(state: AppState) -> ApiResult<Vec<IssueView>> {
    with(state, |app| app.issues())
}
#[tauri::command]
fn known_drill(state: AppState, issue: String) -> ApiResult<Option<DrillView>> {
    with(state, |app| app.known_drill(&issue))
}
#[tauri::command]
fn drill(state: AppState, id: DrillId) -> ApiResult<DrillView> {
    with(state, |app| app.drill_view(id))
}
#[tauri::command]
fn drills_due(state: AppState) -> ApiResult<Vec<DrillView>> {
    with(state, |app| app.drills_due(today()))
}
#[tauri::command]
fn rate_drill(state: AppState, id: DrillId, rating: DrillRating, seconds: u32, note: String) -> ApiResult<DrillView> {
    with(state, |app| app.rate_drill(id, rating, seconds, &note, today()))
}
#[tauri::command]
fn move_drill(state: AppState, id: DrillId, day: i64) -> ApiResult<DrillView> {
    with(state, |app| app.move_drill(id, day, today()))
}
#[tauri::command]
fn quiz_schedule(state: AppState) -> ApiResult<Schedule> {
    with(state, |app| app.quiz_schedule())
}
#[tauri::command]
fn set_quiz_schedule(state: AppState, schedule: Schedule) -> ApiResult<()> {
    with(state, |app| app.set_quiz_schedule(schedule))
}
#[tauri::command]
fn reset_quiz_schedule(state: AppState) -> ApiResult<Schedule> {
    with(state, |app| app.reset_quiz_schedule())
}
#[tauri::command]
fn topic_quizzes(state: AppState) -> ApiResult<Vec<TopicQuizView>> {
    with(state, |app| app.topic_quizzes(today()))
}
#[tauri::command]
fn calendar_month(state: AppState, from: i64, to: i64) -> ApiResult<Vec<CalendarDay>> {
    with(state, |app| app.calendar_month(from, to, today()))
}
#[tauri::command]
fn calendar_day(state: AppState, day: i64) -> ApiResult<DayView> {
    with(state, |app| app.calendar_day(day, today()))
}
#[tauri::command]
fn due_today(state: AppState) -> ApiResult<DueToday> {
    with(state, |app| app.due_today(today()))
}

#[tauri::command]
fn quiz_queue(state: AppState) -> ApiResult<QuizQueue> {
    with(state, |app| app.quiz_queue(today()))
}
#[tauri::command]
fn celebrations(state: AppState) -> ApiResult<Option<String>> {
    with(state, |app| app.celebrations())
}
#[tauri::command]
fn set_celebrations(state: AppState, json: String) -> ApiResult<()> {
    with(state, |app| app.set_celebrations(json))
}
#[tauri::command]
fn reset_celebrations(state: AppState) -> ApiResult<()> {
    with(state, |app| app.reset_celebrations())
}
#[tauri::command]
fn quizzes_a_day(state: AppState) -> ApiResult<usize> {
    with(state, |app| app.quizzes_a_day())
}
#[tauri::command]
fn set_quizzes_a_day(state: AppState, count: usize) -> ApiResult<()> {
    with(state, |app| app.set_quizzes_a_day(count))
}

// ---------- Dates on the calendar (R130 to R137) ----------

#[tauri::command]
fn dates(state: AppState) -> ApiResult<Vec<DateView>> {
    with(state, |app| app.dates(today()))
}
#[tauri::command]
fn add_date(state: AppState, date: DateSave) -> ApiResult<DateId> {
    with(state, |app| app.add_date(date))
}
#[tauri::command]
fn update_date(state: AppState, id: DateId, date: DateSave) -> ApiResult<()> {
    with(state, |app| app.update_date(id, date))
}
#[tauri::command]
fn delete_date(state: AppState, id: DateId) -> ApiResult<()> {
    with(state, |app| app.delete_date(id))
}
#[tauri::command]
fn preview_date(state: AppState, id: Option<DateId>, date: DateSave) -> ApiResult<DatePreview> {
    with(state, |app| app.preview_date(id, date, today()))
}
#[tauri::command]
fn date_panel(state: AppState, id: DateId) -> ApiResult<DatePanel> {
    with(state, |app| app.date_panel(id, today()))
}

// ---------- Backups (R101 to R108) ----------

#[tauri::command]
fn backup_status(state: AppState) -> ApiResult<BackupStatus> {
    with(state, |app| app.backup_status())
}
#[tauri::command]
fn backups(state: AppState) -> ApiResult<Vec<BackupInfo>> {
    with(state, |app| app.backups())
}
#[tauri::command]
fn back_up_now(state: AppState) -> ApiResult<BackupInfo> {
    with(state, |app| app.back_up_now(clock()))
}
#[tauri::command]
fn daily_backup(state: AppState) -> ApiResult<Option<BackupInfo>> {
    with(state, |app| app.daily_backup(clock()))
}
#[tauri::command]
fn restore_preview(state: AppState, name: String) -> ApiResult<RestorePreview> {
    with(state, |app| app.restore_preview(&name))
}
#[tauri::command]
fn restore_backup(state: AppState, name: String) -> ApiResult<BackupInfo> {
    with(state, |app| app.restore_backup(&name, clock()))
}
// ---------- Export and import (R112 to R118) ----------

#[tauri::command]
fn export_summary(state: AppState, request: ExportRequest) -> ApiResult<ExportSummary> {
    with(state, |app| app.export_summary(&request, today()))
}
/// The desktop app saves exports to Downloads and says where.
#[tauri::command]
fn export_file(handle: tauri::AppHandle, state: AppState, request: ExportRequest) -> ApiResult<ExportFile> {
    let (name, bytes) = with(state, |app| app.export(&request, today()))?;
    // The desktop's Downloads folder, or ~/Downloads when it doesn't name one.
    let folder = handle
        .path()
        .download_dir()
        .or_else(|_| handle.path().home_dir().map(|home| home.join("Downloads")))
        .map_err(|_| ApiError { message: "couldn't find your Downloads folder".into() })?;
    std::fs::create_dir_all(&folder).map_err(|error| ApiError { message: format!("couldn't make {}: {error}", folder.display()) })?;
    let path = study_api::transfer::free_path(&folder, &name);
    std::fs::write(&path, bytes).map_err(|error| ApiError { message: format!("couldn't save {}: {error}", path.display()) })?;
    Ok(ExportFile { name, path: Some(path.display().to_string()), data: None })
}
#[tauri::command]
fn import_preview(state: AppState, name: String, data: String) -> ApiResult<ImportPreview> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(data).map_err(|_| ApiError { message: "that file couldn't be read, try picking it again".into() })?;
    with(state, |app| app.import_preview(&bytes, &name))
}
#[tauri::command]
fn import_apply(state: AppState, choice: ImportChoice) -> ApiResult<ImportResult> {
    with(state, |app| app.import_apply(choice, clock()))
}
#[tauri::command]
fn cancel_import(state: AppState) -> ApiResult<()> {
    with(state, |app| {
        app.cancel_import();
        Ok(())
    })
}
#[tauri::command]
fn set_backup_settings(state: AppState, settings: BackupSettings) -> ApiResult<BackupStatus> {
    with(state, |app| app.set_backup_settings(settings, clock()))
}

#[tauri::command]
fn sync_status(state: AppState) -> ApiResult<study_api::sync::SyncStatus> {
    with(state, |app| app.sync_status(clock()))
}
#[tauri::command]
fn sync_now(state: AppState, auto: bool) -> ApiResult<study_api::sync::SyncResult> {
    with(state, |app| app.sync_now(auto, clock()))
}
#[tauri::command]
fn sync_setup(state: AppState, setup: study_api::sync::SyncSetup) -> ApiResult<study_api::sync::SyncResult> {
    with(state, |app| app.sync_setup(setup, clock()))
}
#[tauri::command]
fn sync_rename(state: AppState, name: String) -> ApiResult<study_api::sync::SyncStatus> {
    with(state, |app| {
        app.sync_rename(&name)?;
        app.sync_status(clock())
    })
}
#[tauri::command]
fn sync_off(state: AppState) -> ApiResult<study_api::sync::SyncStatus> {
    with(state, |app| app.sync_off())
}
#[tauri::command]
fn sync_resume(state: AppState) -> ApiResult<study_api::sync::SyncResult> {
    with(state, |app| app.sync_resume(clock()))
}
#[tauri::command]
fn sync_to_check(state: AppState) -> ApiResult<Vec<study_api::sync::ToCheck>> {
    with(state, |app| app.sync_to_check())
}
#[tauri::command]
fn sync_settle(state: AppState, n: i64, other: bool) -> ApiResult<Vec<study_api::sync::ToCheck>> {
    with(state, |app| app.sync_settle(n, other, clock()))
}

pub fn run() {
    tauri::Builder::default()
        .setup(|handle| {
            let dir = handle.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let database = dir.join("study.sqlite");
            let backups = dir.join("backups");
            // An update that upgrades the database copies it first (R102).
            study_api::backup::before_update(&database, &backups, clock()).map_err(|error| error.message)?;
            let store = Store::open(&database).map_err(|error| error.to_string())?;
            let mut app = App::new(store);
            app.set_backups(backups);
            app.set_sync_home(&dir);
            if let Err(error) = app.daily_backup(clock()) {
                eprintln!("couldn't make today's backup: {}", error.message);
            }
            handle.manage(Mutex::new(app));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            slides_to_pdf,
            backup_status,
            backups,
            back_up_now,
            daily_backup,
            restore_preview,
            restore_backup,
            set_backup_settings,
            export_summary,
            export_file,
            import_preview,
            import_apply,
            cancel_import,
            sync_status,
            sync_now,
            sync_setup,
            sync_rename,
            sync_off,
            sync_resume,
            sync_to_check,
            sync_settle,
            projects,
            create_project,
            delete_project,
            project,
            start_project_session,
            end_project_session,
            log_struggle,
            delete_struggle,
            struggles_to_review,
            review_struggle,
            issues,
            known_drill,
            drill,
            drills_due,
            rate_drill,
            move_drill,
            quiz_schedule,
            set_quiz_schedule,
            reset_quiz_schedule,
            topic_quizzes,
            calendar_month,
            calendar_day,
            due_today,
            quiz_queue,
            quizzes_a_day,
            celebrations,
            set_celebrations,
            reset_celebrations,
            set_quizzes_a_day,
            dates,
            add_date,
            update_date,
            delete_date,
            preview_date,
            date_panel,
            cards_home,
            cards_due_count,
            topic_cards,
            note_view,
            save_note,
            delete_note,
            archive_card,
            start_review,
            current_card,
            grade_card,
            undo_card,
            leave_review,
            card_suggestions,
            dismiss_suggestion,
            card_settings,
            set_card_settings,
            reset_card_settings,
            fit_scheduler,
            recall_list,
            recall_due_count,
            create_checklist,
            delete_checklist,
            checklist,
            save_points,
            recall_candidates,
            add_from_items,
            preview_check,
            finish_check,
            recall_history,
            recall_schedule,
            set_recall_schedule,
            reset_recall_schedule,
            modules,
            create_module,
            delete_module,
            update_source,
            delete_source,
            topics,
            create_topic,
            move_topic,
            set_topic_tags,
            delete_topic,
            save_question,
            update_question,
            question,
            questions_in,
            delete_question,
            add_image,
            image,
            add_source,
            sources,
            start_quiz,
            current_question,
            toggle_flag,
            answer,
            question_timed_out,
            answer_text,
            overrule_answer,
            overrule_result,
            try_short_answer,
            total_timed_out,
            quit_quiz,
            retry,
            results,
            give_reason,
            history,
            overview,
            today_day,
            search
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Study App");
}
