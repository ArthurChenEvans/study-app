//! Every command the frontend calls, by name, forwarding to `study_api::App` exactly as
//! src-tauri/src/lib.rs does. Arguments arrive as the same JSON object Tauri's `invoke` gets.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use study_api::{ApiError, ApiResult, App};

/// What a command sends back: JSON, or raw bytes for images and PDFs.
pub enum Reply {
    Json(Value),
    Bytes(Vec<u8>),
}

/// One argument from the JSON object. A missing key reads as null, so optional ones can be left out.
fn arg<T: DeserializeOwned>(args: &Value, name: &str) -> ApiResult<T> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null))
        .map_err(|error| ApiError::new(format!("the {name} argument is missing or wrong: {error}")))
}

fn json<T: Serialize>(result: ApiResult<T>) -> ApiResult<Reply> {
    result.and_then(|value| serde_json::to_value(value).map(Reply::Json).map_err(ApiError::new))
}

fn base64(args: &Value, what: &str) -> ApiResult<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(arg::<String>(args, "data")?)
        .map_err(|_| ApiError::new(format!("the {what} couldn't be read, try adding it again")))
}

/// Runs one command. `slides_to_pdf` is handled by the server, since it doesn't need the app.
pub fn dispatch(app: &mut App, command: &str, args: &Value, today: i64) -> ApiResult<Reply> {
    dispatch_at(app, command, args, today, crate::clock())
}

