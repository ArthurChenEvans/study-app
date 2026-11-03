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

        _ => Err(ApiError::new(format!("Command {command} not found"))),
    }
}
