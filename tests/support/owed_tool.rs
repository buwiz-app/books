use serde_json::{Value, json};

pub fn normalize(name: &str, mut args: Value) -> (&str, Value) {
    let action = match name {
        "books_app_owed_create" => "create",
        "books_app_owed_settle" => "settle",
        "books_app_owed_cancel" => "cancel",
        "books_app_owed_repeat" => {
            let repeat_action = args["action"].clone();
            args["repeat_action"] = repeat_action;
            "repeat"
        }
        _ => return (name, args),
    };
    args["action"] = json!(action);
    ("books_app_owed", args)
}
