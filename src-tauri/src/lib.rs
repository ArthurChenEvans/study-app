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
use study_api::recall::{Candidate, CheckResult, ChecklistView, PointInput, Preview, RecallHistory, RecallRow};
use study_core::recall::Schedule;
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

fn with<T>(state: AppState, action: impl FnOnce(&mut App) -> ApiResult<T>) -> ApiResult<T> {
    let mut app = state.lock().map_err(|_| ApiError {
        message: "the app hit an internal error, please restart it".into(),
    })?;
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
fn add_from_questions(state: AppState, id: ChecklistId, add: Vec<QuestionId>, remove: Vec<QuestionId>) -> ApiResult<Vec<Point>> {
    with(state, |app| app.add_from_questions(id, add, remove))
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

pub fn run() {
    tauri::Builder::default()
        .setup(|handle| {
            let dir = handle.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let store =
                Store::open(&dir.join("study.sqlite")).map_err(|error| error.to_string())?;
            handle.manage(Mutex::new(App::new(store)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            slides_to_pdf,
            recall_list,
            recall_due_count,
            create_checklist,
            delete_checklist,
            checklist,
            save_points,
            recall_candidates,
            add_from_questions,
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