/// The same, with the moment given, for sessions and struggles (and tests).
pub fn dispatch_at(app: &mut App, command: &str, args: &Value, today: i64, clock: study_api::practice::Clock) -> ApiResult<Reply> {
    let a = args;
    match command {
        "modules" => json(app.modules()),
        "create_module" => json(app.create_module(&arg::<String>(a, "name")?)),
        "delete_module" => json(app.delete_module(arg(a, "module")?)),
        "topics" => json(app.topics()),
        "create_topic" => json(app.create_topic(&arg::<String>(a, "name")?, arg(a, "module")?)),
        "move_topic" => json(app.move_topic(arg(a, "topic")?, arg(a, "module")?)),
        "set_topic_tags" => json(app.set_topic_tags(arg(a, "topic")?, arg(a, "tags")?)),
        "delete_topic" => json(app.delete_topic(arg(a, "topic")?)),

        "save_question" => json(app.save_question(arg(a, "input")?)),
        "update_question" => json(app.update_question(arg(a, "id")?, arg(a, "input")?)),
        "question" => json(app.question(arg(a, "id")?)),
        "questions_in" => json(app.questions_in(arg(a, "topic")?)),
        "delete_question" => json(app.delete_question(arg(a, "id")?)),
        "add_image" => json(app.add_image(&base64(a, "image")?)),
        "image" => app.image(arg(a, "id")?).map(Reply::Bytes),

        "add_source" => json(app.add_source(arg(a, "fields")?)),
        "sources" => json(app.sources()),
        "update_source" => json(app.update_source(arg(a, "id")?, arg(a, "fields")?)),
        "delete_source" => json(app.delete_source(arg(a, "id")?)),

        "start_quiz" => json(app.start_quiz(arg(a, "setup")?)),
        "current_question" => json(app.current_question()),
        "toggle_flag" => json(app.toggle_flag()),
        "answer" => json(app.answer(arg(a, "picked")?, today)),
        "question_timed_out" => json(app.question_timed_out(today)),
        "answer_text" => json(app.answer_text(arg(a, "typed")?, today)),
        "overrule_answer" => json(app.overrule_answer(arg(a, "attempt")?, arg(a, "index")?)),
        "overrule_result" => json(app.overrule_result(arg(a, "attempt")?, arg(a, "index")?)),
        "try_short_answer" => json(Ok(study_api::try_short_answer(&arg(a, "check")?, &arg::<String>(a, "typed")?))),
        "total_timed_out" => json(app.total_timed_out(today)),
        "quit_quiz" => json(app.quit_quiz(today)),
        "retry" => json(app.retry(arg(a, "attempt")?)),
        "results" => json(app.results(arg(a, "attempt")?)),
        "give_reason" => json(app.give_reason(arg(a, "attempt")?, arg(a, "index")?, arg(a, "reason")?)),
        "history" => json(app.history()),
        "overview" => json(app.overview(today)),
        "today_day" => json(Ok(today)),
        "search" => json(app.search(arg(a, "query")?)),

        "recall_list" => json(app.recall_list(today)),
        "recall_due_count" => json(app.recall_due_count(today)),
        "create_checklist" => json(app.create_checklist(arg(a, "subject")?, today)),
        "delete_checklist" => json(app.delete_checklist(arg(a, "id")?)),
        "checklist" => json(app.checklist(arg(a, "id")?, today)),
        "save_points" => json(app.save_points(arg(a, "id")?, arg(a, "points")?)),
        "recall_candidates" => json(app.recall_candidates(arg(a, "id")?)),
        "add_from_items" => json(app.add_from_items(arg(a, "id")?, arg(a, "add")?, arg(a, "remove")?)),
        "preview_check" => json(app.preview_check(arg(a, "id")?, arg(a, "recalled")?, today)),
        "finish_check" => json(app.finish_check(arg(a, "id")?, arg(a, "dump")?, arg(a, "recalled")?, today)),
        "recall_history" => json(app.recall_history(arg(a, "id")?)),
        "recall_schedule" => json(app.schedule()),
        "set_recall_schedule" => json(app.set_schedule(arg(a, "schedule")?)),
        "reset_recall_schedule" => json(app.reset_schedule()),

        "cards_home" => json(app.cards_home(today)),
        "cards_due_count" => json(app.cards_due_count(today)),
        "topic_cards" => json(app.topic_cards(arg(a, "topic")?, today)),
        "note_view" => json(app.note_view(arg(a, "id")?, today)),
        "save_note" => json(app.save_note(arg(a, "id")?, arg(a, "input")?, today)),
        "delete_note" => json(app.delete_note(arg(a, "id")?)),
        "archive_card" => json(app.archive_card(arg(a, "id")?, arg(a, "archived")?)),
        "start_review" => json(app.start_review(arg(a, "scope")?, today)),
        "current_card" => json(app.current_card(today)),
        "grade_card" => json(app.grade_card(arg(a, "rating")?, arg(a, "seconds")?, today)),
        "undo_card" => json(app.undo_card(today)),
        "leave_review" => {
            app.leave_review();
            json(Ok(()))
        }
        "card_suggestions" => json(app.card_suggestions(today)),
        "dismiss_suggestion" => json(app.dismiss_suggestion(&arg::<String>(a, "key")?)),
        "card_settings" => json(app.card_settings()),
        "set_card_settings" => json(app.set_card_settings(arg(a, "settings")?)),
        "reset_card_settings" => json(app.reset_card_settings()),
        "fit_scheduler" => json(app.fit_scheduler()),

        "projects" => json(app.projects(today)),
        "create_project" => json(app.create_project(&arg::<String>(a, "name")?, &arg::<String>(a, "kind")?, today)),
        "delete_project" => json(app.delete_project(arg(a, "id")?)),
        "project" => json(app.project_view(arg(a, "id")?, today)),
        "backup_status" => json(app.backup_status()),
        "backups" => json(app.backups()),
        "back_up_now" => json(app.back_up_now(clock)),
        "daily_backup" => json(app.daily_backup(clock)),
        "restore_preview" => json(app.restore_preview(&arg::<String>(a, "name")?)),
        "restore_backup" => json(app.restore_backup(&arg::<String>(a, "name")?, clock)),
        "set_backup_settings" => json(app.set_backup_settings(arg(a, "settings")?, clock)),
        "export_summary" => json(app.export_summary(&arg(a, "request")?, today)),
        // The browser saves the file itself, so it comes back as base64.
        "export_file" => json(app.export(&arg(a, "request")?, today).map(|(name, bytes)| {
            use base64::Engine;
            study_api::transfer::ExportFile { name, path: None, data: Some(base64::engine::general_purpose::STANDARD.encode(bytes)) }
        })),
        "import_preview" => json(app.import_preview(&base64(a, "file")?, &arg::<String>(a, "name")?)),
        "import_apply" => json(app.import_apply(arg(a, "choice")?, clock)),
        "cancel_import" => {
            app.cancel_import();
            json(Ok(()))
        }
        "start_project_session" => json(app.start_session(arg(a, "project")?, clock)),
        "end_project_session" => json(app.end_session(arg(a, "project")?, clock)),
        "log_struggle" => json(app.log_struggle(arg(a, "project")?, arg(a, "input")?, clock)),
        "delete_struggle" => json(app.delete_struggle(arg(a, "id")?)),
        "struggles_to_review" => json(app.struggles_to_review(arg(a, "project")?)),
        "review_struggle" => json(app.review_struggle(arg(a, "id")?, arg(a, "choice")?, today)),
        "issues" => json(app.issues()),
        "known_drill" => json(app.known_drill(&arg::<String>(a, "issue")?)),
        "drill" => json(app.drill_view(arg(a, "id")?)),
        "drills_due" => json(app.drills_due(today)),
        "rate_drill" => json(app.rate_drill(arg(a, "id")?, arg(a, "rating")?, arg(a, "seconds")?, &arg::<String>(a, "note")?, today)),
        "move_drill" => json(app.move_drill(arg(a, "id")?, arg(a, "day")?, today)),
        "quiz_schedule" => json(app.quiz_schedule()),
        "set_quiz_schedule" => json(app.set_quiz_schedule(arg(a, "schedule")?)),
        "reset_quiz_schedule" => json(app.reset_quiz_schedule()),
        "topic_quizzes" => json(app.topic_quizzes(today)),
        "calendar_month" => json(app.calendar_month(arg(a, "from")?, arg(a, "to")?, today)),
        "calendar_day" => json(app.calendar_day(arg(a, "day")?, today)),
        "due_today" => json(app.due_today(today)),

        _ => Err(ApiError::new(format!("Command {command} not found"))),
    }
}
